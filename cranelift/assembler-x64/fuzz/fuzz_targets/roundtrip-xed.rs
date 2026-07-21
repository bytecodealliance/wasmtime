#![no_main]

use cranelift_assembler_x64::{Inst, fuzz};
use libfuzzer_sys::fuzz_target;

// This target drives the Intel XED disassembler oracle instead of Capstone.
// XED understands newer encodings (e.g. APX) that the bundled Capstone does
// not. Building XED from source is only done when the `fuzz-xed` feature is
// enabled; without it this target is a no-op so the default fuzz build does
// not require a C compiler and Python.
fuzz_target!(|inst: Inst<fuzz::FuzzRegs>| {
    #[cfg(feature = "fuzz-xed")]
    fuzz::roundtrip_xed(&inst);
    #[cfg(not(feature = "fuzz-xed"))]
    let _ = inst;
});
