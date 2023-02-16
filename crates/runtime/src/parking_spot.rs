//! Implements thread wait and notify primitives with `std::sync` primitives.
//!
//! This is a simplified version of the `parking_lot_core` crate.
//!
//! There are two main operations that can be performed:
//!
//! - *Parking* refers to suspending the thread while simultaneously enqueuing it
//! on a queue keyed by some address.
//! - *Unparking* refers to dequeuing a thread from a queue keyed by some address
//! and resuming it.

#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![deny(missing_docs)]
#![deny(unsafe_code)]

use crate::WaitResult;
use std::collections::BTreeMap;
use std::sync::{Arc, Condvar, Mutex};
use std::time::Instant;

#[derive(Default, Debug)]
struct Spot {
    /// The number of threads parked on this spot.
    num_parked: u32,

    /// The number of threads that have been unparked but not yet woken up.
    /// This is used to avoid spurious wakeups.
    to_unpark: u32,

    /// The [`Condvar`] used to notify parked threads.
    cvar: Arc<Condvar>,
}

/// The thread global `ParkingSpot`.
#[derive(Default, Debug)]
pub struct ParkingSpot {
    inner: Mutex<BTreeMap<u64, Spot>>,
}

impl ParkingSpot {
    /// Park the current thread until it is unparked or a timeout is reached.
    ///
    /// The `key` is used to identify the parking spot. If another thread calls
    /// `unpark_all` or `unpark` with the same key, the current thread will be unparked.
    ///
    /// The `validate` callback is called before parking.
    /// If it returns `false`, the thread is not parked and `WaitResult::Mismatch` is returned.
    ///
    /// The `timeout` argument specifies the maximum amount of time the thread will be parked.
    pub fn park(
        &self,
        key: u64,
        validate: impl FnOnce() -> bool,
        timeout: impl Into<Option<Instant>>,
    ) -> WaitResult {
        self.park_inner(key, validate, timeout.into())
    }

    fn park_inner(
        &self,
        key: u64,
        validate: impl FnOnce() -> bool,
        timeout: Option<Instant>,
    ) -> WaitResult {
        let mut inner = self
            .inner
            .lock()
            .expect("failed to lock inner parking table");

        // check validation with lock held
        if !validate() {
            return WaitResult::Mismatch;
        }

        // clone the condvar, so we can move the lock
        let cvar = {
            let spot = inner.entry(key).or_insert_with(Spot::default);
            spot.num_parked = spot
                .num_parked
                .checked_add(1)
                .expect("parking spot number overflow");
            spot.cvar.clone()
        };

        loop {
            let timed_out = if let Some(timeout) = timeout {
                let now = Instant::now();
                if now >= timeout {
                    true
                } else {
                    let dur = timeout - now;
                    let (lock, result) = cvar
                        .wait_timeout(inner, dur)
                        .expect("failed to wait for condition");
                    inner = lock;
                    result.timed_out()
                }
            } else {
                inner = cvar.wait(inner).expect("failed to wait for condition");
                false
            };

            let spot = inner.get_mut(&key).expect("failed to get spot");

            if timed_out {
                if let Some(timeout) = timeout {
                    if Instant::now() < timeout {
                        // Did not sleep long enough, try again.
                        continue;
                    }
                }
            } else {
                if spot.to_unpark == 0 {
                    continue;
                }

                spot.to_unpark -= 1;
            }

            spot.num_parked = spot
                .num_parked
                .checked_sub(1)
                .expect("corrupted parking spot state");

            if spot.num_parked == 0 {
                assert_eq!(spot.to_unpark, 0);
                inner
                    .remove(&key)
                    .expect("failed to remove spot from inner parking table");
            }

            if timed_out {
                return WaitResult::TimedOut;
            }

            return WaitResult::Ok;
        }
    }

    /// Unpark at most `n` threads that are parked with the given key.
    ///
    /// Returns the number of threads that were actually unparked.
    pub fn unpark(&self, key: u64, n: u32) -> u32 {
        if n == 0 {
            return 0;
        }
        let mut num_unpark = 0;

        self.with_lot(key, |spot| {
            num_unpark = n.min(spot.num_parked - spot.to_unpark);
            spot.to_unpark += num_unpark;
            if n >= num_unpark {
                spot.cvar.notify_all();
            } else {
                for _ in 0..num_unpark {
                    spot.cvar.notify_one();
                }
            }
        });

        num_unpark
    }

    fn with_lot<F: FnMut(&mut Spot)>(&self, key: u64, mut f: F) {
        let mut inner = self
            .inner
            .lock()
            .expect("failed to lock inner parking table");
        if let Some(spot) = inner.get_mut(&key) {
            f(spot);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ParkingSpot;
    use once_cell::sync::Lazy;
    use std::ptr::addr_of;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::thread;

    static PARKING_SPOT: Lazy<ParkingSpot> = Lazy::new(ParkingSpot::default);

    static ATOMIC: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn atomic_wait_notify() {
        let thread1 = thread::spawn(|| {
            let atomic_key = addr_of!(ATOMIC) as u64;
            ATOMIC.store(1, Ordering::SeqCst);
            PARKING_SPOT.unpark(atomic_key, u32::MAX);
            PARKING_SPOT.park(atomic_key, || ATOMIC.load(Ordering::SeqCst) == 1, None);
        });

        let thread2 = thread::spawn(|| {
            let atomic_key = addr_of!(ATOMIC) as u64;
            while ATOMIC.load(Ordering::SeqCst) != 1 {
                PARKING_SPOT.park(atomic_key, || ATOMIC.load(Ordering::SeqCst) != 1, None);
            }
            ATOMIC.store(2, Ordering::SeqCst);
            PARKING_SPOT.unpark(atomic_key, u32::MAX);
            PARKING_SPOT.park(atomic_key, || ATOMIC.load(Ordering::SeqCst) == 2, None);
        });

        let thread3 = thread::spawn(|| {
            let atomic_key = addr_of!(ATOMIC) as u64;
            while ATOMIC.load(Ordering::SeqCst) != 2 {
                PARKING_SPOT.park(atomic_key, || ATOMIC.load(Ordering::SeqCst) != 2, None);
            }
            ATOMIC.store(3, Ordering::SeqCst);
            PARKING_SPOT.unpark(atomic_key, u32::MAX);

            PARKING_SPOT.park(atomic_key, || ATOMIC.load(Ordering::SeqCst) == 3, None);
        });

        let atomic_key = addr_of!(ATOMIC) as u64;
        while ATOMIC.load(Ordering::SeqCst) != 3 {
            PARKING_SPOT.park(atomic_key, || ATOMIC.load(Ordering::SeqCst) != 3, None);
        }
        ATOMIC.store(4, Ordering::SeqCst);
        PARKING_SPOT.unpark(atomic_key, u32::MAX);

        thread1.join().unwrap();
        thread2.join().unwrap();
        thread3.join().unwrap();
    }

    mod parking_lot {
        // This is a modified version of the parking_lot_core tests,
        // which are licensed under the MIT and Apache 2.0 licenses.
        use super::*;
        use std::sync::atomic::{AtomicIsize, AtomicU32};
        use std::sync::Arc;
        use std::time::Duration;

        macro_rules! test {
            ( $( $name:ident(
                repeats: $repeats:expr,
                latches: $latches:expr,
                delay: $delay:expr,
                threads: $threads:expr,
                single_unparks: $single_unparks:expr);
            )* ) => {
                $(
                #[test]
                fn $name() {
                    if std::env::var("WASMTIME_TEST_NO_HOG_MEMORY").is_ok() {
                        return;
                    }
                    let delay = Duration::from_micros($delay);
                    for _ in 0..$repeats {
                        run_parking_test($latches, delay, $threads, $single_unparks);
                    }
                })*
            };
        }

        test! {
            unpark_all_one_fast(
                repeats: 10000, latches: 1, delay: 0, threads: 1, single_unparks: 0
            );
            unpark_all_hundred_fast(
                repeats: 100, latches: 1, delay: 0, threads: 100, single_unparks: 0
            );
            unpark_one_one_fast(
                repeats: 1000, latches: 1, delay: 0, threads: 1, single_unparks: 1
            );
            unpark_one_hundred_fast(
                repeats: 20, latches: 1, delay: 0, threads: 100, single_unparks: 100
            );
            unpark_one_fifty_then_fifty_all_fast(
                repeats: 50, latches: 1, delay: 0, threads: 100, single_unparks: 50
            );
            unpark_all_one(
                repeats: 100, latches: 1, delay: 10000, threads: 1, single_unparks: 0
            );
            unpark_all_hundred(
                repeats: 100, latches: 1, delay: 10000, threads: 100, single_unparks: 0
            );
            unpark_one_one(
                repeats: 10, latches: 1, delay: 10000, threads: 1, single_unparks: 1
            );
            unpark_one_fifty(
                repeats: 1, latches: 1, delay: 10000, threads: 50, single_unparks: 50
            );
            unpark_one_fifty_then_fifty_all(
                repeats: 2, latches: 1, delay: 10000, threads: 100, single_unparks: 50
            );
            hundred_unpark_all_one_fast(
                repeats: 100, latches: 100, delay: 0, threads: 1, single_unparks: 0
            );
            hundred_unpark_all_one(
                repeats: 1, latches: 100, delay: 10000, threads: 1, single_unparks: 0
            );
        }

        fn run_parking_test(
            num_latches: usize,
            delay: Duration,
            num_threads: u32,
            num_single_unparks: u32,
        ) {
            let mut tests = Vec::with_capacity(num_latches);

            for _ in 0..num_latches {
                let test = Arc::new(SingleLatchTest::new(num_threads));
                let mut threads = Vec::with_capacity(num_threads as _);
                for _ in 0..num_threads {
                    let test = test.clone();
                    threads.push(thread::spawn(move || test.run()));
                }
                tests.push((test, threads));
            }

            for unpark_index in 0..num_single_unparks {
                thread::sleep(delay);
                for (test, _) in &tests {
                    test.unpark_one(unpark_index);
                }
            }

            for (test, threads) in tests {
                test.finish(num_single_unparks);
                for thread in threads {
                    thread.join().expect("Test thread panic");
                }
            }
        }

        struct SingleLatchTest {
            semaphore: AtomicIsize,
            num_awake: AtomicU32,
            /// Total number of threads participating in this test.
            num_threads: u32,
        }

        impl SingleLatchTest {
            pub fn new(num_threads: u32) -> Self {
                Self {
                    // This implements a fair (FIFO) semaphore, and it starts out unavailable.
                    semaphore: AtomicIsize::new(0),
                    num_awake: AtomicU32::new(0),
                    num_threads,
                }
            }

            pub fn run(&self) {
                // Get one slot from the semaphore
                self.down();

                self.num_awake.fetch_add(1, Ordering::SeqCst);
            }

            pub fn unpark_one(&self, _single_unpark_index: u32) {
                let num_awake_before_up = self.num_awake.load(Ordering::SeqCst);

                self.up();

                // Wait for a parked thread to wake up and update num_awake + last_awoken.
                while self.num_awake.load(Ordering::SeqCst) != num_awake_before_up + 1 {
                    thread::yield_now();
                }
            }

            pub fn finish(&self, num_single_unparks: u32) {
                // The amount of threads not unparked via unpark_one
                let mut num_threads_left =
                    self.num_threads.checked_sub(num_single_unparks).unwrap();

                // Wake remaining threads up with unpark_all. Has to be in a loop, because there might
                // still be threads that has not yet parked.
                while num_threads_left > 0 {
                    let mut num_waiting_on_address = 0;
                    PARKING_SPOT.with_lot(self.semaphore_addr(), |thread_data| {
                        num_waiting_on_address = thread_data.num_parked;
                    });
                    assert!(num_waiting_on_address <= num_threads_left);

                    let num_awake_before_unpark = self.num_awake.load(Ordering::SeqCst);

                    let num_unparked = PARKING_SPOT.unpark(self.semaphore_addr(), u32::MAX);
                    assert!(num_unparked >= num_waiting_on_address);
                    assert!(num_unparked <= num_threads_left);

                    // Wait for all unparked threads to wake up and update num_awake + last_awoken.
                    while self.num_awake.load(Ordering::SeqCst)
                        != num_awake_before_unpark + num_unparked
                    {
                        thread::yield_now();
                    }

                    num_threads_left = num_threads_left.checked_sub(num_unparked).unwrap();
                }
                // By now, all threads should have been woken up
                assert_eq!(self.num_awake.load(Ordering::SeqCst), self.num_threads);

                // Make sure no thread is parked on our semaphore address
                let mut num_waiting_on_address = 0;
                PARKING_SPOT.with_lot(self.semaphore_addr(), |thread_data| {
                    num_waiting_on_address = thread_data.num_parked;
                });
                assert_eq!(num_waiting_on_address, 0);
            }

            pub fn down(&self) {
                let old_semaphore_value = self.semaphore.fetch_sub(1, Ordering::SeqCst);

                if old_semaphore_value > 0 {
                    // We acquired the semaphore. Done.
                    return;
                }

                // We need to wait.
                let validate = || true;
                PARKING_SPOT.park(self.semaphore_addr(), validate, None);
            }

            pub fn up(&self) {
                let old_semaphore_value = self.semaphore.fetch_add(1, Ordering::SeqCst);

                // Check if anyone was waiting on the semaphore. If they were, then pass ownership to them.
                if old_semaphore_value < 0 {
                    // We need to continue until we have actually unparked someone. It might be that
                    // the thread we want to pass ownership to has decremented the semaphore counter,
                    // but not yet parked.
                    loop {
                        match PARKING_SPOT.unpark(self.semaphore_addr(), 1) {
                            1 => break,
                            0 => (),
                            i => panic!("Should not wake up {i} threads"),
                        }
                    }
                }
            }

            fn semaphore_addr(&self) -> u64 {
                addr_of!(self.semaphore) as _
            }
        }
    }
}
