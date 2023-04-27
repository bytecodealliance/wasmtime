use crate::clocks::WasiClocks;
use crate::dir::WasiDir;
use crate::network::WasiNetwork;
use crate::sched::WasiSched;
use crate::stream::{InputStream, OutputStream};
use crate::table::Table;
use crate::tcp_socket::WasiTcpSocket;
use crate::Error;
use cap_net_ext::AddressFamily;
use cap_rand::RngCore;
use cap_std::ambient_authority;
use cap_std::net::Pool;

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

    pool: Option<Pool>,
    network_creator: Option<NetworkCreator>,
    tcp_socket_creator: Option<TcpSocketCreator>,
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

    pub fn set_pool(mut self, pool: Pool) -> Self {
        self.pool = Some(pool);
        self
    }
    pub fn set_network_creator(mut self, network_creator: NetworkCreator) -> Self {
        self.network_creator = Some(network_creator);
        self
    }
    pub fn set_tcp_socket_creator(mut self, tcp_socket_creator: TcpSocketCreator) -> Self {
        self.tcp_socket_creator = Some(tcp_socket_creator);
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

            pool: self.pool.context("required member pool")?,
            network_creator: self
                .network_creator
                .context("required member network_creator")?,
            tcp_socket_creator: self
                .tcp_socket_creator
                .context("required member tcp_socket_creator")?,

            preopens,
            stdin,
            stdout,
            stderr,
        })
    }
}

pub type NetworkCreator = Box<dyn Fn(Pool) -> Result<Box<dyn WasiNetwork>, Error> + Send + Sync>;
pub type TcpSocketCreator =
    Box<dyn Fn(AddressFamily) -> Result<Box<dyn WasiTcpSocket>, Error> + Send + Sync>;

pub struct WasiCtx {
    pub random: Box<dyn RngCore + Send + Sync>,
    pub clocks: WasiClocks,
    pub sched: Box<dyn WasiSched>,
    pub table: Table,
    pub env: Vec<(String, String)>,
    pub args: Vec<String>,
    pub preopens: Vec<(u32, String)>,
    pub stdin: u32,
    pub stdout: u32,
    pub stderr: u32,
    pub pool: Pool,
    pub network_creator: NetworkCreator,
    pub tcp_socket_creator: TcpSocketCreator,
}

impl WasiCtx {
    /// Add network addresses to the pool.
    pub fn insert_addr<A: cap_std::net::ToSocketAddrs>(&mut self, addrs: A) -> std::io::Result<()> {
        self.pool.insert(addrs, ambient_authority())
    }

    /// Add a specific [`cap_std::net::SocketAddr`] to the pool.
    pub fn insert_socket_addr(&mut self, addr: cap_std::net::SocketAddr) {
        self.pool.insert_socket_addr(addr, ambient_authority());
    }

    /// Add a range of network addresses, accepting any port, to the pool.
    ///
    /// Unlike `insert_ip_net`, this function grants access to any requested port.
    pub fn insert_ip_net_port_any(&mut self, ip_net: ipnet::IpNet) {
        self.pool
            .insert_ip_net_port_any(ip_net, ambient_authority())
    }

    /// Add a range of network addresses, accepting a range of ports, to
    /// per-instance networks.
    ///
    /// This grants access to the port range starting at `ports_start` and, if
    /// `ports_end` is provided, ending before `ports_end`.
    pub fn insert_ip_net_port_range(
        &mut self,
        ip_net: ipnet::IpNet,
        ports_start: u16,
        ports_end: Option<u16>,
    ) {
        self.pool
            .insert_ip_net_port_range(ip_net, ports_start, ports_end, ambient_authority())
    }

    /// Add a range of network addresses with a specific port to the pool.
    pub fn insert_ip_net(&mut self, ip_net: ipnet::IpNet, port: u16) {
        self.pool.insert_ip_net(ip_net, port, ambient_authority())
    }

    pub fn table(&self) -> &Table {
        &self.table
    }

    pub fn table_mut(&mut self) -> &mut Table {
        &mut self.table
    }
}
