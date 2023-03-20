use crate::{CompiledModule, ProfilingAgent};
use anyhow::Result;
use std::io::{self, BufWriter, Write};
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

    fn make_line(
        writer: &mut dyn Write,
        name: &str,
        addr: *const u8,
        len: usize,
    ) -> io::Result<()> {
        // Format is documented here: https://github.com/torvalds/linux/blob/master/tools/perf/Documentation/jit-interface.txt
        // Try our best to sanitize the name, since wasm allows for any utf8 string in there.
        let sanitized_name = name.replace('\n', "_").replace('\r', "_");
        write!(writer, "{:x} {:x} {}\n", addr as usize, len, sanitized_name)?;
        Ok(())
    }
}

impl ProfilingAgent for PerfMapAgent {
    /// Sent when a method is compiled and loaded into memory by the VM.
    fn module_load(&self, module: &CompiledModule, _dbg_image: Option<&[u8]>) {
        let mut file = PERFMAP_FILE.lock().unwrap();
        let file = file.as_mut().unwrap();
        let mut file = BufWriter::new(file);

        for (idx, func) in module.finished_functions() {
            let addr = func.as_ptr();
            let len = func.len();
            let name = super::debug_name(module, idx);
            if let Err(err) = Self::make_line(&mut file, &name, addr, len) {
                eprintln!("Error when writing function info to the perf map file: {err}");
                return;
            }
        }

        // Note: these are the trampolines into exported functions.
        for (idx, func, len) in module.trampolines() {
            let (addr, len) = (func as usize as *const u8, len);
            let name = format!("wasm::trampoline[{}]", idx.index());
            if let Err(err) = Self::make_line(&mut file, &name, addr, len) {
                eprintln!("Error when writing export trampoline info to the perf map file: {err}");
                return;
            }
        }

        if let Err(err) = file.flush() {
            eprintln!("Error when flushing the perf map file buffer: {err}");
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
        if let Err(err) = Self::make_line(file, name, addr, size) {
            eprintln!("Error when writing import trampoline info to the perf map file: {err}");
        }
    }
}
