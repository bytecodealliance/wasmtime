//! Utilities for working with Faerie container formats.

use cranelift_codegen::binemit::Reloc;
use target_lexicon::{Architecture, BinaryFormat, Triple};

/// An object file format.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Format {
    /// The ELF object file format.
    ELF,
    /// The Mach-O object file format.
    MachO,
}

/// Translate from a Cranelift `Reloc` to a raw object-file-format-specific
/// relocation code and relocation-implied addend.
pub fn raw_relocation(reloc: Reloc, triple: &Triple) -> (u32, i64) {
    match triple.binary_format {
        BinaryFormat::Elf => {
            use goblin::elf;
            (
                match triple.architecture {
                    Architecture::X86_64 => {
                        match reloc {
                            Reloc::Abs4 => elf::reloc::R_X86_64_32,
                            Reloc::Abs8 => elf::reloc::R_X86_64_64,
                            Reloc::X86PCRel4 | Reloc::X86CallPCRel4 => elf::reloc::R_X86_64_PC32,
                            // TODO: Get Cranelift to tell us when we can use
                            // R_X86_64_GOTPCRELX/R_X86_64_REX_GOTPCRELX.
                            Reloc::X86CallPLTRel4 => elf::reloc::R_X86_64_PLT32,
                            Reloc::X86GOTPCRel4 => elf::reloc::R_X86_64_GOTPCREL,
                            _ => unimplemented!(),
                        }
                    }
                    _ => unimplemented!("unsupported architecture: {}", triple),
                },
                // Most ELF relocations do not include an implicit addend.
                0,
            )
        }
        BinaryFormat::Macho => {
            use goblin::mach;
            match triple.architecture {
                Architecture::X86_64 => {
                    match reloc {
                        Reloc::Abs8 => (u32::from(mach::relocation::R_ABS), 0),
                        // Mach-O doesn't need us to distinguish between PC-relative calls
                        // and PLT calls, but it does need us to distinguish between calls
                        // and non-calls. And, it includes the 4-byte addend implicitly.
                        Reloc::X86PCRel4 => (u32::from(mach::relocation::X86_64_RELOC_SIGNED), 4),
                        Reloc::X86CallPCRel4 | Reloc::X86CallPLTRel4 => {
                            (u32::from(mach::relocation::X86_64_RELOC_BRANCH), 4)
                        }
                        Reloc::X86GOTPCRel4 => {
                            (u32::from(mach::relocation::X86_64_RELOC_GOT_LOAD), 4)
                        }
                        _ => unimplemented!("unsupported mach-o reloc: {}", reloc),
                    }
                }
                _ => unimplemented!("unsupported architecture: {}", triple),
            }
        }
        _ => unimplemented!("unsupported format"),
    }
}
