#![allow(clippy::unwrap_used, clippy::expect_used)]
#![allow(unused)]

#[path = "../src/event_loop/mod.rs"]
#[allow(dead_code, unused_imports)]
mod event_loop;

use std::hint::black_box;
use std::os::fd::OwnedFd;
use std::time::{Duration, Instant};

use criterion::{BatchSize, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use event_loop::{EventLoop, Interest, TimerMode};

#[derive(Clone, Copy)]
pub struct Token([u64; 6]);

const fn token(i: usize) -> Token {
    Token([i as u64; 6])
}

const FAR: Duration = Duration::from_secs(3600);

fn fd_budget() -> usize {
    use rustix::process::{Resource, getrlimit};
    getrlimit(Resource::Nofile)
        .current
        .map_or(usize::MAX, |c| c as usize)
        .saturating_sub(128)
}

fn eventfds(n: usize) -> Vec<OwnedFd> {
    (0..n)
        .map(|_| rustix::event::eventfd(0, rustix::event::EventfdFlags::NONBLOCK).expect("eventfd"))
        .collect()
}

fn kick(fds: &[OwnedFd]) {
    for fd in fds {
        rustix::io::write(fd, &1u64.to_ne_bytes()).expect("write eventfd");
    }
}

fn config() -> Criterion {
    Criterion::default()
        .warm_up_time(Duration::from_secs(1))
        .measurement_time(Duration::from_secs(3))
}

fn floor(c: &mut Criterion) {
    let poller = polling::Poller::new().unwrap();
    let mut events = polling::Events::new();
    c.bench_function("floor/epoll_wait_zero", |b| {
        b.iter(|| {
            events.clear();
            poller.wait(&mut events, Some(Duration::ZERO)).unwrap();
            black_box(events.len());
        });
    });
}

fn emit(c: &mut Criterion) {
    let mut group = c.benchmark_group("emit");
    for n in [1usize, 16, 256, 4096] {
        group.throughput(Throughput::Elements(n as u64));
        let mut el = EventLoop::<Token>::new().unwrap();
        let emitter = el.emitter();
        let mut out = Vec::new();
        group.bench_function(BenchmarkId::from_parameter(n), |b| {
            b.iter(|| {
                for i in 0..n {
                    emitter.emit(token(i));
                }
                el.dispatch(&mut out).unwrap();
                black_box(&out);
            });
        });
    }
    group.finish();
}

fn sender(c: &mut Criterion) {
    let mut group = c.benchmark_group("sender");
    for n in [1usize, 16, 256] {
        group.throughput(Throughput::Elements(n as u64));
        let mut el = EventLoop::<Token>::new().unwrap();
        let tx = el.sender();
        let mut out = Vec::new();
        group.bench_function(BenchmarkId::from_parameter(n), |b| {
            b.iter(|| {
                for i in 0..n {
                    tx.send(token(i)).unwrap();
                }
                el.dispatch(&mut out).unwrap();
                black_box(&out);
            });
        });
    }
    group.finish();
}

fn io(c: &mut Criterion) {
    let mut group = c.benchmark_group("io");
    for (registered, ready) in [(1usize, 1usize), (8, 8), (64, 64), (512, 512), (4096, 8)] {
        if registered > fd_budget() {
            eprintln!(
                "skipping io/{ready}_of_{registered}: needs more fds than the limit allows (raise `ulimit -n`)"
            );
            continue;
        }

        group.throughput(Throughput::Elements(ready as u64));
        let fds = eventfds(registered);
        let mut el = EventLoop::<Token>::new().unwrap();
        let handles: Vec<_> = fds
            .iter()
            .enumerate()
            .map(|(i, fd)| el.register_fd(fd, Interest::Readable, token(i)).unwrap())
            .collect();
        let mut out = Vec::new();

        group.bench_function(
            BenchmarkId::new("ready", format!("{ready}_of_{registered}")),
            |b| {
                b.iter_batched(
                    || kick(&fds[..ready]),
                    |()| {
                        el.dispatch(&mut out).unwrap();
                        black_box(&out);
                    },
                    BatchSize::PerIteration,
                );
            },
        );

        drop(handles);
    }
    group.finish();
}

fn timers(c: &mut Criterion) {
    let mut group = c.benchmark_group("timers");
    for n in [1usize, 64, 1024] {
        group.throughput(Throughput::Elements(n as u64));

        let mut el = EventLoop::<Token>::new().unwrap();
        let mut out = Vec::new();
        group.bench_function(BenchmarkId::new("fire_oneshot", n), |b| {
            b.iter_custom(|iters| {
                let mut total = Duration::ZERO;
                for _ in 0..iters {
                    let at = Instant::now();
                    let handles: Vec<_> = (0..n)
                        .map(|i| el.register_timer(TimerMode::Exact(at), token(i)))
                        .collect();

                    let start = Instant::now();
                    el.dispatch(&mut out).unwrap();
                    total += start.elapsed();

                    black_box(&out);
                    drop(handles);
                }
                total
            });
        });
        drop(el);

        let mut el = EventLoop::<Token>::new().unwrap();
        let handles: Vec<_> = (0..n)
            .map(|i| el.register_timer(TimerMode::Rapid(Duration::from_nanos(1)), token(i)))
            .collect();
        group.bench_function(BenchmarkId::new("fire_rapid", n), |b| {
            b.iter(|| {
                el.dispatch(&mut out).unwrap();
                black_box(&out);
            });
        });
        drop(handles);
        drop(el);

        group.bench_function(BenchmarkId::new("register_cancel", n), |b| {
            b.iter_batched(
                || EventLoop::<Token>::new().unwrap(),
                |mut el| {
                    let handles: Vec<_> = (0..n)
                        .map(|i| el.register_timer(TimerMode::Delay(FAR), token(i)))
                        .collect();
                    drop(handles);
                    el
                },
                BatchSize::PerIteration,
            );
        });
    }
    group.finish();
}

fn rearm(c: &mut Criterion) {
    let mut group = c.benchmark_group("timers_rearm");
    group.sample_size(10);
    for ops in [1_000usize, 100_000] {
        group.throughput(Throughput::Elements(ops as u64));
        group.bench_function(BenchmarkId::from_parameter(ops), |b| {
            b.iter_batched(
                || {
                    let mut el = EventLoop::<Token>::new().unwrap();
                    let live: Vec<_> = (0..1000)
                        .map(|i| Some(el.register_timer(TimerMode::Delay(FAR), token(i))))
                        .collect();
                    (el, live)
                },
                |(mut el, mut live)| {
                    for i in 0..ops {
                        let slot = i % live.len();
                        live[slot] = None; // drop cancels
                        live[slot] = Some(el.register_timer(TimerMode::Delay(FAR), token(i)));
                    }
                    (el, live)
                },
                BatchSize::PerIteration,
            );
        });
    }
    group.finish();
}

fn mixed(c: &mut Criterion) {
    let fds = eventfds(32);
    let mut el = EventLoop::<Token>::new().unwrap();
    let handles: Vec<_> = fds
        .iter()
        .enumerate()
        .map(|(i, fd)| el.register_fd(fd, Interest::Readable, token(i)).unwrap())
        .collect();
    let emitter = el.emitter();
    let mut out = Vec::new();
    c.bench_function("mixed/32_io_16_emit", |b| {
        b.iter_batched(
            || {
                kick(&fds);
                for i in 0..16 {
                    emitter.emit(token(i));
                }
            },
            |()| {
                el.dispatch(&mut out).unwrap();
                black_box(&out);
            },
            BatchSize::PerIteration,
        );
    });
    drop(handles);
}

criterion_group! {
    name = benches;
    config = config();
    targets = floor, emit, sender, io, timers, rearm, mixed
}
criterion_main!(benches);
