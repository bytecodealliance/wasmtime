# Wasmtime for .NET

A .NET API for [Wasmtime](https://github.com/bytecodealliance/wasmtime).

Wasmtime for .NET enables .NET code to instantiate WebAssembly modules and to interact with them in-process.

# Getting Started

## Prerequisites

### .NET Core 3.0

Install a [.NET Core 3.0+ SDK](https://dotnet.microsoft.com/download) for your operating system.

## Introduction to Wasmtime for .NET

See the [introduction to Wasmtime for .NET](https://peterhuene.github.io/wasmtime.net/articles/intro.html) for a complete walkthrough of how to use Wasmtime for .NET.

# Wasmtime for .NET API documentation

See the [Wasmtime for .NET API documentation](https://peterhuene.github.io/wasmtime.net/api/index.html) for documentation on using the Wasmtime for .NET types.

# Running the "Hello World" Example

The "hello world" example demonstrates a simple C# function being called from WebAssembly.

To run the "hello world" example, follow these instructions:

1. `cd examples/hello`
2. `dotnet run`

You should see a `Hello from C#, WebAssembly!` message printed.

# Building Wasmtime for .NET

To build Wasmtime for .NET, follow these instructions:

1. `cd src`.
2. `dotnet build`.

This should produce a `Wasmtime.Dotnet.dll` assembly in the `bin/Debug/netstandard2.1` directory.

To build a release version of Wasmtime for .NET, follow these instructions:

1. `cd src`.
2. `dotnet build -c Release`.

This should produce a `Wasmtime.Dotnet.dll` assembly in the `bin/Release/netstandard2.1` directory.

# Running the tests

To run the Wasmtime for .NET unit tests, follow these instructions:

1. `cd tests`.
2. `dotnet test`.

# Packing Wasmtime for .NET

To create a NuGet package for Wasmtime for .NET, follow these instructions:

1. `cd src`.
2. `dotnet pack -c Release`.

This should produce a `Wasmtime.<version>.nupkg` file in the `bin/Release` directory.

# Implementation Status

## Status

| Feature                               | Status |
|---------------------------------------|--------|
| Wasmtime engine class                 | ✅     |
| Wasmtime store class                  | ✅     |
| Wasmtime module class                 | ✅     |
| Wasmtime instance class               | 🔄     |
| Module function imports               | ✅     |
| Module global imports                 | ✅     |
| Module table imports                  | ✅     |
| Module memory imports                 | ✅     |
| Module function exports               | ✅     |
| Module global exports                 | ✅     |
| Module table exports                  | ✅     |
| Module memory exports                 | ✅     |
| Extern instance functions             | ✅     |
| Extern instance globals               | ✅️     |
| Extern instance tables                | ⬜️     |
| Extern instance memories              | ✅️     |
| Host function import binding          | ✅     |
| Host global import binding            | ✅ ️️    |
| Host table import binding             | ⬜️ ️️    |
| Host memory import binding            | ✅️ ️️    |
| `i32` parameters and return values    | ✅     |
| `i64` parameters and return values    | ✅     |
| `f32` parameters and return values    | ✅     |
| `f64` parameters and return values    | ✅     |
| `AnyRef` parameters and return values | ⬜️     |
| Tuple return types for host functions | ✅     |
| Trap messages                         | ✅     |
| Trap frames                           | ⬜️     |
| Create a NuGet package                | ✅     |

## Legend

| Status | Icon |
|-----------------|--------|
| Not implemented | ⬜️     |
| In progress     | 🔄     |
| Completed       | ✅     |
