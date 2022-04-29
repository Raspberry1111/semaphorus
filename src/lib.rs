#![cfg_attr(any(feature = "nightly", doc), feature(negative_impls))]
#![deny(clippy::all)]
#![warn(clippy::pedantic)]

//! `semaphorus` add a [`Semaphore`] type that behaves like a `RwLock`

pub mod raw;

#[cfg(feature = "wrapper")]
pub mod wrapper;

#[cfg(feature = "wrapper")]
pub use wrapper::*;

#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum SemaphoreError {
    /// The semaphore was already at the maximum amount of references
    AtMaxCount,
}

impl core::fmt::Display for SemaphoreError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            SemaphoreError::AtMaxCount => write!(f, "Already at maximum count!"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for SemaphoreError {}
