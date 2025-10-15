# Calculator with WebAssembly Plugins

This example demonstrates how to embed Wasmtime
to create an application that uses plugins.
The plugins are WebAssembly components.

You can [browse this source code online](https://github.com/bytecodealliance/wasmtime/blob/main/examples/wasip2-plugins)
and clone the wasmtime repository to run this example locally.

This application is a simplified version of the application presented
in Sy Brand's blog post
["Building Native Plugin Systems with WebAssembly Components "](https://tartanllama.xyz/posts/wasm-plugins/).
Consult that blog post for a more complex example of embedding Wasmtime
and using plugins.

## The calculator

The calculator being implemented is very simple;
it takes an expression represented in prefix form, without parentheses,
on the command line. For example:

```
target/release/calculator --plugins plugins/ add 1 2
```

or

```
target/release/calculator --plugins plugins/ subtract -1 -2
```

The set of operations available is defined by the set of plugins present in
the `plugins/` directory.

Each plugin is a component that supports two operations:
* `get_plugin_name`: Returns the name of the arithmetic operation
  that this plugin implements.
* `evaluate`: Takes two signed integer arguments and returns the result
  of evaluating the operation on the arguments.

Two example plugins are included: an `add` plugin implemented in C,
and a `subtract` plugin implemented in JavaScript.
Running `cargo build --release` will generate the plugins:
`c-plugin/add.wasm` and `js-plugin/subtract.wasm`.
To run the code, you should copy both of these files into the
`plugins/` directory that you provide with the `--plugins` option.

> To build the plugins, you must install `wit-bindgen` and the WASI SDK
> (for building the C plugin) and `jco` (for building the JavaScript
> plugin). For instructions, see [the C/C++ section](https://component-model.bytecodealliance.org/language-support/c.html)
> and [the JavaScript section](https://component-model.bytecodealliance.org/language-support/javascript.html)
> of the [Component Model documentation](https://component-model.bytecodealliance.org/).

There are no nested expressions.

## WIT bindings

To define the interface for the plugins, we have to create a `.wit` file.
The contents of this file are:

```wit
package docs:calculator;

interface host {}

world plugin {
    import host;

    export get-plugin-name: func() -> string;
    export evaluate: func(x: s32, y: s32) -> s32;
}
```

The WIT file defines a world that the plugin must implement,
as well as any imports it can expect its host to provide.
In this case, the `host` interface is empty, indicating that
there is no functionality in the host that a plugin can call.

The world has two exports, indicating that the plugin must implement
two functions: a `get-plugin-name` function that returns the name
of the operation that this plugin implements, and an `evaluate` function
that does computation specific to this plugin.

In the calculator code, we write:

```wit
bindgen!("plugin");
```

which uses a macro provided by Wasmtime that automatically runs
the `wit-bindgen` tool to generate bindings for the world.

## `CalculatorState`

We use the `CalculatorState` type to represent the global state of the program,
which in this case is just a mapping from strings that represent operation names
to `PluginDesc`s. A `PluginDesc` represents the information needed in order
to execute a plugin given arguments.

## Loading plugins

The application takes a directory as a command-line argument,
which is expected to contain plugins (as .wasm files).
All plugins in the directory are loaded eagerly.

The `load_plugins()` function starts by calling the Wasmtime library's
`Engine::default()` function to create an `Engine`:

```rust
let engine = Engine::default();
```

An `Engine` is an environment for executing WebAssembly programs.
Only one `Engine` is needed regardless of how many plugins
may be executed. For more details, see the [Wasmtime crate documentation](https://docs.rs/wasmtime/latest/wasmtime/#core-concepts).

Next, it passes the engine to the Wasmtime library's `Linker::new()`
function to create a `Linker`. A `Linker` is parameterized over a state
type.
In this application, we don't need per-plugin state
(plugins implement pure functions), so the state type is `()` (the unit type).

```rust
let linker: Linker<()> = Linker::new(&engine);
```

As with the `Engine`, only one `Linker` is needed for the whole application.
A `Linker` can be used to define functions in the host (in this case, the
calculator application) that can be called by guests (in this case, plugins
loaded by the calculator). We don't define any such functions in our host,
so the linker is only used as an argument to the `instantiate()` function
(which we'll see a little bit later).

The remaining code checks that the provided path for the plugins directory
exists and is really a directory, and if so, calls `load_plugin()` on
each file in the directory that has the `.wasm` extension:

```rust
    if !plugins_dir.is_dir() {
        anyhow::bail!("plugins directory does not exist");
    }

    for entry in fs::read_dir(plugins_dir)? {
        let path = entry?.path();
        if path.is_file() && path.extension().and_then(OsStr::to_str) == Some("wasm") {
            load_plugin(state, &engine, &linker, path)?;
        }
    }
```

Next let's look at the `load_plugin()` function, which loads a single plugin.
The function begins by calling the Wasmtime library's `Component::from_file()`
function, which takes an `Engine` and the name of a binary WebAssembly file.

```rust
let component = Component::from_file(engine, &path)?;
```

`from_file()` loads and compiles WebAssembly code and creates
the in-memory representation of a component, which we assign to
the variable `component`.

The next block of code creates the dynamic representation
of the component, which has all the resources it needs and can have its
functions called:

```rust
    let (plugin_name, plugin, store) = {
        let mut store = Store::new(engine, ());
        let plugin = Plugin::instantiate(&mut store, &component, linker)?;
        (plugin.call_get_plugin_name(&mut store)?, plugin, store)
    };
```

First, it calls the Wasmtime library's `Store::new` function to create a `Store`.
A [`Store`](https://docs.rs/wasmtime/latest/wasmtime/#core-concepts)
represents the state of a particular component. Unlike an `Engine`, it's specific
to each unit of code and so it can't be re-used across different plugins.
The `Store` type is parameterized with a state type `T`, and the third argument
to `Store::new` must have type `T`.
Since we don't use host state in this example,
we pass in `()`, which has type `()`; so we get a `Store<()>` back.

Next, it calls the `Plugin::instantiate()` function, which was generated
automatically by the `bindgen!("plugin")` macro.
The function takes the `Store` we just created,
the `Component` that represents the code for the plugin,
and the `Linker` that was passed in to `load_plugin()`.
`instantiate()` returns a `Plugin`.
The `Plugin` type corresponds to the `plugin` world from our `.wit` file,
and was also automatically generated by the `bindgen!` macro.

Now that we have a fully instantiated plugin, we can call its
`get-plugin-name` function. The `call_get_plugin_name` method was
generated by the `bindgen!` macro; notice that the name
`call_get_plugin_name` is the same as `get-plugin-name` from the `.wit` file,
but with underscores in place of hyphens, and is prefixed by `call_`.
This method takes a `Store`, which in general allows use of host state
by the implementation of the method in the plugin, though not in this case
(since we have a `Store<()>`).

Finally, we update the calculator's state by associating the plugin name
with a structure containing the plugin and store:

```rust
    state
        .plugin_descs
        .insert(plugin_name, PluginDesc { plugin, store });
```

## Running the code

Finally, this line of code is responsible for actually evaluating the
expression that was provided on the command line:

```rust
args.op.run(&mut state)
```

The `BinaryOperation` struct has a `run` method that looks like this:

```rust
    fn run(self, state: &mut CalculatorState) -> anyhow::Result<()> {
        let desc = lookup_plugin(state, self.op.as_ref())?;
        let result = desc.plugin.call_evaluate(&mut desc.store, self.x, self.y)?;
        println!("{}({}, {}) = {}", self.op, self.x, self.y, result);
        Ok(())
    }
```

First, it calls `lookup_plugin`, which simply looks up the operation
name (`self.op`, which is just the name that was given on the command line)
in the hash table that was created by `load_plugins()`.
This returns a `PluginDesc`, which is defined as:

```rust
pub struct PluginDesc {
    pub plugin: Plugin,
    pub store: wasmtime::Store<()>,
}
```

Remember, the type `Plugin` corresponds to the `plugin` world and was
generated automatically by the `bindgen!` macro.

The next line of code:

```rust
let result = desc.plugin.call_evaluate(&mut desc.store, self.x, self.y)?;
```

is the one that actually calls into the plugin to do the computation.
The bindings generator guarantees that a `Plugin` has an `evaluate`
method, which we call using `call_evaluate` (the name it was given
by the bindings generator).
Like `call_get_plugin_name`, it takes a `Store` as the first argument.
The other two arguments are the ones given on the command line.
`result` is a 32-bit integer (`i32`) because that's the return type
of `call_evaluate()`.

Finally, we print the result to `stdout`.

## Writing the plugins

So far we've assumed that the `plugins` directory is populated
with plugins for all the arithmetic operations we want.
How do we actually write the plugins?
The [component model documentation](https://component-model.bytecodealliance.org)
documents how to generate WebAssembly components from various programming languages.

As part of this sample application, two plugins are provided,
one in `c-plugin/` (implementing the `add` operation), and one in `js-plugin/`
(implementing the `subtract`) operation.
The `build.rs` script shows how the code is built.

Any number of plugins could be added, compiled from any language that
has a toolchain with WebAssembly support, implementing any other arithmetic
operations.

## Wrapping up

This is a minimal example showing how to embed Wasmtime to create an
application with dynamically loaded plugins.
The application could be extended in various ways:

* Allow nested expressions (like `add(subtract(1, 2), 3)`)
* Add floating-point operations
* Add unary expressions (like `sqrt(2)`)

The basic mechanism for loading plugins would still be the same,
with only the application-specific logic changing.

