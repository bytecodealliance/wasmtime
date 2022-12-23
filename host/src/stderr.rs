use crate::{wasi_stderr, WasiCtx};
use is_terminal::IsTerminal;
#[cfg(unix)]
use std::os::unix::io::AsRawFd;
#[cfg(windows)]
use std::os::windows::io::AsRawHandle;

#[async_trait::async_trait]
impl wasi_stderr::WasiStderr for WasiCtx {
    async fn print(&mut self, message: String) -> anyhow::Result<()> {
        eprint!("{}", message);
        Ok(())
    }

    async fn is_terminal(&mut self) -> anyhow::Result<bool> {
        Ok(std::io::stderr().is_terminal())
    }

    async fn num_columns(&mut self) -> anyhow::Result<Option<u16>> {
        #[cfg(unix)]
        {
            Ok(
                terminal_size::terminal_size_using_fd(std::io::stderr().as_raw_fd())
                    .map(|(width, _height)| width.0),
            )
        }

        #[cfg(windows)]
        {
            Ok(
                terminal_size::terminal_size_using_handle(std::io::stderr().as_raw_handle())
                    .map(|(width, _height)| width.0),
            )
        }
    }
}
