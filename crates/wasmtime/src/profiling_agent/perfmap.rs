use crate::prelude::*;
use crate::profiling_agent::ProfilingAgent;
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::process;
use std::{fs::File, sync::Mutex};

/// Process-wide perf map file. Perf only reads a unique file per process.
static PERFMAP_FILE: Mutex<Option<File>> = Mutex::new(None);

/// Interface for driving the creation of jitdump files
struct PerfMapAgent;

/// Initialize a JitDumpAgent and write out the header.
pub fn new() -> Result<Box<dyn ProfilingAgent>> {
    let mut file = PERFMAP_FILE.lock().unwrap();
    if file.is_none() {
        let filename = format!("/tmp/perf-{}.map", process::id());

        // Open the file specifically in append mode to handle the case where
        // multiple engines in the same process are all writing to this file.
        *file = Some(
            OpenOptions::new()
                .append(true)
                .write(true)
                .create(true)
                .open(&filename)?,
        );
    }
    Ok(Box::new(PerfMapAgent))
}

impl PerfMapAgent {
    fn make_line(writer: &mut File, name: &str, code: &[u8]) -> io::Result<()> {
        // Format is documented here: https://github.com/torvalds/linux/blob/master/tools/perf/Documentation/jit-interface.txt
        // Try our best to sanitize the name, since wasm allows for any utf8 string in there.
        let sanitized_name = name.replace('\n', "_").replace('\r', "_");
        let line = format!("{:p} {:x} {sanitized_name}\n", code.as_ptr(), code.len());

        // To handle multiple concurrent engines in the same process writing to
        // this file an attempt is made to issue a single `write` syscall with
        // all of the contents. This would mean, though, that partial writes
        // would be an error. In lieu of returning an error the `write_all`
        // helper is used instead which may result in a corrupt file if there
        // are other engines writing to the file at the same time.
        //
        // For more discussion of the tradeoffs here see the `perf_jitdump.rs`
        // file which has to deal with the same problem.
        writer.write_all(line.as_bytes())?;

        Ok(())
    }
}

impl ProfilingAgent for PerfMapAgent {
    fn register_function(&self, name: &str, code: &[u8]) {
        let mut file = PERFMAP_FILE.lock().unwrap();
        let file = file.as_mut().unwrap();
        if let Err(err) = Self::make_line(file, name, code) {
            eprintln!("Error when writing import trampoline info to the perf map file: {err}");
        }
    }
}
