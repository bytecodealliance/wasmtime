# AssemblyScript

[AssemblyScript] has included support for targeting WASI since version 0.10.0. To use it, add
`import "wasi"` at the top of your entrypoint file.

Let's walk through a simple hello world example using the latest AssemblyScript runtime (at the time of this writing, it is AssemblyScript runtime included in version 0.19.x):

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
[Wasmtime API]: ./lang.md
