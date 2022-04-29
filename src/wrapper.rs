use crate::{raw, SemaphoreError};
use core::{
    ops::Deref,
    sync::atomic::{AtomicUsize, Ordering},
};

/// Allows up to `max` references to the data in the Semaphore
///
/// This behaves like [`RwLock<T>`][`std::sync::RwLock`] with some key differences
/// 1. You can't get a `&mut T`, only a `&T`
/// 2. You can have up to a maximum number of references at once
pub struct Semaphore<T: ?Sized> {
    raw: raw::Semaphore,
    data: T,
}

impl<T: ?Sized> Semaphore<T> {
    /// Returns true if the current cound is >= the maximum count
    #[must_use]
    pub fn at_max(&self, ordering: Ordering) -> bool {
        self.raw.at_max(ordering)
    }

    /// Get the current number of references to the data
    #[must_use]
    pub fn count(&self, ordering: Ordering) -> usize {
        self.raw.count(ordering)
    }

    /// This function can be inefficient, as it uses [`std::thread::sleep`] on `std` and [`core::hint::spin_loop`] on `no_std`.
    /// # Panics
    /// This function will panic if `max` == 0 because that will cause an infinite loop
    pub fn get(&self) -> SemaphoreGuard<T> {
        assert_ne!(
            self.raw.max, 0,
            "Calling 'Semaphore::get' on a semaphore with a max of 0 will loop forever!"
        );
        while self.at_max(Ordering::Relaxed) {
            #[cfg(feature = "std")]
            std::thread::sleep(std::time::Duration::from_millis(50));
            #[cfg(not(feature = "std"))]
            core::hint::spin_loop();
        }
        self.try_get().unwrap()
    }

    /// Attempt to get the value in the semaphore.
    ///
    /// This function will never block
    /// # Errors
    /// This function will return [`SemaphoreError::AtMax`] if the current count is >= the maximum count
    #[inline]
    pub fn try_get(&self) -> Result<SemaphoreGuard<T>, SemaphoreError> {
        Ok(SemaphoreGuard::new(self.raw.try_lock()?, &self.data))
    }

    /// Get a mutable reference to the data in the semaphore
    #[inline]
    pub fn get_mut(&mut self) -> &mut T {
        &mut self.data
    }
}

impl<T> Semaphore<T> {
    /// Create a new semaphore with 0 counted references
    pub fn new(value: T, max: usize) -> Self {
        debug_assert_ne!(
            max, 0,
            "A semaphore with a maximum count of '0' generally useless"
        );

        Semaphore {
            raw: raw::Semaphore::new(max),
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
/// For mutable access, consider using a [cell][`std::cell`] type or use [`Semaphore::get_mut`]
#[must_use = "if unused, the guard will immediatly unlock"]
pub struct SemaphoreGuard<'guard, T: ?Sized> {
    _inner: raw::SemaphoreGuard<'guard>,
    data: &'guard T,
}

impl<'guard, T: ?Sized> SemaphoreGuard<'guard, T> {
    /// Create a guard around a `Semaphore`, and increment the reference count
    fn new(raw_guard: raw::SemaphoreGuard<'guard>, data: &'guard T) -> Self {
        SemaphoreGuard {
            _inner: raw_guard,
            data,
        }
    }
}

impl<'guard, T: ?Sized> Deref for SemaphoreGuard<'guard, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.data
    }
}
unsafe impl<'guard, T: ?Sized + Sync> Sync for SemaphoreGuard<'guard, T> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_maximum_count_works() {
        let semaphore = Semaphore::new((), 4);

        let (g1, g2, g3, g4) = (
            semaphore.try_get(),
            semaphore.try_get(),
            semaphore.try_get(),
            semaphore.try_get(),
        );

        assert_eq!(
            (g1.is_ok(), g2.is_ok(), g3.is_ok(), g4.is_ok()),
            (true, true, true, true)
        );

        let g5 = semaphore.try_get();

        assert!(g5.is_err());

        drop(g1);

        let g6 = semaphore.try_get();

        assert!(g6.is_ok());
    }
}
