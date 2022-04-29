use core::{
    marker::PhantomData,
    sync::atomic::{AtomicUsize, Ordering},
};

#[cfg(not(feature = "nightly"))]
#[doc(hidden)]
type PhantomUnsend = core::marker::PhantomData<*mut ()>; // Pointers are never send

/// A counter that has a maximum value
pub struct Semaphore {
    count: AtomicUsize,
    pub max: usize,
}

/// A guard for a Semaphore
/// Increments the count on creation
/// Decrements it on Drop
#[must_use]
pub struct SemaphoreGuard<'guard> {
    semaphore: &'guard Semaphore,
    #[cfg(not(feature = "nightly"))]
    _unsend: PhantomUnsend,
}

impl<'guard> Drop for SemaphoreGuard<'guard> {
    fn drop(&mut self) {
        self.semaphore.count.fetch_sub(1, Ordering::SeqCst);
    }
}

impl<'guard> SemaphoreGuard<'guard> {
    fn new(semaphore: &'guard Semaphore) -> Self {
        semaphore.count.fetch_add(1, Ordering::SeqCst);
        SemaphoreGuard {
            semaphore,
            #[cfg(not(feature = "nightly"))]
            _unsend: PhantomData,
        }
    }
}

#[cfg(any(feature = "nightly", doc))]
impl<'guard> !Send for SemaphoreGuard<'guard> {}

unsafe impl<'guard> Sync for SemaphoreGuard<'guard> {}

impl Semaphore {
    #[must_use]
    pub fn count(&self, ordering: Ordering) -> usize {
        self.count.load(ordering)
    }

    #[must_use]
    pub fn new(max: usize) -> Self {
        Semaphore {
            max,
            count: AtomicUsize::new(0),
        }
    }

    #[must_use]
    pub fn at_max(&self, ordering: Ordering) -> bool {
        self.count.load(ordering) >= self.max
    }

    /// Try to increment the count and return a Guard
    ///
    /// Never blocks
    /// # Errors
    /// Will error if the count is at max already

    pub fn try_lock(&self) -> Result<SemaphoreGuard, crate::SemaphoreError> {
        if self.at_max(Ordering::SeqCst) {
            Err(crate::SemaphoreError::AtMaxCount)
        } else {
            Ok(SemaphoreGuard::new(self))
        }
    }
}
