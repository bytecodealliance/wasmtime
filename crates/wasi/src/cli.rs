#[repr(transparent)]
pub struct WasiCliImpl<T>(pub T);

impl<T: WasiCliView> WasiCliView for &mut T {
    type InputStream = T::InputStream;
    type OutputStream = T::OutputStream;

    fn cli(&mut self) -> &WasiCliCtx<T::InputStream, T::OutputStream> {
        (**self).cli()
    }
}

impl<T: WasiCliView> WasiCliView for WasiCliImpl<T> {
    type InputStream = T::InputStream;
    type OutputStream = T::OutputStream;

    fn cli(&mut self) -> &WasiCliCtx<T::InputStream, T::OutputStream> {
        self.0.cli()
    }
}

impl<I: Send, O: Send> WasiCliView for WasiCliCtx<I, O> {
    type InputStream = I;
    type OutputStream = O;

    fn cli(&mut self) -> &WasiCliCtx<I, O> {
        self
    }
}

pub trait WasiCliView: Send {
    type InputStream;
    type OutputStream;

    fn cli(&mut self) -> &WasiCliCtx<Self::InputStream, Self::OutputStream>;
}

#[derive(Default)]
pub struct WasiCliCtx<I, O> {
    pub environment: Vec<(String, String)>,
    pub arguments: Vec<String>,
    pub initial_cwd: Option<String>,
    pub stdin: I,
    pub stdout: O,
    pub stderr: O,
}

pub trait IsTerminal {
    /// Returns whether this stream is backed by a TTY.
    fn is_terminal(&self) -> bool;
}

impl IsTerminal for tokio::io::Empty {
    fn is_terminal(&self) -> bool {
        false
    }
}

impl IsTerminal for std::io::Empty {
    fn is_terminal(&self) -> bool {
        false
    }
}

impl IsTerminal for tokio::io::Stdin {
    fn is_terminal(&self) -> bool {
        std::io::stdin().is_terminal()
    }
}

impl IsTerminal for std::io::Stdin {
    fn is_terminal(&self) -> bool {
        std::io::IsTerminal::is_terminal(self)
    }
}

impl IsTerminal for tokio::io::Stdout {
    fn is_terminal(&self) -> bool {
        std::io::stdout().is_terminal()
    }
}

impl IsTerminal for std::io::Stdout {
    fn is_terminal(&self) -> bool {
        std::io::IsTerminal::is_terminal(self)
    }
}

impl IsTerminal for tokio::io::Stderr {
    fn is_terminal(&self) -> bool {
        std::io::stderr().is_terminal()
    }
}

impl IsTerminal for std::io::Stderr {
    fn is_terminal(&self) -> bool {
        std::io::IsTerminal::is_terminal(self)
    }
}
