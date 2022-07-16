# WASI Document Guide

To get started using WASI, see [the intro document](WASI-intro.md) and
[the tutorial](WASI-tutorial.md).

For more detail on what WASI is, see [the overview](WASI-overview.md).

For specifics on the API, see the [API documentation](https://github.com/WebAssembly/WASI/blob/master/phases/snapshot/docs.md).
Additionally, a C header file describing the WASI API is
[here](https://github.com/WebAssembly/wasi-libc/blob/master/libc-bottom-half/headers/public/wasi/api.h).

The WASI C/C++ SDK repository is [wasi-sdk](https://github.com/WebAssembly/wasi-sdk/).

The WASI libc repository, used by wasi-sdk, is [wasi-libc](https://github.com/WebAssembly/wasi-libc/).

For some discussion of capability-based design, see the [Capabilities document](WASI-capabilities.md).

For some discussion of WASI's design inspiration, see the [Background document](WASI-background.md).

For background on some of the design decisions in WASI, see [the rationale](WASI-rationale.md).

For documentation of the exports required of programs using, see
[the application ABI](https://github.com/WebAssembly/WASI/blob/main/legacy/application-abi.md).

For some ideas of things that we may want to change about WASI in the
short term, see the [possible changes](WASI-some-possible-changes.md) document.
For longer-term ideas, see the [possible future features](WASI-possible-future-features.md)
document.
