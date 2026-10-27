mod emitter;
pub use emitter::Emitter;
mod io;
pub use io::Handle as IoHandle;
pub use io::Interest;
mod sender;
pub use sender::LoopClosed;
pub use sender::Sender;
mod timer;
pub use timer::Handle as TimerHandle;
pub use timer::TimerMode;

pub struct EventLoop<T: Clone = crate::token::Token> {
    poller: std::sync::Arc<polling::Poller>,
    events: polling::Events,

    sources: io::Sources<T>,
    timers: timer::Timers<T>,
    emitted: std::rc::Rc<std::cell::RefCell<Vec<T>>>,
    mailbox: sender::Mailbox<T>,
}

impl<T: Clone> EventLoop<T> {
    pub fn new() -> std::io::Result<Self> {
        let poller = std::sync::Arc::new(polling::Poller::new()?);
        let events = polling::Events::new();
        let sources = io::Sources::new();
        let timers = timer::Timers::new();
        let emitted = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let mailbox = sender::Mailbox::new();

        Ok(Self {
            poller,
            events,

            sources,
            timers,
            emitted,
            mailbox,
        })
    }

    pub fn dispatch(&mut self, triggered: &mut Vec<T>) -> Result<usize, std::io::Error> {
        triggered.clear();
        self.drain_ready_emitts_into(triggered);
        self.drain_ready_recivied_into(triggered);

        self.events.clear();

        let timeout = if triggered.is_empty() {
            self.timers.timeout()
        } else {
            Some(std::time::Duration::ZERO)
        };

        self.poller.wait(&mut self.events, timeout)?;

        self.drain_ready_timers_into(triggered);
        self.drain_ready_io_into(triggered);

        Ok(triggered.len())
    }
}
