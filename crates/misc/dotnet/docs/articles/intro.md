# Introduction to Wasmtime for .NET

[Wasmtime](https://github.com/bytecodealliance/wasmtime) is a standalone runtime capable of executing [WebAssembly](https://webassembly.org/) outside of a web browser.

Wasmtime for .NET is a .NET API for Wasmtime.  It enables .NET developers to easily instantiate and execute WebAssembly modules.

For this tutorial, we will create a WebAssembly module from a program written in Rust and use that WebAssembly module from a .NET Core 3.0 application.

# Creating a simple WebAssembly module

One of the reasons why WebAssembly is so exciting is that [many languages are able to target WebAssembly](https://github.com/appcypher/awesome-wasm-langs).  This means, for example, a plugin model based on WebAssembly could enable developers to write sandboxed, cross-platform plugins in any number of languages.

Here I've decided to use [Rust](https://www.rust-lang.org/) for the implementation of the WebAssembly module.  Rust is a modern systems programming language that can easily target WebAssembly.

If you wish to skip creating the WebAssembly module, download the [prebuilt WebAssembly module](https://raw.githubusercontent.com/bytecodealliance/wasmtime/master/crates/misc/dotnet/docs/wasm/intro/hello.wasm) from this tutorial, copy it to your .NET project directory, and continue from the _[Using the WebAssembly module from .NET](#using-the-webassembly-module-from-net)_ section.

## Installing a Rust toolchain

To get started with Rust, install [rustup](https://rustup.rs/), the manager for Rust toolchains.

This will install both a `rustup` command and a `cargo` command (for the active Rust toolchain) to your PATH.

## Installing the WebAssembly target

To target WebAssembly with the active Rust toolchain, install the WebAssembly [target triple](https://forge.rust-lang.org/release/platform-support.html):

```text
rustup target add wasm32-unknown-unknown
```

## Creating the Rust project

Create a new Rust library project named `hello`:

```text
cargo new --lib hello
cd hello
```

To target WebAssembly, the library needs to be built as a `cdylib` (dynamic library) rather than the default of a static Rust library.  Add the following to the `Cargo.toml` file in the project root:

```toml
[lib]
crate-type = ["cdylib"]
```

## Implementing the WebAssembly code

The WebAssembly implementation will import a `print` function from the host environment and pass it a string to print.  It will export a `run` function that will invoke the imported `print` function.

Replace the code in `src/lib.rs` with the following Rust code:

```rust
extern "C" {
    fn print(address: i32, length: i32);
}

#[no_mangle]
pub unsafe extern fn run() {
    let message = "Hello world!";
    print(message.as_ptr() as i32, message.len() as i32);
}
```

Note that this example passes the string as a pair of _address and length_.  This is because WebAssembly only supports a few core types (such as integers and floats) and a "string" has no native representation in WebAssembly.

In the future, WebAssembly will support [interface types](https://hacks.mozilla.org/2019/08/webassembly-interface-types/) that will enable higher-level abstractions of types like strings so they can be represented in a natural way.

Also note that the _address_ is not actually a physical memory address within the address space of a process but an address within the _[WebAssembly memory](https://hacks.mozilla.org/2017/07/memory-in-webassembly-and-why-its-safer-than-you-think/)_ of the module.  Thus the WebAssembly module has no direct access to the memory of the host environment.

## Building the WebAssembly module

Use `cargo build` to build the WebAssembly module:

```text
cargo build --target wasm32-unknown-unknown --release
```

This should create a `hello.wasm` file in the `target/wasm32-unknown-unknown/release` directory.  We will use `hello.wasm` in the next section of the tutorial.

As this example is very simple and does not require any of the data from the custom sections of the WebAssembly module, you may use `wasm-strip` if you have the [WebAssembly Binary Toolkit](https://github.com/WebAssembly/wabt) installed:

```text
wasm-strip target/wasm32-unknown-unknown/release/hello.wasm
```

The resulting file should be less than 200 bytes.

# Using the WebAssembly module from .NET

## Installing a .NET Core 3.0 SDK

Install a [.NET Core 3.0 SDK](https://dotnet.microsoft.com/download/dotnet-core/3.0) for your platform if you haven't already.

This will add a `dotnet` command to your PATH.

## Creating the .NET Core project

The .NET program will be a simple console application, so create a new console project with `dotnet new`:

```text
mkdir tutorial
cd tutorial
dotnet new console
```

## Referencing the Wasmtime for .NET package

To use Wasmtime for .NET from the project, we need to add a reference to the [Wasmtime NuGet package](https://www.nuget.org/packages/Wasmtime):

```text
dotnet add package --version 0.0.1-alpha1 wasmtime
```

_Note that the `--version` option is required because the package is currently prerelease._

This will add a `PackageReference` to the project file so that Wasmtime for .NET can be used.

## Implementing the .NET code

Replace the contents of `Program.cs` with the following:

```c#
using System;
using Wasmtime;

namespace Tutorial
{
    class Host : IHost
    {
        public Instance Instance { get; set; }

        [Import("print", Module="env")]
        public void Print(int address, int length)
        {
            var message = Instance.Externs.Memories[0].ReadString(address, length);
            Console.WriteLine(message);
        }
    }

    class Program
    {
        static void Main(string[] args)
        {
            using var engine = new Engine();
            using var store = engine.CreateStore();
            using var module = store.CreateModule("hello.wasm");
            using dynamic instance = module.Instantiate(new Host());

            instance.run();
        }
    }
}
```

The `Host` class is responsible for implementing the imported [functions](https://webassembly.github.io/spec/core/syntax/modules.html#functions), [globals](https://webassembly.github.io/spec/core/syntax/modules.html#globals), [memories](https://webassembly.github.io/spec/core/syntax/modules.html#memories), and [tables](https://webassembly.github.io/spec/core/syntax/modules.html#syntax-table) for the WebAssembly module.  For Wasmtime for .NET, this is done via the [`Import`](https://peterhuene.github.io/wasmtime.net/api/Wasmtime.ImportAttribute.html) attribute applied to functions and fields of type [`Global<T>`](https://peterhuene.github.io/wasmtime.net/api/Wasmtime.Global-1.html), [`MutableGlobal<T>`](https://peterhuene.github.io/wasmtime.net/api/Wasmtime.MutableGlobal-1.html), and [`Memory`](https://peterhuene.github.io/wasmtime.net/api/Wasmtime.Memory.html) (support for WebAssembly tables is not yet implemented).  The [`Instance`](https://peterhuene.github.io/wasmtime.net/api/Wasmtime.IHost.html#Wasmtime_IHost_Instance) property of the host is set during instantiation of the WebAssembly module.

Here the host is implementing an import of `print` in the `env` module, which is the default import module name for WebAssembly modules compiled using the Rust toolchain.

The [`Engine`](https://peterhuene.github.io/wasmtime.net/api/Wasmtime.Engine.html) is used to create a [`Store`](https://peterhuene.github.io/wasmtime.net/api/Wasmtime.Store.html) that will store all Wasmtime runtime objects, such as WebAssembly modules and their instantiations.

A WebAssembly module _instantiation_ is the stateful representation of a module that can be executed.  Here, the code is casting the [`Instance`](https://peterhuene.github.io/wasmtime.net/api/Wasmtime.Instance.html) to [`dynamic`](https://docs.microsoft.com/en-us/dotnet/csharp/programming-guide/types/using-type-dynamic) which allows us to easily invoke the `run` function that was exported by the WebAssembly module.

Alternatively, the `run` function could be invoked without using the runtime binding of the `dynamic` feature like this:

```c#
...
using var instance = module.Instantiate(new Host());
instance.Externs.Functions[0].Invoke();
...
```

## Building the .NET application

Use `dotnet build` to build the .NET application:

```text
dotnet build
```

This will create a `tutorial.dll` in the `bin/Debug/netcoreapp3.0` directory that implements the .NET Core application.  An executable `tutorial` (or `tutorial.exe` on Windows) should also be present in the same directory to run the application.

## Running the .NET application
 
Before running the application, we need to copy the `hello.wasm` file to the project directory.

Once the WebAssembly module is present in project directory, we can run the application:

```text
dotnet run
```

Alternatively, we can execute the program directly without building the application again:

```text
bin/Debug/netcoreapp3.0/tutorial
```

This should result in the following output:

```text
Hello world!
```

# Wrapping up

We did it!  We executed a function written in Rust from .NET and a function implemented in .NET from Rust without much trouble at all.  And, thanks to the design of WebAssembly, the Rust code was effectively sandboxed from accessing the memory of the .NET application.

Hopefully this introduction to Wasmtime for .NET has offered a small glipse of the potential of using WebAssembly from .NET.

One last note: _Wasmtime for .NET is currently in a very early stage of development and the API might change dramatically in the future_.