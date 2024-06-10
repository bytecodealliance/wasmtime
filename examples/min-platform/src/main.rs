use anyhow::Result;

#[cfg(not(target_os = "linux"))]
fn main() -> Result<()> {
    eprintln!("This example only runs on Linux right now");
    Ok(())
}

#[cfg(target_os = "linux")]
fn main() -> Result<()> {
    use anyhow::{anyhow, Context};
    use libloading::os::unix::{Library, Symbol, RTLD_GLOBAL, RTLD_NOW};
    use object::{Object, ObjectSymbol};
    use std::io::Write;
    use wasmtime::{Config, Engine};

    let mut args = std::env::args();
    let _current_exe = args.next();
    let triple = args
        .next()
        .ok_or_else(|| anyhow!("missing argument 1: triple"))?;
    let embedding_so_path = args
        .next()
        .ok_or_else(|| anyhow!("missing argument 2: path to libembedding.so"))?;
    let platform_so_path = args
        .next()
        .ok_or_else(|| anyhow!("missing argument 3: path to libwasmtime-platform.so"))?;

    // Path to the artifact which is the build of the embedding.
    //
    // In this example this is a dynamic library intended to be run on Linux.
    // Note that this is just an example of an artifact and custom build
    // processes can produce different kinds of artifacts.
    let binary = std::fs::read(&embedding_so_path)?;
    let object = object::File::parse(&binary[..])?;

    // Showcase verification that the dynamic library in question doesn't depend
    // on much. Wasmtime build in a "minimal platform" mode is allowed to
    // depend on some standard C symbols such as `memcpy` but any OS-related
    // symbol must be prefixed by `wasmtime_*` and be documented in
    // `crates/wasmtime/src/runtime/vm/sys/custom/capi.rs`.
    //
    // This is effectively a double-check of the above assertion and showing how
    // running `libembedding.so` in this case requires only minimal
    // dependencies.
    for sym in object.symbols() {
        if !sym.is_undefined() || sym.is_weak() {
            continue;
        }

        match sym.name()? {
            "memmove" | "memset" | "memcmp" | "memcpy" | "bcmp" | "__tls_get_addr" => {}
            s if s.starts_with("wasmtime_") => {}
            other => {
                panic!("unexpected dependency on symbol `{other}`")
            }
        }
    }

    // Precompile modules for the embedding. Right now Wasmtime in no_std mode
    // does not have support for Cranelift meaning that AOT mode must be used.
    // Modules are compiled here and then given to the embedding via the `run`
    // function below.
    //
    // Note that `Config::target` is used here to enable cross-compilation.
    let mut config = Config::new();
    config.target(&triple)?;
    let engine = Engine::new(&config)?;
    let smoke = engine.precompile_module(b"(module)")?;
    let simple_add = engine.precompile_module(
        br#"
            (module
                (func (export "add") (param i32 i32) (result i32)
                    (i32.add (local.get 0) (local.get 1)))
            )
        "#,
    )?;
    let simple_host_fn = engine.precompile_module(
        br#"
            (module
                (import "host" "multiply" (func $multiply (param i32 i32) (result i32)))
                (func (export "add_and_mul") (param i32 i32 i32) (result i32)
                    (i32.add (call $multiply (local.get 0) (local.get 1)) (local.get 2)))
            )
        "#,
    )?;

    // Next is an example of running this embedding, which also serves as test
    // that basic functionality actually works.
    //
    // Here the `wasmtime_*` symbols are implemented by
    // `./embedding/wasmtime-platform.c` which is an example implementation
    // against glibc on Linux. This library is compiled into
    // `libwasmtime-platform.so` and is dynamically opened here to make it
    // available for later symbol resolution. This is just an implementation
    // detail of this exable to enably dynamically loading `libembedding.so`
    // next.
    //
    // Next the `libembedding.so` library is opened and the `run` symbol is
    // run. The dependencies of `libembedding.so` are either satisfied by our
    // ambient libc (e.g. `memcpy` and friends) or `libwasmtime-platform.so`
    // (e.g. `wasmtime_*` symbols).
    //
    // The embedding is then run to showcase an example and then an error, if
    // any, is written to stderr.
    unsafe {
        let _platform_symbols = Library::open(Some(&platform_so_path), RTLD_NOW | RTLD_GLOBAL)
            .with_context(|| {
                format!(
                    "failed to open {platform_so_path:?}; cwd = {:?}",
                    std::env::current_dir()
                )
            })?;

        let lib = Library::new(&embedding_so_path).context("failed to create new library")?;
        let run: Symbol<
            extern "C" fn(
                *mut u8,
                usize,
                *const u8,
                usize,
                *const u8,
                usize,
                *const u8,
                usize,
            ) -> usize,
        > = lib
            .get(b"run")
            .context("failed to find the `run` symbol in the library")?;

        let mut error_buf = Vec::with_capacity(1024);
        let len = run(
            error_buf.as_mut_ptr(),
            error_buf.capacity(),
            smoke.as_ptr(),
            smoke.len(),
            simple_add.as_ptr(),
            simple_add.len(),
            simple_host_fn.as_ptr(),
            simple_host_fn.len(),
        );
        error_buf.set_len(len);

        std::io::stderr().write_all(&error_buf).unwrap();
    }
    Ok(())
}
