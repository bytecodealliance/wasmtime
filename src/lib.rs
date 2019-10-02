use wasmtime_jit::CompilationStrategy;

pub fn pick_compilation_strategy(
    always_cranelift: bool,
    always_lightbeam: bool,
) -> CompilationStrategy {
    // Decide how to compile.
    match (always_lightbeam, always_cranelift) {
        #[cfg(feature = "lightbeam")]
        (true, false) => CompilationStrategy::AlwaysLightbeam,
        #[cfg(not(feature = "lightbeam"))]
        (true, false) => panic!("--always-lightbeam given, but Lightbeam support is not enabled"),
        (false, true) => CompilationStrategy::AlwaysCranelift,
        (false, false) => CompilationStrategy::Auto,
        (true, true) => {
            panic!("Can't enable --always-cranelift and --always-lightbeam at the same time")
        }
    }
}

pub fn init_file_per_thread_logger(prefix: &'static str) {
    file_per_thread_logger::initialize(prefix);

    // Extending behavior of default spawner:
    // https://docs.rs/rayon/1.1.0/rayon/struct.ThreadPoolBuilder.html#method.spawn_handler
    // Source code says DefaultSpawner is implementation detail and
    // shouldn't be used directly.
    rayon::ThreadPoolBuilder::new()
        .spawn_handler(move |thread| {
            let mut b = std::thread::Builder::new();
            if let Some(name) = thread.name() {
                b = b.name(name.to_owned());
            }
            if let Some(stack_size) = thread.stack_size() {
                b = b.stack_size(stack_size);
            }
            b.spawn(move || {
                file_per_thread_logger::initialize(prefix);
                thread.run()
            })?;
            Ok(())
        })
        .build_global()
        .unwrap();
}
