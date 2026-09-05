use input::event::{EventTrait, keyboard::KeyboardEventTrait, pointer::PointerScrollEvent};
use std::os::fd::{AsFd, AsRawFd};

pub struct InputWorker {
    event_loop: crate::event_loop::EventLoop<InputToken>,
    libinput: input::Libinput,
    sender: crate::event_loop::EventSender<crate::token::Token>,

    devices: slab::Slab<input::Device>,
    device_keys: std::collections::HashMap<input::Device, super::InputDeviceKey>,
}

impl InputWorker {
    pub fn new(
        sender: crate::event_loop::EventSender<crate::token::Token>,
    ) -> Result<(crate::event_loop::EventSender<InputToken>, Self), Box<dyn std::error::Error>>
    {
        let mut inner_event_loop = crate::event_loop::EventLoop::new()?;
        let libinput = input::Libinput::new_with_udev(LibinputInterface {
            sender: sender.clone(),
        });

        inner_event_loop.register_source(
            &libinput.as_fd(),
            polling::PollMode::Edge,
            InputToken::Libinput,
        )?;

        Ok((
            inner_event_loop.sender(),
            Self {
                event_loop: inner_event_loop,
                libinput,
                sender,
                devices: slab::Slab::with_capacity(1),
                device_keys: std::collections::HashMap::with_capacity(1),
            },
        ))
    }
}

// SAFETY: InputWorker is fully constructed on the calling thread and then moved
// in its entirety into the worker thread's closure. After the move, no other
// thread ever touches it, and none of them are
// accessed concurrently from two threads. Libinput's C objects have no
// documented same-thread-only requirement, they just aren't safe to use from
// multiple threads at once, which this usage never does.
#[allow(clippy::non_send_fields_in_send_ty)]
unsafe impl Send for InputWorker {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum InputToken {
    Libinput,
}

pub struct LibinputInterface {
    sender: crate::event_loop::EventSender<crate::token::Token>,
}

impl input::LibinputInterface for LibinputInterface {
    fn open_restricted(
        &mut self,
        path: &std::path::Path,
        _flags: i32,
    ) -> std::result::Result<std::os::fd::OwnedFd, i32> {
        let (reply, response) = crossbeam_channel::bounded(1);

        self.sender.send(crate::token::Token::Seat(
            crate::backend::seat::SeatEvent::OpenRequest(Box::new(
                crate::backend::seat::SeatOpenData {
                    path: path.to_path_buf(),
                    reply,
                },
            )),
        ));

        match response.recv() {
            Ok(Ok(fd)) => Ok(fd),
            Ok(Err(io_error)) => Err(io_error.raw_os_error().unwrap_or(13)),
            Err(_) => Err(13),
        }
    }

    fn close_restricted(&mut self, fd: std::os::fd::OwnedFd) {
        let raw_fd = fd.as_raw_fd();
        std::mem::drop(fd);
        self.sender.send(crate::token::Token::Seat(
            crate::backend::seat::SeatEvent::CloseRequest(raw_fd),
        ));
    }
}

fn drain_libinput_events(worker: &mut InputWorker) {
    let InputWorker {
        libinput,
        devices,
        device_keys,
        sender,
        ..
    } = worker;

    if let Err(error) = libinput.dispatch() {
        tracing::error!(?error, "Input worker failed to dispatch libinput");
        return;
    }

    for event in &mut *libinput {}
}
