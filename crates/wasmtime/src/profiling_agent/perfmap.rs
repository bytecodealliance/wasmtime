use crate::prelude::*;
use crate::profiling_agent::ProfilingAgent;
use anyhow::Result;
use std::io::{self, BufWriter, Write};
use std::process;
use std::{fs::File, sync::Mutex};

/// Process-wide perf map file. Perf only reads a unique file per process.
static PERFMAP_FILE: Mutex<Option<BufWriter<File>>> = Mutex::new(None);

/// Interface for driving the creation of jitdump files
struct PerfMapAgent;

/// Initialize a JitDumpAgent and write out the header.
pub fn new() -> Result<Box<dyn ProfilingAgent>> {
    let mut file = PERFMAP_FILE.lock().unwrap();
    if file.is_none() {
        let filename = format!("/tmp/perf-{}.map", process::id());
        *file = Some(BufWriter::new(File::create(filename)?));
    }
    Ok(Box::new(PerfMapAgent))
}

impl PerfMapAgent {
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
        writer.flush()?;
        Ok(())
    }
}

impl ProfilingAgent for PerfMapAgent {
    fn register_function(&self, name: &str, addr: *const u8, size: usize) {
        let mut file = PERFMAP_FILE.lock().unwrap();
        let file = file.as_mut().unwrap();
        if let Err(err) = Self::make_line(file, name, addr, size) {
            eprintln!("Error when writing import trampoline info to the perf map file: {err}");
        }
    }
}
