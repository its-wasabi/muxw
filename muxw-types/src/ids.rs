// TODO: Try to finding some way of sourcing ids without wasting so much memory for free list and
// keeping the O(1) acquire and release

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Id(u64);

#[derive(Debug, Default)]
pub struct IdSource {
    next: u64,
    free: Vec<u64>,
}

impl IdSource {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            next: 0,
            free: Vec::new(),
        }
    }

    pub fn acquire(&mut self) -> Id {
        Id(self.free.pop().unwrap_or_else(|| {
            let next = self.next;
            self.next += 1;
            next
        }))
    }

    #[allow(clippy::needless_pass_by_value)]
    pub fn release(&mut self, id: Id) {
        self.free.push(id.0);
    }
}

impl mlua::UserData for Id {}
