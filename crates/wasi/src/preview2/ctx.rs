use crate::preview2::{
    clocks::{self, WasiClocks},
    filesystem::{Dir, TableFsExt},
    pipe, random, stdio,
    stream::{InputStream, OutputStream, TableStreamExt},
    DirPerms, FilePerms, Table,
};
use cap_rand::{Rng, RngCore, SeedableRng};

#[derive(Default)]
pub struct WasiCtxBuilder {
    stdin: Option<Box<dyn InputStream>>,
    stdout: Option<Box<dyn OutputStream>>,
    stderr: Option<Box<dyn OutputStream>>,
    env: Vec<(String, String)>,
    args: Vec<String>,
    preopens: Vec<(Dir, String)>,

    random: Option<Box<dyn RngCore + Send + Sync>>,
    insecure_random: Option<Box<dyn RngCore + Send + Sync>>,
    insecure_random_seed: Option<u128>,
    clocks: Option<WasiClocks>,
}

impl WasiCtxBuilder {
    pub fn new() -> Self {
        // For the insecure random API, use `SmallRng`, which is fast. It's
        // also insecure, but that's the deal here.
        let insecure_random = cap_rand::rngs::SmallRng::from_entropy();

        // For the insecure random seed, use a `u128` generated from
        // `thread_rng()`, so that it's not guessable from the insecure_random
        // API.
        let insecure_random_seed =
            cap_rand::thread_rng(cap_rand::ambient_authority()).gen::<u128>();

        let mut result = Self::default()
            .set_clocks(clocks::host::clocks_ctx())
            .set_insecure_random(insecure_random)
            .set_insecure_random_seed(insecure_random_seed)
            .set_stdin(pipe::ReadPipe::new(std::io::empty()))
            .set_stdout(pipe::WritePipe::new(std::io::sink()))
            .set_stderr(pipe::WritePipe::new(std::io::sink()));
        result.random = Some(random::thread_rng());
        result
    }

    pub fn set_stdin(mut self, stdin: impl InputStream + 'static) -> Self {
        self.stdin = Some(Box::new(stdin));
        self
    }

    pub fn set_stdout(mut self, stdout: impl OutputStream + 'static) -> Self {
        self.stdout = Some(Box::new(stdout));
        self
    }

    pub fn set_stderr(mut self, stderr: impl OutputStream + 'static) -> Self {
        self.stderr = Some(Box::new(stderr));
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

    // TODO: optionally read-only
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

    /// Set the generator for the secure random number generator. This
    /// is only avalabile under `cfg(test)` as it's only intended for use
    /// by tests.
    #[cfg(test)]
    pub fn set_random(mut self, random: impl RngCore + Send + Sync + 'static) -> Self {
        self.random = Some(Box::new(random));
        self
    }

    pub fn set_insecure_random(
        mut self,
        insecure_random: impl RngCore + Send + Sync + 'static,
    ) -> Self {
        self.insecure_random = Some(Box::new(insecure_random));
        self
    }
    pub fn set_insecure_random_seed(mut self, insecure_random_seed: u128) -> Self {
        self.insecure_random_seed = Some(insecure_random_seed);
        self
    }
    pub fn set_clocks(mut self, clocks: WasiClocks) -> Self {
        self.clocks = Some(clocks);
        self
    }

    pub fn build(self, table: &mut Table) -> Result<WasiCtx, anyhow::Error> {
        use anyhow::Context;

        let stdin = table
            .push_input_stream(self.stdin.context("required member stdin")?)
            .context("stdin")?;
        let stdout = table
            .push_output_stream(self.stdout.context("required member stdout")?)
            .context("stdout")?;
        let stderr = table
            .push_output_stream(self.stderr.context("required member stderr")?)
            .context("stderr")?;

        let mut preopens = Vec::new();
        for (dir, path) in self.preopens {
            let dirfd = table
                .push_dir(dir)
                .with_context(|| format!("preopen {path:?}"))?;
            preopens.push((dirfd, path));
        }

        Ok(WasiCtx {
            random: self.random.context("required member random")?,
            insecure_random: self
                .insecure_random
                .context("required member insecure_random")?,
            insecure_random_seed: self
                .insecure_random_seed
                .context("required member insecure_random_seed")?,
            clocks: self.clocks.context("required member clocks")?,
            env: self.env,
            args: self.args,
            preopens,
            stdin,
            stdout,
            stderr,
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
    pub insecure_random: Box<dyn RngCore + Send + Sync>,
    pub insecure_random_seed: u128,
    pub clocks: WasiClocks,
    pub env: Vec<(String, String)>,
    pub args: Vec<String>,
    pub preopens: Vec<(u32, String)>,
    pub stdin: u32,
    pub stdout: u32,
    pub stderr: u32,
}

impl WasiCtx {
    /// Create an empty `WasiCtxBuilder`
    pub fn builder() -> WasiCtxBuilder {
        WasiCtxBuilder::default()
    }
}
