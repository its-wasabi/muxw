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

macro_rules! define_id {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub struct $name(u64);

        impl Id for $name {
            #[inline]
            fn new(id: u64) -> Self {
                Self(id)
            }

            #[inline]
            fn inner(&self) -> u64 {
                self.0
            }
        }
    };
}

define_id!(DeviceId);
define_id!(OutputId);
define_id!(WindowId);
define_id!(EventId);
