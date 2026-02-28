/// A globally initialized, read-only after initialization value.
///
///# Safety contract
///
/// - `init()` **must** be called exactly once before any `Deref` / `get()` call.
/// - After `init()`, the value must not be mutated.
/// - In debug builds, violations of these rules panic.
/// - In release builds, no checks are performed; you are fully responsible.
pub struct Global<T> {
    value: core::cell::UnsafeCell<core::mem::MaybeUninit<T>>,

    #[cfg(debug_assertions)]
    initialized: core::sync::atomic::AtomicBool,
}

// SAFETY:
// - T must be Sync to allow safe concurrent reads after init.
// - Mutation is forbidden after init, so concurrent immutable access is safe.
unsafe impl<T: Sync> Sync for Global<T> {}

impl<T> Global<T> {
    #[inline(always)]
    pub const fn new() -> Self {
        Self {
            value: core::cell::UnsafeCell::new(core::mem::MaybeUninit::uninit()),

            #[cfg(debug_assertions)]
            initialized: core::sync::atomic::AtomicBool::new(false),
        }
    }

    #[inline(always)]
    pub fn init(&self, value: T) {
        #[cfg(debug_assertions)]
        {
            if self
                .initialized
                .compare_exchange(
                    false,
                    true,
                    core::sync::atomic::Ordering::AcqRel,
                    core::sync::atomic::Ordering::Acquire,
                )
                .is_err()
            {
                panic!("Global initialized twice");
            }
        }

        // SAFETY:
        // - We write only once to the backing UnsafeCell<MaybeUninit<T>>.
        // - No references to the inner value exist yet, so no aliasing.
        unsafe {
            (*self.value.get()).write(value);
        }
    }

    #[inline(always)]
    pub fn get(&self) -> &T {
        #[cfg(debug_assertions)]
        {
            if !self.initialized.load(core::sync::atomic::Ordering::Acquire) {
                panic!("Global accessed before initialization");
            }
        }

        // SAFETY:
        // - Ensure that `init()` has been called before returning this reference.
        // - After initialization, the value is never mutated again.
        // - Lifetime is 'static because the backing storage is static/global.
        unsafe { &*(*self.value.get()).as_ptr() }
    }
}

impl<T> core::ops::Deref for Global<T> {
    type Target = T;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        self.get()
    }
}

impl<T: core::fmt::Debug> core::fmt::Debug for Global<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        #[cfg(debug_assertions)]
        let initialized = self.initialized.load(core::sync::atomic::Ordering::Acquire);
        #[cfg(not(debug_assertions))]
        let initialized = true;

        write!(
            f,
            "Global({})",
            if initialized {
                let val = unsafe { &*(*self.value.get()).as_ptr() };
                format!("{:?}", val)
            } else {
                "<uninitialized>".to_string()
            }
        )
    }
}
