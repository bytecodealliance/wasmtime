# WASI Proposals Support

The following table summarizes Wasmtime's support for WASI [proposals]. If a
proposal is not listed, then it is not supported by Wasmtime.

[proposals]: https://github.com/WebAssembly/WASI/blob/main/Proposals.md

| WASI Proposal                          | Supported in Wasmtime?  | Enabled by default?  | CLI Flag Name [^cli]        |
|----------------------------------------|-------------------------|----------------------|-----------------------------|
| [I/O][wasi-io]                         | **Yes**                 | **Yes**              | `wasi-common`               |
| [Filesystem][wasi-filesystem]          | **Yes**                 | **Yes**              | `wasi-common`               |
| [Clocks][wasi-clocks]                  | **Yes**                 | **Yes**              | `wasi-common`               |
| [Random][wasi-random]                  | **Yes**                 | **Yes**              | `wasi-common`               |
| [Poll][wasi-poll]                      | **Yes**                 | **Yes**              | `wasi-common`               |
| [Machine Learning (wasi-nn)][wasi-nn]  | **Yes**                 | No                   | `experimental-wasi-nn`      |
| [Blob Store][wasi-blob-store]          | No                      | No                   | N/A                         |
| [Crypto][wasi-crypto]                  | No                      | No                   | N/A                         |
| [Distributed Lock Service][wasi-distributed-lock-service] | No   | No                   | N/A                         |
| [gRPC][wasi-grpc]                      | No                      | No                   | N/A                         |
| [HTTP][wasi-http]                      | No                      | No                   | N/A                         |
| [Key-value Store][wasi-kv-store]       | No                      | No                   | N/A                         |
| [Message Queue][wasi-message-queue]    | No                      | No                   | N/A                         |
| [Parallel][wasi-parallel]              | No (see [#4949])        | No                   | N/A                         |
| [Pub/sub][wasi-pubsub]                 | No                      | No                   | N/A                         |
| [Runtime Config][wasi-runtime-config]  | No                      | No                   | N/A                         |
| [Sockets][wasi-sockets]                | No                      | No                   | N/A                         |
| [SQL][wasi-sql]                        | No                      | No                   | N/A                         |
| [Threads][wasi-threads]                | **Yes**                 | No                   | `experimental-wasi-threads` |

[^cli]: The CLI flag name refers to to the `--wasi-modules` argument of the
    `wasmtime` executable; e.g., `--wasi-modules=wasi-crypto`. See `wasmtime run
    --help` for more information on the flag's default value and configuration.
[^crypto]: Build Wasmtime with `--features=wasi-crypto` to enable this.

[#4949]: https://github.com/bytecodealliance/wasmtime/pull/4949
[wasi-blob-store]: https://github.com/WebAssembly/wasi-blob-store
[wasi-clocks]: https://github.com/WebAssembly/wasi-clocks
[wasi-classic-command]: https://github.com/WebAssembly/wasi-classic-command
[wasi-crypto]: https://github.com/WebAssembly/wasi-crypto
[wasi-data]: https://github.com/singlestore-labs/wasi-data
[wasi-distributed-lock-service]: https://github.com/WebAssembly/wasi-distributed-lock-service
[wasi-filesystem]: https://github.com/WebAssembly/wasi-filesystem
[wasi-grpc]: https://github.com/WebAssembly/wasi-grpc
[wasi-handle-index]: https://github.com/WebAssembly/wasi-handle-index
[wasi-http]: https://github.com/WebAssembly/wasi-http
[wasi-io]: https://github.com/WebAssembly/wasi-io
[wasi-kv-store]: https://github.com/WebAssembly/wasi-kv-store
[wasi-message-queue]: https://github.com/WebAssembly/wasi-message-queue
[wasi-misc]: https://github.com/WebAssembly/wasi-misc
[wasi-threads]: https://github.com/WebAssembly/wasi-native-threads
[wasi-nn]: https://github.com/WebAssembly/wasi-nn
[wasi-random]: https://github.com/WebAssembly/wasi-random
[wasi-parallel]: https://github.com/WebAssembly/wasi-parallel
[wasi-poll]: https://github.com/WebAssembly/wasi-poll
[wasi-proxy-wasm]: https://github.com/proxy-wasm/spec
[wasi-pubsub]: https://github.com/WebAssembly/wasi-pubsub
[wasi-runtime-config]: https://github.com/WebAssembly/wasi-runtime-config
[wasi-sockets]: https://github.com/WebAssembly/wasi-sockets
[wasi-sql]: https://github.com/WebAssembly/wasi-sql
[wasi-url]: https://github.com/WebAssembly/wasi-url
