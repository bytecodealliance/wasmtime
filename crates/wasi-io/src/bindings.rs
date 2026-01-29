wasmtime::component::bindgen!({
    path: "wit",
    with: {
        "wasi:io/poll.pollable": crate::poll::DynPollable,
        "wasi:io/streams.input-stream": crate::streams::DynInputStream,
        "wasi:io/streams.output-stream": crate::streams::DynOutputStream,
        "wasi:io/error.error": crate::streams::Error,
    },
    imports: {
        "wasi:io/poll.poll": async | trappable | tracing,
        "wasi:io/poll.[method]pollable.block": async | trappable | tracing,
        "wasi:io/poll.[method]pollable.ready": async | trappable | tracing,
        "wasi:io/streams.[method]input-stream.blocking-read": async | trappable | tracing,
        "wasi:io/streams.[method]input-stream.blocking-skip": async | trappable | tracing,
        "wasi:io/streams.[drop]input-stream": async | trappable | tracing,
        "wasi:io/streams.[method]output-stream.blocking-splice": async | trappable | tracing,
        "wasi:io/streams.[method]output-stream.blocking-flush": async | trappable | tracing,
        "wasi:io/streams.[method]output-stream.blocking-write-and-flush": async | trappable | tracing,
        "wasi:io/streams.[method]output-stream.blocking-write-zeroes-and-flush": async | trappable | tracing,
        "wasi:io/streams.[drop]output-stream": async | trappable,
        default: trappable | tracing,
    },
    trappable_error_type: {
        "wasi:io/streams.stream-error" => crate::streams::StreamError,
    }
});
