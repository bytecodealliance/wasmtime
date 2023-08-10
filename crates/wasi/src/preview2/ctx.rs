use super::clocks::host::{monotonic_clock, wall_clock};
use crate::preview2::{
    clocks::{self, HostMonotonicClock, HostWallClock},
    filesystem::{Dir, TableFsExt},
    pipe, random, stdio,
    stream::{HostInputStream, HostOutputStream, TableStreamExt},
    DirPerms, FilePerms, Table,
};
use cap_rand::{Rng, RngCore, SeedableRng};
use std::mem;

pub struct WasiCtxBuilder {
    stdin: Box<dyn HostInputStream>,
    stdout: Box<dyn HostOutputStream>,
    stderr: Box<dyn HostOutputStream>,
    env: Vec<(String, String)>,
    args: Vec<String>,
    preopens: Vec<(Dir, String)>,

    random: Box<dyn RngCore + Send + Sync>,
    insecure_random: Box<dyn RngCore + Send + Sync>,
    insecure_random_seed: u128,
    wall_clock: Box<dyn HostWallClock + Send + Sync>,
    monotonic_clock: Box<dyn HostMonotonicClock + Send + Sync>,
    built: bool,
}

impl WasiCtxBuilder {
    /// Creates a builder for a new context with default parameters set.
    ///
    /// The current defaults are:
    ///
    /// * stdin is closed
    /// * stdout and stderr eat all input but it doesn't go anywhere
    /// * no env vars
    /// * no arguments
    /// * no preopens
    /// * clocks use the host implementation of wall/monotonic clocks
    /// * RNGs are all initialized with random state and suitable generator
    ///   quality to satisfy the requirements of WASI APIs.
    ///
    /// These defaults can all be updated via the various builder configuration
    /// methods below.
    ///
    /// Note that each builder can only be used once to produce a [`WasiCtx`].
    /// Invoking the [`build`](WasiCtxBuilder::build) method will panic on the
    /// second attempt.
    pub fn new() -> Self {
        // For the insecure random API, use `SmallRng`, which is fast. It's
        // also insecure, but that's the deal here.
        let insecure_random = Box::new(cap_rand::rngs::SmallRng::from_entropy());

        // For the insecure random seed, use a `u128` generated from
        // `thread_rng()`, so that it's not guessable from the insecure_random
        // API.
        let insecure_random_seed =
            cap_rand::thread_rng(cap_rand::ambient_authority()).gen::<u128>();
        Self {
            stdin: Box::new(pipe::ClosedInputStream),
            stdout: Box::new(pipe::SinkOutputStream),
            stderr: Box::new(pipe::SinkOutputStream),
            env: Vec::new(),
            args: Vec::new(),
            preopens: Vec::new(),
            random: random::thread_rng(),
            insecure_random,
            insecure_random_seed,
            wall_clock: wall_clock(),
            monotonic_clock: monotonic_clock(),
            built: false,
        }
    }

    pub fn stdin(&mut self, stdin: impl HostInputStream + 'static) -> &mut Self {
        self.stdin = Box::new(stdin);
        self
    }

    pub fn stdout(&mut self, stdout: impl HostOutputStream + 'static) -> &mut Self {
        self.stdout = Box::new(stdout);
        self
    }

    pub fn stderr(&mut self, stderr: impl HostOutputStream + 'static) -> &mut Self {
        self.stderr = Box::new(stderr);
        self
    }

    pub fn inherit_stdin(&mut self) -> &mut Self {
        self.stdin(stdio::stdin())
    }

    pub fn inherit_stdout(&mut self) -> &mut Self {
        self.stdout(stdio::stdout())
    }

    pub fn inherit_stderr(&mut self) -> &mut Self {
        self.stderr(stdio::stderr())
    }

    pub fn inherit_stdio(&mut self) -> &mut Self {
        self.inherit_stdin().inherit_stdout().inherit_stderr()
    }

    pub fn envs(&mut self, env: &[(impl AsRef<str>, impl AsRef<str>)]) -> &mut Self {
        self.env.extend(
            env.iter()
                .map(|(k, v)| (k.as_ref().to_owned(), v.as_ref().to_owned())),
        );
        self
    }

    pub fn env(&mut self, k: impl AsRef<str>, v: impl AsRef<str>) -> &mut Self {
        self.env
            .push((k.as_ref().to_owned(), v.as_ref().to_owned()));
        self
    }

    pub fn args(&mut self, args: &[impl AsRef<str>]) -> &mut Self {
        self.args.extend(args.iter().map(|a| a.as_ref().to_owned()));
        self
    }

    pub fn arg(&mut self, arg: impl AsRef<str>) -> &mut Self {
        self.args.push(arg.as_ref().to_owned());
        self
    }

    pub fn preopened_dir(
        &mut self,
        dir: cap_std::fs::Dir,
        perms: DirPerms,
        file_perms: FilePerms,
        path: impl AsRef<str>,
    ) -> &mut Self {
        self.preopens
            .push((Dir::new(dir, perms, file_perms), path.as_ref().to_owned()));
        self
    }

    /// Set the generator for the secure random number generator to the custom
    /// generator specified.
    ///
    /// Note that contexts have a default RNG configured which is a suitable
    /// generator for WASI and is configured with a random seed per-context.
    ///
    /// Guest code may rely on this random number generator to produce fresh
    /// unpredictable random data in order to maintain its security invariants,
    /// and ideally should use the insecure random API otherwise, so using any
    /// prerecorded or otherwise predictable data may compromise security.
    pub fn secure_random(&mut self, random: impl RngCore + Send + Sync + 'static) -> &mut Self {
        self.random = Box::new(random);
        self
    }

    pub fn insecure_random(
        &mut self,
        insecure_random: impl RngCore + Send + Sync + 'static,
    ) -> &mut Self {
        self.insecure_random = Box::new(insecure_random);
        self
    }
    pub fn insecure_random_seed(&mut self, insecure_random_seed: u128) -> &mut Self {
        self.insecure_random_seed = insecure_random_seed;
        self
    }

    pub fn wall_clock(&mut self, clock: impl clocks::HostWallClock + 'static) -> &mut Self {
        self.wall_clock = Box::new(clock);
        self
    }

    pub fn monotonic_clock(
        &mut self,
        clock: impl clocks::HostMonotonicClock + 'static,
    ) -> &mut Self {
        self.monotonic_clock = Box::new(clock);
        self
    }

    /// Uses the configured context so far to construct the final `WasiCtx`.
    ///
    /// This will insert resources into the provided `table`.
    ///
    /// Note that each `WasiCtxBuilder` can only be used to "build" once, and
    /// calling this method twice will panic.
    ///
    /// # Panics
    ///
    /// Panics if this method is called twice.
    pub fn build(&mut self, table: &mut Table) -> Result<WasiCtx, anyhow::Error> {
        assert!(!self.built);

        use anyhow::Context;
        let Self {
            stdin,
            stdout,
            stderr,
            env,
            args,
            preopens,
            random,
            insecure_random,
            insecure_random_seed,
            wall_clock,
            monotonic_clock,
            built: _,
        } = mem::replace(self, Self::new());
        self.built = true;

        let stdin = table.push_input_stream(stdin).context("stdin")?;
        let stdout = table.push_output_stream(stdout).context("stdout")?;
        let stderr = table.push_output_stream(stderr).context("stderr")?;

        let preopens = preopens
            .into_iter()
            .map(|(dir, path)| {
                let dirfd = table
                    .push_dir(dir)
                    .with_context(|| format!("preopen {path:?}"))?;
                Ok((dirfd, path))
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        Ok(WasiCtx {
            stdin,
            stdout,
            stderr,
            env,
            args,
            preopens,
            random,
            insecure_random,
            insecure_random_seed,
            wall_clock,
            monotonic_clock,
        })
    }
}

pub trait WasiView: Send {
    fn table(&self) -> &Table;
    fn table_mut(&mut self) -> &mut Table;
    fn ctx(&self) -> &WasiCtx;
    fn ctx_mut(&mut self) -> &mut WasiCtx;
}

pub struct WasiCtx {
    pub(crate) random: Box<dyn RngCore + Send + Sync>,
    pub(crate) insecure_random: Box<dyn RngCore + Send + Sync>,
    pub(crate) insecure_random_seed: u128,
    pub(crate) wall_clock: Box<dyn HostWallClock + Send + Sync>,
    pub(crate) monotonic_clock: Box<dyn HostMonotonicClock + Send + Sync>,
    pub(crate) env: Vec<(String, String)>,
    pub(crate) args: Vec<String>,
    pub(crate) preopens: Vec<(u32, String)>,
    pub(crate) stdin: u32,
    pub(crate) stdout: u32,
    pub(crate) stderr: u32,
}

impl WasiCtx {
    /// Flush all buffered output and wait for it to reach the destination.
    ///
    /// NOTE: This function should be used when [`WasiCtx`] is used in an async embedding
    /// (i.e. with [`crate::preview2::command::add_to_linker`]). Use its counterpart
    /// `sync_join_background_tasks` in a synchronous embedding (i.e. with
    /// [`crate::preview2::command::sync::add_to_linker`].
    ///
    /// In order to implement non-blocking streams, we often often need to offload async
    /// operations to background `tokio::task`s. These tasks are aborted when the resources
    /// in the `Table` referencing them are dropped. In some cases, this abort may occur before
    /// buffered output has been flushed. Use this function to wait for all
    /// written data to reach its destination gracefully, even if wasm didn't
    /// explicitly wait for this.
    ///
    /// In some embeddings, a misbehaving client might cause this graceful exit to await for an
    /// unbounded amount of time, so we recommend bounding this with a timeout or other mechanism.
    pub async fn flush_output(&mut self, table: &mut Table) {
        let keys = table.keys().cloned().collect::<Vec<u32>>();
        for k in keys {
            match table.delete_output_stream(k) {
                Ok(mut ostream) => {
                    let _ = ostream.flush().await;
                }
                _ => {}
            }
        }
    }

    /// Same as [`WasiCtx::flush_output`] except suitable for synchronous
    /// embeddings.
    ///
    /// This will block the current thread up to the `timeout` specified, or
    /// forever if `None` is specified, until all wasm output has been flushed
    /// out.
    pub fn sync_join_background_tasks<'a>(
        &mut self,
        table: &'a mut Table,
        timeout: Option<std::time::Duration>,
    ) {
        crate::preview2::in_tokio(async move {
            if let Some(timeout) = timeout {
                let _ = tokio::time::timeout(timeout, self.flush_output(table)).await;
            } else {
                self.flush_output(table).await
            }
        })
    }
}
