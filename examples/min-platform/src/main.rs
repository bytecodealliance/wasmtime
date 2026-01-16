use wasmtime::Result;

#[cfg(not(target_os = "linux"))]
fn main() -> Result<()> {
    eprintln!("This example only runs on Linux right now");
    Ok(())
}

#[cfg(target_os = "linux")]
fn main() -> Result<()> {
    use libloading::os::unix::{Library, RTLD_GLOBAL, RTLD_NOW, Symbol};
    use object::{Object, ObjectSymbol};
    use wasmtime::{Config, Engine, bail, error::Context as _, format_err};

    let mut args = std::env::args();
    let _current_exe = args.next();
    let triple = args
        .next()
        .ok_or_else(|| format_err!("missing argument 1: triple"))?;
    let embedding_so_path = args
        .next()
        .ok_or_else(|| format_err!("missing argument 2: path to libembedding.so"))?;
    let platform_so_path = args
        .next()
        .ok_or_else(|| format_err!("missing argument 3: path to libwasmtime-platform.so"))?;

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

    // If signals-based-traps are disabled then that additionally means that
    // some configuration knobs need to be turned to match the expectations of
    // the guest program being loaded.
    if !cfg!(feature = "custom") {
        config.memory_init_cow(false);
        config.memory_reservation(0);
        config.memory_guard_size(0);
        config.memory_reservation_for_growth(0);
        config.signals_based_traps(false);
    }

    // For x86_64 targets be sure to enable relevant CPU features to avoid
    // float-related libcalls which is required for the `x86_64-unknown-none`
    // target.
    //
    // Note that the embedding will need to check that these features are
    // actually available at runtime. CPU support for these features has
    // existed since 2013 (Haswell) on Intel chips and 2012 (Piledriver) on
    // AMD chips.
    if cfg!(target_arch = "x86_64") {
        unsafe {
            config.cranelift_flag_enable("has_sse3");
            config.cranelift_flag_enable("has_ssse3");
            config.cranelift_flag_enable("has_sse41");
            config.cranelift_flag_enable("has_sse42");
            config.cranelift_flag_enable("has_fma");
        }
    }

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
    let simple_floats = engine.precompile_module(
        br#"
            (module
                (func (export "frob") (param f32 f32) (result f32)
                    (f32.ceil (local.get 0))
                    (f32.floor (local.get 1))
                    f32.add)
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
            simple_floats.as_ptr(),
            simple_floats.len(),
        );
        error_buf.set_len(len);

        if len > 0 {
            bail!("{}", String::from_utf8_lossy(&error_buf));
        }

        #[cfg(feature = "wasi")]
        {
            let wasi_component_path = args
                .next()
                .ok_or_else(|| format_err!("missing argument 4: path to wasi component"))?;
            let wasi_component = std::fs::read(&wasi_component_path)?;
            let wasi_component = engine.precompile_component(&wasi_component)?;

            let run_wasi: Symbol<extern "C" fn(*mut u8, *mut usize, *const u8, usize) -> usize> =
                lib.get(b"run_wasi")
                    .context("failed to find the `run_wasi` symbol in the library")?;

            const PRINT_CAPACITY: usize = 1024 * 1024;
            let mut print_buf = Vec::with_capacity(PRINT_CAPACITY);
            let mut print_len = PRINT_CAPACITY;
            let status = run_wasi(
                print_buf.as_mut_ptr(),
                std::ptr::from_mut(&mut print_len),
                wasi_component.as_ptr(),
                wasi_component.len(),
            );
            print_buf.set_len(print_len);
            let print_buf = String::from_utf8_lossy(&print_buf);

            if status > 0 {
                bail!("{print_buf}");
            } else {
                println!("{print_buf}");
            }
        }
    }
    Ok(())
}
