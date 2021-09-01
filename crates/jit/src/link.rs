//! Linking for JIT-compiled code.

use object::read::{Object, Relocation, RelocationTarget};
use object::{File, NativeEndian as NE, ObjectSymbol, RelocationEncoding, RelocationKind};
use std::convert::TryFrom;
use wasmtime_runtime::libcalls;

type I32 = object::I32Bytes<NE>;
type U64 = object::U64Bytes<NE>;

/// Applies the relocation `r` at `offset` within `code`, according to the
/// symbols found in `obj`.
///
/// This method is used at runtime to resolve relocations in ELF images,
/// typically with respect to where the memory was placed in the final address
/// in memory.
pub fn apply_reloc(obj: &File, code: &mut [u8], offset: u64, r: Relocation) {
    let target_func_address: usize = match r.target() {
        RelocationTarget::Symbol(i) => {
            // Processing relocation target is a named symbols that is compiled
            // wasm function or runtime libcall.
            let sym = obj.symbol_by_index(i).unwrap();
            if sym.is_local() {
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
            }
        }
        _ => panic!("unexpected relocation target: not a symbol"),
    };

    match (r.kind(), r.encoding(), r.size()) {
        (RelocationKind::Absolute, RelocationEncoding::Generic, 64) => {
            let reloc_address = reloc_address::<U64>(code, offset);
            let reloc_abs = (target_func_address as u64)
                .checked_add(r.addend() as u64)
                .unwrap();
            reloc_address.set(NE, reloc_abs);
        }

        // FIXME(#3009) after the old backend is removed this won't ever show up
        // again so it can be removed.
        (RelocationKind::Relative, RelocationEncoding::Generic, 32) => {
            let reloc_address = reloc_address::<I32>(code, offset);
            let val = (target_func_address as i64)
                .wrapping_add(r.addend())
                .wrapping_sub(reloc_address as *const _ as i64);
            reloc_address.set(NE, i32::try_from(val).expect("relocation out-of-bounds"));
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
