//! Implementation of the `wasmtime objdump` CLI command.

use anyhow::{bail, Context, Result};
use capstone::InsnGroupType::{CS_GRP_JUMP, CS_GRP_RET};
use clap::Parser;
use cranelift_codegen::isa::lookup_by_name;
use cranelift_codegen::settings::Flags;
use object::read::elf::ElfFile64;
use object::{Architecture, Endianness, FileFlags, Object, ObjectSection, ObjectSymbol};
use pulley_interpreter::decode::{Decoder, DecodingError, OpVisitor};
use pulley_interpreter::disas::Disassembler;
use std::io::{IsTerminal, Read, Write};
use std::iter::{self, Peekable};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use wasmtime::Engine;
use wasmtime_environ::{obj, FilePos, Trap};

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

    /// What functions should be printed (all|wasm|trampoline|builtin|libcall, default: wasm)
    #[arg(long, value_parser = Func::from_str)]
    funcs: Vec<Func>,

    /// String filter to apply to function names to only print some functions.
    #[arg(long)]
    filter: Option<String>,

    /// Whether or not instruction bytes are disassembled.
    #[arg(long)]
    bytes: bool,

    /// Whether or not to use color.
    #[arg(long, default_value = "auto")]
    color: ColorChoice,

    /// Whether or not to interleave instructions with address maps.
    #[arg(long)]
    addrmap: bool,

    /// Column width of how large an address is rendered as.
    #[arg(long, default_value = "10")]
    address_width: usize,

    /// Whether or not to show information about what instructions can trap.
    #[arg(long)]
    traps: bool,
}

impl ObjdumpCommand {
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

            let kind = if name.starts_with("wasmtime_builtin") {
                Func::Builtin
            } else if name.contains("]::function[") {
                Func::Wasm
            } else if name.contains("trampoline") {
                Func::Trampoline
            } else if name.contains("libcall") {
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

                // And now finally after an instruction is printed try to
                // collect any "decorations" or annotations for this
                // instruction. This is for example the address map, stack maps,
                // etc.
                //
                // Once they're collected then print them after the instruction
                // attempting to use some unicode characters to make it easier
                // to read/scan.
                let mut decorations = Vec::new();
                decorator.decorate(address, &mut decorations);

                let print_whitespace_to_decoration = |stdout: &mut StandardStream| -> Result<()> {
                    write!(stdout, "{:width$}  ", "")?;
                    if self.bytes {
                        for _ in 0..inline_bytes + 1 {
                            write!(stdout, "   ")?;
                        }
                    }
                    Ok(())
                };
                for (i, decoration) in decorations.iter().enumerate() {
                    print_whitespace_to_decoration(&mut stdout)?;
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
                            print_whitespace_to_decoration(&mut stdout)?;
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum Func {
    All,
    Wasm,
    Trampoline,
    Builtin,
    Libcall,
}

impl FromStr for Func {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Func> {
        Ok(match s {
            "all" => Func::All,
            "wasm" => Func::Wasm,
            "trampoline" => Func::Trampoline,
            "builtin" => Func::Builtin,
            "libcall" => Func::Libcall,
            other => {
                bail!(
                    "unknown function kind `{other}`, \
                     must be [all|wasm|trampoline|builtin|libcall]"
                )
            }
        })
    }
}

struct Decorator<'a> {
    objdump: &'a ObjdumpCommand,
    addrmap: Option<Peekable<Box<dyn Iterator<Item = (u32, FilePos)> + 'a>>>,
    traps: Option<Peekable<Box<dyn Iterator<Item = (u32, Trap)> + 'a>>>,
}

impl Decorator<'_> {
    fn decorate(&mut self, address: u64, list: &mut Vec<String>) {
        self.addrmap(address, list);
        self.traps(address, list);
    }

    fn addrmap(&mut self, address: u64, list: &mut Vec<String>) {
        if !self.objdump.addrmap {
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
        if !self.objdump.traps {
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
}
