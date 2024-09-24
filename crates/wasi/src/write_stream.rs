use crate::{HostOutputStream, StreamError, Subscribe};
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
    fn check_error(&mut self) -> Result<(), StreamError> {
        if let Some(e) = self.error.take() {
            return Err(StreamError::LastOperationFailed(e));
        }
        if !self.alive {
            return Err(StreamError::Closed);
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
    async fn ready(&self) {
        loop {
            {
                let state = self.state();
                if state.error.is_some()
                    || !state.alive
                    || (!state.flush_pending && state.write_budget > 0)
                {
                    return;
                }
            }
            self.write_ready_changed.notified().await;
        }
    }
    fn check_write(&self) -> Result<usize, StreamError> {
        let mut state = self.state();
        if let Err(e) = state.check_error() {
            return Err(e);
        }

        if state.flush_pending || state.write_budget == 0 {
            return Ok(0);
        }

        Ok(state.write_budget)
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
        self.write_ready_changed.notify_one();
    }
    async fn work<T: tokio::io::AsyncWrite + Send + Unpin + 'static>(&self, mut writer: T) {
        use tokio::io::AsyncWriteExt;
        loop {
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

                self.write_ready_changed.notify_one();
            }
            self.new_work.notified().await;
        }
    }
}

/// Provides a [`HostOutputStream`] impl from a [`tokio::io::AsyncWrite`] impl
pub struct AsyncWriteStream {
    worker: Arc<Worker>,
    join_handle: Option<crate::runtime::AbortOnDropJoinHandle<()>>,
}

impl AsyncWriteStream {
    /// Create a [`AsyncWriteStream`]. In order to use the [`HostOutputStream`] impl
    /// provided by this struct, the argument must impl [`tokio::io::AsyncWrite`].
    pub fn new<T: tokio::io::AsyncWrite + Send + Unpin + 'static>(
        write_budget: usize,
        writer: T,
    ) -> Self {
        let worker = Arc::new(Worker::new(write_budget));

        let w = Arc::clone(&worker);
        let join_handle = crate::runtime::spawn(async move { w.work(writer).await });

        AsyncWriteStream {
            worker,
            join_handle: Some(join_handle),
        }
    }
}

#[async_trait::async_trait]
impl HostOutputStream for AsyncWriteStream {
    fn write(&mut self, bytes: Bytes) -> Result<(), StreamError> {
        let mut state = self.worker.state();
        state.check_error()?;
        if state.flush_pending {
            return Err(StreamError::Trap(anyhow!(
                "write not permitted while flush pending"
            )));
        }
        match state.write_budget.checked_sub(bytes.len()) {
            Some(remaining_budget) => {
                state.write_budget = remaining_budget;
                state.items.push_back(bytes);
            }
            None => return Err(StreamError::Trap(anyhow!("write exceeded budget"))),
        }
        drop(state);
        self.worker.new_work.notify_one();
        Ok(())
    }
    fn flush(&mut self) -> Result<(), StreamError> {
        let mut state = self.worker.state();
        state.check_error()?;

        state.flush_pending = true;
        self.worker.new_work.notify_one();

        Ok(())
    }

    fn check_write(&mut self) -> Result<usize, StreamError> {
        self.worker.check_write()
    }

    async fn cancel(&mut self) {
        match self.join_handle.take() {
            Some(task) => _ = task.cancel().await,
            None => {}
        }
    }
}
#[async_trait::async_trait]
impl Subscribe for AsyncWriteStream {
    async fn ready(&mut self) {
        self.worker.ready().await;
    }
}
