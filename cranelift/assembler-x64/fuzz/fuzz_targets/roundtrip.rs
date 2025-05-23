#![no_main]

use cranelift_assembler_x64::{Inst, fuzz};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|inst: Inst<fuzz::FuzzRegs>| {
    fuzz::roundtrip(&inst);
});
