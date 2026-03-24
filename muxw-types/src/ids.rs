pub trait Id {
    fn new(id: u64) -> Self;
    fn inner(&self) -> u64;
}

#[derive(Debug, Default)]
pub struct IdSource<ID: Id> {
    next: u64,
    free: Vec<u64>,
    id_type: std::marker::PhantomData<ID>,
}

impl<ID: Id> IdSource<ID> {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            next: 0,
            free: Vec::new(),
            id_type: std::marker::PhantomData,
        }
    }

    pub fn acquire(&mut self) -> ID {
        let raw = self.free.pop().unwrap_or_else(|| {
            let next = self.next;
            self.next += 1;
            next
        });

        ID::new(raw)
    }

    #[allow(clippy::needless_pass_by_value)]
    pub fn release(&mut self, id: ID) {
        self.free.push(id.inner());
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DeviceId(u64);
impl Id for DeviceId {
    fn new(id: u64) -> Self {
        Self(id)
    }

    fn inner(&self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WindowId(u64);
impl Id for WindowId {
    fn new(id: u64) -> Self {
        Self(id)
    }

    fn inner(&self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OutputId(u64);
impl Id for OutputId {
    fn new(id: u64) -> Self {
        Self(id)
    }

    fn inner(&self) -> u64 {
        self.0
    }
}
