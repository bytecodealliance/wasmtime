//! Test command for verifying the unwind emitted for each function.
//!
//! The `unwind` test command runs each function through the full code generator pipeline.
#![cfg_attr(feature = "cargo-clippy", allow(clippy::cast_ptr_alignment))]

use crate::subtest::{run_filecheck, Context, SubTest, SubtestResult};
use cranelift_codegen;
use cranelift_codegen::binemit::{FrameUnwindKind, FrameUnwindOffset, FrameUnwindSink, Reloc};
use cranelift_codegen::ir;
use cranelift_reader::TestCommand;
use std::borrow::Cow;
use std::fmt::Write;

struct TestUnwind;

pub fn subtest(parsed: &TestCommand) -> SubtestResult<Box<dyn SubTest>> {
    assert_eq!(parsed.command, "fde");
    if !parsed.options.is_empty() {
        Err(format!("No options allowed on {}", parsed))
    } else {
        Ok(Box::new(TestUnwind))
    }
}

impl SubTest for TestUnwind {
    fn name(&self) -> &'static str {
        "fde"
    }

    fn is_mutating(&self) -> bool {
        false
    }

    fn needs_isa(&self) -> bool {
        true
    }

    fn run(&self, func: Cow<ir::Function>, context: &Context) -> SubtestResult<()> {
        let isa = context.isa.expect("unwind needs an ISA");

        if func.signature.call_conv != cranelift_codegen::isa::CallConv::SystemV {
            return run_filecheck(&"No unwind information.", context);
        }

        let mut comp_ctx = cranelift_codegen::Context::for_function(func.into_owned());
        comp_ctx.func.collect_frame_layout_info();

        comp_ctx.compile(isa).expect("failed to compile function");

        struct SimpleUnwindSink(pub Vec<u8>, pub usize, pub Vec<(Reloc, usize)>);
        impl FrameUnwindSink for SimpleUnwindSink {
            fn len(&self) -> FrameUnwindOffset {
                self.0.len()
            }
            fn bytes(&mut self, b: &[u8]) {
                self.0.extend_from_slice(b);
            }
            fn reloc(&mut self, r: Reloc, off: FrameUnwindOffset) {
                self.2.push((r, off));
            }
            fn set_entry_offset(&mut self, off: FrameUnwindOffset) {
                self.1 = off;
            }
        }

        let mut sink = SimpleUnwindSink(Vec::new(), 0, Vec::new());
        comp_ctx.emit_unwind_info(isa, FrameUnwindKind::Libunwind, &mut sink);

        let mut text = String::new();
        if sink.0.is_empty() {
            writeln!(text, "No unwind information.").unwrap();
        } else {
            print_unwind_info(&mut text, &sink.0, isa.pointer_bytes());
            writeln!(text, "Entry: {}", sink.1).unwrap();
            writeln!(text, "Relocs: {:?}", sink.2).unwrap();
        }

        run_filecheck(&text, context)
    }
}

fn register_name<'a>(register: gimli::Register) -> std::borrow::Cow<'a, str> {
    Cow::Owned(format!("r{}", register.0))
}

fn print_unwind_info(text: &mut String, mem: &[u8], address_size: u8) {
    let mut eh_frame = gimli::EhFrame::new(mem, gimli::LittleEndian);
    eh_frame.set_address_size(address_size);
    let bases = gimli::BaseAddresses::default();
    dwarfdump::dump_eh_frame(text, &eh_frame, &bases, &register_name).unwrap();
}

mod dwarfdump {
    // Copied from https://github.com/gimli-rs/gimli/blob/1e49ffc9af4ec64a1b7316924d73c933dd7157c5/examples/dwarfdump.rs
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
            Error::GimliError(err)
        }
    }

    impl From<fmt::Error> for Error {
        fn from(_: fmt::Error) -> Self {
            Error::IoError
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
