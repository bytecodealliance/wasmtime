//! Implementation of the `wasmtime objdump` CLI command.

use capstone::InsnGroupType::{CS_GRP_JUMP, CS_GRP_RET};
use clap::Parser;
use cranelift_codegen::isa::lookup_by_name;
use cranelift_codegen::settings::Flags;
use object::read::elf::ElfFile64;
use object::{Architecture, Endianness, FileFlags, Object, ObjectSection, ObjectSymbol};
use pulley_interpreter::decode::{Decoder, DecodingError, OpVisitor};
use pulley_interpreter::disas::Disassembler;
use smallvec::SmallVec;
use std::io::{IsTerminal, Read, Write};
use std::iter::{self, Peekable};
use std::path::{Path, PathBuf};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use wasmtime::{Engine, Result, bail, error::Context as _};
use wasmtime_environ::{
    FilePos, FrameInstPos, FrameStackShape, FrameStateSlot, FrameTable, FrameTableDescriptorIndex,
    StackMap, Trap, obj,
};
use wasmtime_unwinder::{ExceptionHandler, ExceptionTable};

/// A helper utility in wasmtime to explore the compiled object file format of
/// a `*.cwasm` file.
#[derive(Parser)]
pub struct ObjdumpCommand {
    /// The path to a compiled `*.cwasm` file.
    ///
    /// If this is `-` or not provided then stdin is used as input.
    cwasm: Option<PathBuf>,

    /// Whether or not to display function/instruction addresses.
    #[arg(long)]
    addresses: bool,

    /// Whether or not to try to only display addresses of instruction jump
    /// targets.
    #[arg(long)]
    address_jumps: bool,

    /// What functions should be printed
    #[arg(long, default_value = "wasm", value_name = "KIND")]
    funcs: Vec<Func>,

    /// String filter to apply to function names to only print some functions.
    #[arg(long, value_name = "STR")]
    filter: Option<String>,

    /// Whether or not instruction bytes are disassembled.
    #[arg(long)]
    bytes: bool,

    /// Whether or not to use color.
    #[arg(long, default_value = "auto")]
    color: ColorChoice,

    /// Whether or not to interleave instructions with address maps.
    #[arg(long, require_equals = true, value_name = "true|false")]
    addrmap: Option<Option<bool>>,

    /// Column width of how large an address is rendered as.
    #[arg(long, default_value = "10", value_name = "N")]
    address_width: usize,

    /// Whether or not to show information about what instructions can trap.
    #[arg(long, require_equals = true, value_name = "true|false")]
    traps: Option<Option<bool>>,

    /// Whether or not to show information about stack maps.
    #[arg(long, require_equals = true, value_name = "true|false")]
    stack_maps: Option<Option<bool>>,

    /// Whether or not to show information about exception tables.
    #[arg(long, require_equals = true, value_name = "true|false")]
    exception_tables: Option<Option<bool>>,

    /// Whether or not to show information about frame tables.
    #[arg(long, require_equals = true, value_name = "true|false")]
    frame_tables: Option<Option<bool>>,
}

fn optional_flag_with_default(flag: Option<Option<bool>>, default: bool) -> bool {
    match flag {
        None => default,
        Some(None) => true,
        Some(Some(val)) => val,
    }
}

impl ObjdumpCommand {
    fn addrmap(&self) -> bool {
        optional_flag_with_default(self.addrmap, false)
    }

    fn traps(&self) -> bool {
        optional_flag_with_default(self.traps, true)
    }

    fn stack_maps(&self) -> bool {
        optional_flag_with_default(self.stack_maps, true)
    }

    fn exception_tables(&self) -> bool {
        optional_flag_with_default(self.exception_tables, true)
    }

    fn frame_tables(&self) -> bool {
        optional_flag_with_default(self.frame_tables, true)
    }

    /// Executes the command.
    pub fn execute(self) -> Result<()> {
        // Setup stdout handling color options. Also build some variables used
        // below to configure colors of certain items.
        let mut choice = self.color;
        if choice == ColorChoice::Auto && !std::io::stdout().is_terminal() {
            choice = ColorChoice::Never;
        }
        let mut stdout = StandardStream::stdout(choice);

        let mut color_address = ColorSpec::new();
        color_address.set_bold(true).set_fg(Some(Color::Yellow));
        let mut color_bytes = ColorSpec::new();
        color_bytes.set_fg(Some(Color::Magenta));

        let bytes = self.read_cwasm()?;

        // Double-check this is a `*.cwasm`
        if Engine::detect_precompiled(&bytes).is_none() {
            bail!("not a `*.cwasm` file from wasmtime: {:?}", self.cwasm);
        }

        // Parse the input as an ELF file, extract the `.text` section.
        let elf = ElfFile64::<Endianness>::parse(&bytes)?;
        let text = elf
            .section_by_name(".text")
            .context("missing .text section")?;
        let text = text.data()?;

        let frame_table_descriptors = elf
            .section_by_name(obj::ELF_WASMTIME_FRAMES)
            .and_then(|section| section.data().ok())
            .and_then(|bytes| FrameTable::parse(bytes, text).ok());

        let mut breakpoints = frame_table_descriptors
            .iter()
            .flat_map(|ftd| ftd.breakpoint_patches())
            .map(|(wasm_pc, patch)| (wasm_pc, patch.offset, SmallVec::from(patch.enable)))
            .collect::<Vec<_>>();
        breakpoints.sort_by_key(|(_wasm_pc, native_offset, _patch)| *native_offset);
        let breakpoints: Box<dyn Iterator<Item = _>> = Box::new(breakpoints.into_iter());
        let breakpoints = breakpoints.peekable();

        // Build the helper that'll get used to attach decorations/annotations
        // to various instructions.
        let mut decorator = Decorator {
            addrmap: elf
                .section_by_name(obj::ELF_WASMTIME_ADDRMAP)
                .and_then(|section| section.data().ok())
                .and_then(|bytes| wasmtime_environ::iterate_address_map(bytes))
                .map(|i| (Box::new(i) as Box<dyn Iterator<Item = _>>).peekable()),
            traps: elf
                .section_by_name(obj::ELF_WASMTIME_TRAPS)
                .and_then(|section| section.data().ok())
                .and_then(|bytes| wasmtime_environ::iterate_traps(bytes))
                .map(|i| (Box::new(i) as Box<dyn Iterator<Item = _>>).peekable()),
            stack_maps: elf
                .section_by_name(obj::ELF_WASMTIME_STACK_MAP)
                .and_then(|section| section.data().ok())
                .and_then(|bytes| StackMap::iter(bytes))
                .map(|i| (Box::new(i) as Box<dyn Iterator<Item = _>>).peekable()),
            exception_tables: elf
                .section_by_name(obj::ELF_WASMTIME_EXCEPTIONS)
                .and_then(|section| section.data().ok())
                .and_then(|bytes| ExceptionTable::parse(bytes).ok())
                .map(|table| table.into_iter())
                .map(|i| (Box::new(i) as Box<dyn Iterator<Item = _>>).peekable()),
            frame_tables: elf
                .section_by_name(obj::ELF_WASMTIME_FRAMES)
                .and_then(|section| section.data().ok())
                .and_then(|bytes| FrameTable::parse(bytes, text).ok())
                .map(|table| table.into_program_points())
                .map(|i| (Box::new(i) as Box<dyn Iterator<Item = _>>).peekable()),

            breakpoints,

            frame_table_descriptors,

            objdump: &self,
        };

        // Iterate over all symbols which will be functions for a cwasm and
        // we'll disassemble them all.
        let mut first = true;
        for sym in elf.symbols() {
            let name = match sym.name() {
                Ok(name) => name,
                Err(_) => continue,
            };
            let bytes = &text[sym.address() as usize..][..sym.size() as usize];

            let kind = if name.starts_with("wasmtime_builtin")
                || name.starts_with("wasmtime_patchable_builtin")
            {
                Func::Builtin
            } else if name.contains("]::function[") {
                Func::Wasm
            } else if name.contains("trampoline")
                || name.ends_with("_array_call")
                || name.ends_with("_wasm_call")
            {
                Func::Trampoline
            } else if name.contains("libcall") || name.starts_with("component") {
                Func::Libcall
            } else {
                panic!("unknown symbol: {name}")
            };

            // Apply any filters, if provided, to this function to look at just
            // one function in the disassembly.
            if self.funcs.is_empty() {
                if kind != Func::Wasm {
                    continue;
                }
            } else {
                if !(self.funcs.contains(&Func::All) || self.funcs.contains(&kind)) {
                    continue;
                }
            }
            if let Some(filter) = &self.filter {
                if !name.contains(filter) {
                    continue;
                }
            }

            // Place a blank line between functions.
            if first {
                first = false;
            } else {
                writeln!(stdout)?;
            }

            // Print the function's address, if so desired. Then print the
            // function name.
            if self.addresses {
                stdout.set_color(color_address.clone().set_bold(true))?;
                write!(stdout, "{:08x} ", sym.address())?;
                stdout.reset()?;
            }
            stdout.set_color(ColorSpec::new().set_bold(true).set_fg(Some(Color::Green)))?;
            write!(stdout, "{name}")?;
            stdout.reset()?;
            writeln!(stdout, ":")?;

            // Tracking variables for rough heuristics of printing targets of
            // jump instructions for `--address-jumps` mode.
            let mut prev_jump = false;
            let mut write_offsets = false;

            for inst in self.disas(&elf, bytes, sym.address())? {
                let Inst {
                    address,
                    is_jump,
                    is_return,
                    disassembly: disas,
                    bytes,
                } = inst;

                // Generate an infinite list of bytes to make printing below
                // easier, but only limit `inline_bytes` to get printed before
                // an instruction.
                let mut bytes = bytes.iter().map(Some).chain(iter::repeat(None));
                let inline_bytes = 9;
                let width = self.address_width;

                // Collect any "decorations" or annotations for this
                // instruction. This includes the address map, stack
                // maps, exception handlers, etc.
                //
                // Once they're collected then we print them before or
                // after the instruction attempting to use some
                // unicode characters to make it easier to read/scan.
                //
                // Note that some decorations occur "before" an
                // instruction: for example, exception handler entries
                // logically occur at the return point after a call,
                // so "before" the instruction following the call.
                let mut pre_decorations = Vec::new();
                let mut post_decorations = Vec::new();
                decorator.decorate(address, &mut pre_decorations, &mut post_decorations);

                let print_whitespace_to_decoration = |stdout: &mut StandardStream| -> Result<()> {
                    write!(stdout, "{:width$}  ", "")?;
                    if self.bytes {
                        for _ in 0..inline_bytes + 1 {
                            write!(stdout, "   ")?;
                        }
                    }
                    Ok(())
                };

                let print_decorations =
                    |stdout: &mut StandardStream, decorations: Vec<String>| -> Result<()> {
                        for (i, decoration) in decorations.iter().enumerate() {
                            print_whitespace_to_decoration(stdout)?;
                            let mut color = ColorSpec::new();
                            color.set_fg(Some(Color::Cyan));
                            stdout.set_color(&color)?;
                            let final_decoration = i == decorations.len() - 1;
                            if !final_decoration {
                                write!(stdout, "├")?;
                            } else {
                                write!(stdout, "╰")?;
                            }
                            for (i, line) in decoration.lines().enumerate() {
                                if i == 0 {
                                    write!(stdout, "─╼ ")?;
                                } else {
                                    print_whitespace_to_decoration(stdout)?;
                                    if final_decoration {
                                        write!(stdout, "    ")?;
                                    } else {
                                        write!(stdout, "│   ")?;
                                    }
                                }
                                writeln!(stdout, "{line}")?;
                            }
                            stdout.reset()?;
                        }
                        Ok(())
                    };

                print_decorations(&mut stdout, pre_decorations)?;

                // Some instructions may disassemble to multiple lines, such as
                // `br_table` with Pulley. Handle separate lines per-instruction
                // here.
                for (i, line) in disas.lines().enumerate() {
                    let print_address = self.addresses
                        || (self.address_jumps && (write_offsets || (prev_jump && !is_jump)));
                    if i == 0 && print_address {
                        stdout.set_color(&color_address)?;
                        write!(stdout, "{address:>width$x}: ")?;
                        stdout.reset()?;
                    } else {
                        write!(stdout, "{:width$}  ", "")?;
                    }

                    // If we're printing inline bytes then print up to
                    // `inline_bytes` of instruction data, and any remaining
                    // data will go on the next line, if any, or after the
                    // instruction below.
                    if self.bytes {
                        stdout.set_color(&color_bytes)?;
                        for byte in bytes.by_ref().take(inline_bytes) {
                            match byte {
                                Some(byte) => write!(stdout, "{byte:02x} ")?,
                                None => write!(stdout, "   ")?,
                            }
                        }
                        write!(stdout, "  ")?;
                        stdout.reset()?;
                    }

                    writeln!(stdout, "{line}")?;
                }

                // Flip write_offsets to true once we've seen a `ret`, as
                // instructions that follow the return are often related to trap
                // tables.
                write_offsets |= is_return;
                prev_jump = is_jump;

                // After the instruction is printed then finish printing the
                // instruction bytes if any are present. Still limit to
                // `inline_bytes` per line.
                if self.bytes {
                    let mut inline = 0;
                    stdout.set_color(&color_bytes)?;
                    for byte in bytes {
                        let Some(byte) = byte else { break };
                        if inline == 0 {
                            write!(stdout, "{:width$}  ", "")?;
                        } else {
                            write!(stdout, " ")?;
                        }
                        write!(stdout, "{byte:02x}")?;
                        inline += 1;
                        if inline == inline_bytes {
                            writeln!(stdout)?;
                            inline = 0;
                        }
                    }
                    stdout.reset()?;
                    if inline > 0 {
                        writeln!(stdout)?;
                    }
                }

                print_decorations(&mut stdout, post_decorations)?;
            }
        }
        Ok(())
    }

    /// Disassembles `func` contained within `elf` returning a list of
    /// instructions that represent the function.
    fn disas(&self, elf: &ElfFile64<'_, Endianness>, func: &[u8], addr: u64) -> Result<Vec<Inst>> {
        let cranelift_target = match elf.architecture() {
            Architecture::X86_64 => "x86_64",
            Architecture::Aarch64 => "aarch64",
            Architecture::S390x => "s390x",
            Architecture::Riscv64 => {
                let e_flags = match elf.flags() {
                    FileFlags::Elf { e_flags, .. } => e_flags,
                    _ => bail!("not an ELF file"),
                };
                if e_flags & (obj::EF_WASMTIME_PULLEY32 | obj::EF_WASMTIME_PULLEY64) != 0 {
                    return self.disas_pulley(func, addr);
                } else {
                    "riscv64"
                }
            }
            other => bail!("unknown architecture {other:?}"),
        };
        let builder =
            lookup_by_name(cranelift_target).context("failed to load cranelift ISA builder")?;
        let flags = cranelift_codegen::settings::builder();
        let isa = builder.finish(Flags::new(flags))?;
        let isa = &*isa;
        let capstone = isa
            .to_capstone()
            .context("failed to create a capstone disassembler")?;

        let insts = capstone
            .disasm_all(func, addr)?
            .into_iter()
            .map(|inst| {
                let detail = capstone.insn_detail(&inst).ok();
                let detail = detail.as_ref();
                let is_jump = detail
                    .map(|d| {
                        d.groups()
                            .iter()
                            .find(|g| g.0 as u32 == CS_GRP_JUMP)
                            .is_some()
                    })
                    .unwrap_or(false);

                let is_return = detail
                    .map(|d| {
                        d.groups()
                            .iter()
                            .find(|g| g.0 as u32 == CS_GRP_RET)
                            .is_some()
                    })
                    .unwrap_or(false);

                let disassembly = match (inst.mnemonic(), inst.op_str()) {
                    (Some(i), Some(o)) => {
                        if o.is_empty() {
                            format!("{i}")
                        } else {
                            format!("{i:7} {o}")
                        }
                    }
                    (Some(i), None) => format!("{i}"),
                    _ => unreachable!(),
                };

                let address = inst.address();
                Inst {
                    address,
                    is_jump,
                    is_return,
                    bytes: inst.bytes().to_vec(),
                    disassembly,
                }
            })
            .collect::<Vec<_>>();
        Ok(insts)
    }

    /// Same as `dias` above, but just for Pulley.
    fn disas_pulley(&self, func: &[u8], addr: u64) -> Result<Vec<Inst>> {
        let mut result = vec![];

        let mut disas = Disassembler::new(func);
        disas.offsets(false);
        disas.hexdump(false);
        disas.start_offset(usize::try_from(addr).unwrap());
        let mut decoder = Decoder::new();
        let mut last_disas_pos = 0;
        loop {
            let start_addr = disas.bytecode().position();

            match decoder.decode_one(&mut disas) {
                // If we got EOF at the initial position, then we're done disassembling.
                Err(DecodingError::UnexpectedEof { position }) if position == start_addr => break,

                // Otherwise, propagate the error.
                Err(e) => {
                    return Err(e).context("failed to disassembly pulley bytecode");
                }

                Ok(()) => {
                    let bytes_range = start_addr..disas.bytecode().position();
                    let disassembly = disas.disas()[last_disas_pos..].trim();
                    last_disas_pos = disas.disas().len();
                    let address = u64::try_from(start_addr).unwrap() + addr;
                    let is_jump = disassembly.contains("jump") || disassembly.contains("br_");
                    let is_return = disassembly == "ret";
                    result.push(Inst {
                        bytes: func[bytes_range].to_vec(),
                        address,
                        is_jump,
                        is_return,
                        disassembly: disassembly.to_string(),
                    });
                }
            }
        }

        Ok(result)
    }

    /// Helper to read the input bytes of the `*.cwasm` handling stdin
    /// automatically.
    fn read_cwasm(&self) -> Result<Vec<u8>> {
        if let Some(path) = &self.cwasm {
            if path != Path::new("-") {
                return std::fs::read(path).with_context(|| format!("failed to read {path:?}"));
            }
        }

        let mut stdin = Vec::new();
        std::io::stdin()
            .read_to_end(&mut stdin)
            .context("failed to read stdin")?;
        Ok(stdin)
    }
}

/// Helper structure to package up metadata about an instruction.
struct Inst {
    address: u64,
    is_jump: bool,
    is_return: bool,
    disassembly: String,
    bytes: Vec<u8>,
}

#[derive(clap::ValueEnum, Clone, Copy, PartialEq, Eq)]
enum Func {
    All,
    Wasm,
    Trampoline,
    Builtin,
    Libcall,
}

struct Decorator<'a> {
    objdump: &'a ObjdumpCommand,
    addrmap: Option<Peekable<Box<dyn Iterator<Item = (u32, FilePos)> + 'a>>>,
    traps: Option<Peekable<Box<dyn Iterator<Item = (u32, Trap)> + 'a>>>,
    stack_maps: Option<Peekable<Box<dyn Iterator<Item = (u32, StackMap<'a>)> + 'a>>>,
    exception_tables:
        Option<Peekable<Box<dyn Iterator<Item = (u32, Option<u32>, Vec<ExceptionHandler>)> + 'a>>>,
    frame_tables: Option<
        Peekable<
            Box<
                dyn Iterator<
                        Item = (
                            u32,
                            FrameInstPos,
                            Vec<(u32, FrameTableDescriptorIndex, FrameStackShape)>,
                        ),
                    > + 'a,
            >,
        >,
    >,

    // Breakpoint table, sorted by native offset instead so we can
    // display inline with disassembly (the table in the image is
    // sorted by Wasm PC).
    breakpoints: Peekable<Box<dyn Iterator<Item = (u32, usize, SmallVec<[u8; 8]>)>>>,

    frame_table_descriptors: Option<FrameTable<'a>>,
}

impl Decorator<'_> {
    fn decorate(&mut self, address: u64, pre_list: &mut Vec<String>, post_list: &mut Vec<String>) {
        self.addrmap(address, post_list);
        self.traps(address, post_list);
        self.stack_maps(address, post_list);
        self.exception_table(address, pre_list);
        self.frame_table(address, pre_list, post_list);
        self.breakpoints(address, pre_list);
    }

    fn addrmap(&mut self, address: u64, list: &mut Vec<String>) {
        if !self.objdump.addrmap() {
            return;
        }
        let Some(addrmap) = &mut self.addrmap else {
            return;
        };
        while let Some((addr, pos)) = addrmap.next_if(|(addr, _pos)| u64::from(*addr) <= address) {
            if u64::from(addr) != address {
                continue;
            }
            if let Some(offset) = pos.file_offset() {
                list.push(format!("addrmap: {offset:#x}"));
            }
        }
    }

    fn traps(&mut self, address: u64, list: &mut Vec<String>) {
        if !self.objdump.traps() {
            return;
        }
        let Some(traps) = &mut self.traps else {
            return;
        };
        while let Some((addr, trap)) = traps.next_if(|(addr, _pos)| u64::from(*addr) <= address) {
            if u64::from(addr) != address {
                continue;
            }
            list.push(format!("trap: {trap:?}"));
        }
    }

    fn stack_maps(&mut self, address: u64, list: &mut Vec<String>) {
        if !self.objdump.stack_maps() {
            return;
        }
        let Some(stack_maps) = &mut self.stack_maps else {
            return;
        };
        while let Some((addr, stack_map)) =
            stack_maps.next_if(|(addr, _pos)| u64::from(*addr) <= address)
        {
            if u64::from(addr) != address {
                continue;
            }
            list.push(format!(
                "stack_map: frame_size={}, frame_offsets={:?}",
                stack_map.frame_size(),
                stack_map.offsets().collect::<Vec<_>>()
            ));
        }
    }

    fn exception_table(&mut self, address: u64, list: &mut Vec<String>) {
        if !self.objdump.exception_tables() {
            return;
        }
        let Some(exception_tables) = &mut self.exception_tables else {
            return;
        };
        while let Some((addr, frame_offset, handlers)) =
            exception_tables.next_if(|(addr, _, _)| u64::from(*addr) <= address)
        {
            if u64::from(addr) != address {
                continue;
            }
            if let Some(frame_offset) = frame_offset {
                list.push(format!(
                    "exception frame offset: SP = FP - 0x{frame_offset:x}",
                ));
            }
            for handler in &handlers {
                let tag = match handler.tag {
                    Some(tag) => format!("tag={tag}"),
                    None => "default handler".to_string(),
                };
                let context = match handler.context_sp_offset {
                    Some(offset) => format!("context at [SP+0x{offset:x}]"),
                    None => "no dynamic context".to_string(),
                };
                list.push(format!(
                    "exception handler: {tag}, {context}, handler=0x{:x}",
                    handler.handler_offset
                ));
            }
        }
    }

    fn frame_table(
        &mut self,
        address: u64,
        pre_list: &mut Vec<String>,
        post_list: &mut Vec<String>,
    ) {
        if !self.objdump.frame_tables() {
            return;
        }
        let (Some(frame_table_iter), Some(frame_tables)) =
            (&mut self.frame_tables, &self.frame_table_descriptors)
        else {
            return;
        };

        while let Some((addr, pos, frames)) =
            frame_table_iter.next_if(|(addr, _, _)| u64::from(*addr) <= address)
        {
            if u64::from(addr) != address {
                continue;
            }
            let list = match pos {
                // N.B.: the "post" position means that we are
                // attached to the end of the previous instruction
                // (its "post"); which means that from this
                // instruction's PoV, we print before the instruction
                // (the "pre list"). And vice versa for the "pre"
                // position. Hence the reversal here.
                FrameInstPos::Post => &mut *pre_list,
                FrameInstPos::Pre => &mut *post_list,
            };
            let pos = match pos {
                FrameInstPos::Post => "after previous inst",
                FrameInstPos::Pre => "before next inst",
            };
            for (wasm_pc, frame_descriptor, stack_shape) in frames {
                let (frame_descriptor_data, offset) =
                    frame_tables.frame_descriptor(frame_descriptor).unwrap();
                let frame_descriptor = FrameStateSlot::parse(frame_descriptor_data).unwrap();

                let local_shape = Self::describe_local_shape(&frame_descriptor);
                let stack_shape = Self::describe_stack_shape(&frame_descriptor, stack_shape);
                let func_key = frame_descriptor.func_key();
                list.push(format!("debug frame state ({pos}): func key {func_key:?}, wasm PC {wasm_pc}, slot at FP-0x{offset:x}, locals {local_shape}, stack {stack_shape}"));
            }
        }
    }

    fn breakpoints(&mut self, address: u64, list: &mut Vec<String>) {
        while let Some((wasm_pc, addr, patch)) = self.breakpoints.next_if(|(_, addr, patch)| {
            u64::try_from(*addr).unwrap() + u64::try_from(patch.len()).unwrap() <= address
        }) {
            if u64::try_from(addr).unwrap() + u64::try_from(patch.len()).unwrap() != address {
                continue;
            }
            list.push(format!(
                "breakpoint patch: wasm PC {wasm_pc}, patch bytes {patch:?}"
            ));
        }
    }

    fn describe_local_shape(desc: &FrameStateSlot<'_>) -> String {
        let mut parts = vec![];
        for (offset, ty) in desc.locals() {
            parts.push(format!("{ty:?} @ slot+0x{:x}", offset.offset()));
        }
        parts.join(", ")
    }

    fn describe_stack_shape(desc: &FrameStateSlot<'_>, shape: FrameStackShape) -> String {
        let mut parts = vec![];
        for (offset, ty) in desc.stack(shape) {
            parts.push(format!("{ty:?} @ slot+0x{:x}", offset.offset()));
        }
        parts.reverse();
        parts.join(", ")
    }
}
