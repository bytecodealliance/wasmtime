use crate::prelude::*;

cfg_if::cfg_if! {
    if #[cfg(all(feature = "profiling", target_os = "linux"))] {
        mod jitdump;
        pub use jitdump::new as new_jitdump;
    } else {
        pub fn new_jitdump() -> Result<Box<dyn ProfilingAgent>> {
            if cfg!(feature = "profiling") {
                bail!("jitdump is not supported on this platform");
            } else {
                bail!("jitdump support disabled at compile time");
            }
        }
    }
}

cfg_if::cfg_if! {
    if #[cfg(all(unix, feature = "std"))] {
        mod perfmap;
        pub use perfmap::new as new_perfmap;
    } else {
        pub fn new_perfmap() -> Result<Box<dyn ProfilingAgent>> {
            bail!("perfmap support not supported on this platform");
        }
    }
}

cfg_if::cfg_if! {
    // Note: VTune support is disabled on windows mingw because the ittapi crate doesn't compile
    // there; see also https://github.com/bytecodealliance/wasmtime/pull/4003 for rationale.
    if #[cfg(all(feature = "profiling", target_arch = "x86_64", not(any(target_os = "android", all(target_os = "windows", target_env = "gnu")))))] {
        mod vtune;
        pub use vtune::new as new_vtune;
    } else {
        pub fn new_vtune() -> Result<Box<dyn ProfilingAgent>> {
            if cfg!(feature = "profiling") {
                bail!("VTune is not supported on this platform.");
            } else {
                bail!("VTune support disabled at compile time.");
            }
        }
    }
}

/// Common interface for profiling tools.
pub trait ProfilingAgent: Send + Sync + 'static {
    fn register_function(&self, name: &str, addr: *const u8, size: usize);

    fn register_module(&self, code: &[u8], custom_name: &dyn Fn(usize) -> Option<String>) {
        use object::{File, Object as _, ObjectSection, ObjectSymbol, SectionKind, SymbolKind};

        let image = match File::parse(code) {
            Ok(image) => image,
            Err(_) => return,
        };

        let text_base = match image.sections().find(|s| s.kind() == SectionKind::Text) {
            Some(section) => match section.data() {
                Ok(data) => data.as_ptr() as usize,
                Err(_) => return,
            },
            None => return,
        };

        for sym in image.symbols() {
            if !sym.is_definition() {
                continue;
            }
            if sym.kind() != SymbolKind::Text {
                continue;
            }
            let address = sym.address();
            let size = sym.size();
            if size == 0 {
                continue;
            }
            if let Ok(name) = sym.name() {
                let addr = text_base + address as usize;
                let owned;
                let name = match custom_name(address as usize) {
                    Some(name) => {
                        owned = name;
                        &owned
                    }
                    None => name,
                };
                self.register_function(name, addr as *const u8, size as usize);
            }
        }
    }
}

pub fn new_null() -> Box<dyn ProfilingAgent> {
    Box::new(NullProfilerAgent)
}

#[derive(Debug, Default, Clone, Copy)]
struct NullProfilerAgent;

impl ProfilingAgent for NullProfilerAgent {
    fn register_function(&self, _name: &str, _addr: *const u8, _size: usize) {}
    fn register_module(&self, _code: &[u8], _custom_name: &dyn Fn(usize) -> Option<String>) {}
}
