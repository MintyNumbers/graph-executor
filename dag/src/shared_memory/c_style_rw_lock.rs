// use std::cell::UnsafeCell;
// use std::ops::{Deref, DerefMut};
// use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU8, Ordering};

// use atomic_wait::{wait, wake_all, wake_one};

// // https://marabos.nl/atomics/building-locks.html#reader-writer-lock

// #[derive(Debug, serde::Serialize, serde::Deserialize)]
// pub struct CStyleRwLock {
//     state: AtomicU32,
// }

// impl CStyleRwLock {
//     pub const fn new() -> Self {
//         Self {
//             state: AtomicU32::new(0), // u32::MAX indicates write lock
//         }
//     }

//     // Shared reference
//     pub fn read_lock(&self) -> ReadGuard {
//         let mut state = self.state.load(Ordering::Relaxed);
//         loop {
//             // Create reader
//             if state < u32::MAX {
//                 assert!(state < u32::MAX - 1, "too many readers");
//                 match self.state.compare_exchange_weak(state, state + 1, Ordering::Acquire, Ordering::Relaxed) {
//                     Ok(_) => return ReadGuard { rwlock: self },
//                     Err(e) => state = e,
//                 }
//             }
//             // Wait for writer release
//             if state == u32::MAX {
//                 wait(&self.state, u32::MAX);
//                 state = self.state.load(Ordering::Relaxed);
//             }
//         }
//     }

//     // Exclusive reference
//     pub fn write_lock(&self) -> WriteGuard {
//         while let Err(state) = self.state.compare_exchange(0, u32::MAX, Ordering::Acquire, Ordering::Relaxed) {
//             // Wait while already locked.
//             wait(&self.state, state);
//         }
//         WriteGuard { rwlock: self }
//     }
// }

// // Read Guard for RwSpinLock
// pub struct ReadGuard<'a> {
//     rwlock: &'a CStyleRwLock,
// }

// impl Deref for ReadGuard<'_> {
//     type Target = CStyleRwLock;
//     fn deref(&self) -> &CStyleRwLock {
//         self.rwlock
//     }
// }

// impl Drop for ReadGuard<'_> {
//     fn drop(&mut self) {
//         if self.rwlock.state.fetch_sub(1, Ordering::Release) == 1 {
//             // Wake up a waiting writer, if any.
//             wake_one(&self.rwlock.state);
//         }
//     }
// }

// // Write Guard for RwSpinLock
// pub struct WriteGuard<'a> {
//     rwlock: &'a CStyleRwLock,
// }

// impl Deref for WriteGuard<'_> {
//     type Target = CStyleRwLock;
//     fn deref(&self) -> &CStyleRwLock {
//         self.rwlock
//     }
// }

// impl Drop for WriteGuard<'_> {
//     fn drop(&mut self) {
//         self.rwlock.state.store(0, Ordering::Release);
//         // Wake up all waiting readers and writers.
//         wake_all(&self.rwlock.state);
//     }
// }
