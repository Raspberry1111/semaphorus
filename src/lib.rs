#![cfg_attr(feature = "nightly", feature(negative_impls))]

//! `semaphorus` add a `Semaphore` type that behaves like a mutex with some key differences
//! 1. You can't get a &mut T, only a &T
//! 2. You can have up to a maximum number of references at once
//!`

#[cfg(not(feature = "nightly"))]
#[doc(hidden)]
type PhantomUnsend = core::marker::PhantomData<*mut ()>; // Pointers are never send

use core::{
    ops::Deref,
    sync::atomic::{AtomicUsize, Ordering},
};
#[derive(Clone, Debug)]
pub enum SemaphoreError {
    /// The semaphore was already at the maximum amount of references
    AtMax,
}

impl core::fmt::Display for SemaphoreError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        use SemaphoreError::*;
        match self {
            AtMax => write!(f, "Already at maximum count!"),
            _ => Ok(()),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for SemaphoreError {}

/// Allows up to `max` references to the data in the Semaphore
pub struct Semaphore<T: ?Sized> {
    count: AtomicUsize,
    pub max: usize,
    data: T,
}

impl<T: ?Sized> Semaphore<T> {
    /// Returns true if the current cound is >= the maximum count
    pub fn at_max(&self, ordering: Ordering) -> bool {
        self.count.load(ordering) >= self.max
    }

    /// Get the current number of references to the data
    pub fn count(&self, ordering: Ordering) -> usize {
        self.count.load(ordering)
    }

    /// WARNING: This function uses std::thread:sleep for 50 mils between checks
    /// This can be inefficient, and it may be more useful to use try_get in your own loop
    pub fn get(&self) -> SemaphoreGuard<T> {
        assert_ne!(
            self.max, 0,
            "Calling 'Semaphore::get' on a semaphore with a max of 0 will loop forever!"
        );
        while self.at_max(Ordering::Relaxed) {
            #[cfg(feature = "std")]
            std::thread::sleep(std::time::Duration::from_millis(50));
            #[cfg(not(feature = "std"))]
            core::hint::spin_loop();
        }
        SemaphoreGuard::new(self)
    }

    /// Attempt to get the value in the semaphore, returns `SemaphoreError::AtMax` if the maximum number of references already exist
    pub fn try_get(&self) -> Result<SemaphoreGuard<T>, SemaphoreError> {
        if self.at_max(Ordering::Relaxed) {
            Err(SemaphoreError::AtMax)
        } else {
            Ok(SemaphoreGuard::new(self))
        }
    }

    /// Get a mutable reference to the data in the semaphore
    pub fn get_mut(&mut self) -> &mut T {
        &mut self.data
    }
}

impl<T> Semaphore<T> {
    /// Create a new semaphore with 0 references
    pub fn new(value: T, max: usize) -> Self {
        debug_assert_ne!(
            max, 0,
            "A semaphore with a maximum count of '0' generally useless"
        );

        Semaphore {
            max,
            count: AtomicUsize::new(0),
            data: value,
        }
    }

    /// Move the value out of the semaphore
    pub fn into_inner(self) -> T {
        self.data
    }
}

unsafe impl<T: ?Sized + Send> Send for Semaphore<T> {}
unsafe impl<T: ?Sized + Send> Sync for Semaphore<T> {}

/// A wrapper around a reference to the data in the semaphore
/// Automatically decrements the reference count when it is dropped
/// For mutable access, consider using a cell type or `Semaphore::get_mut`
#[must_use = "if unused the guard will immediatly unlock"]
pub struct SemaphoreGuard<'guard, T: ?Sized> {
    inner: &'guard Semaphore<T>,
    #[cfg(not(feature = "nightly"))]
    _unsend: PhantomUnsend,
}

impl<'guard, T: ?Sized> SemaphoreGuard<'guard, T> {
    /// Create a guard around a `Semaphore`, and increment the reference count
    pub fn new(semaphore: &'guard Semaphore<T>) -> Self {
        semaphore.count.fetch_add(1, Ordering::SeqCst);
        SemaphoreGuard {
            inner: semaphore,
            #[cfg(not(feature = "nightly"))]
            _unsend: core::marker::PhantomData,
        }
    }
}

impl<'guard, T: ?Sized> Deref for SemaphoreGuard<'guard, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner.data
    }
}

#[cfg(feature = "nightly")]
impl<'guard, T: ?Sized> !Send for SemaphoreGuard<'guard, T> {}

unsafe impl<'guard, T: ?Sized + Sync> Sync for SemaphoreGuard<'guard, T> {}

impl<'guard, T: ?Sized> Drop for SemaphoreGuard<'guard, T> {
    fn drop(&mut self) {
        self.inner.count.fetch_sub(1, Ordering::Relaxed);
    }
}
