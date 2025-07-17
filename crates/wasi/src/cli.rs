use std::rc::Rc;
use std::sync::Arc;

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

impl<T: ?Sized + IsTerminal> IsTerminal for &T {
    fn is_terminal(&self) -> bool {
        T::is_terminal(self)
    }
}

impl<T: ?Sized + IsTerminal> IsTerminal for &mut T {
    fn is_terminal(&self) -> bool {
        T::is_terminal(self)
    }
}

impl<T: ?Sized + IsTerminal> IsTerminal for Box<T> {
    fn is_terminal(&self) -> bool {
        T::is_terminal(self)
    }
}

impl<T: ?Sized + IsTerminal> IsTerminal for Rc<T> {
    fn is_terminal(&self) -> bool {
        T::is_terminal(self)
    }
}

impl<T: ?Sized + IsTerminal> IsTerminal for Arc<T> {
    fn is_terminal(&self) -> bool {
        T::is_terminal(self)
    }
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
