//! Linking for JIT-compiled code.

use object::read::{Object, ObjectSection, Relocation, RelocationTarget};
use object::{File, NativeEndian as NE, ObjectSymbol, RelocationEncoding, RelocationKind};
use std::convert::TryFrom;
use wasmtime_runtime::libcalls;

type U64 = object::U64Bytes<NE>;

/// Links a module that has been compiled with `compiled_module` in `wasmtime-environ`.
///
/// Performs all required relocations inside the function code, provided the necessary metadata.
/// The relocations data provided in the object file, see object.rs for details.
///
/// Currently, the produced ELF image can be trusted.
/// TODO refactor logic to remove panics and add defensive code the image data
/// becomes untrusted.
pub fn link_module(obj: &File, code_range: &mut [u8]) {
    // Read the ".text" section and process its relocations.
    let text_section = obj.section_by_name(".text").unwrap();

    for (offset, r) in text_section.relocations() {
        apply_reloc(obj, code_range, offset, r);
    }
}

fn apply_reloc(obj: &File, code: &mut [u8], offset: u64, r: Relocation) {
    let sym = match r.target() {
        RelocationTarget::Symbol(i) => obj.symbol_by_index(i).unwrap(),
        _ => panic!("unexpected relocation target"),
    };
    let target_func_address: usize = if sym.is_local() {
        &code[sym.address() as usize] as *const u8 as usize
    } else {
        match sym.name() {
            Ok(name) => {
                if let Some(addr) = to_libcall_address(name) {
                    addr
                } else {
                    panic!("unknown function to link: {}", name);
                }
            }
            Err(_) => panic!("unexpected relocation target: not a symbol"),
        }
    };

    match (r.kind(), r.encoding(), r.size()) {
        (RelocationKind::Absolute, RelocationEncoding::Generic, 64) => {
            let reloc_address = reloc_address::<U64>(code, offset);
            let reloc_abs = (target_func_address as u64)
                .checked_add(r.addend() as u64)
                .unwrap();
            reloc_address.set(NE, reloc_abs);
        }
        other => panic!("unsupported reloc kind: {:?}", other),
    }
}

fn reloc_address<T: object::Pod>(code: &mut [u8], offset: u64) -> &mut T {
    let (reloc, _rest) = usize::try_from(offset)
        .ok()
        .and_then(move |offset| code.get_mut(offset..))
        .and_then(|range| object::from_bytes_mut(range).ok())
        .expect("invalid reloc offset");
    reloc
}

fn to_libcall_address(name: &str) -> Option<usize> {
    use self::libcalls::*;
    use wasmtime_environ::for_each_libcall;
    macro_rules! add_libcall_symbol {
        [$(($libcall:ident, $export:ident)),*] => {
            Some(match name {
                $(
                    stringify!($export) => $export as usize,
                )+
                _ => {
                    return None;
                }
            })
        };
    }
    for_each_libcall!(add_libcall_symbol)
}
