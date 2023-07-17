# Using `samply` on Linux/macOS

One profiler supported by Wasmtime is [`samply`](https://github.com/mstange/samply) for Linux and macOS. As of 17th July 2023, the
latest version of samply (on crates.io) is 0.11.0 which does not seem to support perfmaps. To use this, you either need a
newer version of samply, if by the time you read this, a newer version has been released, or you can build samply from source.

## Profiling with `perfmap`

Simple profiling support with `samply` generates a "perfmap" file that the `samply` CLI will
automatically look for, when running into unresolved symbols. This requires runtime support from
Wasmtime itself, so you will need to manually change a few things to enable profiling support in
your application. Enabling runtime support depends on how you're using Wasmtime:

* **Rust API** - you'll want to call the [`Config::profiler`] method with
  `ProfilingStrategy::PerfMap` to enable profiling of your wasm modules.

* **C API** - you'll want to call the `wasmtime_config_profiler_set` API with a
  `WASMTIME_PROFILING_STRATEGY_PERFMAP` value.

* **Command Line** - you'll want to pass the `--profile=perfmap` flag on the command
  line.

Once perfmap support is enabled, you'll use `samply record` like usual to record
your application's performance.

For example if you're using the CLI, you'll execute:

```sh
$ samply record wasmtime --profile=perfmap foo.wasm
```

This will record your application's performance and open the Firefox profiler UI to view the
results. It will also dump its own profile data to a json file (called `profile.json`) in the current directory.

Note that support for perfmap is still relatively new in Wasmtime, so if you
have any problems, please don't hesitate to [file an issue]!

[file an issue]: https://github.com/bytecodealliance/wasmtime/issues/new


