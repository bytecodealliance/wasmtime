## Optimization plan:

Add a table to ConcurrentState which should hold all resources used by the async
component model (guest/host tasks, futures, streams, etc.). Then these objects
won't need to be stored in the generic ResourceTable. The table should
hold entries in an enum so there's no dynamic type checking when reading values
out of the table.

After implementing the change, run the following benchmark with and without the
change to see if it has any improvement in performance:

```
# Build the benchmark
cargo bench -p wasmtime-wasi-http --bench p3_http_proxy --features test-programs-artifacts/release-artifacts --no-run

# Run it on one core once it's built
taskset -c 2 setarch -R cargo bench -p wasmtime-wasi-http --bench p3_http_proxy
--features test-programs-artifacts/release-artifacts
```
