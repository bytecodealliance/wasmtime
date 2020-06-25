# Using WebAssembly from .NET

The [Wasmtime](https://www.nuget.org/packages/Wasmtime) NuGet package can be used to
programmatically interact with WebAssembly modules.

This guide will go over adding Wasmtime to your project and demonstrate a simple
example of using a WebAssembly module from C#.

Make sure you have a [.NET Core SDK 3.0 SDK or later](https://dotnet.microsoft.com/download)
installed before we get started!

## Getting started and simple example

Start by creating a new .NET Core console project:

```text
$ mkdir gcd
$ cd gcd
$ dotnet new console
```

Next, add a reference to the Wasmtime NuGet package to your project:


```text
$ dotnet add package --version 0.15.0-preview1 wasmtime
```

Copy this example WebAssembly text module into your project directory as `gcd.wat`.

```wat
{{#include ../examples/gcd.wat}}
```

This module exports a function for calculating the greatest common denominator of two numbers.

Replace the code in `Program.cs` with the following:

```c#
using System;
using Wasmtime;

namespace Tutorial
{
    class Program
    {
        static void Main(string[] args)
        {
            using var host = new Host();
            using var module = host.LoadModuleText("gcd.wat");

            using dynamic instance = host.Instantiate(module);
            Console.WriteLine($"gcd(27, 6) = {instance.gcd(27, 6)}");
        }
    }
}
```

Run the .NET core program:

```text
$ dotnet run
```

The program should output:

```text
gcd(27, 6) = 3
```


If this is the output you see, congrats! You've successfully ran your first
WebAssembly code in .NET!

## More examples and contributing

The [.NET embedding of Wasmtime repository](https://github.com/bytecodealliance/wasmtime-dotnet)
contains the source code for the Wasmtime NuGet package.

The repository also has more [examples](https://github.com/bytecodealliance/wasmtime-dotnet/tree/main/examples)
as well.

Feel free to browse those, but if you find anything missing don't
hesitate to [open an issue](https://github.com/bytecodealliance/wasmtime-dotnet/issues/new) and let us
know if you have any questions!
