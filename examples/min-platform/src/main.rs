use anyhow::Result;

#[cfg(not(target_os = "linux"))]
fn main() -> Result<()> {
    eprintln!("This example only runs on Linux right now");
    Ok(())
}

#[cfg(target_os = "linux")]
fn main() -> Result<()> {
    use libloading::os::unix::{Library, Symbol, RTLD_GLOBAL, RTLD_NOW};
    use object::{Object, ObjectSymbol, SymbolKind};
    use std::io::Write;
    use std::path::Path;

    let target = std::env::args().nth(1).unwrap();
    let target = Path::new(&target).file_stem().unwrap().to_str().unwrap();
    // Path to the artifact which is the build of the embedding.
    //
    // In this example this is a dynamic library intended to be run on Linux.
    // Note that this is just an example of an artifact and custom build
    // processes can produce different kinds of artifacts.
    let lib = format!("../../target/{target}/release/libembedding.so");
    let binary = std::fs::read(&lib)?;
    let object = object::File::parse(&binary[..])?;

    // Showcase verification that the dynamic library in question doesn't depend
    // on much. Wasmtime build in a "minimal platform" mode is allowed to
    // depend on some standard C symbols such as `memcpy` but any OS-related
    // symbol must be prefixed by `wasmtime_*` and be documented in
    // `crates/wasmtime/src/runtime/vm/sys/custom/capi.rs`.
    //
    // This is effectively a double-check of the above assesrtion and showing
    // how running `libembedding.so` in this case requires only minimal
    // dependencies.
    for sym in object.symbols() {
        if !sym.is_undefined() || sym.is_weak() || sym.kind() == SymbolKind::Null {
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
        let _platform_symbols =
            Library::open(Some("./libwasmtime-platform.so"), RTLD_NOW | RTLD_GLOBAL)?;

        let lib = Library::new(&lib)?;
        let run: Symbol<extern "C" fn(*mut u8, usize) -> usize> = lib.get(b"run")?;

        let mut buf = Vec::with_capacity(1024);
        let len = run(buf.as_mut_ptr(), buf.capacity());
        buf.set_len(len);

        std::io::stderr().write_all(&buf).unwrap();
    }
    Ok(())
}
