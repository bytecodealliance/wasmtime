//! Debug utils for WebAssembly using Cranelift.

#![allow(clippy::cast_ptr_alignment)]

use anyhow::Error;
use more_asserts::assert_gt;
use object::write::{Object, Relocation, StandardSegment};
use object::{RelocationEncoding, RelocationKind, SectionKind};
use std::collections::HashMap;
use target_lexicon::BinaryFormat;
use wasmtime_environ::isa::TargetIsa;

pub use crate::read_debuginfo::{read_debuginfo, DebugInfoData, WasmFileInfo};
pub use crate::write_debuginfo::{emit_dwarf, DwarfSection};

mod gc;
mod read_debuginfo;
mod transform;
mod write_debuginfo;

pub fn write_debugsections(obj: &mut Object, sections: Vec<DwarfSection>) -> Result<(), Error> {
    let (bodies, relocs) = sections
        .into_iter()
        .map(|s| ((s.name.clone(), s.body), (s.name, s.relocs)))
        .unzip::<_, _, Vec<_>, Vec<_>>();
    let mut ids = HashMap::new();
    for (name, body) in bodies {
        let segment = obj.segment_name(StandardSegment::Debug).to_vec();
        let section_id = obj.add_section(segment, name.as_bytes().to_vec(), SectionKind::Debug);
        ids.insert(name, section_id);
        obj.append_section_data(section_id, &body, 1);
    }
    for (name, relocs) in relocs {
        let section_id = *ids.get(&name).unwrap();
        for reloc in relocs {
            let target_symbol = if reloc.target.starts_with("_wasm_function") {
                obj.symbol_id(reloc.target.as_bytes()).unwrap()
            } else {
                obj.section_symbol(*ids.get(&reloc.target).unwrap())
            };
            obj.add_relocation(
                section_id,
                Relocation {
                    offset: u64::from(reloc.offset),
                    size: reloc.size << 3,
                    kind: RelocationKind::Absolute,
                    encoding: RelocationEncoding::Generic,
                    symbol: target_symbol,
                    addend: i64::from(reloc.addend),
                },
            )?;
        }
    }

    Ok(())
}

fn patch_dwarf_sections(sections: &mut [DwarfSection], funcs: &[*const u8]) {
    for section in sections {
        const FUNC_SYMBOL_PREFIX: &str = "_wasm_function_";
        for reloc in section.relocs.iter() {
            if !reloc.target.starts_with(FUNC_SYMBOL_PREFIX) {
                // Fixing only "all" section relocs -- all functions are merged
                // into one blob.
                continue;
            }
            let func_index = reloc.target[FUNC_SYMBOL_PREFIX.len()..]
                .parse::<usize>()
                .expect("func index");
            let target = (funcs[func_index] as u64).wrapping_add(reloc.addend as i64 as u64);
            let entry_ptr = section.body
                [reloc.offset as usize..reloc.offset as usize + reloc.size as usize]
                .as_mut_ptr();
            unsafe {
                match reloc.size {
                    4 => std::ptr::write(entry_ptr as *mut u32, target as u32),
                    8 => std::ptr::write(entry_ptr as *mut u64, target),
                    _ => panic!("unexpected reloc entry size"),
                }
            }
        }
        section
            .relocs
            .retain(|r| !r.target.starts_with(FUNC_SYMBOL_PREFIX));
    }
}

pub fn write_debugsections_image(
    isa: &dyn TargetIsa,
    mut sections: Vec<DwarfSection>,
    code_region: (*const u8, usize),
    funcs: &[*const u8],
) -> Result<Vec<u8>, Error> {
    let mut obj = Object::new(BinaryFormat::Elf, isa.triple().architecture);

    assert!(!code_region.0.is_null() && code_region.1 > 0);
    assert_gt!(funcs.len(), 0);

    let body = unsafe { std::slice::from_raw_parts(code_region.0, code_region.1) };
    let section_id = obj.add_section(vec![], ".text.all".as_bytes().to_vec(), SectionKind::Text);
    obj.append_section_data(section_id, body, 1);

    // Get DWARF sections and patch relocs
    patch_dwarf_sections(&mut sections, funcs);

    write_debugsections(&mut obj, sections)?;

    // LLDB is too "magical" about mach-o, generating elf
    let mut bytes = obj.write()?;
    // elf is still missing details...
    convert_object_elf_to_loadable_file(&mut bytes, code_region.0);

    // let mut file = ::std::fs::File::create(::std::path::Path::new("test.o")).expect("file");
    // ::std::io::Write::write_all(&mut file, &bytes).expect("write");

    Ok(bytes)
}

fn convert_object_elf_to_loadable_file(bytes: &mut Vec<u8>, code_ptr: *const u8) {
    use object::elf::*;
    use object::endian::LittleEndian;
    use std::ffi::CStr;
    use std::mem::size_of;
    use std::os::raw::c_char;

    let e = LittleEndian;
    let header: &FileHeader64<LittleEndian> =
        unsafe { &*(bytes.as_mut_ptr() as *const FileHeader64<_>) };
    assert!(
        header.e_ident.class == ELFCLASS64 && header.e_ident.data == ELFDATA2LSB,
        "bits and endianess in .ELF",
    );
    assert!(
        header.e_phoff.get(e) == 0 && header.e_phnum.get(e) == 0,
        "program header table is empty"
    );
    let e_shentsize = header.e_shentsize.get(e);
    assert_eq!(
        e_shentsize as usize,
        size_of::<SectionHeader64<LittleEndian>>(),
        "size of sh"
    );

    let e_shoff = header.e_shoff.get(e);
    let e_shnum = header.e_shnum.get(e);
    let mut shstrtab_off = 0;
    for i in 0..e_shnum {
        let off = e_shoff as isize + i as isize * e_shentsize as isize;
        let section: &SectionHeader64<LittleEndian> =
            unsafe { &*(bytes.as_ptr().offset(off) as *const SectionHeader64<_>) };
        if section.sh_type.get(e) != SHT_STRTAB {
            continue;
        }
        shstrtab_off = section.sh_offset.get(e);
    }
    let mut segment = None;
    for i in 0..e_shnum {
        let off = e_shoff as isize + i as isize * e_shentsize as isize;
        let section: &mut SectionHeader64<LittleEndian> =
            unsafe { &mut *(bytes.as_mut_ptr().offset(off) as *mut SectionHeader64<_>) };
        if section.sh_type.get(e) != SHT_PROGBITS {
            continue;
        }
        // It is a SHT_PROGBITS, but we need to check sh_name to ensure it is our function
        let sh_name_off = section.sh_name.get(e);
        let sh_name = unsafe {
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
        // Functions was added at write_debugsections_image as .text.all.
        // Patch vaddr, and save file location and its size.
        section.sh_addr.set(e, code_ptr as u64);
        let sh_offset = section.sh_offset.get(e);
        let sh_size = section.sh_size.get(e);
        segment = Some((sh_offset, code_ptr, sh_size));
        // Fix name too: cut it to just ".text"
        bytes[(shstrtab_off + sh_name_off as u64) as usize + ".text".len()] = 0;
    }

    // LLDB wants segment with virtual address set, placing them at the end of ELF.
    let ph_off = bytes.len();
    let e_phentsize = size_of::<ProgramHeader64<LittleEndian>>();
    if let Some((sh_offset, v_offset, sh_size)) = segment {
        bytes.resize(ph_off + e_phentsize, 0);
        let program: &mut ProgramHeader64<LittleEndian> =
            unsafe { &mut *(bytes.as_ptr().add(ph_off) as *mut ProgramHeader64<_>) };
        program.p_type.set(e, PT_LOAD);
        program.p_offset.set(e, sh_offset);
        program.p_vaddr.set(e, v_offset as u64);
        program.p_paddr.set(e, v_offset as u64);
        program.p_filesz.set(e, sh_size as u64);
        program.p_memsz.set(e, sh_size as u64);
    } else {
        unreachable!();
    }

    // It is somewhat loadable ELF file at this moment.
    let header: &mut FileHeader64<LittleEndian> =
        unsafe { &mut *(bytes.as_mut_ptr() as *mut FileHeader64<_>) };
    header.e_type.set(e, ET_DYN);
    header.e_phoff.set(e, ph_off as u64);
    header.e_phentsize.set(e, e_phentsize as u16);
    header.e_phnum.set(e, 1u16);
}
