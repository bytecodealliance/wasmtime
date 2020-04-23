//! Debug utils for WebAssembly using Cranelift.

#![allow(clippy::cast_ptr_alignment)]

use anyhow::Error;
use faerie::{Artifact, Decl};
use gimli::write::{Address, FrameTable};
use more_asserts::assert_gt;
use target_lexicon::BinaryFormat;
use wasmtime_environ::isa::{unwind::UnwindInfo, TargetIsa};
use wasmtime_environ::{Compilation, ModuleAddressMap, ModuleVmctxInfo, ValueLabelsRanges};

pub use crate::read_debuginfo::{read_debuginfo, DebugInfoData, WasmFileInfo};
pub use crate::transform::transform_dwarf;
pub use crate::write_debuginfo::{emit_dwarf, ResolvedSymbol, SymbolResolver};

mod gc;
mod read_debuginfo;
mod transform;
mod write_debuginfo;

struct FunctionRelocResolver {}
impl SymbolResolver for FunctionRelocResolver {
    fn resolve_symbol(&self, symbol: usize, addend: i64) -> ResolvedSymbol {
        let name = format!("_wasm_function_{}", symbol);
        ResolvedSymbol::Reloc { name, addend }
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

pub fn emit_debugsections(
    obj: &mut Artifact,
    vmctx_info: &ModuleVmctxInfo,
    isa: &dyn TargetIsa,
    debuginfo_data: &DebugInfoData,
    at: &ModuleAddressMap,
    ranges: &ValueLabelsRanges,
    compilation: &Compilation,
) -> Result<(), Error> {
    let resolver = FunctionRelocResolver {};
    let dwarf = transform_dwarf(isa, debuginfo_data, at, vmctx_info, ranges)?;
    let frame_table = create_frame_table(isa, compilation.into_iter().map(|f| &f.unwind_info));

    emit_dwarf(obj, dwarf, &resolver, frame_table)?;
    Ok(())
}

struct ImageRelocResolver<'a> {
    func_offsets: &'a Vec<u64>,
}

impl<'a> SymbolResolver for ImageRelocResolver<'a> {
    fn resolve_symbol(&self, symbol: usize, addend: i64) -> ResolvedSymbol {
        let func_start = self.func_offsets[symbol];
        ResolvedSymbol::PhysicalAddress(func_start + addend as u64)
    }
}

pub fn emit_debugsections_image(
    isa: &dyn TargetIsa,
    debuginfo_data: &DebugInfoData,
    vmctx_info: &ModuleVmctxInfo,
    at: &ModuleAddressMap,
    ranges: &ValueLabelsRanges,
    funcs: &[(*const u8, usize)],
    compilation: &Compilation,
) -> Result<Vec<u8>, Error> {
    let func_offsets = &funcs
        .iter()
        .map(|(ptr, _)| *ptr as u64)
        .collect::<Vec<u64>>();
    let mut obj = Artifact::new(isa.triple().clone(), String::from("module"));
    let resolver = ImageRelocResolver { func_offsets };
    let dwarf = transform_dwarf(isa, debuginfo_data, at, vmctx_info, ranges)?;

    // Assuming all functions in the same code block, looking min/max of its range.
    assert_gt!(funcs.len(), 0);
    let mut segment_body: (usize, usize) = (!0, 0);
    for (body_ptr, body_len) in funcs {
        segment_body.0 = std::cmp::min(segment_body.0, *body_ptr as usize);
        segment_body.1 = std::cmp::max(segment_body.1, *body_ptr as usize + body_len);
    }
    let segment_body = (segment_body.0 as *const u8, segment_body.1 - segment_body.0);

    let body = unsafe { std::slice::from_raw_parts(segment_body.0, segment_body.1) };
    obj.declare_with("all", Decl::function(), body.to_vec())?;

    let frame_table = create_frame_table(isa, compilation.into_iter().map(|f| &f.unwind_info));
    emit_dwarf(&mut obj, dwarf, &resolver, frame_table)?;

    // LLDB is too "magical" about mach-o, generating elf
    let mut bytes = obj.emit_as(BinaryFormat::Elf)?;
    // elf is still missing details...
    convert_faerie_elf_to_loadable_file(&mut bytes, segment_body.0);

    // let mut file = ::std::fs::File::create(::std::path::Path::new("test.o")).expect("file");
    // ::std::io::Write::write(&mut file, &bytes).expect("write");

    Ok(bytes)
}

fn convert_faerie_elf_to_loadable_file(bytes: &mut Vec<u8>, code_ptr: *const u8) {
    use std::ffi::CStr;
    use std::os::raw::c_char;

    assert!(
        bytes[0x4] == 2 && bytes[0x5] == 1,
        "bits and endianess in .ELF"
    );
    let e_phoff = unsafe { *(bytes.as_ptr().offset(0x20) as *const u64) };
    let e_phnum = unsafe { *(bytes.as_ptr().offset(0x38) as *const u16) };
    assert!(
        e_phoff == 0 && e_phnum == 0,
        "program header table is empty"
    );
    let e_phentsize = unsafe { *(bytes.as_ptr().offset(0x36) as *const u16) };
    assert_eq!(e_phentsize, 0x38, "size of ph");
    let e_shentsize = unsafe { *(bytes.as_ptr().offset(0x3A) as *const u16) };
    assert_eq!(e_shentsize, 0x40, "size of sh");

    let e_shoff = unsafe { *(bytes.as_ptr().offset(0x28) as *const u64) };
    let e_shnum = unsafe { *(bytes.as_ptr().offset(0x3C) as *const u16) };
    let mut shstrtab_off = 0;
    let mut segment = None;
    for i in 0..e_shnum {
        let off = e_shoff as isize + i as isize * e_shentsize as isize;
        let sh_type = unsafe { *(bytes.as_ptr().offset(off + 0x4) as *const u32) };
        if sh_type == /* SHT_SYMTAB */ 3 {
            shstrtab_off = unsafe { *(bytes.as_ptr().offset(off + 0x18) as *const u64) };
        }
        if sh_type != /* SHT_PROGBITS */ 1 {
            continue;
        }
        // It is a SHT_PROGBITS, but we need to check sh_name to ensure it is our function
        let sh_name = unsafe {
            let sh_name_off = *(bytes.as_ptr().offset(off) as *const u32);
            CStr::from_ptr(
                bytes
                    .as_ptr()
                    .offset((shstrtab_off + sh_name_off as u64) as isize)
                    as *const c_char,
            )
            .to_str()
            .expect("name")
        };
        if sh_name != ".text.all" {
            continue;
        }

        assert!(segment.is_none());
        // Functions was added at emit_debugsections_image as .text.all.
        // Patch vaddr, and save file location and its size.
        unsafe {
            *(bytes.as_ptr().offset(off + 0x10) as *mut u64) = code_ptr as u64;
        };
        let sh_offset = unsafe { *(bytes.as_ptr().offset(off + 0x18) as *const u64) };
        let sh_size = unsafe { *(bytes.as_ptr().offset(off + 0x20) as *const u64) };
        segment = Some((sh_offset, code_ptr, sh_size));
        // Fix name too: cut it to just ".text"
        unsafe {
            let sh_name_off = *(bytes.as_ptr().offset(off) as *const u32);
            bytes[(shstrtab_off + sh_name_off as u64) as usize + ".text".len()] = 0;
        }
    }

    // LLDB wants segment with virtual address set, placing them at the end of ELF.
    let ph_off = bytes.len();
    if let Some((sh_offset, v_offset, sh_size)) = segment {
        let segment = vec![0; 0x38];
        unsafe {
            *(segment.as_ptr() as *mut u32) = /* PT_LOAD */ 0x1;
            *(segment.as_ptr().offset(0x8) as *mut u64) = sh_offset;
            *(segment.as_ptr().offset(0x10) as *mut u64) = v_offset as u64;
            *(segment.as_ptr().offset(0x18) as *mut u64) = v_offset as u64;
            *(segment.as_ptr().offset(0x20) as *mut u64) = sh_size;
            *(segment.as_ptr().offset(0x28) as *mut u64) = sh_size;
        }
        bytes.extend_from_slice(&segment);
    } else {
        unreachable!();
    }

    // It is somewhat loadable ELF file at this moment.
    // Update e_flags, e_phoff and e_phnum.
    unsafe {
        *(bytes.as_ptr().offset(0x10) as *mut u16) = /* ET_DYN */ 3;
        *(bytes.as_ptr().offset(0x20) as *mut u64) = ph_off as u64;
        *(bytes.as_ptr().offset(0x38) as *mut u16) = 1 as u16;
    }
}
