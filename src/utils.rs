pub fn init_file_per_thread_logger() {
    use super::LOG_FILENAME_PREFIX;

    file_per_thread_logger::initialize(LOG_FILENAME_PREFIX);

    // Extending behavior of default spawner:
    // https://docs.rs/rayon/1.1.0/rayon/struct.ThreadPoolBuilder.html#method.spawn_handler
    // Source code says DefaultSpawner is implementation detail and
    // shouldn't be used directly.
    rayon::ThreadPoolBuilder::new()
        .spawn_handler(|thread| {
            let mut b = std::thread::Builder::new();
            if let Some(name) = thread.name() {
                b = b.name(name.to_owned());
            }
            if let Some(stack_size) = thread.stack_size() {
                b = b.stack_size(stack_size);
            }
            b.spawn(|| {
                file_per_thread_logger::initialize(LOG_FILENAME_PREFIX);
                thread.run()
            })?;
            Ok(())
        })
        .build_global()
        .unwrap();
}
