//! Test command for verifying the unwind emitted for each function.
//!
//! The `unwind` test command runs each function through the full code generator pipeline.
#![cfg_attr(feature = "cargo-clippy", allow(clippy::cast_ptr_alignment))]

use crate::subtest::{run_filecheck, Context, SubTest};
use cranelift_codegen::{self, ir, isa::unwind::UnwindInfo};
use cranelift_reader::TestCommand;
use gimli::{
    write::{Address, EhFrame, EndianVec, FrameTable},
    LittleEndian,
};
use std::borrow::Cow;

struct TestUnwind;

pub fn subtest(parsed: &TestCommand) -> anyhow::Result<Box<dyn SubTest>> {
    assert_eq!(parsed.command, "unwind");
    if !parsed.options.is_empty() {
        anyhow::bail!("No options allowed on {}", parsed);
    }
    Ok(Box::new(TestUnwind))
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

    fn run(&self, func: Cow<ir::Function>, context: &Context) -> anyhow::Result<()> {
        let isa = context.isa.expect("unwind needs an ISA");
        let mut comp_ctx = cranelift_codegen::Context::for_function(func.into_owned());

        comp_ctx.compile(isa).expect("failed to compile function");

        let mut text = String::new();
        match comp_ctx.create_unwind_info(isa).expect("unwind info") {
            Some(UnwindInfo::WindowsX64(info)) => {
                let mut mem = vec![0; info.emit_size()];
                info.emit(&mut mem);
                windowsx64::dump(&mut text, &mem);
            }
            Some(UnwindInfo::SystemV(info)) => {
                let mut table = FrameTable::default();
                let cie = isa
                    .create_systemv_cie()
                    .expect("the ISA should support a System V CIE");

                let cie_id = table.add_cie(cie);
                table.add_fde(cie_id, info.to_fde(Address::Constant(0)));

                let mut eh_frame = EhFrame(EndianVec::new(LittleEndian));
                table.write_eh_frame(&mut eh_frame).unwrap();
                systemv::dump(&mut text, &eh_frame.0.into_vec(), isa.pointer_bytes())
            }
            Some(ui) => {
                anyhow::bail!("Unexpected unwind info type: {:?}", ui);
            }
            None => {}
        }

        run_filecheck(&text, context)
    }
}

mod windowsx64 {
    use std::fmt::Write;

    pub fn dump<W: Write>(text: &mut W, mem: &[u8]) {
        let info = UnwindInfo::from_slice(mem);

        writeln!(text, "              version: {}", info.version).unwrap();
        writeln!(text, "                flags: {}", info.flags).unwrap();
        writeln!(text, "        prologue size: {}", info.prologue_size).unwrap();
        writeln!(text, "       frame register: {}", info.frame_register).unwrap();
        writeln!(
            text,
            "frame register offset: {}",
            info.frame_register_offset
        )
        .unwrap();
        writeln!(text, "         unwind codes: {}", info.unwind_codes.len()).unwrap();

        for code in info.unwind_codes.iter().rev() {
            writeln!(text).unwrap();
            writeln!(text, "               offset: {}", code.offset).unwrap();
            writeln!(text, "                   op: {:?}", code.op).unwrap();
            writeln!(text, "                 info: {}", code.info).unwrap();
            match code.value {
                UnwindValue::None => {}
                UnwindValue::U16(v) => {
                    writeln!(text, "                value: {} (u16)", v).unwrap()
                }
                UnwindValue::U32(v) => {
                    writeln!(text, "                value: {} (u32)", v).unwrap()
                }
            };
        }
    }

    #[derive(Debug)]
    struct UnwindInfo {
        version: u8,
        flags: u8,
        prologue_size: u8,
        unwind_code_count_raw: u8,
        frame_register: u8,
        frame_register_offset: u8,
        unwind_codes: Vec<UnwindCode>,
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
        offset: u8,
        op: UnwindOperation,
        info: u8,
        value: UnwindValue,
    }

    impl UnwindCode {
        fn from_slice(mem: &[u8]) -> Self {
            let offset = mem[0];
            let op_and_info = mem[1];
            let op = UnwindOperation::from(op_and_info & 0xF);
            let info = (op_and_info & 0xF0) >> 4;
            let unwind_le_bytes = |bytes| match (bytes, &mem[2..]) {
                (2, &[b0, b1, ..]) => UnwindValue::U16(u16::from_le_bytes([b0, b1])),
                (4, &[b0, b1, b2, b3, ..]) => {
                    UnwindValue::U32(u32::from_le_bytes([b0, b1, b2, b3]))
                }
                (_, _) => panic!("not enough bytes to unwind value"),
            };

            let value = match (&op, info) {
                (UnwindOperation::LargeStackAlloc, 0) => unwind_le_bytes(2),
                (UnwindOperation::LargeStackAlloc, 1) => unwind_le_bytes(4),
                (UnwindOperation::LargeStackAlloc, _) => {
                    panic!("unexpected stack alloc info value")
                }
                (UnwindOperation::SaveNonvolatileRegister, _) => unwind_le_bytes(2),
                (UnwindOperation::SaveNonvolatileRegisterFar, _) => unwind_le_bytes(4),
                (UnwindOperation::SaveXmm128, _) => unwind_le_bytes(2),
                (UnwindOperation::SaveXmm128Far, _) => unwind_le_bytes(4),
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
        PushNonvolatileRegister = 0,
        LargeStackAlloc = 1,
        SmallStackAlloc = 2,
        SetFramePointer = 3,
        SaveNonvolatileRegister = 4,
        SaveNonvolatileRegisterFar = 5,
        SaveXmm128 = 8,
        SaveXmm128Far = 9,
        PushMachineFrame = 10,
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
                8 => Self::SaveXmm128,
                9 => Self::SaveXmm128Far,
                10 => Self::PushMachineFrame,
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
}

mod systemv {
    fn register_name<'a>(register: gimli::Register) -> std::borrow::Cow<'a, str> {
        Cow::Owned(format!("r{}", register.0))
    }

    pub fn dump<W: Write>(text: &mut W, bytes: &[u8], address_size: u8) {
        let mut eh_frame = gimli::EhFrame::new(bytes, gimli::LittleEndian);
        eh_frame.set_address_size(address_size);
        let bases = gimli::BaseAddresses::default();
        dump_eh_frame(text, &eh_frame, &bases, &register_name).unwrap();
    }

    // Remainder copied from https://github.com/gimli-rs/gimli/blob/1e49ffc9af4ec64a1b7316924d73c933dd7157c5/examples/dwarfdump.rs
    use gimli::UnwindSection;
    use std::borrow::Cow;
    use std::collections::HashMap;
    use std::fmt::{self, Debug, Write};
    use std::result;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub(super) enum Error {
        GimliError(gimli::Error),
        IoError,
    }

    impl fmt::Display for Error {
        #[inline]
        fn fmt(&self, f: &mut fmt::Formatter) -> ::std::result::Result<(), fmt::Error> {
            Debug::fmt(self, f)
        }
    }

    impl From<gimli::Error> for Error {
        fn from(err: gimli::Error) -> Self {
            Self::GimliError(err)
        }
    }

    impl From<fmt::Error> for Error {
        fn from(_: fmt::Error) -> Self {
            Self::IoError
        }
    }

    pub(super) type Result<T> = result::Result<T, Error>;

    pub(super) trait Reader: gimli::Reader<Offset = usize> + Send + Sync {}

    impl<'input, Endian> Reader for gimli::EndianSlice<'input, Endian> where
        Endian: gimli::Endianity + Send + Sync
    {
    }

    pub(super) fn dump_eh_frame<R: Reader, W: Write>(
        w: &mut W,
        eh_frame: &gimli::EhFrame<R>,
        bases: &gimli::BaseAddresses,
        register_name: &dyn Fn(gimli::Register) -> Cow<'static, str>,
    ) -> Result<()> {
        let mut cies = HashMap::new();

        let mut entries = eh_frame.entries(bases);
        loop {
            match entries.next()? {
                None => return Ok(()),
                Some(gimli::CieOrFde::Cie(cie)) => {
                    writeln!(w, "{:#010x}: CIE", cie.offset())?;
                    writeln!(w, "        length: {:#010x}", cie.entry_len())?;
                    // TODO: CIE_id
                    writeln!(w, "       version: {:#04x}", cie.version())?;
                    // TODO: augmentation
                    writeln!(w, "    code_align: {}", cie.code_alignment_factor())?;
                    writeln!(w, "    data_align: {}", cie.data_alignment_factor())?;
                    writeln!(w, "   ra_register: {:#x}", cie.return_address_register().0)?;
                    if let Some(encoding) = cie.lsda_encoding() {
                        writeln!(w, " lsda_encoding: {:#02x}", encoding.0)?;
                    }
                    if let Some((encoding, personality)) = cie.personality_with_encoding() {
                        write!(w, "   personality: {:#02x} ", encoding.0)?;
                        dump_pointer(w, personality)?;
                        writeln!(w)?;
                    }
                    if let Some(encoding) = cie.fde_address_encoding() {
                        writeln!(w, "  fde_encoding: {:#02x}", encoding.0)?;
                    }
                    dump_cfi_instructions(
                        w,
                        cie.instructions(eh_frame, bases),
                        true,
                        register_name,
                    )?;
                    writeln!(w)?;
                }
                Some(gimli::CieOrFde::Fde(partial)) => {
                    let mut offset = None;
                    let fde = partial.parse(|_, bases, o| {
                        offset = Some(o);
                        cies.entry(o)
                            .or_insert_with(|| eh_frame.cie_from_offset(bases, o))
                            .clone()
                    })?;

                    writeln!(w)?;
                    writeln!(w, "{:#010x}: FDE", fde.offset())?;
                    writeln!(w, "        length: {:#010x}", fde.entry_len())?;
                    writeln!(w, "   CIE_pointer: {:#010x}", offset.unwrap().0)?;
                    // TODO: symbolicate the start address like the canonical dwarfdump does.
                    writeln!(w, "    start_addr: {:#018x}", fde.initial_address())?;
                    writeln!(
                        w,
                        "    range_size: {:#018x} (end_addr = {:#018x})",
                        fde.len(),
                        fde.initial_address() + fde.len()
                    )?;
                    if let Some(lsda) = fde.lsda() {
                        write!(w, "          lsda: ")?;
                        dump_pointer(w, lsda)?;
                        writeln!(w)?;
                    }
                    dump_cfi_instructions(
                        w,
                        fde.instructions(eh_frame, bases),
                        false,
                        register_name,
                    )?;
                    writeln!(w)?;
                }
            }
        }
    }

    fn dump_pointer<W: Write>(w: &mut W, p: gimli::Pointer) -> Result<()> {
        match p {
            gimli::Pointer::Direct(p) => {
                write!(w, "{:#018x}", p)?;
            }
            gimli::Pointer::Indirect(p) => {
                write!(w, "({:#018x})", p)?;
            }
        }
        Ok(())
    }

    #[allow(clippy::unneeded_field_pattern)]
    fn dump_cfi_instructions<R: Reader, W: Write>(
        w: &mut W,
        mut insns: gimli::CallFrameInstructionIter<R>,
        is_initial: bool,
        register_name: &dyn Fn(gimli::Register) -> Cow<'static, str>,
    ) -> Result<()> {
        use gimli::CallFrameInstruction::*;

        // TODO: we need to actually evaluate these instructions as we iterate them
        // so we can print the initialized state for CIEs, and each unwind row's
        // registers for FDEs.
        //
        // TODO: We should print DWARF expressions for the CFI instructions that
        // embed DWARF expressions within themselves.

        if !is_initial {
            writeln!(w, "  Instructions:")?;
        }

        loop {
            match insns.next() {
                Err(e) => {
                    writeln!(w, "Failed to decode CFI instruction: {}", e)?;
                    return Ok(());
                }
                Ok(None) => {
                    if is_initial {
                        writeln!(w, "  Instructions: Init State:")?;
                    }
                    return Ok(());
                }
                Ok(Some(op)) => match op {
                    SetLoc { address } => {
                        writeln!(w, "                DW_CFA_set_loc ({:#x})", address)?;
                    }
                    AdvanceLoc { delta } => {
                        writeln!(w, "                DW_CFA_advance_loc ({})", delta)?;
                    }
                    DefCfa { register, offset } => {
                        writeln!(
                            w,
                            "                DW_CFA_def_cfa ({}, {})",
                            register_name(register),
                            offset
                        )?;
                    }
                    DefCfaSf {
                        register,
                        factored_offset,
                    } => {
                        writeln!(
                            w,
                            "                DW_CFA_def_cfa_sf ({}, {})",
                            register_name(register),
                            factored_offset
                        )?;
                    }
                    DefCfaRegister { register } => {
                        writeln!(
                            w,
                            "                DW_CFA_def_cfa_register ({})",
                            register_name(register)
                        )?;
                    }
                    DefCfaOffset { offset } => {
                        writeln!(w, "                DW_CFA_def_cfa_offset ({})", offset)?;
                    }
                    DefCfaOffsetSf { factored_offset } => {
                        writeln!(
                            w,
                            "                DW_CFA_def_cfa_offset_sf ({})",
                            factored_offset
                        )?;
                    }
                    DefCfaExpression { expression: _ } => {
                        writeln!(w, "                DW_CFA_def_cfa_expression (...)")?;
                    }
                    Undefined { register } => {
                        writeln!(
                            w,
                            "                DW_CFA_undefined ({})",
                            register_name(register)
                        )?;
                    }
                    SameValue { register } => {
                        writeln!(
                            w,
                            "                DW_CFA_same_value ({})",
                            register_name(register)
                        )?;
                    }
                    Offset {
                        register,
                        factored_offset,
                    } => {
                        writeln!(
                            w,
                            "                DW_CFA_offset ({}, {})",
                            register_name(register),
                            factored_offset
                        )?;
                    }
                    OffsetExtendedSf {
                        register,
                        factored_offset,
                    } => {
                        writeln!(
                            w,
                            "                DW_CFA_offset_extended_sf ({}, {})",
                            register_name(register),
                            factored_offset
                        )?;
                    }
                    ValOffset {
                        register,
                        factored_offset,
                    } => {
                        writeln!(
                            w,
                            "                DW_CFA_val_offset ({}, {})",
                            register_name(register),
                            factored_offset
                        )?;
                    }
                    ValOffsetSf {
                        register,
                        factored_offset,
                    } => {
                        writeln!(
                            w,
                            "                DW_CFA_val_offset_sf ({}, {})",
                            register_name(register),
                            factored_offset
                        )?;
                    }
                    Register {
                        dest_register,
                        src_register,
                    } => {
                        writeln!(
                            w,
                            "                DW_CFA_register ({}, {})",
                            register_name(dest_register),
                            register_name(src_register)
                        )?;
                    }
                    Expression {
                        register,
                        expression: _,
                    } => {
                        writeln!(
                            w,
                            "                DW_CFA_expression ({}, ...)",
                            register_name(register)
                        )?;
                    }
                    ValExpression {
                        register,
                        expression: _,
                    } => {
                        writeln!(
                            w,
                            "                DW_CFA_val_expression ({}, ...)",
                            register_name(register)
                        )?;
                    }
                    Restore { register } => {
                        writeln!(
                            w,
                            "                DW_CFA_restore ({})",
                            register_name(register)
                        )?;
                    }
                    RememberState => {
                        writeln!(w, "                DW_CFA_remember_state")?;
                    }
                    RestoreState => {
                        writeln!(w, "                DW_CFA_restore_state")?;
                    }
                    ArgsSize { size } => {
                        writeln!(w, "                DW_CFA_GNU_args_size ({})", size)?;
                    }
                    Nop => {
                        writeln!(w, "                DW_CFA_nop")?;
                    }
                },
            }
        }
    }
}
