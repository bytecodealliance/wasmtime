wasmtime::component::bindgen!({
    path: "wit",
    trappable_imports: true,
    with: {
        "wasi:io/poll/pollable": crate::poll::DynPollable,
        "wasi:io/streams/input-stream": crate::streams::DynInputStream,
        "wasi:io/streams/output-stream": crate::streams::DynOutputStream,
        "wasi:io/error/error": crate::streams::Error,
    },
    async: {
        only_imports: [
            "poll",
            "[method]pollable.block",
            "[method]pollable.ready",
            "[method]input-stream.blocking-read",
            "[method]input-stream.blocking-skip",
            "[drop]input-stream",
            "[method]output-stream.blocking-splice",
            "[method]output-stream.blocking-flush",
            "[method]output-stream.blocking-write",
            "[method]output-stream.blocking-write-and-flush",
            "[method]output-stream.blocking-write-zeroes-and-flush",
            "[drop]output-stream",
        ]
    },
    trappable_error_type: {
        "wasi:io/streams/stream-error" => crate::streams::StreamError,
    }
});
