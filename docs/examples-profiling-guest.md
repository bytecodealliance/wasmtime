# Using Wasmtime's cross-platform profiler

The guest profiling strategy enables in-process sampling and will write the
captured profile to a file which can be viewed at
<https://profiler.firefox.com/>.

To use this profiler with the Wasmtime CLI, pass the
`--profile=guest[,path[,interval]]` flag.

- `path` is where to write the profile, `wasmtime-guest-profile.json` by default
- `interval` is the duration between samples, 10ms by default

When used with `--wasm-timeout`, the timeout will be rounded up to the nearest
multiple of the profiling interval.
