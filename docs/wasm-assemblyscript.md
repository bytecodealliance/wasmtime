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

Let's walk through a simple hello world example.

## `wasi-hello-world.ts`

```typescript
{{#include ./assemblyscript-hello-world/wasi-hello-world.ts}}
```

This uses [as-wasi] as a dependency to make working with the AssemblyScript WASI
bindings easier. Then, you can run:

```sh
asc wasi-hello-world.ts -b wasi-hello-world.wasm
```

to compile it to wasm, and

```sh
wasmtime wasi-hello-world.wasm
```

to run it from the command-line. Or you can instantiate it using the [Wasmtime API].

## `package.json`

It can also be packaged using a `package.json` file:

```json
{{#include ./assemblyscript-hello-world/package.json}}
```

You can also [browse this source code online][code] and clone the wasmtime
repository to run the example locally.

[code]: https://github.com/bytecodealliance/wasmtime/blob/main/docs/assemblyscript-hello-world
[AssemblyScript]: https://assemblyscript.org
[as-wasi]: https://github.com/jedisct1/as-wasi
[half runtime]: https://docs.assemblyscript.org/details/runtime#runtime-variants
[Wasmtime API]: ./lang.md
