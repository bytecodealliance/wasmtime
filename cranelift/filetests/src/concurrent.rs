//! Run tests concurrently.
//!
//! This module provides the `ConcurrentRunner` struct which uses a pool of threads to run tests
//! concurrently.

use crate::runone;
use cranelift_codegen::dbg::LOG_FILENAME_PREFIX;
use cranelift_codegen::timing;
use log::error;
use std::panic::catch_unwind;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// Request sent to worker threads contains jobid and path.
struct Request(usize, PathBuf);

/// Reply from worker thread,
pub enum Reply {
    Starting {
        jobid: usize,
    },
    Done {
        jobid: usize,
        result: anyhow::Result<Duration>,
    },
    Tick,
}

/// Manage threads that run test jobs concurrently.
pub struct ConcurrentRunner {
    /// Channel for sending requests to the worker threads.
    /// The workers are sharing the receiver with an `Arc<Mutex<Receiver>>`.
    /// This is `None` when shutting down.
    request_tx: Option<Sender<Request>>,

    /// Channel for receiving replies from the workers.
    /// Workers have their own `Sender`.
    reply_rx: Receiver<Reply>,

    handles: Vec<thread::JoinHandle<timing::PassTimes>>,
}

impl ConcurrentRunner {
    /// Create a new `ConcurrentRunner` with threads spun up.
    pub fn new() -> Self {
        let (request_tx, request_rx) = channel();
        let request_mutex = Arc::new(Mutex::new(request_rx));
        let (reply_tx, reply_rx) = channel();

        heartbeat_thread(reply_tx.clone());

        let num_threads = std::env::var("CRANELIFT_FILETESTS_THREADS")
            .ok()
            .map(|s| {
                use std::str::FromStr;
                let n = usize::from_str(&s).unwrap();
                assert!(n > 0);
                n
            })
            .unwrap_or_else(|| num_cpus::get());
        let handles = (0..num_threads)
            .map(|num| worker_thread(num, request_mutex.clone(), reply_tx.clone()))
            .collect();

        Self {
            request_tx: Some(request_tx),
            reply_rx,
            handles,
        }
    }

    /// Shut down worker threads orderly. They will finish any queued jobs first.
    pub fn shutdown(&mut self) {
        self.request_tx = None;
    }

    /// Join all the worker threads.
    /// Transfer pass timings from the worker threads to the current thread.
    pub fn join(&mut self) -> timing::PassTimes {
        assert!(self.request_tx.is_none(), "must shutdown before join");
        let mut pass_times = timing::PassTimes::default();
        for h in self.handles.drain(..) {
            match h.join() {
                Ok(t) => pass_times.add(&t),
                Err(e) => println!("worker panicked: {e:?}"),
            }
        }
        pass_times
    }

    /// Add a new job to the queues.
    pub fn put(&mut self, jobid: usize, path: &Path) {
        self.request_tx
            .as_ref()
            .expect("cannot push after shutdown")
            .send(Request(jobid, path.to_owned()))
            .expect("all the worker threads are gone");
    }

    /// Get a job reply without blocking.
    pub fn try_get(&mut self) -> Option<Reply> {
        self.reply_rx.try_recv().ok()
    }

    /// Get a job reply, blocking until one is available.
    pub fn get(&mut self) -> Option<Reply> {
        self.reply_rx.recv().ok()
    }
}

/// Spawn a heartbeat thread which sends ticks down the reply channel every second.
/// This lets us implement timeouts without the not yet stable `recv_timeout`.
fn heartbeat_thread(replies: Sender<Reply>) -> thread::JoinHandle<()> {
    thread::Builder::new()
        .name("heartbeat".to_string())
        .spawn(move || {
            file_per_thread_logger::initialize(LOG_FILENAME_PREFIX);
            while replies.send(Reply::Tick).is_ok() {
                thread::sleep(Duration::from_secs(1));
            }
        })
        .unwrap()
}

/// Spawn a worker thread running tests.
fn worker_thread(
    thread_num: usize,
    requests: Arc<Mutex<Receiver<Request>>>,
    replies: Sender<Reply>,
) -> thread::JoinHandle<timing::PassTimes> {
    thread::Builder::new()
        .name(format!("worker #{thread_num}"))
        .spawn(move || {
            file_per_thread_logger::initialize(LOG_FILENAME_PREFIX);
            loop {
                // Lock the mutex only long enough to extract a request.
                let Request(jobid, path) = match requests.lock().unwrap().recv() {
                    Err(..) => break, // TX end shut down. exit thread.
                    Ok(req) => req,
                };

                // Tell them we're starting this job.
                // The receiver should always be present for this as long as we have jobs.
                replies.send(Reply::Starting { jobid }).unwrap();

                let result = catch_unwind(|| runone::run(path.as_path(), None, None))
                    .unwrap_or_else(|e| {
                        // The test panicked, leaving us a `Box<Any>`.
                        // Panics are usually strings.
                        if let Some(msg) = e.downcast_ref::<String>() {
                            anyhow::bail!("panicked in worker #{}: {}", thread_num, msg)
                        } else if let Some(msg) = e.downcast_ref::<&'static str>() {
                            anyhow::bail!("panicked in worker #{}: {}", thread_num, msg)
                        } else {
                            anyhow::bail!("panicked in worker #{}", thread_num)
                        }
                    });

                if let Err(ref msg) = result {
                    error!("FAIL: {}", msg);
                }

                replies.send(Reply::Done { jobid, result }).unwrap();
            }

            // Timing is accumulated independently per thread.
            // Timings from this worker thread will be aggregated by `ConcurrentRunner::join()`.
            timing::take_current()
        })
        .unwrap()
}
