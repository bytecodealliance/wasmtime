# Go

Wasmtime [is available as a Go
Module](https://pkg.go.dev/github.com/bytecodealliance/wasmtime-go). This guide
will go over adding Wasmtime to your project, and some provided examples of what
can be done with WebAssembly modules.

Make sure you're using Go 1.12 or later with modules support.

## Getting started and simple example

First up you'll want to start a new module:

```sh
$ mkdir hello-wasm
$ cd hello-wasm
$ go mod init hello-wasm
```

Next, copy this example WebAssembly text module into your project. It exports a
function for calculating the greatest common denominator of two numbers.

```wat
{{#include ../examples/gcd.wat}}
```

Next, we can write our code in `main.go` which reads this file and runs it:

```go
package main

import (
    "fmt"
    "github.com/bytecodealliance/wasmtime-go"
)

func main() {
    engine := wasmtime.NewEngine()
    store := wasmtime.NewStore(engine)
    module, err := wasmtime.NewModuleFromFile(engine, "gcd.wat")
    check(err)
    instance, err := wasmtime.NewInstance(store, module, []*wasmtime.Extern{})
    check(err)

    gcd := instance.GetExport("gcd").Func()
    val, err := gcd.Call(6, 27)
    check(err)
    fmt.Printf("gcd(6, 27) = %d\n", val.(int32))
}

func check(err error) {
    if err != nil {
        panic(err)
    }
}
```

And finally we can build and run it:

```sh
$ go run main.go
gcd(6, 27) = 3
```

If this is the output you see, congrats! You've successfully ran your first
WebAssembly code in Go!

## More examples and contributing

The `wasmtime` Go package [lives in its own
repository](https://github.com/bytecodealliance/wasmtime-go) and has a [number
of other more advanced
examples](https://pkg.go.dev/github.com/bytecodealliance/wasmtime-go?tab=doc#pkg-examples)
as well. Feel free to browse those, but if you find anything missing don't
hesitate to [open an
issue](https://github.com/bytecodealliance/wasmtime-go/issues/new) and let us
know if you have any questions!
