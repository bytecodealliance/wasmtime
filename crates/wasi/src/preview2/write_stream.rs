use crate::preview2::{HostOutputStream, OutputStreamError};
use anyhow::anyhow;
use bytes::Bytes;
use std::sync::{Arc, Mutex};

#[derive(Debug)]
struct WorkerState {
    alive: bool,
    items: std::collections::VecDeque<Bytes>,
    write_budget: usize,
    flush_pending: bool,
    error: Option<anyhow::Error>,
}

impl WorkerState {
    fn check_error(&mut self) -> Result<(), OutputStreamError> {
        if let Some(e) = self.error.take() {
            return Err(OutputStreamError::LastOperationFailed(e));
        }
        if !self.alive {
            return Err(OutputStreamError::Closed);
        }
        Ok(())
    }
}

struct Worker {
    state: Mutex<WorkerState>,
    new_work: tokio::sync::Notify,
    write_ready_changed: tokio::sync::Notify,
}

enum Job {
    Flush,
    Write(Bytes),
}

enum WriteStatus<'a> {
    Done(Result<usize, OutputStreamError>),
    Pending(tokio::sync::futures::Notified<'a>),
}

impl Worker {
    fn new(write_budget: usize) -> Self {
        Self {
            state: Mutex::new(WorkerState {
                alive: true,
                items: std::collections::VecDeque::new(),
                write_budget,
                flush_pending: false,
                error: None,
            }),
            new_work: tokio::sync::Notify::new(),
            write_ready_changed: tokio::sync::Notify::new(),
        }
    }
    fn check_write(&self) -> WriteStatus<'_> {
        let mut state = self.state();
        if let Err(e) = state.check_error() {
            return WriteStatus::Done(Err(e));
        }

        if state.flush_pending || state.write_budget == 0 {
            return WriteStatus::Pending(self.write_ready_changed.notified());
        }

        WriteStatus::Done(Ok(state.write_budget))
    }
    fn state(&self) -> std::sync::MutexGuard<WorkerState> {
        self.state.lock().unwrap()
    }
    fn pop(&self) -> Option<Job> {
        let mut state = self.state();
        if state.items.is_empty() {
            if state.flush_pending {
                return Some(Job::Flush);
            }
        } else if let Some(bytes) = state.items.pop_front() {
            return Some(Job::Write(bytes));
        }

        None
    }
    fn report_error(&self, e: std::io::Error) {
        {
            let mut state = self.state();
            state.alive = false;
            state.error = Some(e.into());
            state.flush_pending = false;
        }
        self.write_ready_changed.notify_waiters();
    }
    async fn work<T: tokio::io::AsyncWrite + Send + Sync + Unpin + 'static>(&self, mut writer: T) {
        use tokio::io::AsyncWriteExt;
        loop {
            let notified = self.new_work.notified();
            while let Some(job) = self.pop() {
                match job {
                    Job::Flush => {
                        if let Err(e) = writer.flush().await {
                            self.report_error(e);
                            return;
                        }

                        tracing::debug!("worker marking flush complete");
                        self.state().flush_pending = false;
                    }

                    Job::Write(mut bytes) => {
                        tracing::debug!("worker writing: {bytes:?}");
                        let len = bytes.len();
                        match writer.write_all_buf(&mut bytes).await {
                            Err(e) => {
                                self.report_error(e);
                                return;
                            }
                            Ok(_) => {
                                self.state().write_budget += len;
                            }
                        }
                    }
                }

                self.write_ready_changed.notify_waiters();
            }

            notified.await;
        }
    }
}

/// Provides a [`HostOutputStream`] impl from a [`tokio::io::AsyncWrite`] impl
pub struct AsyncWriteStream {
    worker: Arc<Worker>,
    _join_handle: crate::preview2::AbortOnDropJoinHandle<()>,
}

impl AsyncWriteStream {
    /// Create a [`AsyncWriteStream`]. In order to use the [`HostOutputStream`] impl
    /// provided by this struct, the argument must impl [`tokio::io::AsyncWrite`].
    pub fn new<T: tokio::io::AsyncWrite + Send + Sync + Unpin + 'static>(
        write_budget: usize,
        writer: T,
    ) -> Self {
        let worker = Arc::new(Worker::new(write_budget));

        let w = Arc::clone(&worker);
        let join_handle = crate::preview2::spawn(async move { w.work(writer).await });

        AsyncWriteStream {
            worker,
            _join_handle: join_handle,
        }
    }
}

#[async_trait::async_trait]
impl HostOutputStream for AsyncWriteStream {
    fn write(&mut self, bytes: Bytes) -> Result<(), OutputStreamError> {
        let mut state = self.worker.state();
        state.check_error()?;
        if state.flush_pending {
            return Err(OutputStreamError::Trap(anyhow!(
                "write not permitted while flush pending"
            )));
        }
        match state.write_budget.checked_sub(bytes.len()) {
            Some(remaining_budget) => {
                state.write_budget = remaining_budget;
                state.items.push_back(bytes);
            }
            None => return Err(OutputStreamError::Trap(anyhow!("write exceeded budget"))),
        }
        drop(state);
        self.worker.new_work.notify_waiters();
        Ok(())
    }
    fn flush(&mut self) -> Result<(), OutputStreamError> {
        let mut state = self.worker.state();
        state.check_error()?;

        state.flush_pending = true;
        self.worker.new_work.notify_waiters();

        Ok(())
    }

    async fn write_ready(&mut self) -> Result<usize, OutputStreamError> {
        loop {
            match self.worker.check_write() {
                WriteStatus::Done(r) => return r,
                WriteStatus::Pending(notifier) => notifier.await,
            }
        }
    }
}
