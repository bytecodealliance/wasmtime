# Using the `wasmtime` CLI

In addition to the embedding API which allows you to use Wasmtime as a
library, the Wasmtime project also provides a `wasmtime` CLI tool to conveniently
execute WebAssembly modules from the command line.

This section will provide a guide to the `wasmtime` CLI and major functionality
that it contains. In short, however, you can execute a WebAssembly file
(actually doing work as part of the `start` function) like so:

```console
wasmtime foo.wasm
```

Or similarly if you want to invoke a "start" function, such as with WASI
modules, you can execute

```console
wasmtime --invoke _start foo.wasm
```

For more information be sure to check out [how to install the
CLI](cli-install.md), [the list of options you can
pass](cli-options.md), and [how to enable logging](cli-logging.md).

## `wasmtime serve` command

The `wasmtime serve` command runs a WASI HTTP component as a local HTTP server
for development and testing purposes.

```console
wasmtime serve app.wasm
```

This starts an HTTP server on `http://0.0.0.0:8080` that routes incoming requests
to your component.

### ⚠️ Security Warning

> **Not recommended for production use**
>
> The `wasmtime serve` command is intended solely for local development and
> testing. It **does not** implement safeguards against:
> - Unbounded outbound HTTP requests
> - Rate limiting or connection throttling
> - DDoS protections
> - Request size limits
> - TLS/HTTPS termination
>
> **Do not deploy `wasmtime serve` in a production environment** without an
> additional reverse proxy or gateway layer (e.g., Nginx, Envoy, or a dedicated
> WASI host) that enforces request limits, authentication, and security controls.

### Options

Run `wasmtime serve --help` for a complete list of options.
