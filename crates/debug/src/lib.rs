//! Debug utils for WebAssembly using Cranelift.

#![allow(clippy::cast_ptr_alignment)]

use anyhow::{bail, ensure, Error};
use object::{RelocationEncoding, RelocationKind};
use std::collections::HashMap;

pub use crate::write_debuginfo::{emit_dwarf, DwarfSection, DwarfSectionRelocTarget};

mod gc;
mod transform;
mod write_debuginfo;

pub fn create_gdbjit_image(
    mut bytes: Vec<u8>,
    code_region: (*const u8, usize),
    defined_funcs_offset: usize,
    funcs: &[*const u8],
) -> Result<Vec<u8>, Error> {
    ensure_supported_elf_format(&mut bytes)?;

    // patch relocs
    relocate_dwarf_sections(&mut bytes, defined_funcs_offset, funcs)?;

    // elf is still missing details...
    convert_object_elf_to_loadable_file(&mut bytes, code_region);

    // let mut file = ::std::fs::File::create(::std::path::Path::new("test.o")).expect("file");
    // ::std::io::Write::write_all(&mut file, &bytes).expect("write");

    Ok(bytes)
}

fn relocate_dwarf_sections(
    bytes: &mut [u8],
    defined_funcs_offset: usize,
    funcs: &[*const u8],
) -> Result<(), Error> {
    use object::read::{File, Object, ObjectSection, RelocationTarget};

    let obj = File::parse(bytes)?;
    let mut func_symbols = HashMap::new();
    for (id, sym) in obj.symbols() {
        match (sym.name(), sym.section_index()) {
            (Some(name), Some(_section_index)) if name.starts_with("_wasm_function_") => {
                let index = name["_wasm_function_".len()..].parse::<usize>()?;
                let data = funcs[index - defined_funcs_offset];
                func_symbols.insert(id, data);
            }
            _ => (),
        }
    }

    for section in obj.sections() {
        for (off, r) in section.relocations() {
            if r.kind() != RelocationKind::Absolute
                || r.encoding() != RelocationEncoding::Generic
                || r.size() != 64
            {
                continue;
            }

            let data = match r.target() {
                RelocationTarget::Symbol(ref index) => func_symbols.get(index),
                _ => None,
            };
            let data: *const u8 = match data {
                Some(data) => *data,
                None => {
                    continue;
                }
            };

            let target = (data as u64).wrapping_add(r.addend() as u64);

            let entry_ptr = section.data_range(off, 8).unwrap().unwrap().as_ptr();
            unsafe {
                std::ptr::write(entry_ptr as *mut u64, target);
            }
        }
    }
    Ok(())
}

fn ensure_supported_elf_format(bytes: &mut Vec<u8>) -> Result<(), Error> {
    use object::elf::*;
    use object::endian::LittleEndian;
    use std::mem::size_of;

    let e = LittleEndian;
    let header: &FileHeader64<LittleEndian> =
        unsafe { &*(bytes.as_mut_ptr() as *const FileHeader64<_>) };
    ensure!(
        header.e_ident.class == ELFCLASS64 && header.e_ident.data == ELFDATA2LSB,
        "bits and endianess in .ELF",
    );
    match header.e_machine.get(e) {
        EM_X86_64 => (),
        machine => {
            bail!("Unsupported ELF target machine: {:x}", machine);
        }
    }
    ensure!(
        header.e_phoff.get(e) == 0 && header.e_phnum.get(e) == 0,
        "program header table is empty"
    );
    let e_shentsize = header.e_shentsize.get(e);
    ensure!(
        e_shentsize as usize == size_of::<SectionHeader64<LittleEndian>>(),
        "size of sh"
    );
    Ok(())
}

fn convert_object_elf_to_loadable_file(bytes: &mut Vec<u8>, code_region: (*const u8, usize)) {
    use object::elf::*;
    use object::endian::LittleEndian;
    use std::ffi::CStr;
    use std::mem::size_of;
    use std::os::raw::c_char;

    let e = LittleEndian;
    let header: &FileHeader64<LittleEndian> =
        unsafe { &*(bytes.as_mut_ptr() as *const FileHeader64<_>) };

    let e_shentsize = header.e_shentsize.get(e);
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
    let mut segment: Option<_> = None;
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
        if sh_name != ".text" {
            continue;
        }

        assert!(segment.is_none());
        // Patch vaddr, and save file location and its size.
        section.sh_addr.set(e, code_region.0 as u64);
        let sh_offset = section.sh_offset.get(e);
        let sh_size = section.sh_size.get(e);
        segment = Some((sh_offset, sh_size));
    }

    // LLDB wants segment with virtual address set, placing them at the end of ELF.
    let ph_off = bytes.len();
    let e_phentsize = size_of::<ProgramHeader64<LittleEndian>>();
    let e_phnum = 1;
    bytes.resize(ph_off + e_phentsize * e_phnum, 0);
    if let Some((sh_offset, sh_size)) = segment {
        let (v_offset, size) = code_region;
        let program: &mut ProgramHeader64<LittleEndian> =
            unsafe { &mut *(bytes.as_ptr().add(ph_off) as *mut ProgramHeader64<_>) };
        program.p_type.set(e, PT_LOAD);
        program.p_offset.set(e, sh_offset);
        program.p_vaddr.set(e, v_offset as u64);
        program.p_paddr.set(e, v_offset as u64);
        program.p_filesz.set(e, sh_size as u64);
        program.p_memsz.set(e, size as u64);
    } else {
        unreachable!();
    }

    // It is somewhat loadable ELF file at this moment.
    let header: &mut FileHeader64<LittleEndian> =
        unsafe { &mut *(bytes.as_mut_ptr() as *mut FileHeader64<_>) };
    header.e_type.set(e, ET_DYN);
    header.e_phoff.set(e, ph_off as u64);
    header.e_phentsize.set(e, e_phentsize as u16);
    header.e_phnum.set(e, e_phnum as u16);
}
