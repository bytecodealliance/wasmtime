use crate::preview2::{
    clocks::{self, HostMonotonicClock, HostWallClock},
    filesystem::{Dir, TableFsExt},
    pipe, random, stdio,
    stream::{InputStream, OutputStream, TableStreamExt},
    DirPerms, FilePerms, Table,
};
use cap_rand::{Rng, RngCore, SeedableRng};

use super::clocks::host::{monotonic_clock, wall_clock};

pub struct WasiCtxBuilder {
    stdin: Box<dyn InputStream>,
    stdout: Box<dyn OutputStream>,
    stderr: Box<dyn OutputStream>,
    env: Vec<(String, String)>,
    args: Vec<String>,
    preopens: Vec<(Dir, String)>,

    random: Box<dyn RngCore + Send + Sync>,
    insecure_random: Box<dyn RngCore + Send + Sync>,
    insecure_random_seed: u128,
    wall_clock: Box<dyn HostWallClock + Send + Sync>,
    monotonic_clock: Box<dyn HostMonotonicClock + Send + Sync>,
}

impl WasiCtxBuilder {
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
            stdin: Box::new(pipe::ReadPipe::new(std::io::empty())),
            stdout: Box::new(pipe::WritePipe::new(std::io::sink())),
            stderr: Box::new(pipe::WritePipe::new(std::io::sink())),
            env: Vec::new(),
            args: Vec::new(),
            preopens: Vec::new(),
            random: random::thread_rng(),
            insecure_random,
            insecure_random_seed,
            wall_clock: wall_clock(),
            monotonic_clock: monotonic_clock(),
        }
    }

    pub fn set_stdin(mut self, stdin: impl InputStream + 'static) -> Self {
        self.stdin = Box::new(stdin);
        self
    }

    pub fn set_stdout(mut self, stdout: impl OutputStream + 'static) -> Self {
        self.stdout = Box::new(stdout);
        self
    }

    pub fn set_stderr(mut self, stderr: impl OutputStream + 'static) -> Self {
        self.stderr = Box::new(stderr);
        self
    }

    pub fn inherit_stdin(self) -> Self {
        self.set_stdin(stdio::stdin())
    }

    pub fn inherit_stdout(self) -> Self {
        self.set_stdout(stdio::stdout())
    }

    pub fn inherit_stderr(self) -> Self {
        self.set_stderr(stdio::stderr())
    }

    pub fn inherit_stdio(self) -> Self {
        self.inherit_stdin().inherit_stdout().inherit_stderr()
    }

    pub fn set_env(mut self, env: &[(impl AsRef<str>, impl AsRef<str>)]) -> Self {
        self.env = env
            .iter()
            .map(|(k, v)| (k.as_ref().to_owned(), v.as_ref().to_owned()))
            .collect();
        self
    }

    pub fn push_env(mut self, k: impl AsRef<str>, v: impl AsRef<str>) -> Self {
        self.env
            .push((k.as_ref().to_owned(), v.as_ref().to_owned()));
        self
    }

    pub fn set_args(mut self, args: &[impl AsRef<str>]) -> Self {
        self.args = args.iter().map(|a| a.as_ref().to_owned()).collect();
        self
    }

    pub fn push_arg(mut self, arg: impl AsRef<str>) -> Self {
        self.args.push(arg.as_ref().to_owned());
        self
    }

    pub fn push_preopened_dir(
        mut self,
        dir: cap_std::fs::Dir,
        perms: DirPerms,
        file_perms: FilePerms,
        path: impl AsRef<str>,
    ) -> Self {
        self.preopens
            .push((Dir::new(dir, perms, file_perms), path.as_ref().to_owned()));
        self
    }

    /// Set the generator for the secure random number generator.
    ///
    /// This initializes the random number generator using
    /// [`cap_rand::thread_rng`].
    pub fn set_secure_random(mut self) -> Self {
        self.random = random::thread_rng();
        self
    }

    /// Set the generator for the secure random number generator to a custom
    /// generator.
    ///
    /// This function is usually not needed; use [`set_secure_random`] to
    /// install the default generator, which is intended to be sufficient for
    /// most use cases.
    ///
    /// Guest code may rely on this random number generator to produce fresh
    /// unpredictable random data in order to maintain its security invariants,
    /// and ideally should use the insecure random API otherwise, so using any
    /// prerecorded or otherwise predictable data may compromise security.
    ///
    /// [`set_secure_random`]: Self::set_secure_random
    pub fn set_secure_random_to_custom_generator(
        mut self,
        random: impl RngCore + Send + Sync + 'static,
    ) -> Self {
        self.random = Box::new(random);
        self
    }

    pub fn set_insecure_random(
        mut self,
        insecure_random: impl RngCore + Send + Sync + 'static,
    ) -> Self {
        self.insecure_random = Box::new(insecure_random);
        self
    }
    pub fn set_insecure_random_seed(mut self, insecure_random_seed: u128) -> Self {
        self.insecure_random_seed = insecure_random_seed;
        self
    }

    pub fn set_wall_clock(mut self, clock: impl clocks::HostWallClock + 'static) -> Self {
        self.wall_clock = Box::new(clock);
        self
    }

    pub fn set_monotonic_clock(mut self, clock: impl clocks::HostMonotonicClock + 'static) -> Self {
        self.monotonic_clock = Box::new(clock);
        self
    }

    pub fn build(self, table: &mut Table) -> Result<WasiCtx, anyhow::Error> {
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
        } = self;

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
