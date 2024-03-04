// Generate traits for synchronous bindings.
//
// Note that this is only done for interfaces which can block, or those which
// have some functions in `only_imports` below for being async.
pub mod sync_io {
    pub(crate) mod _internal {
        use crate::{FsError, StreamError};

        wasmtime::component::bindgen!({
            path: "wit",
            interfaces: "
                    import wasi:io/poll@0.2.0;
                    import wasi:io/streams@0.2.0;
                    import wasi:filesystem/types@0.2.0;
                ",
            tracing: true,
            trappable_error_type: {
                "wasi:io/streams/stream-error" => StreamError,
                "wasi:filesystem/types/error-code" => FsError,
            },
            with: {
                "wasi:clocks/wall-clock": crate::bindings::clocks::wall_clock,
                "wasi:filesystem/types/descriptor": super::super::filesystem::types::Descriptor,
                "wasi:filesystem/types/directory-entry-stream": super::super::filesystem::types::DirectoryEntryStream,
                "wasi:io/poll/pollable": super::super::io::poll::Pollable,
                "wasi:io/streams/input-stream": super::super::io::streams::InputStream,
                "wasi:io/streams/output-stream": super::super::io::streams::OutputStream,
                "wasi:io/error/error": super::super::io::error::Error,
            }
        });
    }
    pub use self::_internal::wasi::{filesystem, io};
}

wasmtime::component::bindgen!({
    path: "wit",
    world: "wasi:cli/imports",
    tracing: true,
    async: {
        // Only these functions are `async` and everything else is sync
        // meaning that it basically doesn't need to block. These functions
        // are the only ones that need to block.
        //
        // Note that at this time `only_imports` works on function names
        // which in theory can be shared across interfaces, so this may
        // need fancier syntax in the future.
        only_imports: [
            "[method]descriptor.access-at",
            "[method]descriptor.advise",
            "[method]descriptor.change-directory-permissions-at",
            "[method]descriptor.change-file-permissions-at",
            "[method]descriptor.create-directory-at",
            "[method]descriptor.get-flags",
            "[method]descriptor.get-type",
            "[method]descriptor.is-same-object",
            "[method]descriptor.link-at",
            "[method]descriptor.lock-exclusive",
            "[method]descriptor.lock-shared",
            "[method]descriptor.metadata-hash",
            "[method]descriptor.metadata-hash-at",
            "[method]descriptor.open-at",
            "[method]descriptor.read",
            "[method]descriptor.read-directory",
            "[method]descriptor.readlink-at",
            "[method]descriptor.remove-directory-at",
            "[method]descriptor.rename-at",
            "[method]descriptor.set-size",
            "[method]descriptor.set-times",
            "[method]descriptor.set-times-at",
            "[method]descriptor.stat",
            "[method]descriptor.stat-at",
            "[method]descriptor.symlink-at",
            "[method]descriptor.sync",
            "[method]descriptor.sync-data",
            "[method]descriptor.try-lock-exclusive",
            "[method]descriptor.try-lock-shared",
            "[method]descriptor.unlink-file-at",
            "[method]descriptor.unlock",
            "[method]descriptor.write",
            "[method]input-stream.read",
            "[method]input-stream.blocking-read",
            "[method]input-stream.blocking-skip",
            "[method]input-stream.skip",
            "[method]output-stream.forward",
            "[method]output-stream.splice",
            "[method]output-stream.blocking-splice",
            "[method]output-stream.blocking-flush",
            "[method]output-stream.blocking-write",
            "[method]output-stream.blocking-write-and-flush",
            "[method]output-stream.blocking-write-zeroes-and-flush",
            "[method]directory-entry-stream.read-directory-entry",
            "poll",
            "[method]pollable.block",
            "[method]pollable.ready",
        ],
    },
    trappable_error_type: {
        "wasi:io/streams/stream-error" => crate::StreamError,
        "wasi:filesystem/types/error-code" => crate::FsError,
        "wasi:sockets/network/error-code" => crate::SocketError,
    },
    with: {
        "wasi:sockets/network/network": super::network::Network,
        "wasi:sockets/tcp/tcp-socket": super::tcp::TcpSocket,
        "wasi:sockets/udp/udp-socket": super::udp::UdpSocket,
        "wasi:sockets/udp/incoming-datagram-stream": super::udp::IncomingDatagramStream,
        "wasi:sockets/udp/outgoing-datagram-stream": super::udp::OutgoingDatagramStream,
        "wasi:sockets/ip-name-lookup/resolve-address-stream": super::ip_name_lookup::ResolveAddressStream,
        "wasi:filesystem/types/directory-entry-stream": super::filesystem::ReaddirIterator,
        "wasi:filesystem/types/descriptor": super::filesystem::Descriptor,
        "wasi:io/streams/input-stream": super::stream::InputStream,
        "wasi:io/streams/output-stream": super::stream::OutputStream,
        "wasi:io/error/error": super::stream::Error,
        "wasi:io/poll/pollable": super::poll::Pollable,
        "wasi:cli/terminal-input/terminal-input": super::stdio::TerminalInput,
        "wasi:cli/terminal-output/terminal-output": super::stdio::TerminalOutput,
    },
});

pub use wasi::*;
