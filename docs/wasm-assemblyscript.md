# AssemblyScript

[AssemblyScript] 0.10.0 includes support for targeting WASI. To use it, add
`import "wasi"` at the top of your entrypoint file.

To create a program which can be run directly as a command, pass `--runtime half`
to the AssemblyScript linker. This selects the [half runtime], which ensures that
the generated wasm module doesn't contain any extraneous exports. (This isn't
strictly required today, but the handling of extraneous exports may change in
the future, so it's encouraged. As a bonus, it also reduces code size.)

To create a program which can be loaded as a library and used from other modules,
no special options are needed.

[Here is a repository containing an example Hello World program][WASI hello world]
using WASI and AssemblyScript.

[AssemblyScript]: https://assemblyscript.org
[half runtime]: https://docs.assemblyscript.org/details/runtime#runtime-variants
[WASI hello world]: https://github.com/torch2424/as-playground/tree/master/wasi-hello-world
