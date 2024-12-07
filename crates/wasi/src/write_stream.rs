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
    shutdown_pending: bool,
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
    Shutdown,
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
                shutdown_pending: false,
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
                    || (!state.flush_pending && !state.shutdown_pending && state.write_budget > 0)
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

        if state.flush_pending || state.shutdown_pending || state.write_budget == 0 {
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
            if state.shutdown_pending {
                return Some(Job::Shutdown);
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
            state.shutdown_pending = false;
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

                    Job::Shutdown => {
                        if let Err(e) = writer.shutdown().await {
                            self.report_error(e);
                            return;
                        }
                        self.state().shutdown_pending = false;
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
    shutdown_join_handle: Option<tokio::task::AbortHandle>,
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
            shutdown_join_handle: None,
        }
    }

    /// Create a [`AsyncWriteStream`]. In order to use the [`HostOutputStream`] impl
    /// provided by this struct, the argument must impl [`tokio::io::AsyncWrite`].
    ///
    /// The [`AsyncWriteStream`] created by this constructor can be shut down (that is,
    /// graceful EOF) by sending a message through the sender side of the `shutdown_rx`
    /// sync channel.
    pub fn new_closeable<T: tokio::io::AsyncWrite + Send + Unpin + 'static>(
        write_budget: usize,
        writer: T,
        mut shutdown_rx: tokio::sync::mpsc::Receiver<()>,
    ) -> Self {
        let worker = Arc::new(Worker::new(write_budget));

        let w = Arc::clone(&worker);
        let join_handle = crate::runtime::spawn(async move { w.work(writer).await });

        let w_clone = worker.clone();
        let shutdown_join_handle = tokio::spawn(async move {
            let shutdown_msg = shutdown_rx.recv().await;
            if shutdown_msg.is_some() {
                let mut state = w_clone.state();
                if state.check_error().is_err() {
                    // The stream is already failing - no point shutting it down.
                    return;
                }

                state.shutdown_pending = true;
                w_clone.new_work.notify_one();
            }
        })
        .abort_handle();

        AsyncWriteStream {
            worker,
            join_handle: Some(join_handle),
            shutdown_join_handle: Some(shutdown_join_handle),
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
        if let Some(handle) = self.shutdown_join_handle.take() {
            handle.abort();
        };
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
