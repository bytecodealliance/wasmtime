use std::cmp::min;
use target_lexicon::{OperatingSystem, Triple};
use wasmtime_environ::Tunables;

/// Return a `Tunables` instance tuned for the given target platform.
pub fn target_tunables(triple: &Triple) -> Tunables {
    let mut result = Tunables::default();

    match triple.operating_system {
        OperatingSystem::Windows => {
            // For now, use a smaller footprint on Windows so that we don't
            // don't outstrip the paging file.
            // TODO: Make this configurable.
            result.static_memory_bound = min(result.static_memory_bound, 0x100);
            result.static_memory_offset_guard_size =
                min(result.static_memory_offset_guard_size, 0x10000);
        }
        _ => {}
    }

    result
}
