mod host;

use crate::cli::{IsTerminal, WasiCliCtx, WasiCliImpl, WasiCliView};
use crate::p3::bindings::cli;
use std::rc::Rc;
use std::sync::Arc;
use tokio::io::{
    AsyncRead, AsyncWrite, Empty, Stderr, Stdin, Stdout, empty, stderr, stdin, stdout,
};
use wasmtime::component::{HasData, Linker};

impl Default for WasiCliCtx<Box<dyn InputStream + Send>, Box<dyn OutputStream + Send>> {
    fn default() -> Self {
        Self {
            environment: Vec::default(),
            arguments: Vec::default(),
            initial_cwd: None,
            stdin: Box::new(empty()),
            stdout: Box::new(empty()),
            stderr: Box::new(empty()),
        }
    }
}

/// Add all WASI interfaces from this module into the `linker` provided.
///
/// This function will add all interfaces implemented by this module to the
/// [`Linker`], which corresponds to the `wasi:cli/imports` world supported by
/// this module.
///
/// This is low-level API for advanced use cases,
/// [`wasmtime_wasi::p3::add_to_linker`](crate::p3::add_to_linker) can be used instead
/// to add *all* wasip3 interfaces (including the ones from this module) to the `linker`.
///
/// # Example
///
/// ```
/// use wasmtime::{Engine, Result, Store, Config};
/// use wasmtime::component::{Linker, ResourceTable};
/// use wasmtime_wasi::ResourceView;
/// use wasmtime_wasi::cli::{WasiCliView, WasiCliCtx};
/// use wasmtime_wasi::p3::cli::{InputStream, OutputStream};
///
/// fn main() -> Result<()> {
///     let mut config = Config::new();
///     config.async_support(true);
///     config.wasm_component_model_async(true);
///     let engine = Engine::new(&config)?;
///
///     let mut linker = Linker::<MyState>::new(&engine);
///     wasmtime_wasi::p3::cli::add_to_linker(&mut linker)?;
///     // ... add any further functionality to `linker` if desired ...
///
///     let mut store = Store::new(
///         &engine,
///         MyState::default(),
///     );
///
///     // ... use `linker` to instantiate within `store` ...
///
///     Ok(())
/// }
///
/// #[derive(Default)]
/// struct MyState {
///     cli: WasiCliCtx<Box<dyn InputStream + Send>, Box<dyn OutputStream + Send>>,
///     table: ResourceTable,
/// }
///
/// impl ResourceView for MyState {
///     fn table(&mut self) -> &mut ResourceTable { &mut self.table }
/// }
///
/// impl WasiCliView for MyState {
///     type InputStream = Box<dyn InputStream + Send>;
///     type OutputStream = Box<dyn OutputStream + Send>;
///
///     fn cli(&mut self) -> &WasiCliCtx<Self::InputStream, Self::OutputStream> { &self.cli }
/// }
/// ```
pub fn add_to_linker<T>(linker: &mut Linker<T>) -> wasmtime::Result<()>
where
    T: WasiCliView + 'static,
    T::InputStream: InputStream,
    T::OutputStream: OutputStream,
{
    let exit_options = cli::exit::LinkOptions::default();
    add_to_linker_impl(linker, &exit_options, |x| WasiCliImpl(x))
}

/// Similar to [`add_to_linker`], but with the ability to enable unstable features.
pub fn add_to_linker_with_options<T>(
    linker: &mut Linker<T>,
    exit_options: &cli::exit::LinkOptions,
) -> anyhow::Result<()>
where
    T: WasiCliView + 'static,
    T::InputStream: InputStream,
    T::OutputStream: OutputStream,
{
    add_to_linker_impl(linker, exit_options, |x| WasiCliImpl(x))
}

pub(crate) fn add_to_linker_impl<T, U>(
    linker: &mut Linker<T>,
    exit_options: &cli::exit::LinkOptions,
    host_getter: fn(&mut T) -> WasiCliImpl<&mut U>,
) -> wasmtime::Result<()>
where
    T: Send,
    U: WasiCliView + 'static,
    U::InputStream: InputStream,
    U::OutputStream: OutputStream,
{
    cli::exit::add_to_linker::<_, WasiCli<U>>(linker, exit_options, host_getter)?;
    cli::environment::add_to_linker::<_, WasiCli<U>>(linker, host_getter)?;
    cli::stdin::add_to_linker::<_, WasiCli<U>>(linker, host_getter)?;
    cli::stdout::add_to_linker::<_, WasiCli<U>>(linker, host_getter)?;
    cli::stderr::add_to_linker::<_, WasiCli<U>>(linker, host_getter)?;
    cli::terminal_input::add_to_linker::<_, WasiCli<U>>(linker, host_getter)?;
    cli::terminal_output::add_to_linker::<_, WasiCli<U>>(linker, host_getter)?;
    cli::terminal_stdin::add_to_linker::<_, WasiCli<U>>(linker, host_getter)?;
    cli::terminal_stdout::add_to_linker::<_, WasiCli<U>>(linker, host_getter)?;
    cli::terminal_stderr::add_to_linker::<_, WasiCli<U>>(linker, host_getter)?;
    Ok(())
}

struct WasiCli<T>(T);

impl<T: 'static> HasData for WasiCli<T> {
    type Data<'a> = WasiCliImpl<&'a mut T>;
}

pub struct TerminalInput;
pub struct TerminalOutput;

pub trait InputStream: IsTerminal {
    fn reader(&self) -> Box<dyn AsyncRead + Send + Sync + Unpin>;
}

impl<T: ?Sized + InputStream> InputStream for &T {
    fn reader(&self) -> Box<dyn AsyncRead + Send + Sync + Unpin> {
        (**self).reader()
    }
}

impl<T: ?Sized + InputStream> InputStream for &mut T {
    fn reader(&self) -> Box<dyn AsyncRead + Send + Sync + Unpin> {
        (**self).reader()
    }
}

impl<T: ?Sized + InputStream> InputStream for Box<T> {
    fn reader(&self) -> Box<dyn AsyncRead + Send + Sync + Unpin> {
        (**self).reader()
    }
}

impl<T: ?Sized + InputStream> InputStream for Rc<T> {
    fn reader(&self) -> Box<dyn AsyncRead + Send + Sync + Unpin> {
        (**self).reader()
    }
}

impl<T: ?Sized + InputStream> InputStream for Arc<T> {
    fn reader(&self) -> Box<dyn AsyncRead + Send + Sync + Unpin> {
        (**self).reader()
    }
}

impl InputStream for Empty {
    fn reader(&self) -> Box<dyn AsyncRead + Send + Sync + Unpin> {
        Box::new(empty())
    }
}

impl InputStream for std::io::Empty {
    fn reader(&self) -> Box<dyn AsyncRead + Send + Sync + Unpin> {
        Box::new(empty())
    }
}

impl InputStream for Stdin {
    fn reader(&self) -> Box<dyn AsyncRead + Send + Sync + Unpin> {
        Box::new(stdin())
    }
}

impl InputStream for std::io::Stdin {
    fn reader(&self) -> Box<dyn AsyncRead + Send + Sync + Unpin> {
        Box::new(stdin())
    }
}

pub trait OutputStream: IsTerminal {
    fn writer(&self) -> Box<dyn AsyncWrite + Send + Sync + Unpin>;
}

impl<T: ?Sized + OutputStream> OutputStream for &T {
    fn writer(&self) -> Box<dyn AsyncWrite + Send + Sync + Unpin> {
        (**self).writer()
    }
}

impl<T: ?Sized + OutputStream> OutputStream for &mut T {
    fn writer(&self) -> Box<dyn AsyncWrite + Send + Sync + Unpin> {
        (**self).writer()
    }
}

impl<T: ?Sized + OutputStream> OutputStream for Box<T> {
    fn writer(&self) -> Box<dyn AsyncWrite + Send + Sync + Unpin> {
        (**self).writer()
    }
}

impl<T: ?Sized + OutputStream> OutputStream for Rc<T> {
    fn writer(&self) -> Box<dyn AsyncWrite + Send + Sync + Unpin> {
        (**self).writer()
    }
}

impl<T: ?Sized + OutputStream> OutputStream for Arc<T> {
    fn writer(&self) -> Box<dyn AsyncWrite + Send + Sync + Unpin> {
        (**self).writer()
    }
}

impl OutputStream for Empty {
    fn writer(&self) -> Box<dyn AsyncWrite + Send + Sync + Unpin> {
        Box::new(empty())
    }
}

impl OutputStream for std::io::Empty {
    fn writer(&self) -> Box<dyn AsyncWrite + Send + Sync + Unpin> {
        Box::new(empty())
    }
}

impl OutputStream for Stdout {
    fn writer(&self) -> Box<dyn AsyncWrite + Send + Sync + Unpin> {
        Box::new(stdout())
    }
}

impl OutputStream for std::io::Stdout {
    fn writer(&self) -> Box<dyn AsyncWrite + Send + Sync + Unpin> {
        Box::new(stdout())
    }
}

impl OutputStream for Stderr {
    fn writer(&self) -> Box<dyn AsyncWrite + Send + Sync + Unpin> {
        Box::new(stderr())
    }
}

impl OutputStream for std::io::Stderr {
    fn writer(&self) -> Box<dyn AsyncWrite + Send + Sync + Unpin> {
        Box::new(stderr())
    }
}
