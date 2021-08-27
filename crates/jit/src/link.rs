//! Linking for JIT-compiled code.

use object::read::{Object, Relocation, RelocationTarget};
use object::{elf, File, NativeEndian as NE, ObjectSymbol, RelocationEncoding, RelocationKind};
use std::convert::TryFrom;
use wasmtime_runtime::libcalls;

type U32 = object::U32Bytes<NE>;
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
        _ => panic!("unexpected relocation target"),
    };

    match (r.kind(), r.encoding(), r.size()) {
        #[cfg(target_pointer_width = "64")]
        (RelocationKind::Absolute, RelocationEncoding::Generic, 64) => {
            let reloc_address = reloc_address::<U64>(code, offset);
            let reloc_abs = (target_func_address as u64)
                .checked_add(r.addend() as u64)
                .unwrap();
            reloc_address.set(NE, reloc_abs);
        }
        #[cfg(target_pointer_width = "32")]
        (RelocationKind::Relative, RelocationEncoding::Generic, 32) => {
            let reloc_address = reloc_address::<U32>(code, offset);
            let reloc_delta_u32 = (target_func_address as u32)
                .wrapping_sub(reloc_address as *const _ as u32)
                .checked_add(r.addend() as u32)
                .unwrap();
            reloc_address.set(NE, reloc_delta_u32);
        }
        #[cfg(target_pointer_width = "32")]
        (RelocationKind::Relative, RelocationEncoding::X86Branch, 32) => {
            let reloc_address = reloc_address::<U32>(code, offset);
            let reloc_delta_u32 = (target_func_address as u32)
                .wrapping_sub(reloc_address as *const _ as u32)
                .wrapping_add(r.addend() as u32);
            reloc_address.set(NE, reloc_delta_u32);
        }
        #[cfg(target_pointer_width = "64")]
        (RelocationKind::Relative, RelocationEncoding::Generic, 32) => {
            let reloc_address = reloc_address::<I32>(code, offset);
            let reloc_delta_i64 = (target_func_address as i64)
                .wrapping_sub(reloc_address as *const _ as i64)
                .wrapping_add(r.addend());
            // TODO implement far calls mode in x64 new backend.
            reloc_address.set(
                NE,
                i32::try_from(reloc_delta_i64).expect("relocation too large to fit in i32"),
            );
        }
        #[cfg(target_pointer_width = "64")]
        (RelocationKind::Relative, RelocationEncoding::S390xDbl, 32) => {
            let reloc_address = reloc_address::<I32>(code, offset);
            let reloc_delta_i64 = (target_func_address as i64)
                .wrapping_sub(reloc_address as *const _ as i64)
                .wrapping_add(r.addend())
                >> 1;
            reloc_address.set(
                NE,
                i32::try_from(reloc_delta_i64).expect("relocation too large to fit in i32"),
            );
        }
        (RelocationKind::Elf(elf::R_AARCH64_CALL26), RelocationEncoding::Generic, 32) => {
            let reloc_address = reloc_address::<U32>(code, offset);
            let reloc_delta = (target_func_address as u64).wrapping_sub(r.addend() as u64);
            // TODO: come up with a PLT-like solution for longer calls. We can't extend the
            // code segment at this point, but we could conservatively allocate space at the
            // end of the function during codegen, a fixed amount per call, to allow for
            // potential branch islands.
            assert!((reloc_delta as i64) < (1 << 27));
            assert!((reloc_delta as i64) >= -(1 << 27));
            let reloc_delta = reloc_delta as u32;
            let reloc_delta = reloc_delta.wrapping_add(r.addend() as u32);
            let delta_bits = reloc_delta >> 2;
            let insn = reloc_address.get(NE);
            let new_insn = (insn & 0xfc00_0000) | (delta_bits & 0x03ff_ffff);
            reloc_address.set(NE, new_insn);
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
