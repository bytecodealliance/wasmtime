pub use crate::read_debuginfo::{read_debuginfo, DebugInfoData, WasmFileInfo};
pub use crate::transform::transform_dwarf;
use gimli::write::{Address, Dwarf, EndianVec, FrameTable, Result, Sections, Writer};
use gimli::{RunTimeEndian, SectionId};
use wasmtime_environ::isa::{unwind::UnwindInfo, TargetIsa};
use wasmtime_environ::{Compilation, ModuleAddressMap, ModuleVmctxInfo, ValueLabelsRanges};

#[derive(Clone)]
pub enum DwarfSectionRelocTarget {
    Func(usize),
    Section(&'static str),
}

#[derive(Clone)]
pub struct DwarfSectionReloc {
    pub target: DwarfSectionRelocTarget,
    pub offset: u32,
    pub addend: i32,
    pub size: u8,
}

pub struct DwarfSection {
    pub name: &'static str,
    pub body: Vec<u8>,
    pub relocs: Vec<DwarfSectionReloc>,
}

fn emit_dwarf_sections(
    mut dwarf: Dwarf,
    frames: Option<FrameTable>,
) -> anyhow::Result<Vec<DwarfSection>> {
    let mut sections = Sections::new(WriterRelocate::default());
    dwarf.write(&mut sections)?;
    if let Some(frames) = frames {
        frames.write_debug_frame(&mut sections.debug_frame)?;
    }

    let mut result = Vec::new();
    sections.for_each_mut(|id, s| -> anyhow::Result<()> {
        let name = id.name();
        let body = s.writer.take();
        let mut relocs = vec![];
        ::std::mem::swap(&mut relocs, &mut s.relocs);
        result.push(DwarfSection { name, body, relocs });
        Ok(())
    })?;

    Ok(result)
}

#[derive(Clone)]
pub struct WriterRelocate {
    relocs: Vec<DwarfSectionReloc>,
    writer: EndianVec<RunTimeEndian>,
}

impl Default for WriterRelocate {
    fn default() -> Self {
        WriterRelocate {
            relocs: Vec::new(),
            writer: EndianVec::new(RunTimeEndian::Little),
        }
    }
}

impl Writer for WriterRelocate {
    type Endian = RunTimeEndian;

    fn endian(&self) -> Self::Endian {
        self.writer.endian()
    }

    fn len(&self) -> usize {
        self.writer.len()
    }

    fn write(&mut self, bytes: &[u8]) -> Result<()> {
        self.writer.write(bytes)
    }

    fn write_at(&mut self, offset: usize, bytes: &[u8]) -> Result<()> {
        self.writer.write_at(offset, bytes)
    }

    fn write_address(&mut self, address: Address, size: u8) -> Result<()> {
        match address {
            Address::Constant(val) => self.write_udata(val, size),
            Address::Symbol { symbol, addend } => {
                let offset = self.len() as u32;
                self.relocs.push(DwarfSectionReloc {
                    target: DwarfSectionRelocTarget::Func(symbol),
                    offset,
                    size,
                    addend: addend as i32,
                });
                self.write_udata(addend as u64, size)
            }
        }
    }

    fn write_offset(&mut self, val: usize, section: SectionId, size: u8) -> Result<()> {
        let offset = self.len() as u32;
        let target = DwarfSectionRelocTarget::Section(section.name());
        self.relocs.push(DwarfSectionReloc {
            target,
            offset,
            size,
            addend: val as i32,
        });
        self.write_udata(val as u64, size)
    }

    fn write_offset_at(
        &mut self,
        offset: usize,
        val: usize,
        section: SectionId,
        size: u8,
    ) -> Result<()> {
        let target = DwarfSectionRelocTarget::Section(section.name());
        self.relocs.push(DwarfSectionReloc {
            target,
            offset: offset as u32,
            size,
            addend: val as i32,
        });
        self.write_udata_at(offset, val as u64, size)
    }
}

fn create_frame_table<'a>(
    isa: &dyn TargetIsa,
    infos: impl Iterator<Item = &'a Option<UnwindInfo>>,
) -> Option<FrameTable> {
    let mut table = FrameTable::default();

    let cie_id = table.add_cie(isa.create_systemv_cie()?);

    for (i, info) in infos.enumerate() {
        if let Some(UnwindInfo::SystemV(info)) = info {
            table.add_fde(
                cie_id,
                info.to_fde(Address::Symbol {
                    symbol: i,
                    addend: 0,
                }),
            );
        }
    }

    Some(table)
}

pub fn emit_dwarf(
    isa: &dyn TargetIsa,
    debuginfo_data: &DebugInfoData,
    at: &ModuleAddressMap,
    vmctx_info: &ModuleVmctxInfo,
    ranges: &ValueLabelsRanges,
    compilation: &Compilation,
) -> anyhow::Result<Vec<DwarfSection>> {
    let dwarf = transform_dwarf(isa, debuginfo_data, at, vmctx_info, ranges)?;
    let frame_table = create_frame_table(isa, compilation.into_iter().map(|f| &f.unwind_info));
    let sections = emit_dwarf_sections(dwarf, frame_table)?;
    Ok(sections)
}
