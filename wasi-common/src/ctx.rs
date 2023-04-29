use crate::clocks::WasiClocks;
use crate::dir::WasiDir;
use crate::sched::WasiSched;
use crate::stream::{InputStream, OutputStream};
use crate::table::Table;
use cap_rand::RngCore;

#[derive(Default)]
pub struct WasiCtxBuilder {
    stdin: Option<Box<dyn InputStream>>,
    stdout: Option<Box<dyn OutputStream>>,
    stderr: Option<Box<dyn OutputStream>>,
    env: Vec<(String, String)>,
    args: Vec<String>,
    preopens: Vec<(Box<dyn WasiDir>, String)>,

    random: Option<Box<dyn RngCore + Send + Sync>>,
    clocks: Option<WasiClocks>,

    sched: Option<Box<dyn WasiSched>>,
}

impl WasiCtxBuilder {
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
        dir: impl WasiDir + 'static,
        path: impl AsRef<str>,
    ) -> Self {
        self.preopens
            .push((Box::new(dir), path.as_ref().to_owned()));
        self
    }

    pub fn set_random(mut self, random: impl RngCore + Send + Sync + 'static) -> Self {
        self.random = Some(Box::new(random));
        self
    }
    pub fn set_clocks(mut self, clocks: WasiClocks) -> Self {
        self.clocks = Some(clocks);
        self
    }

    pub fn set_sched(mut self, sched: impl WasiSched + 'static) -> Self {
        self.sched = Some(Box::new(sched));
        self
    }

    pub fn build(self, mut table: Table) -> Result<WasiCtx, anyhow::Error> {
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
            table,

            random: self.random.context("required member random")?,
            clocks: self.clocks.context("required member clocks")?,
            sched: self.sched.context("required member sched")?,
            env: self.env,
            args: self.args,
            preopens,
            stdin,
            stdout,
            stderr,
        })
    }
}

pub struct WasiCtx {
    pub table: Table,

    pub random: Box<dyn RngCore + Send + Sync>,
    pub clocks: WasiClocks,
    pub sched: Box<dyn WasiSched>,
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

    pub fn table(&self) -> &Table {
        &self.table
    }

    pub fn table_mut(&mut self) -> &mut Table {
        &mut self.table
    }
}
