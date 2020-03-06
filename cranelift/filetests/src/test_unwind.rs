//! Test command for verifying the unwind emitted for each function.
//!
//! The `unwind` test command runs each function through the full code generator pipeline.
#![cfg_attr(feature = "cargo-clippy", allow(clippy::cast_ptr_alignment))]

use crate::subtest::{run_filecheck, Context, SubTest, SubtestResult};
use byteorder::{ByteOrder, LittleEndian};
use cranelift_codegen;
use cranelift_codegen::binemit::{FrameUnwindKind, FrameUnwindOffset, FrameUnwindSink, Reloc};
use cranelift_codegen::ir;
use cranelift_reader::TestCommand;
use std::borrow::Cow;
use std::fmt::Write;

struct TestUnwind;

pub fn subtest(parsed: &TestCommand) -> SubtestResult<Box<dyn SubTest>> {
    assert_eq!(parsed.command, "unwind");
    if !parsed.options.is_empty() {
        Err(format!("No options allowed on {}", parsed))
    } else {
        Ok(Box::new(TestUnwind))
    }
}

impl SubTest for TestUnwind {
    fn name(&self) -> &'static str {
        "unwind"
    }

    fn is_mutating(&self) -> bool {
        false
    }

    fn needs_isa(&self) -> bool {
        true
    }

    fn run(&self, func: Cow<ir::Function>, context: &Context) -> SubtestResult<()> {
        let isa = context.isa.expect("unwind needs an ISA");
        let mut comp_ctx = cranelift_codegen::Context::for_function(func.into_owned());

        comp_ctx.compile(isa).expect("failed to compile function");

        struct Sink(Vec<u8>);
        impl FrameUnwindSink for Sink {
            fn len(&self) -> FrameUnwindOffset {
                self.0.len()
            }
            fn bytes(&mut self, b: &[u8]) {
                self.0.extend_from_slice(b);
            }
            fn reloc(&mut self, _: Reloc, _: FrameUnwindOffset) {
                unimplemented!();
            }
            fn set_entry_offset(&mut self, _: FrameUnwindOffset) {
                unimplemented!();
            }
        }

        let mut sink = Sink(Vec::new());
        comp_ctx.emit_unwind_info(isa, FrameUnwindKind::Fastcall, &mut sink);

        let mut text = String::new();
        if sink.0.is_empty() {
            writeln!(text, "No unwind information.").unwrap();
        } else {
            print_unwind_info(&mut text, &sink.0);
        }

        run_filecheck(&text, context)
    }
}

fn print_unwind_info(text: &mut String, mem: &[u8]) {
    let info = UnwindInfo::from_slice(mem);

    // Assert correct alignment and padding of the unwind information
    assert!(mem.len() % 4 == 0);
    assert_eq!(
        mem.len(),
        4 + ((info.unwind_code_count_raw as usize) * 2)
            + if (info.unwind_code_count_raw & 1) == 1 {
                2
            } else {
                0
            }
    );

    writeln!(text, "{:#?}", info).unwrap();
}

#[derive(Debug)]
struct UnwindInfo {
    pub version: u8,
    pub flags: u8,
    pub prologue_size: u8,
    pub unwind_code_count_raw: u8,
    pub frame_register: u8,
    pub frame_register_offset: u8,
    pub unwind_codes: Vec<UnwindCode>,
}

impl UnwindInfo {
    fn from_slice(mem: &[u8]) -> Self {
        let version_and_flags = mem[0];
        let prologue_size = mem[1];
        let unwind_code_count_raw = mem[2];
        let frame_register_and_offset = mem[3];
        let mut unwind_codes = Vec::new();

        let mut i = 0;
        while i < unwind_code_count_raw {
            let code = UnwindCode::from_slice(&mem[(4 + (i * 2) as usize)..]);

            i += match &code.value {
                UnwindValue::None => 1,
                UnwindValue::U16(_) => 2,
                UnwindValue::U32(_) => 3,
            };

            unwind_codes.push(code);
        }

        Self {
            version: version_and_flags & 0x3,
            flags: (version_and_flags & 0xF8) >> 3,
            prologue_size,
            unwind_code_count_raw,
            frame_register: frame_register_and_offset & 0xF,
            frame_register_offset: (frame_register_and_offset & 0xF0) >> 4,
            unwind_codes,
        }
    }
}

#[derive(Debug)]
struct UnwindCode {
    pub offset: u8,
    pub op: UnwindOperation,
    pub info: u8,
    pub value: UnwindValue,
}

impl UnwindCode {
    fn from_slice(mem: &[u8]) -> Self {
        let offset = mem[0];
        let op_and_info = mem[1];
        let op = UnwindOperation::from(op_and_info & 0xF);
        let info = (op_and_info & 0xF0) >> 4;

        let value = match op {
            UnwindOperation::LargeStackAlloc => match info {
                0 => UnwindValue::U16(LittleEndian::read_u16(&mem[2..])),
                1 => UnwindValue::U32(LittleEndian::read_u32(&mem[2..])),
                _ => panic!("unexpected stack alloc info value"),
            },
            UnwindOperation::SaveNonvolatileRegister => {
                UnwindValue::U16(LittleEndian::read_u16(&mem[2..]))
            }
            UnwindOperation::SaveNonvolatileRegisterFar => {
                UnwindValue::U32(LittleEndian::read_u32(&mem[2..]))
            }
            UnwindOperation::SaveXmm128 => UnwindValue::U16(LittleEndian::read_u16(&mem[2..])),
            UnwindOperation::SaveXmm128Far => UnwindValue::U32(LittleEndian::read_u32(&mem[2..])),
            _ => UnwindValue::None,
        };

        Self {
            offset,
            op,
            info,
            value,
        }
    }
}

#[derive(Debug)]
enum UnwindOperation {
    PushNonvolatileRegister,
    LargeStackAlloc,
    SmallStackAlloc,
    SetFramePointer,
    SaveNonvolatileRegister,
    SaveNonvolatileRegisterFar,
    SaveXmm128,
    SaveXmm128Far,
    PushMachineFrame,
}

impl From<u8> for UnwindOperation {
    fn from(value: u8) -> Self {
        // The numerical value is specified as part of the Windows x64 ABI
        match value {
            0 => Self::PushNonvolatileRegister,
            1 => Self::LargeStackAlloc,
            2 => Self::SmallStackAlloc,
            3 => Self::SetFramePointer,
            4 => Self::SaveNonvolatileRegister,
            5 => Self::SaveNonvolatileRegisterFar,
            6 => Self::SaveXmm128,
            7 => Self::SaveXmm128Far,
            8 => Self::PushMachineFrame,
            _ => panic!("unsupported unwind operation"),
        }
    }
}

#[derive(Debug)]
enum UnwindValue {
    None,
    U16(u16),
    U32(u32),
}
