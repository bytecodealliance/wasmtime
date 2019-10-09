use wasmtime_jit::CompilationStrategy;

pub fn pick_compilation_strategy(compiler: Option<&str>) -> CompilationStrategy {
    // Decide how to compile.
    match compiler {
        #[cfg(feature = "lightbeam")]
        Some("lightbeam") => CompilationStrategy::Lightbeam,
        #[cfg(not(feature = "lightbeam"))]
        Some("lightbeam") => panic!("--lightbeam given, but Lightbeam support is not enabled"),
        Some("cranelift") => CompilationStrategy::Cranelift,
        Some(name) => panic!("Unknown compiler: {}", name),
        None => CompilationStrategy::Auto,
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
