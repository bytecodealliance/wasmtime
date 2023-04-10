use crate::clocks::WasiClocks;
use crate::dir::WasiDir;
use crate::file::WasiFile;
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

pub struct WasiCtx {
    pub random: Box<dyn RngCore + Send + Sync>,
    pub clocks: WasiClocks,
    pub sched: Box<dyn WasiSched>,
    pub table: Table,
    pub env: Vec<(String, String)>,
    pub args: Vec<String>,
    pub preopens: Vec<(Box<dyn WasiDir>, String)>,
    pub pool: Pool,
    pub network_creator: Box<dyn Fn(Pool) -> Result<Box<dyn WasiNetwork>, Error> + Send + Sync>,
    pub tcp_socket_creator:
        Box<dyn Fn(AddressFamily) -> Result<Box<dyn WasiTcpSocket>, Error> + Send + Sync>,
}

impl WasiCtx {
    pub fn new(
        random: Box<dyn RngCore + Send + Sync>,
        clocks: WasiClocks,
        sched: Box<dyn WasiSched>,
        table: Table,
        network_creator: Box<dyn Fn(Pool) -> Result<Box<dyn WasiNetwork>, Error> + Send + Sync>,
        tcp_socket_creator: Box<
            dyn Fn(AddressFamily) -> Result<Box<dyn WasiTcpSocket>, Error> + Send + Sync,
        >,
    ) -> Self {
        let mut s = WasiCtx {
            random,
            clocks,
            sched,
            table,
            env: Vec::new(),
            args: Vec::new(),
            preopens: Vec::new(),
            pool: Pool::new(),
            network_creator,
            tcp_socket_creator,
        };
        s.set_stdin(Box::new(crate::pipe::ReadPipe::new(std::io::empty())));
        s.set_stdout(Box::new(crate::pipe::WritePipe::new(std::io::sink())));
        s.set_stderr(Box::new(crate::pipe::WritePipe::new(std::io::sink())));
        s
    }

    pub fn insert_file(&mut self, fd: u32, file: Box<dyn WasiFile>) {
        self.table_mut().insert_at(fd, Box::new(file));
    }

    pub fn insert_input_stream(&mut self, fd: u32, stream: Box<dyn InputStream>) {
        self.table_mut().insert_at(fd, Box::new(stream));
    }

    pub fn insert_output_stream(&mut self, fd: u32, stream: Box<dyn OutputStream>) {
        self.table_mut().insert_at(fd, Box::new(stream));
    }

    pub fn insert_listener(&mut self, fd: u32, listener: Box<dyn WasiTcpSocket>) {
        self.table_mut().insert_at(fd, Box::new(listener));
    }

    pub fn push_file(&mut self, file: Box<dyn WasiFile>) -> Result<u32, Error> {
        self.table_mut().push(Box::new(file))
    }

    pub fn insert_dir(&mut self, fd: u32, dir: Box<dyn WasiDir>) {
        self.table_mut().insert_at(fd, Box::new(dir))
    }

    pub fn push_dir(&mut self, dir: Box<dyn WasiDir>) -> Result<u32, Error> {
        self.table_mut().push(Box::new(dir))
    }

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

    pub fn set_stdin(&mut self, s: Box<dyn InputStream>) {
        self.insert_input_stream(0, s);
    }

    pub fn set_stdout(&mut self, s: Box<dyn OutputStream>) {
        self.insert_output_stream(1, s);
    }

    pub fn set_stderr(&mut self, s: Box<dyn OutputStream>) {
        self.insert_output_stream(2, s);
    }

    pub fn push_env(&mut self, name: &str, value: &str) {
        self.env.push((name.to_owned(), value.to_owned()))
    }

    pub fn push_arg(&mut self, arg: &str) {
        self.args.push(arg.to_owned())
    }

    pub fn set_args(&mut self, args: &[impl AsRef<str>]) {
        self.args = args
            .iter()
            .map(|a| a.as_ref().to_string())
            .collect::<Vec<String>>();
    }

    pub fn push_preopened_dir(&mut self, dir: Box<dyn WasiDir>, path: &str) -> anyhow::Result<()> {
        self.preopens.push((dir, path.to_owned()));
        Ok(())
    }
}
