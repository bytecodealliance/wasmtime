# P3 HTTP Optimization Design Doc

## High Level Goals

The overarching goal is to optimize the performance of a Wasm guests running in
wasmtime as an HTTP server or proxy. These guests will generally be inspecting and
manipulating headers, but passing through HTTP bodies untouched. We only care
about the optimization of the wasm component itself and the direct
implementation of host functions that it calls. How downstream request are sent
out is out of scope because users will be responsible for suplying their own
implementations of the WasiHttpHooks trait.

## Targets

Optimize the benchmark existing on the abk/proxy_profiling branch which runs a
simple P3 proxy. Reduce the latency as much as possible using techniques which
will generally be helpful for the High Level Goals. Don't apply optimizations
which would only help this benchmark and not help general P3 proxy/servers.

## Benchmarks
In order to assist with optimization. Create benchmarks which measure the time
for a wasm guest to execute the WASI APIs to create a request (request.new),
consume a body (request.consume_body), and send a request (request.send).

## Running Benchmarks
When running benchmarks or profiling them, be sure to use `taskset -c 2` and
`setarch -R`. Additionally be sure to apply the settings in `~/bench-tune.sh`
when benchmarking.
