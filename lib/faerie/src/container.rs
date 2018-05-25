//! Utilities for working with Faerie container formats.

use cretonne_codegen::binemit::Reloc;
use target_lexicon::BinaryFormat;

/// An object file format.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Format {
    /// The ELF object file format.
    ELF,
    /// The Mach-O object file format.
    MachO,
}

/// Translate from a Cretonne `Reloc` to a raw object-file-format-specific
/// relocation code.
pub fn raw_relocation(reloc: Reloc, format: BinaryFormat) -> u32 {
    match format {
        BinaryFormat::Elf => {
            use goblin::elf;
            match reloc {
                Reloc::Abs4 => elf::reloc::R_X86_64_32,
                Reloc::Abs8 => elf::reloc::R_X86_64_64,
                Reloc::X86PCRel4 => elf::reloc::R_X86_64_PC32,
                // TODO: Get Cretonne to tell us when we can use
                // R_X86_64_GOTPCRELX/R_X86_64_REX_GOTPCRELX.
                Reloc::X86GOTPCRel4 => elf::reloc::R_X86_64_GOTPCREL,
                Reloc::X86PLTRel4 => elf::reloc::R_X86_64_PLT32,
                _ => unimplemented!(),
            }
        }
        BinaryFormat::Macho => unimplemented!("macho relocations"),
        _ => unimplemented!("unsupported format"),
    }
}
