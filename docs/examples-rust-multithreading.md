# Multi-threading

When using Rust you're effectively immune from a whole class of threading issues
such as data races due to the inherent checks in the compiler and traits like
`Send` and `Sync`. The `wasmtime` API, like other safe Rust APIs, is 100% safe
to use relative to threading if you never have any `unsafe` yourself. In
addition to all of this, however, it's important to be aware of the limitations
of `wasmtime` types and how this might affect your embedding use case.

## Types that are `Send` and `Sync`

Wasmtime has a number of types which implement both the `Send` and `Sync`
traits:

* [`Config`](https://docs.wasmtime.dev/api/wasmtime/struct.Config.html)
* [`Engine`](https://docs.wasmtime.dev/api/wasmtime/struct.Engine.html)
* [`Module`](https://docs.wasmtime.dev/api/wasmtime/struct.Module.html)
* [`Trap`](https://docs.wasmtime.dev/api/wasmtime/struct.Trap.html)
* [`InterruptHandle`](https://docs.wasmtime.dev/api/wasmtime/struct.InterruptHandle.html)
* Type-descriptions of items
  * [`ValType`](https://docs.wasmtime.dev/api/wasmtime/struct.ValType.html)
  * [`ExportType`](https://docs.wasmtime.dev/api/wasmtime/struct.ExportType.html)
  * [`ExternType`](https://docs.wasmtime.dev/api/wasmtime/struct.ExternType.html)
  * [`ImportType`](https://docs.wasmtime.dev/api/wasmtime/struct.ImportType.html)
  * [`FuncType`](https://docs.wasmtime.dev/api/wasmtime/struct.FuncType.html)
  * [`GlobalType`](https://docs.wasmtime.dev/api/wasmtime/struct.GlobalType.html)
  * [`MemoryType`](https://docs.wasmtime.dev/api/wasmtime/struct.MemoryType.html)
  * [`ModuleType`](https://docs.wasmtime.dev/api/wasmtime/struct.ModuleType.html)
  * [`TableType`](https://docs.wasmtime.dev/api/wasmtime/struct.TableType.html)
  * [`InstanceType`](https://docs.wasmtime.dev/api/wasmtime/struct.InstanceType.html)

These types, as the traits imply, are safe to send and share across threads.
Note that the major types to call out here are `Module` and `Engine`. The
`Engine` is important because it enables sharing compilation configuration for
an entire application. Each `Engine` is intended to be long-lived for this
reason.

Additionally `Module`, the compiled version of a WebAssembly module, is safe to
send and share across threads. This notably means that you can compile a module
once and then instantiate it on multiple threads simultaneously. There's no need
to recompile a module on each thread.

## Types that are neither `Send` nor `Sync`

Wasmtime also has a number of types which are thread-"unsafe". These types do
not have the `Send` or `Sync` traits implemented which means that you won't be
able to send them across threads by default.

* [`Store`](https://docs.wasmtime.dev/api/wasmtime/struct.Store.html)
* [`Linker`](https://docs.wasmtime.dev/api/wasmtime/struct.Linker.html)
* [`Instance`](https://docs.wasmtime.dev/api/wasmtime/struct.Instance.html)
* [`Extern`](https://docs.wasmtime.dev/api/wasmtime/struct.Extern.html)
* [`Func`](https://docs.wasmtime.dev/api/wasmtime/struct.Func.html)
* [`Global`](https://docs.wasmtime.dev/api/wasmtime/struct.Global.html)
* [`Table`](https://docs.wasmtime.dev/api/wasmtime/struct.Table.html)
* [`Memory`](https://docs.wasmtime.dev/api/wasmtime/struct.Memory.html)
* [`Val`](https://docs.wasmtime.dev/api/wasmtime/struct.Val.html)
* [`ExternRef`](https://docs.wasmtime.dev/api/wasmtime/struct.ExternRef.html)

These types are all considered as "connected to a store", and everything
connected to a store is neither `Send` nor `Sync`. The Rust compiler will not
allow you to have values of these types cross thread boundaries or get shared
between multiple threads. Doing so would require some form of `unsafe` glue.

It's important to note that the WebAssembly specification itself fundamentally
limits some of the concurrent possibilities here. For example it's not allowed
to concurrently call `global.set` or `table.set` on the same global/table. This
means that Wasmtime is designed to prevent at the very least concurrent usage of
these primitives.

Apart from the WebAssembly specification, though, Wasmtime additionally has some
fundamental design decision which results in these types not implementing either
`Send` or `Sync`:

* All objects are independently-owned `'static` values that internally retain
  anything necessary to implement the API provided. This necessitates some form
  of reference counting, and also requires the usage of non-atomic reference
  counting. Once reference counting is used Rust only allows shared references
  (`&T`) to the internals, and due to the wasm restriction of disallowing
  concurrent usage non-atomic reference counting is used.

* Insertion of user-defined objects into `Store` does not require all objects to
  be either `Send` or `Sync`. For example `Func::wrap` will insert the
  host-defined function into the `Store`, but there are no extra trait bounds on
  this. Similar restrictions apply to `Store::set` as well.

* The implementation of `ExternRef` allows arbitrary `'static` types `T` to get
  wrapped up and is also implemented with non-atomic reference counting.

Overall the design decisions of Wasmtime itself leads all of these types to not
implement either the `Send` or `Sync` traits.

## Multithreading without `Send`

Due to the lack of `Send` on types like `Store` and everything connected, it's
not always as trivial to add multithreaded execution of WebAssembly to an
embedding of Wasmtime as it is for other Rust code in general. The exact way
that multithreading could work for you depends on your specific embedding, but
some possibilities include:

* If your workload involves instantiating a singular wasm module on a separate
  thread, then it will need to live on that thread and communicate to other
  threads via threadsafe means (e.g. channels, locks/queues, etc).

* If you have something like a multithreaded web server, for example, then the
  WebAssembly executed for each request will need to live within the thread that
  the original `Store` was created on. This could be multithreaded, though, by
  having a pool of threads executing WebAssembly. Each request would have a
  scheduling decision of which pool to route to which would be up to the
  application. In situations such as this it's recommended to [enable fuel
  consumption](https://docs.wasmtime.dev/api/wasmtime/struct.Config.html#method.consume_fuel)
  as well as [yielding when out of
  fuel](https://docs.wasmtime.dev/api/wasmtime/struct.Store.html#method.out_of_fuel_async_yield).
  This will ensure that no one request entirely hogs a thread executing
  WebAssembly and all requests scheduled onto that thread are able to execute.
  It's also worth pointing out that the threads executing WebAssembly may or may
  not be the same as the threads performing I/O for your server requests.

* If absolutely required, Wasmtime is engineered such that it is dynamically safe
  to move a `Store` as a whole to a separate thread. This option is not
  recommended due to its complexity, but it is one that Wasmtime tests in CI and
  considers supported. The principle here is that all objects connected to a
  `Store` are safe to move to a separate thread *if and only if*:

  * All objects are moved all at once. For example you can't leave behind
    references to a `Func` or perhaps a `Store` in TLS.

  * All host objects living inside of a store (e.g. those inserted via
    `Store::set` or `Func::wrap`) implement the `Send` trait.

  If these requirements are met it is technically safe to move a store and its
  objects between threads. When you move a store to another thread, it is
  required that you run the `Store::notify_switched_thread()` method after the
  store has landed on the new thread, so that per-thread initialization is
  correctly re-run. Failure to do so may cause wasm traps to crash the whole
  application.

  The reason that this strategy isn't recommended, however, is that you will
  receive no assistance from the Rust compiler in verifying that the transfer
  across threads is indeed actually safe. This will require auditing your
  embedding of Wasmtime itself to ensure it meets these requirements.

  It's important to note that the requirements here also apply to the futures
  returned from `Func::call_async`. These futures are not `Send` due to them
  closing over `Store`-related values. In addition to the above requirements
  though to safely send across threads embedders must *also* ensure that any
  host futures returned from `Func::wrapN_async` are actually `Send` and safe to
  send across threads. Again, though, there is no compiler assistance in doing
  this.

Overall the recommended story for multithreading with Wasmtime is "don't move a
`Store` between threads" and to architect your application around this
assumption.
