# semaphorus

This crate adds syncronous semaphores to rust.

This is different from like the [`semaphore`](https://github.com/srijs/rust-semaphore) crate because
1. `semaphorus` supports `#![no_std]`
2. `semaphorus` doesn't use `Arc` under the hood and behaves more like `RwLock<T>`, this does require the semaphores to be in an `Arc` for multithreading as they don't implement clone