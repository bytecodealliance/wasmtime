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
    fn get_stdin(&mut self) -> Result<Resource<streams::DynInputStream>, wasmtime::Error> {
        let stream = self.ctx.stdin.p2_stream();
        Ok(self.table.push(stream)?)
    }
}

impl stdout::Host for WasiCliCtxView<'_> {
    fn get_stdout(&mut self) -> Result<Resource<streams::DynOutputStream>, wasmtime::Error> {
        let stream = self.ctx.stdout.p2_stream();
        Ok(self.table.push(stream)?)
    }
}

impl stderr::Host for WasiCliCtxView<'_> {
    fn get_stderr(&mut self) -> Result<Resource<streams::DynOutputStream>, wasmtime::Error> {
        let stream = self.ctx.stderr.p2_stream();
        Ok(self.table.push(stream)?)
    }
}

pub struct TerminalInput;
pub struct TerminalOutput;

impl terminal_input::Host for WasiCliCtxView<'_> {}
impl terminal_input::HostTerminalInput for WasiCliCtxView<'_> {
    fn drop(&mut self, r: Resource<TerminalInput>) -> wasmtime::Result<()> {
        self.table.delete(r)?;
        Ok(())
    }
}
impl terminal_output::Host for WasiCliCtxView<'_> {}
impl terminal_output::HostTerminalOutput for WasiCliCtxView<'_> {
    fn drop(&mut self, r: Resource<TerminalOutput>) -> wasmtime::Result<()> {
        self.table.delete(r)?;
        Ok(())
    }
}
impl terminal_stdin::Host for WasiCliCtxView<'_> {
    fn get_terminal_stdin(&mut self) -> wasmtime::Result<Option<Resource<TerminalInput>>> {
        if self.ctx.stdin.is_terminal() {
            let fd = self.table.push(TerminalInput)?;
            Ok(Some(fd))
        } else {
            Ok(None)
        }
    }
}
impl terminal_stdout::Host for WasiCliCtxView<'_> {
    fn get_terminal_stdout(&mut self) -> wasmtime::Result<Option<Resource<TerminalOutput>>> {
        if self.ctx.stdout.is_terminal() {
            let fd = self.table.push(TerminalOutput)?;
            Ok(Some(fd))
        } else {
            Ok(None)
        }
    }
}
impl terminal_stderr::Host for WasiCliCtxView<'_> {
    fn get_terminal_stderr(&mut self) -> wasmtime::Result<Option<Resource<TerminalOutput>>> {
        if self.ctx.stderr.is_terminal() {
            let fd = self.table.push(TerminalOutput)?;
            Ok(Some(fd))
        } else {
            Ok(None)
        }
    }
}

mod named {
    use crate::cli::WasiCliNamedView;
    use crate::p2::bindings::named_imports::wasi::cli::{
        stderr, stdin, stdout, terminal_input, terminal_output, terminal_stderr, terminal_stdin,
        terminal_stdout,
    };
    use crate::p2::stdio::{TerminalInput, TerminalOutput};
    use crate::{NamedId, WasiCtxNamedView};
    use wasmtime::component::Resource;
    use wasmtime_wasi_io::streams;

    impl<T> stdin::Host for WasiCtxNamedView<'_, T>
    where
        T: WasiCliNamedView,
    {
        fn get_stdin(
            &mut self,
            id: NamedId,
        ) -> Result<Resource<streams::DynInputStream>, wasmtime::Error> {
            super::stdin::Host::get_stdin(&mut self.0.cli(id))
        }
    }

    impl<T> stdout::Host for WasiCtxNamedView<'_, T>
    where
        T: WasiCliNamedView,
    {
        fn get_stdout(
            &mut self,
            id: NamedId,
        ) -> Result<Resource<streams::DynOutputStream>, wasmtime::Error> {
            super::stdout::Host::get_stdout(&mut self.0.cli(id))
        }
    }

    impl<T> stderr::Host for WasiCtxNamedView<'_, T>
    where
        T: WasiCliNamedView,
    {
        fn get_stderr(
            &mut self,
            id: NamedId,
        ) -> Result<Resource<streams::DynOutputStream>, wasmtime::Error> {
            super::stderr::Host::get_stderr(&mut self.0.cli(id))
        }
    }

    impl<T> terminal_input::Host for WasiCtxNamedView<'_, T> where T: WasiCliNamedView {}
    impl<T> terminal_input::HostTerminalInput for WasiCtxNamedView<'_, T>
    where
        T: WasiCliNamedView,
    {
        fn drop(&mut self, id: NamedId, r: Resource<TerminalInput>) -> wasmtime::Result<()> {
            super::terminal_input::HostTerminalInput::drop(&mut self.0.cli(id), r)
        }
    }

    impl<T> terminal_output::Host for WasiCtxNamedView<'_, T> where T: WasiCliNamedView {}
    impl<T> terminal_output::HostTerminalOutput for WasiCtxNamedView<'_, T>
    where
        T: WasiCliNamedView,
    {
        fn drop(&mut self, id: NamedId, r: Resource<TerminalOutput>) -> wasmtime::Result<()> {
            super::terminal_output::HostTerminalOutput::drop(&mut self.0.cli(id), r)
        }
    }

    impl<T> terminal_stdin::Host for WasiCtxNamedView<'_, T>
    where
        T: WasiCliNamedView,
    {
        fn get_terminal_stdin(
            &mut self,
            id: NamedId,
        ) -> wasmtime::Result<Option<Resource<TerminalInput>>> {
            super::terminal_stdin::Host::get_terminal_stdin(&mut self.0.cli(id))
        }
    }

    impl<T> terminal_stdout::Host for WasiCtxNamedView<'_, T>
    where
        T: WasiCliNamedView,
    {
        fn get_terminal_stdout(
            &mut self,
            id: NamedId,
        ) -> wasmtime::Result<Option<Resource<TerminalOutput>>> {
            super::terminal_stdout::Host::get_terminal_stdout(&mut self.0.cli(id))
        }
    }

    impl<T> terminal_stderr::Host for WasiCtxNamedView<'_, T>
    where
        T: WasiCliNamedView,
    {
        fn get_terminal_stderr(
            &mut self,
            id: NamedId,
        ) -> wasmtime::Result<Option<Resource<TerminalOutput>>> {
            super::terminal_stderr::Host::get_terminal_stderr(&mut self.0.cli(id))
        }
    }
}
