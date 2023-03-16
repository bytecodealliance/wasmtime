use crate::{CompiledModule, ProfilingAgent};
use anyhow::Result;
use std::io::Write as _;
use std::process;
use std::{fs::File, sync::Mutex};
use wasmtime_environ::EntityRef as _;

/// Process-wide perf map file. Perf only reads a unique file per process.
static PERFMAP_FILE: Mutex<Option<File>> = Mutex::new(None);

/// Interface for driving the creation of jitdump files
pub struct PerfMapAgent;

impl PerfMapAgent {
    /// Intialize a JitDumpAgent and write out the header.
    pub fn new() -> Result<Self> {
        let mut file = PERFMAP_FILE.lock().unwrap();
        if file.is_none() {
            let filename = format!("/tmp/perf-{}.map", process::id());
            *file = Some(File::create(filename)?);
        }
        Ok(PerfMapAgent)
    }

    fn make_line(name: &str, addr: *const u8, len: usize) -> String {
        format!("{:x} {len:x} {name}\n", addr as usize)
    }
}

impl ProfilingAgent for PerfMapAgent {
    /// Sent when a method is compiled and loaded into memory by the VM.
    fn module_load(&self, module: &CompiledModule, _dbg_image: Option<&[u8]>) {
        let mut file = PERFMAP_FILE.lock().unwrap();
        let file = file.as_mut().unwrap();

        for (idx, func) in module.finished_functions() {
            let addr = func.as_ptr();
            let len = func.len();
            let name = super::debug_name(module, idx);
            let _ = file.write_all(Self::make_line(&name, addr, len).as_bytes());
        }

        // Note: these are the trampolines into exported functions.
        for (idx, func, len) in module.trampolines() {
            let (addr, len) = (func as usize as *const u8, len);
            let name = format!("wasm::trampoline[{}]", idx.index());
            let _ = file.write_all(Self::make_line(&name, addr, len).as_bytes());
        }
    }

    fn load_single_trampoline(
        &self,
        name: &str,
        addr: *const u8,
        size: usize,
        _pid: u32,
        _tid: u32,
    ) {
        let mut file = PERFMAP_FILE.lock().unwrap();
        let file = file.as_mut().unwrap();
        let _ = file.write_all(Self::make_line(name, addr, size).as_bytes());
    }
}
