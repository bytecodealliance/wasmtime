# Using Wasmtime's cross-platform profiler

The guest profiling strategy enables in-process sampling and will write the
captured profile to a file which can be viewed at
<https://profiler.firefox.com/>.

To use this profiler with the Wasmtime CLI, pass the
`--profile=guest[,path[,interval]]` flag.

- `path` is where to write the profile, `wasmtime-guest-profile.json` by default
- `interval` is the duration between samples, 10ms by default

When used with `-W timeout=N`, the timeout will be rounded up to the nearest
multiple of the profiling interval.

To use this profiler when embedding Wasmtime, see [`GuestProfiler`]. An
embedder that profiles the same modules many times, for example once per
instance, can prepare them once with [`GuestProfilerModules`] and start each
profile with [`GuestProfiler::new_with_modules`]; `wasmtime serve` does this.

[`GuestProfiler`]: https://docs.wasmtime.dev/api/wasmtime/struct.GuestProfiler.html
[`GuestProfilerModules`]: https://docs.wasmtime.dev/api/wasmtime/struct.GuestProfilerModules.html
[`GuestProfiler::new_with_modules`]: https://docs.wasmtime.dev/api/wasmtime/struct.GuestProfiler.html#method.new_with_modules
