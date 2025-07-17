mod host;

use crate::cli::{IsTerminal, WasiCliCtx};
use crate::p3::bindings::cli;
use std::sync::Arc;
use tokio::io::{
    AsyncRead, AsyncWrite, Empty, Stderr, Stdin, Stdout, empty, stderr, stdin, stdout,
};
use wasmtime::component::{HasData, Linker, ResourceTable};

pub struct WasiCliCtxView<'a> {
    pub ctx: &'a mut WasiCliCtx<Box<dyn InputStream>, Box<dyn OutputStream>>,
    pub table: &'a mut ResourceTable,
}

impl<T: WasiCliView> WasiCliView for &mut T {
    fn cli(&mut self) -> WasiCliCtxView<'_> {
        T::cli(self)
    }
}

impl<T: WasiCliView> WasiCliView for Box<T> {
    fn cli(&mut self) -> WasiCliCtxView<'_> {
        T::cli(self)
    }
}

pub trait WasiCliView: Send {
    fn cli(&mut self) -> WasiCliCtxView<'_>;
}

impl Default for WasiCliCtx<Box<dyn InputStream>, Box<dyn OutputStream>> {
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
/// use wasmtime_wasi::cli::WasiCliCtx;
/// use wasmtime_wasi::p3::cli::{InputStream, OutputStream, WasiCliView, WasiCliCtxView};
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
///     cli: WasiCliCtx<Box<dyn InputStream>, Box<dyn OutputStream>>,
///     table: ResourceTable,
/// }
///
/// impl WasiCliView for MyState {
///     fn cli(&mut self) -> WasiCliCtxView<'_> {
///         WasiCliCtxView {
///             ctx: &mut self.cli,
///             table: &mut self.table,
///         }
///     }
/// }
/// ```
pub fn add_to_linker<T>(linker: &mut Linker<T>) -> wasmtime::Result<()>
where
    T: WasiCliView + 'static,
{
    let exit_options = cli::exit::LinkOptions::default();
    add_to_linker_impl(linker, &exit_options, T::cli)
}

/// Similar to [`add_to_linker`], but with the ability to enable unstable features.
pub fn add_to_linker_with_options<T>(
    linker: &mut Linker<T>,
    exit_options: &cli::exit::LinkOptions,
) -> anyhow::Result<()>
where
    T: WasiCliView + 'static,
{
    add_to_linker_impl(linker, exit_options, T::cli)
}

pub(crate) fn add_to_linker_impl<T: Send>(
    linker: &mut Linker<T>,
    exit_options: &cli::exit::LinkOptions,
    host_getter: fn(&mut T) -> WasiCliCtxView<'_>,
) -> wasmtime::Result<()> {
    cli::exit::add_to_linker::<_, WasiCli>(linker, exit_options, host_getter)?;
    cli::environment::add_to_linker::<_, WasiCli>(linker, host_getter)?;
    cli::stdin::add_to_linker::<_, WasiCli>(linker, host_getter)?;
    cli::stdout::add_to_linker::<_, WasiCli>(linker, host_getter)?;
    cli::stderr::add_to_linker::<_, WasiCli>(linker, host_getter)?;
    cli::terminal_input::add_to_linker::<_, WasiCli>(linker, host_getter)?;
    cli::terminal_output::add_to_linker::<_, WasiCli>(linker, host_getter)?;
    cli::terminal_stdin::add_to_linker::<_, WasiCli>(linker, host_getter)?;
    cli::terminal_stdout::add_to_linker::<_, WasiCli>(linker, host_getter)?;
    cli::terminal_stderr::add_to_linker::<_, WasiCli>(linker, host_getter)?;
    Ok(())
}

struct WasiCli;

impl HasData for WasiCli {
    type Data<'a> = WasiCliCtxView<'a>;
}

pub struct TerminalInput;
pub struct TerminalOutput;

pub trait InputStream: IsTerminal + Send {
    fn reader(&self) -> Box<dyn AsyncRead + Send + Sync>;
}

impl<T: ?Sized + InputStream + Sync> InputStream for &T {
    fn reader(&self) -> Box<dyn AsyncRead + Send + Sync> {
        T::reader(self)
    }
}

impl<T: ?Sized + InputStream> InputStream for &mut T {
    fn reader(&self) -> Box<dyn AsyncRead + Send + Sync> {
        T::reader(self)
    }
}

impl<T: ?Sized + InputStream> InputStream for Box<T> {
    fn reader(&self) -> Box<dyn AsyncRead + Send + Sync> {
        T::reader(self)
    }
}

impl<T: ?Sized + InputStream + Sync> InputStream for Arc<T> {
    fn reader(&self) -> Box<dyn AsyncRead + Send + Sync> {
        T::reader(self)
    }
}

impl InputStream for Empty {
    fn reader(&self) -> Box<dyn AsyncRead + Send + Sync> {
        Box::new(empty())
    }
}

impl InputStream for std::io::Empty {
    fn reader(&self) -> Box<dyn AsyncRead + Send + Sync> {
        Box::new(empty())
    }
}

impl InputStream for Stdin {
    fn reader(&self) -> Box<dyn AsyncRead + Send + Sync> {
        Box::new(stdin())
    }
}

impl InputStream for std::io::Stdin {
    fn reader(&self) -> Box<dyn AsyncRead + Send + Sync> {
        Box::new(stdin())
    }
}

pub trait OutputStream: IsTerminal + Send {
    fn writer(&self) -> Box<dyn AsyncWrite + Send + Sync>;
}

impl<T: ?Sized + OutputStream + Sync> OutputStream for &T {
    fn writer(&self) -> Box<dyn AsyncWrite + Send + Sync> {
        T::writer(self)
    }
}

impl<T: ?Sized + OutputStream> OutputStream for &mut T {
    fn writer(&self) -> Box<dyn AsyncWrite + Send + Sync> {
        T::writer(self)
    }
}

impl<T: ?Sized + OutputStream> OutputStream for Box<T> {
    fn writer(&self) -> Box<dyn AsyncWrite + Send + Sync> {
        T::writer(self)
    }
}

impl<T: ?Sized + OutputStream + Sync> OutputStream for Arc<T> {
    fn writer(&self) -> Box<dyn AsyncWrite + Send + Sync> {
        T::writer(self)
    }
}

impl OutputStream for Empty {
    fn writer(&self) -> Box<dyn AsyncWrite + Send + Sync> {
        Box::new(empty())
    }
}

impl OutputStream for std::io::Empty {
    fn writer(&self) -> Box<dyn AsyncWrite + Send + Sync> {
        Box::new(empty())
    }
}

impl OutputStream for Stdout {
    fn writer(&self) -> Box<dyn AsyncWrite + Send + Sync> {
        Box::new(stdout())
    }
}

impl OutputStream for std::io::Stdout {
    fn writer(&self) -> Box<dyn AsyncWrite + Send + Sync> {
        Box::new(stdout())
    }
}

impl OutputStream for Stderr {
    fn writer(&self) -> Box<dyn AsyncWrite + Send + Sync> {
        Box::new(stderr())
    }
}

impl OutputStream for std::io::Stderr {
    fn writer(&self) -> Box<dyn AsyncWrite + Send + Sync> {
        Box::new(stderr())
    }
}
