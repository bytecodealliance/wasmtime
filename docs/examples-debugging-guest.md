# Guest (Wasm-only) Debugging with `lldb`

The following steps describe how to use `lldb` to debug the Wasm guest
on its own -- as if it were running on a virtual Wasm-instruction-set
computer with hostcalls as single-step indivisible actions. This
functionality, called "guest debugging", allows for disassembly and
single instruction stepping at the Wasm bytecode level.

1. Compile your WebAssembly with debug info enabled, usually `-g`; for
   example:

    ```console
    clang foo.c -g -o foo.wasm
    ```

2. Ensure that you have a build of Wasmtime that has the `gdbstub`
   feature enabled, which is off by default:
   
   - Published CLI binary releases already have this feature.
   - If building from source, use(`cargo build --features gdbstub`.

3. Run Wasmtime, enabling the gdbstub server:

    ```console
    wasmtime run -g 1234 foo.wasm
    
    Debugger: Debugger listening on 127.0.0.1:1234
    Debugger: In LLDB, attach with: process connect --plugin wasm connect://127.0.0.1:1234
    ```
    
   This will start a "debug server" waiting on local TCP port 1234 for
   a debugger to connect. Execution will not start until the debugger
   connects and issues a "continue" command.
   
4. Run LLDB, connect and debug.

   You'll need a recent version of LLDB (v32 or later) with Wasm
   support enabled. The [wasi-sdk] distribution provides such a
   build.
   
    ```console
    /opt/wasi-sdk/bin/lldb
    (lldb) process connect --plugin wasm connect://0:1234
    Process 1 stopped
    * thread #1, stop reason = signal SIGTRAP
        frame #0: 0x00000000
    error: memory read failed for 0x0
    (lldb) b my_function
    (lldb) continue
    ```
    
   and use LLDB like normal, setting breakpoints, continuing and
   stepping, examining memory and variable state, etc.
   
This functionality should work on any platform that Wasmtime runs on:
debugging is based on code instrumentation, so does not depend on any
native system debugging interfaces or introspection capabilities; and
is supported on all native-compilation ISAs and on Pulley, Wasmtime's
bytecode platform that runs everywhere.
   
[wasi-sdk]: https://github.com/WebAssembly/wasi-sdk/
