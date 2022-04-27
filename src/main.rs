#![feature(negative_impls)]

use std::{
    marker::PhantomData,
    ops::Deref,
    ptr::NonNull,
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};

enum SephamoreError {
    AtMaxCount,
}

#[repr(C)]
struct SemaphoreInner<T: ?Sized> {
    count: AtomicUsize,
    max: usize,
    data: T,
}

impl<T: ?Sized> SemaphoreInner<T> {
    fn at_max(&self, ordering: Ordering) -> bool {
        self.count.load(ordering) >= self.max
    }
}

struct Semaphore<T: ?Sized> {
    inner: NonNull<SemaphoreInner<T>>,
    _phantom: PhantomData<SemaphoreInner<T>>,
}

impl<T: ?Sized> Semaphore<T> {
    pub fn get(&self) -> SephamoreGuard<T> {
        // TODO: Dont spin  loop
        let inner = unsafe { self.inner.as_ref() };
        while inner.at_max(Ordering::Relaxed) {
            std::thread::sleep(Duration::from_millis(50));
        }
        SephamoreGuard::new(self)
    }
    pub fn try_get(&self) -> Result<SephamoreGuard<T>, SephamoreError> {
        if unsafe { self.inner.as_ref() }.at_max(Ordering::Relaxed) {
            Err(SephamoreError::AtMaxCount)
        } else {
            Ok(SephamoreGuard::new(self))
        }
    }
}

impl<T> Semaphore<T> {
    pub fn new(value: T, max: usize) -> Self {
        let inner = Box::leak(Box::new(SemaphoreInner {
            max,
            count: AtomicUsize::new(0),
            data: value,
        }));
        Semaphore {
            inner: unsafe { NonNull::new_unchecked(inner as *mut _) }, // Creating a non null from a reference is always safe
            _phantom: PhantomData,
        }
    }
}

impl<T: ?Sized> Drop for Semaphore<T> {
    fn drop(&mut self) {
        unsafe { Box::from_raw(self.inner.as_ptr()) };
    }
}

unsafe impl<T: ?Sized + Send> Send for Semaphore<T> {}
unsafe impl<T: ?Sized + Send> Sync for Semaphore<T> {}

#[must_use = "if unused the guard will immediatly unlock"]
struct SephamoreGuard<'guard, T: ?Sized> {
    inner: NonNull<SemaphoreInner<T>>,
    _phantom: PhantomData<&'guard T>,
}

impl<'guard, T: ?Sized> SephamoreGuard<'guard, T> {
    fn new(sephamore: &Semaphore<T>) -> Self {
        unsafe { sephamore.inner.as_ref() }
            .count
            .fetch_add(1, Ordering::SeqCst);
        SephamoreGuard {
            inner: sephamore.inner,
            _phantom: PhantomData,
        }
    }
}

impl<'guard, T: ?Sized> Deref for SephamoreGuard<'guard, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &unsafe { self.inner.as_ref() }.data
    }
}

impl<'guard, T: ?Sized> !Send for SephamoreGuard<'guard, T> {}
unsafe impl<'guard, T: ?Sized + Sync> Sync for SephamoreGuard<'guard, T> {}

impl<'guard, T: ?Sized> Drop for SephamoreGuard<'guard, T> {
    fn drop(&mut self) {
        unsafe { self.inner.as_ref() }
            .count
            .fetch_sub(1, Ordering::Relaxed);
    }
}

fn main() {
    let sephamore = Semaphore::new("Hello World", 5);
    let sephamore = std::sync::Arc::new(sephamore);
    let mut threads = Vec::with_capacity(10);
    for i in 0..10 {
        let sephamore = sephamore.clone();
        threads.push(std::thread::spawn(move || {
            let value = sephamore.get();
            println!("[{i}]: {}", *value);
            std::thread::sleep(Duration::from_secs(i));
        }))
    }

    for thread in threads {
        thread.join().unwrap();
    }
}
