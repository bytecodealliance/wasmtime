//! `.eh_frame` section emission for [`ObjectModule`].
//!
//! Cranelift's code generator already produces per-function `UnwindInfo`. On
//! System V targets that information is enough to construct a DWARF Call Frame
//! Information `.eh_frame` section consumable by libgcc/libunwind. This module
//! aggregates the generated FDEs into a single per-object section, with
//! relocations against each function's text symbol.
//!
//! [`ObjectModule`]: crate::ObjectModule

use anyhow::{Result, anyhow};
use cranelift_codegen::isa::TargetIsa;
use cranelift_codegen::isa::unwind::UnwindInfo;
use gimli::write::{
    Address, CieId, EhFrame, EndianVec, FrameTable, RelocateWriter, Relocation, RelocationTarget,
};
use object::write::{Object, Relocation as ObjectRelocation, StandardSection, SymbolId};
use object::{BinaryFormat, RelocationEncoding, RelocationFlags, RelocationKind};

pub(crate) struct UnwindBuilder {
    section: EhFrameSection,
    frame_table: FrameTable,
    cie_id: Option<CieId>,
    /// Maps gimli's opaque symbol indices back to `object` `SymbolId`s.
    symbols: Vec<SymbolId>,
}

impl UnwindBuilder {
    pub(crate) fn new(endian: object::Endianness) -> Self {
        let endian = match endian {
            object::Endianness::Little => gimli::RunTimeEndian::Little,
            object::Endianness::Big => gimli::RunTimeEndian::Big,
        };
        Self {
            section: EhFrameSection {
                writer: EndianVec::new(endian),
                relocations: Vec::new(),
            },
            frame_table: FrameTable::default(),
            cie_id: None,
            symbols: Vec::new(),
        }
    }

    /// Record an FDE for the given function.
    ///
    /// Windows-flavored unwind info is silently ignored: `.pdata`/`.xdata`
    /// emission lives on a separate code path. The CIE is created lazily on
    /// the first System V FDE so that an unwind builder shared across many
    /// objects does not pay for an empty CIE when the target produces no
    /// System V info at all.
    pub(crate) fn add_function(
        &mut self,
        isa: &dyn TargetIsa,
        func_symbol: SymbolId,
        info: UnwindInfo,
    ) {
        let UnwindInfo::SystemV(sysv) = info else {
            return;
        };
        let cie_id = match self.cie_id {
            Some(id) => id,
            None => {
                let Some(cie) = isa.create_systemv_cie() else {
                    return;
                };
                let id = self.frame_table.add_cie(cie);
                self.cie_id = Some(id);
                id
            }
        };
        let symbol_index = self.symbols.len();
        self.symbols.push(func_symbol);
        let address = Address::Symbol {
            symbol: symbol_index,
            addend: 0,
        };
        self.frame_table.add_fde(cie_id, sysv.to_fde(address));
    }

    /// Serialize all collected FDEs into an `.eh_frame` section on `object`.
    ///
    /// Returns without writing anything if no FDE was ever added (e.g. when
    /// the user enabled unwind info on a target that does not produce System V
    /// frames). The pointer-width unwrap is safe because [`ObjectBuilder`]
    /// has already rejected unknown architectures during construction.
    ///
    /// [`ObjectBuilder`]: crate::ObjectBuilder
    pub(crate) fn finish(self, object: &mut Object<'static>, isa: &dyn TargetIsa) -> Result<()> {
        let UnwindBuilder {
            section,
            frame_table,
            cie_id,
            symbols,
        } = self;
        if cie_id.is_none() {
            return Ok(());
        }
        let mut eh_frame = EhFrame(section);
        frame_table
            .write_eh_frame(&mut eh_frame)
            .map_err(|err| anyhow!("failed to write .eh_frame: {err}"))?;
        let EhFrame(EhFrameSection {
            writer,
            relocations,
        }) = eh_frame;

        let section_id = object.section_id(StandardSection::EhFrame);
        let alignment = u64::from(isa.triple().pointer_width().unwrap().bytes());
        object.append_section_data(section_id, &writer.into_vec(), alignment);

        let format = object.format();
        for reloc in relocations {
            let symbol_id = match reloc.target {
                RelocationTarget::Symbol(index) => symbols[index],
                // gimli's writer only emits section-relative relocations for
                // DWARF debug sections, never for `.eh_frame`. Skip defensively.
                RelocationTarget::Section(_) => continue,
            };
            let flags = translate_eh_pe(reloc.eh_pe, reloc.size, format)?;
            object
                .add_relocation(
                    section_id,
                    ObjectRelocation {
                        offset: reloc.offset as u64,
                        symbol: symbol_id,
                        addend: reloc.addend,
                        flags,
                    },
                )
                .map_err(|err| anyhow!("failed to record .eh_frame relocation: {err}"))?;
        }
        Ok(())
    }
}

struct EhFrameSection {
    writer: EndianVec<gimli::RunTimeEndian>,
    relocations: Vec<Relocation>,
}

impl RelocateWriter for EhFrameSection {
    type Writer = EndianVec<gimli::RunTimeEndian>;

    fn writer(&self) -> &Self::Writer {
        &self.writer
    }

    fn writer_mut(&mut self) -> &mut Self::Writer {
        &mut self.writer
    }

    fn relocate(&mut self, relocation: Relocation) {
        self.relocations.push(relocation);
    }
}

/// Translate a gimli `.eh_frame` relocation request into the `object` crate's
/// relocation flags for the target binary format.
///
/// `eh_pe` is `None` for non-pointer relocations (rare in `.eh_frame`); in that
/// case we treat the request as an absolute pointer of the requested width.
fn translate_eh_pe(
    eh_pe: Option<gimli::constants::DwEhPe>,
    size: u8,
    format: BinaryFormat,
) -> Result<RelocationFlags> {
    use gimli::constants::*;

    let Some(pe) = eh_pe else {
        return Ok(RelocationFlags::Generic {
            kind: RelocationKind::Absolute,
            encoding: RelocationEncoding::Generic,
            size: size * 8,
        });
    };

    let application = pe.application();
    let kind = if application == DW_EH_PE_absptr {
        RelocationKind::Absolute
    } else if application == DW_EH_PE_pcrel {
        RelocationKind::Relative
    } else {
        return Err(anyhow!(
            "unsupported eh_frame pointer application {application:?}"
        ));
    };
    let format_byte = pe.format();
    let bit_size = if format_byte == DW_EH_PE_absptr {
        size * 8
    } else if format_byte == DW_EH_PE_udata2 || format_byte == DW_EH_PE_sdata2 {
        16
    } else if format_byte == DW_EH_PE_udata4 || format_byte == DW_EH_PE_sdata4 {
        32
    } else if format_byte == DW_EH_PE_udata8 || format_byte == DW_EH_PE_sdata8 {
        64
    } else {
        return Err(anyhow!(
            "unsupported eh_frame pointer format {format_byte:?}"
        ));
    };

    // Mach-O encodes PC-relative `.eh_frame` references as a SUBTRACTOR /
    // UNSIGNED relocation pair, which is out of scope here. Surface a clear
    // error rather than emitting the wrong relocation.
    //
    // TODO: arm64 Mach-O `__TEXT,__eh_frame` is feasible via that reloc pair;
    // see rust-lang/rustc_codegen_cranelift#1634 for the approach.
    if matches!(format, BinaryFormat::MachO) && kind == RelocationKind::Relative {
        return Err(anyhow!("Mach-O .eh_frame emission is not yet supported"));
    }

    Ok(RelocationFlags::Generic {
        kind,
        encoding: RelocationEncoding::Generic,
        size: bit_size,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use gimli::constants::*;

    #[track_caller]
    fn assert_generic(flags: RelocationFlags, expected_kind: RelocationKind, expected_size: u8) {
        match flags {
            RelocationFlags::Generic {
                kind,
                encoding,
                size,
            } => {
                assert_eq!(kind, expected_kind, "wrong relocation kind");
                assert_eq!(
                    encoding,
                    RelocationEncoding::Generic,
                    "wrong relocation encoding"
                );
                assert_eq!(size, expected_size, "wrong relocation size in bits");
            }
            other => panic!("expected RelocationFlags::Generic, got {other:?}"),
        }
    }

    #[test]
    fn missing_eh_pe_falls_back_to_absolute_pointer() {
        let flags = translate_eh_pe(None, 8, BinaryFormat::Elf).unwrap();
        assert_generic(flags, RelocationKind::Absolute, 64);
    }

    #[test]
    fn absolute_eh_pointer_on_elf() {
        let flags = translate_eh_pe(Some(DW_EH_PE_absptr), 8, BinaryFormat::Elf).unwrap();
        assert_generic(flags, RelocationKind::Absolute, 64);
    }

    #[test]
    fn pcrel_sdata4_on_elf() {
        let pe = DwEhPe(DW_EH_PE_pcrel.0 | DW_EH_PE_sdata4.0);
        let flags = translate_eh_pe(Some(pe), 8, BinaryFormat::Elf).unwrap();
        assert_generic(flags, RelocationKind::Relative, 32);
    }

    #[test]
    fn pcrel_sdata4_on_coff() {
        let pe = DwEhPe(DW_EH_PE_pcrel.0 | DW_EH_PE_sdata4.0);
        let flags = translate_eh_pe(Some(pe), 8, BinaryFormat::Coff).unwrap();
        assert_generic(flags, RelocationKind::Relative, 32);
    }

    #[test]
    fn pcrel_on_macho_is_rejected() {
        let pe = DwEhPe(DW_EH_PE_pcrel.0 | DW_EH_PE_sdata4.0);
        let err = translate_eh_pe(Some(pe), 8, BinaryFormat::MachO).unwrap_err();
        assert!(
            err.to_string().contains("Mach-O"),
            "expected Mach-O scoping error, got: {err}"
        );
    }

    #[test]
    fn unknown_pointer_application_errors() {
        let pe = DwEhPe(0xee); // DW_EH_PE_indirect plus garbage; not handled.
        let err = translate_eh_pe(Some(pe), 8, BinaryFormat::Elf).unwrap_err();
        assert!(err.to_string().contains("application"));
    }
}
