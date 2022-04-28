# semaphorus

This crate adds syncronous semaphores to rust.

This is different from the `semaphore` crate because
1. It supports `#![no_std]`
2. It doesn't use `Arc` under the hood and behaves more like `RwLock<T>`, this does require it to be in an `Arc` for multithreading