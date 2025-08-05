use crate::cli::{IsTerminal, WasiCliCtxView};
use crate::p2::bindings::cli::{
    stderr, stdin, stdout, terminal_input, terminal_output, terminal_stderr, terminal_stdin,
    terminal_stdout,
};
use wasmtime::component::Resource;
use wasmtime_wasi_io::streams;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IsATTY {
    Yes,
    No,
}

impl stdin::Host for WasiCliCtxView<'_> {
    fn get_stdin(&mut self) -> Result<Resource<streams::DynInputStream>, anyhow::Error> {
        let stream = self.ctx.stdin.p2_stream();
        Ok(self.table.push(stream)?)
    }
}

impl stdout::Host for WasiCliCtxView<'_> {
    fn get_stdout(&mut self) -> Result<Resource<streams::DynOutputStream>, anyhow::Error> {
        let stream = self.ctx.stdout.p2_stream();
        Ok(self.table.push(stream)?)
    }
}

impl stderr::Host for WasiCliCtxView<'_> {
    fn get_stderr(&mut self) -> Result<Resource<streams::DynOutputStream>, anyhow::Error> {
        let stream = self.ctx.stderr.p2_stream();
        Ok(self.table.push(stream)?)
    }
}

pub struct TerminalInput;
pub struct TerminalOutput;

impl terminal_input::Host for WasiCliCtxView<'_> {}
impl terminal_input::HostTerminalInput for WasiCliCtxView<'_> {
    fn drop(&mut self, r: Resource<TerminalInput>) -> anyhow::Result<()> {
        self.table.delete(r)?;
        Ok(())
    }
}
impl terminal_output::Host for WasiCliCtxView<'_> {}
impl terminal_output::HostTerminalOutput for WasiCliCtxView<'_> {
    fn drop(&mut self, r: Resource<TerminalOutput>) -> anyhow::Result<()> {
        self.table.delete(r)?;
        Ok(())
    }
}
impl terminal_stdin::Host for WasiCliCtxView<'_> {
    fn get_terminal_stdin(&mut self) -> anyhow::Result<Option<Resource<TerminalInput>>> {
        if self.ctx.stdin.is_terminal() {
            let fd = self.table.push(TerminalInput)?;
            Ok(Some(fd))
        } else {
            Ok(None)
        }
    }
}
impl terminal_stdout::Host for WasiCliCtxView<'_> {
    fn get_terminal_stdout(&mut self) -> anyhow::Result<Option<Resource<TerminalOutput>>> {
        if self.ctx.stdout.is_terminal() {
            let fd = self.table.push(TerminalOutput)?;
            Ok(Some(fd))
        } else {
            Ok(None)
        }
    }
}
impl terminal_stderr::Host for WasiCliCtxView<'_> {
    fn get_terminal_stderr(&mut self) -> anyhow::Result<Option<Resource<TerminalOutput>>> {
        if self.ctx.stderr.is_terminal() {
            let fd = self.table.push(TerminalOutput)?;
            Ok(Some(fd))
        } else {
            Ok(None)
        }
    }
}
