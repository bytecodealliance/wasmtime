# Using WebAssembly from Ruby

Wasmtime [is available on RubyGems](https://rubygems.org/gems/wasmtime) and can
be used programmatically to interact with Wasm modules. This guide will go over
installing the Wasmtime gem and running a simple Wasm module from Ruby.

Make sure you've got Ruby 3.0 or newer installed locally, and we can get
started!

## Getting started and simple example

First, copy this example WebAssembly text module into your project. It exports
a function for calculating the greatest common divisor of two numbers.

```wat
{{#include ../examples/gcd.wat}}
```

Next, install the Wasmtime Ruby gems by either adding it your project's
`Gemfile`:

```bash
bundle add wasmtime
```

Or by using the `gem` command directly:

```bash
gem install wasmtime
```

The gem has a Rust-based native extension, but thanks to precompiled gems, you
should not have to compile anything. It'll just work!

Now that you have the Wasmtime gem installed, let's create a Ruby script to
execute the `gcd` module from before.

```ruby
require "wasmtime"

engine = Wasmtime::Engine.new
mod = Wasmtime::Module.from_file(engine, "gcd.wat")
store = Wasmtime::Store.new(engine)
instance = Wasmtime::Instance.new(store, mod)

puts "gcd(27, 6) = #{instance.invoke("gcd", 27, 6)}"
```

This script should output

```bash
gcd(27, 6) = 3
```

If this is the output you see, congrats! You've successfully ran your first
WebAssembly code in Ruby!

## More examples and contributing

To learn more, check out the [more advanced examples](https://github.com/bytecodealliance/wasmtime-rb/tree/main/examples)
and the [API documentation](https://bytecodealliance.github.io/wasmtime-rb/latest/).
If you have any questions, do not hesitate to open an issue on the
[GitHub repository](https://github.com/bytecodealliance/wasmtime-rb).
