mod epoll;
pub use epoll::Event;
pub use epoll::Events;
pub use epoll::Interest;
pub use epoll::Result;

pub mod fds;
pub mod sources;

pub mod event_loop;
