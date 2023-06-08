# AssemblyScript

[AssemblyScript] has included support for targeting WASI since version 0.10.0. If you're not familiar with AssemblyScript, check out the [docs](https://www.assemblyscript.org/introduction.html) or the [discord server](https://discord.gg/assemblyscript).
To setup this demo, you need a valid installation of [NodeJS](https://nodejs.org/), [Deno](https://deno.com/runtime), or [Bun](https://bun.sh/) along with a installation of [wasmtime](https://github.com/bytecodealliance/wasmtime).

For the rest of this documentation, we'll default to NPM as our package manager. Feel free to use the manager of your choice.

Let's walk through a simple hello world example using the latest AssemblyScript runtime (at the time of this writing, it is AssemblyScript runtime included in version 0.27.x):

## Hello World!

Enabling WASI support in AssemblyScript requires some configuration and dependencies in order to compile with WASI support.

First, we'll install [assemblyscript](https://github.com/AssemblyScript/AssemblyScript) along with [wasi-shim](https://github.com/AssemblyScript/wasi-shim) which is a plugin that adds support for WASI.

```sh
$ npm install --save-dev assemblyscript @assemblyscript/wasi-shim
```

Next, we'll use the built in `asinit` command to create our project files. When prompted, type `y` and return.

```sh
$ npx asinit .
```

Next, we need to configure our project to use WASI as a build target. Navigate to `asconfig.json` and add the following line.

`asconfig.json`
```json
{
    // "targets": { ... },
    "extends": "./node_modules/@assemblyscript/wasi-shim/asconfig.json"
}
```

With AssemblyScript now configured to use WASI, we can enter `./assembly/index.ts` and change it to the following. This will tell WASI to print the string to the terminal.

`assembly/index.ts`
```js
console.log("Hello World!");
```

Now, compile our WASI module using the `asc` command and run it using `wasmtime`.

```sh
$ npx asc assembly/index.ts --target release
$ wasmtime ./build/release.wasm
```

Now that we know how to use WASI, we'll test the capabilities of WASI using a demo.

## WASI Demo

First, clone the [wasmtime](https://github.com/bytecodealliance/wasmtime) repository and navigate to the `docs/assemblyscript_demo` directory.

```sh
$ git clone https://github.com/bytecodealliance/wasmtime
$ cd wasmtime/docs/assemblyscript_demo
```

Install our dependencies with NPM or your preferred package manager.

```sh
$ npm install
```

Take a look at the code in `docs/assemblyscript_demo/wasi-demo.ts` and then build the WASM/WASI binary by running

```sh
$ npx asc wasi-demo.ts --target wasi-demo
```

Lastly, run the demo using

```sh
$ wasmtime ./build/wasi-demo.wasm
```
