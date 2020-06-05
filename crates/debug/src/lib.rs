//! Debug utils for WebAssembly using Cranelift.

#![allow(clippy::cast_ptr_alignment)]

use anyhow::Error;
use faerie::{Artifact, Decl, SectionKind};
use more_asserts::assert_gt;
use target_lexicon::BinaryFormat;
use wasmtime_environ::isa::TargetIsa;

pub use crate::read_debuginfo::{read_debuginfo, DebugInfoData, WasmFileInfo};
pub use crate::write_debuginfo::{emit_dwarf, DwarfSection};

mod gc;
mod read_debuginfo;
mod transform;
mod write_debuginfo;

pub fn write_debugsections(obj: &mut Artifact, sections: Vec<DwarfSection>) -> Result<(), Error> {
    let (bodies, relocs) = sections
        .into_iter()
        .map(|s| ((s.name.clone(), s.body), (s.name, s.relocs)))
        .unzip::<_, _, Vec<_>, Vec<_>>();
    for (name, body) in bodies {
        obj.declare_with(name, Decl::section(SectionKind::Debug), body)?;
    }
    for (name, relocs) in relocs {
        for reloc in relocs {
            obj.link_with(
                faerie::Link {
                    from: &name,
                    to: &reloc.target,
                    at: reloc.offset as u64,
                },
                faerie::Reloc::Debug {
                    size: reloc.size,
                    addend: reloc.addend,
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
    let mut obj = Artifact::new(isa.triple().clone(), String::from("module"));

    assert!(!code_region.0.is_null() && code_region.1 > 0);
    assert_gt!(funcs.len(), 0);

    let body = unsafe { std::slice::from_raw_parts(code_region.0, code_region.1) };
    obj.declare_with("all", Decl::function(), body.to_vec())?;

    // Get DWARF sections and patch relocs
    patch_dwarf_sections(&mut sections, funcs);

    write_debugsections(&mut obj, sections)?;

    // LLDB is too "magical" about mach-o, generating elf
    let mut bytes = obj.emit_as(BinaryFormat::Elf)?;
    // elf is still missing details...
    convert_faerie_elf_to_loadable_file(&mut bytes, code_region.0);

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
        // Functions was added at write_debugsections_image as .text.all.
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
