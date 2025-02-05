#![no_main]

use cranelift_assembler_x64::{fuzz, Inst};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|inst: Inst<fuzz::FuzzRegs>| {
    fuzz::roundtrip(&inst);
});
