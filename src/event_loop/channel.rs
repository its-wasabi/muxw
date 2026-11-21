#[derive(Debug)]
pub struct LoopClosed;

impl std::fmt::Display for LoopClosed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("the event loop has shut down")
    }
}

impl std::error::Error for LoopClosed {}

pub struct Sender<T> {
    sender: crossbeam_channel::Sender<T>,
    poller: std::sync::Arc<polling::Poller>,
}

impl<T> Sender<T> {
    pub fn send(&self, token: impl Into<T>) -> Result<(), LoopClosed> {
        self.sender.send(token.into()).map_err(|_| LoopClosed)?;
        self.poller.notify();
        Ok(())
    }
}

pub(super) struct Mailbox<T> {
    sender: crossbeam_channel::Sender<T>,
    receiver: crossbeam_channel::Receiver<T>,
}

impl<T> Mailbox<T> {
    pub(super) fn new() -> Self {
        let (sender, receiver) = crossbeam_channel::unbounded();
        Self { sender, receiver }
    }

    fn sender(&self) -> crossbeam_channel::Sender<T> {
        self.sender.clone()
    }

    fn recv_into(&self, triggered: &mut Vec<T>) {
        triggered.reserve(self.receiver.len());
        triggered.extend(self.receiver.try_iter());
    }
}

impl<T: Clone> super::EventLoop<T> {
    pub fn sender(&self) -> Sender<T> {
        Sender {
            sender: self.mailbox.sender(),
            poller: self.poller.clone(),
        }
    }

    pub(super) fn drain_ready_recivied_into(&mut self, triggered: &mut Vec<T>) {
        self.mailbox.recv_into(triggered);
    }
}
