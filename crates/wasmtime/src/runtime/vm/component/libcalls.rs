//! Implementation of string transcoding required by the component model.

use crate::prelude::*;
use crate::runtime::vm::component::{ComponentInstance, VMComponentContext};
use core::cell::Cell;
use core::slice;
use wasmtime_environ::component::TypeResourceTableIndex;

const UTF16_TAG: usize = 1 << 31;

#[repr(C)] // this is read by Cranelift code so it's layout must be as-written
pub struct VMComponentLibcalls {
    builtins: VMComponentBuiltins,
    transcoders: VMBuiltinTranscodeArray,
}

impl VMComponentLibcalls {
    pub const INIT: VMComponentLibcalls = VMComponentLibcalls {
        builtins: VMComponentBuiltins::INIT,
        transcoders: VMBuiltinTranscodeArray::INIT,
    };
}

macro_rules! signature {
    (@ty size) => (usize);
    (@ty ptr_u8) => (*mut u8);
    (@ty ptr_u16) => (*mut u16);
    (@ty ptr_size) => (*mut usize);
    (@ty u8) => (u8);
    (@ty u32) => (u32);
    (@ty u64) => (u64);
    (@ty vmctx) => (*mut VMComponentContext);
}

/// Defines a `VMComponentBuiltins` structure which contains any builtins such
/// as resource-related intrinsics.
macro_rules! define_builtins {
    (
        $(
            $( #[$attr:meta] )*
            $name:ident( $( $pname:ident: $param:ident ),* ) $( -> $result:ident )?;
        )*
    ) => {
        /// An array that stores addresses of builtin functions. We translate code
        /// to use indirect calls. This way, we don't have to patch the code.
        #[repr(C)]
        struct VMComponentBuiltins {
            $(
                $name: unsafe extern "C" fn(
                    $(signature!(@ty $param),)*
                ) $( -> signature!(@ty $result))?,
            )*
        }

        impl VMComponentBuiltins {
            const INIT: VMComponentBuiltins = VMComponentBuiltins {
                $($name: trampolines::$name,)*
            };
        }
    };
}

wasmtime_environ::foreach_builtin_component_function!(define_builtins);

/// Macro to define the `VMBuiltinTranscodeArray` type which contains all of the
/// function pointers to the actual transcoder functions. This structure is read
/// by Cranelift-generated code, hence the `repr(C)`.
///
/// Note that this references the `trampolines` module rather than the functions
/// below as the `trampolines` module has the raw ABI.
///
/// This is modeled after the similar macros and usages in `libcalls.rs` and
/// `vmcontext.rs`
macro_rules! define_transcoders {
    (
        $(
            $( #[$attr:meta] )*
            $name:ident( $( $pname:ident: $param:ident ),* ) $( -> $result:ident )?;
        )*
    ) => {
        /// An array that stores addresses of builtin functions. We translate code
        /// to use indirect calls. This way, we don't have to patch the code.
        #[repr(C)]
        struct VMBuiltinTranscodeArray {
            $(
                $name: unsafe extern "C" fn(
                    $(signature!(@ty $param),)*
                ) $( -> signature!(@ty $result))?,
            )*
        }

        impl VMBuiltinTranscodeArray {
            const INIT: VMBuiltinTranscodeArray = VMBuiltinTranscodeArray {
                $($name: trampolines::$name,)*
            };
        }
    };
}

wasmtime_environ::foreach_transcoder!(define_transcoders);

/// Submodule with macro-generated constants which are the actual libcall
/// transcoders that are invoked by Cranelift. These functions have a specific
/// ABI defined by the macro itself and will defer to the actual bodies of each
/// implementation following this submodule.
#[allow(improper_ctypes_definitions)]
mod trampolines {
    use super::VMComponentContext;

    macro_rules! shims {
        (
            $(
                $( #[$attr:meta] )*
                $name:ident( $( $pname:ident: $param:ident ),* ) $( -> $result:ident )?;
            )*
        ) => (
            $(
                pub unsafe extern "C" fn $name(
                    $($pname : signature!(@ty $param),)*
                ) $( -> signature!(@ty $result))? {
                    $(shims!(@validate_param $pname $param);)*

                    // Always catch panics to avoid trying to unwind from Rust
                    // into Cranelift-generated code which would lead to a Bad
                    // Time.
                    //
                    // Additionally assume that every function below returns a
                    // `Result` where errors turn into traps.
                    let result = crate::runtime::vm::traphandlers::catch_unwind_and_longjmp(|| {
                        shims!(@invoke $name() $($pname)*)
                    });
                    match result {
                        Ok(ret) => shims!(@convert_ret ret $($pname: $param)*),
                        Err(err) => crate::runtime::vm::traphandlers::raise_trap(
                            crate::runtime::vm::traphandlers::TrapReason::User(err)
                        ),
                    }
                }
            )*
        );

        // Helper macro to convert a 2-tuple automatically when the last
        // parameter is a `ptr_size` argument.
        (@convert_ret $ret:ident) => ($ret);
        (@convert_ret $ret:ident $retptr:ident: ptr_size) => ({
            let (a, b) = $ret;
            *$retptr = b;
            a
        });
        (@convert_ret $ret:ident $name:ident: $ty:ident $($rest:tt)*) => (
            shims!(@convert_ret $ret $($rest)*)
        );

        (@validate_param $arg:ident ptr_u16) => ({
            // This should already be guaranteed by the canonical ABI and our
            // adapter modules, but double-check here to be extra-sure. If this
            // is a perf concern it can become a `debug_assert!`.
            assert!(($arg as usize) % 2 == 0, "unaligned 16-bit pointer");
        });
        (@validate_param $arg:ident $ty:ident) => ();

        // Helper macro to invoke `$m` with all of the tokens passed except for
        // any argument named `ret2`
        (@invoke $m:ident ($($args:tt)*)) => (super::$m($($args)*));

        // ignore `ret2`-named arguments
        (@invoke $m:ident ($($args:tt)*) ret2 $($rest:tt)*) => (
            shims!(@invoke $m ($($args)*) $($rest)*)
        );

        // move all other arguments into the `$args` list
        (@invoke $m:ident ($($args:tt)*) $param:ident $($rest:tt)*) => (
            shims!(@invoke $m ($($args)* $param,) $($rest)*)
        );
    }

    wasmtime_environ::foreach_builtin_component_function!(shims);
    wasmtime_environ::foreach_transcoder!(shims);
}

/// This property should already be guaranteed by construction in the component
/// model but assert it here to be extra sure. Nothing below is sound if regions
/// can overlap.
fn assert_no_overlap<T, U>(a: &[T], b: &[U]) {
    let a_start = a.as_ptr() as usize;
    let a_end = a_start + (a.len() * core::mem::size_of::<T>());
    let b_start = b.as_ptr() as usize;
    let b_end = b_start + (b.len() * core::mem::size_of::<U>());

    if a_start < b_start {
        assert!(a_end < b_start);
    } else {
        assert!(b_end < a_start);
    }
}

/// Converts a utf8 string to a utf8 string.
///
/// The length provided is length of both the source and the destination
/// buffers. No value is returned other than whether an invalid string was
/// found.
unsafe fn utf8_to_utf8(src: *mut u8, len: usize, dst: *mut u8) -> Result<()> {
    let src = slice::from_raw_parts(src, len);
    let dst = slice::from_raw_parts_mut(dst, len);
    assert_no_overlap(src, dst);
    log::trace!("utf8-to-utf8 {len}");
    let src = core::str::from_utf8(src).map_err(|_| anyhow!("invalid utf8 encoding"))?;
    dst.copy_from_slice(src.as_bytes());
    Ok(())
}

/// Converts a utf16 string to a utf16 string.
///
/// The length provided is length of both the source and the destination
/// buffers. No value is returned other than whether an invalid string was
/// found.
unsafe fn utf16_to_utf16(src: *mut u16, len: usize, dst: *mut u16) -> Result<()> {
    let src = slice::from_raw_parts(src, len);
    let dst = slice::from_raw_parts_mut(dst, len);
    assert_no_overlap(src, dst);
    log::trace!("utf16-to-utf16 {len}");
    run_utf16_to_utf16(src, dst)?;
    Ok(())
}

/// Transcodes utf16 to itself, returning whether all code points were inside of
/// the latin1 space.
fn run_utf16_to_utf16(src: &[u16], mut dst: &mut [u16]) -> Result<bool> {
    let mut all_latin1 = true;
    for ch in core::char::decode_utf16(src.iter().map(|i| u16::from_le(*i))) {
        let ch = ch.map_err(|_| anyhow!("invalid utf16 encoding"))?;
        all_latin1 = all_latin1 && u8::try_from(u32::from(ch)).is_ok();
        let result = ch.encode_utf16(dst);
        let size = result.len();
        for item in result {
            *item = item.to_le();
        }
        dst = &mut dst[size..];
    }
    Ok(all_latin1)
}

/// Converts a latin1 string to a latin1 string.
///
/// Given that all byte sequences are valid latin1 strings this is simply a
/// memory copy.
unsafe fn latin1_to_latin1(src: *mut u8, len: usize, dst: *mut u8) -> Result<()> {
    let src = slice::from_raw_parts(src, len);
    let dst = slice::from_raw_parts_mut(dst, len);
    assert_no_overlap(src, dst);
    log::trace!("latin1-to-latin1 {len}");
    dst.copy_from_slice(src);
    Ok(())
}

/// Converts a latin1 string to a utf16 string.
///
/// This simply inflates the latin1 characters to the u16 code points. The
/// length provided is the same length of the source and destination buffers.
unsafe fn latin1_to_utf16(src: *mut u8, len: usize, dst: *mut u16) -> Result<()> {
    let src = slice::from_raw_parts(src, len);
    let dst = slice::from_raw_parts_mut(dst, len);
    assert_no_overlap(src, dst);
    for (src, dst) in src.iter().zip(dst) {
        *dst = u16::from(*src).to_le();
    }
    log::trace!("latin1-to-utf16 {len}");
    Ok(())
}

/// Converts utf8 to utf16.
///
/// The length provided is the same unit length of both buffers, and the
/// returned value from this function is how many u16 units were written.
unsafe fn utf8_to_utf16(src: *mut u8, len: usize, dst: *mut u16) -> Result<usize> {
    let src = slice::from_raw_parts(src, len);
    let dst = slice::from_raw_parts_mut(dst, len);
    assert_no_overlap(src, dst);

    let result = run_utf8_to_utf16(src, dst)?;
    log::trace!("utf8-to-utf16 {len} => {result}");
    Ok(result)
}

fn run_utf8_to_utf16(src: &[u8], dst: &mut [u16]) -> Result<usize> {
    let src = core::str::from_utf8(src).map_err(|_| anyhow!("invalid utf8 encoding"))?;
    let mut amt = 0;
    for (i, dst) in src.encode_utf16().zip(dst) {
        *dst = i.to_le();
        amt += 1;
    }
    Ok(amt)
}

/// Converts utf16 to utf8.
///
/// Each buffer is specified independently here and the returned value is a pair
/// of the number of code units read and code units written. This might perform
/// a partial transcode if the destination buffer is not large enough to hold
/// the entire contents.
unsafe fn utf16_to_utf8(
    src: *mut u16,
    src_len: usize,
    dst: *mut u8,
    dst_len: usize,
) -> Result<(usize, usize)> {
    let src = slice::from_raw_parts(src, src_len);
    let mut dst = slice::from_raw_parts_mut(dst, dst_len);
    assert_no_overlap(src, dst);

    // This iterator will convert to native endianness and additionally count
    // how many items have been read from the iterator so far. This
    // count is used to return how many of the source code units were read.
    let src_iter_read = Cell::new(0);
    let src_iter = src.iter().map(|i| {
        src_iter_read.set(src_iter_read.get() + 1);
        u16::from_le(*i)
    });

    let mut src_read = 0;
    let mut dst_written = 0;

    for ch in core::char::decode_utf16(src_iter) {
        let ch = ch.map_err(|_| anyhow!("invalid utf16 encoding"))?;

        // If the destination doesn't have enough space for this character
        // then the loop is ended and this function will be called later with a
        // larger destination buffer.
        if dst.len() < 4 && dst.len() < ch.len_utf8() {
            break;
        }

        // Record that characters were read and then convert the `char` to
        // utf-8, advancing the destination buffer.
        src_read = src_iter_read.get();
        let len = ch.encode_utf8(dst).len();
        dst_written += len;
        dst = &mut dst[len..];
    }

    log::trace!("utf16-to-utf8 {src_len}/{dst_len} => {src_read}/{dst_written}");
    Ok((src_read, dst_written))
}

/// Converts latin1 to utf8.
///
/// Receives the independent size of both buffers and returns the number of code
/// units read and code units written (both bytes in this case).
///
/// This may perform a partial encoding if the destination is not large enough.
unsafe fn latin1_to_utf8(
    src: *mut u8,
    src_len: usize,
    dst: *mut u8,
    dst_len: usize,
) -> Result<(usize, usize)> {
    let src = slice::from_raw_parts(src, src_len);
    let dst = slice::from_raw_parts_mut(dst, dst_len);
    assert_no_overlap(src, dst);
    let (read, written) = encoding_rs::mem::convert_latin1_to_utf8_partial(src, dst);
    log::trace!("latin1-to-utf8 {src_len}/{dst_len} => ({read}, {written})");
    Ok((read, written))
}

/// Converts utf16 to "latin1+utf16", probably using a utf16 encoding.
///
/// The length specified is the length of both the source and destination
/// buffers. If the source string has any characters that don't fit in the
/// latin1 code space (0xff and below) then a utf16-tagged length will be
/// returned. Otherwise the string is "deflated" from a utf16 string to a latin1
/// string and the latin1 length is returned.
unsafe fn utf16_to_compact_probably_utf16(
    src: *mut u16,
    len: usize,
    dst: *mut u16,
) -> Result<usize> {
    let src = slice::from_raw_parts(src, len);
    let dst = slice::from_raw_parts_mut(dst, len);
    assert_no_overlap(src, dst);
    let all_latin1 = run_utf16_to_utf16(src, dst)?;
    if all_latin1 {
        let (left, dst, right) = dst.align_to_mut::<u8>();
        assert!(left.is_empty());
        assert!(right.is_empty());
        for i in 0..len {
            dst[i] = dst[2 * i];
        }
        log::trace!("utf16-to-compact-probably-utf16 {len} => latin1 {len}");
        Ok(len)
    } else {
        log::trace!("utf16-to-compact-probably-utf16 {len} => utf16 {len}");
        Ok(len | UTF16_TAG)
    }
}

/// Converts a utf8 string to latin1.
///
/// The length specified is the same length of both the input and the output
/// buffers.
///
/// Returns the number of code units read from the source and the number of code
/// units written to the destination.
///
/// Note that this may not convert the entire source into the destination if the
/// original utf8 string has usvs not representable in latin1.
unsafe fn utf8_to_latin1(src: *mut u8, len: usize, dst: *mut u8) -> Result<(usize, usize)> {
    let src = slice::from_raw_parts(src, len);
    let dst = slice::from_raw_parts_mut(dst, len);
    assert_no_overlap(src, dst);
    let read = encoding_rs::mem::utf8_latin1_up_to(src);
    let written = encoding_rs::mem::convert_utf8_to_latin1_lossy(&src[..read], dst);
    log::trace!("utf8-to-latin1 {len} => ({read}, {written})");
    Ok((read, written))
}

/// Converts a utf16 string to latin1
///
/// This is the same as `utf8_to_latin1` in terms of parameters/results.
unsafe fn utf16_to_latin1(src: *mut u16, len: usize, dst: *mut u8) -> Result<(usize, usize)> {
    let src = slice::from_raw_parts(src, len);
    let dst = slice::from_raw_parts_mut(dst, len);
    assert_no_overlap(src, dst);

    let mut size = 0;
    for (src, dst) in src.iter().zip(dst) {
        let src = u16::from_le(*src);
        match u8::try_from(src) {
            Ok(src) => *dst = src,
            Err(_) => break,
        }
        size += 1;
    }
    log::trace!("utf16-to-latin1 {len} => {size}");
    Ok((size, size))
}

/// Converts a utf8 string to a utf16 string which has been partially converted
/// as latin1 prior.
///
/// The original string has already been partially transcoded with
/// `utf8_to_latin1` and that was determined to not be able to transcode the
/// entire string. The substring of the source that couldn't be encoded into
/// latin1 is passed here via `src` and `src_len`.
///
/// The destination buffer is specified by `dst` and `dst_len`. The first
/// `latin1_bytes_so_far` bytes (not code units) of the `dst` buffer have
/// already been filled in with latin1 characters and need to be inflated
/// in-place to their utf16 equivalents.
///
/// After the initial latin1 code units have been inflated the entirety of `src`
/// is then transcoded into the remaining space within `dst`.
unsafe fn utf8_to_compact_utf16(
    src: *mut u8,
    src_len: usize,
    dst: *mut u16,
    dst_len: usize,
    latin1_bytes_so_far: usize,
) -> Result<usize> {
    let src = slice::from_raw_parts(src, src_len);
    let dst = slice::from_raw_parts_mut(dst, dst_len);
    assert_no_overlap(src, dst);

    let dst = inflate_latin1_bytes(dst, latin1_bytes_so_far);
    let result = run_utf8_to_utf16(src, dst)?;
    log::trace!("utf8-to-compact-utf16 {src_len}/{dst_len}/{latin1_bytes_so_far} => {result}");
    Ok(result + latin1_bytes_so_far)
}

/// Same as `utf8_to_compact_utf16` but for utf16 source strings.
unsafe fn utf16_to_compact_utf16(
    src: *mut u16,
    src_len: usize,
    dst: *mut u16,
    dst_len: usize,
    latin1_bytes_so_far: usize,
) -> Result<usize> {
    let src = slice::from_raw_parts(src, src_len);
    let dst = slice::from_raw_parts_mut(dst, dst_len);
    assert_no_overlap(src, dst);

    let dst = inflate_latin1_bytes(dst, latin1_bytes_so_far);
    run_utf16_to_utf16(src, dst)?;
    let result = src.len();
    log::trace!("utf16-to-compact-utf16 {src_len}/{dst_len}/{latin1_bytes_so_far} => {result}");
    Ok(result + latin1_bytes_so_far)
}

/// Inflates the `latin1_bytes_so_far` number of bytes written to the beginning
/// of `dst` into u16 codepoints.
///
/// Returns the remaining space in the destination that can be transcoded into,
/// slicing off the prefix of the string that was inflated from the latin1
/// bytes.
fn inflate_latin1_bytes(dst: &mut [u16], latin1_bytes_so_far: usize) -> &mut [u16] {
    // Note that `latin1_bytes_so_far` is a byte measure while `dst` is a region
    // of u16 units. This `split_at_mut` uses the byte index as an index into
    // the u16 unit because each of the latin1 bytes will become a whole code
    // unit in the destination which is 2 bytes large.
    let (to_inflate, rest) = dst.split_at_mut(latin1_bytes_so_far);

    // Use a byte-oriented view to inflate the original latin1 bytes.
    let (left, mid, right) = unsafe { to_inflate.align_to_mut::<u8>() };
    assert!(left.is_empty());
    assert!(right.is_empty());
    for i in (0..latin1_bytes_so_far).rev() {
        mid[2 * i] = mid[i];
        mid[2 * i + 1] = 0;
    }

    return rest;
}

unsafe fn resource_new32(vmctx: *mut VMComponentContext, resource: u32, rep: u32) -> Result<u32> {
    let resource = TypeResourceTableIndex::from_u32(resource);
    ComponentInstance::from_vmctx(vmctx, |instance| instance.resource_new32(resource, rep))
}

unsafe fn resource_rep32(vmctx: *mut VMComponentContext, resource: u32, idx: u32) -> Result<u32> {
    let resource = TypeResourceTableIndex::from_u32(resource);
    ComponentInstance::from_vmctx(vmctx, |instance| instance.resource_rep32(resource, idx))
}

unsafe fn resource_drop(vmctx: *mut VMComponentContext, resource: u32, idx: u32) -> Result<u64> {
    let resource = TypeResourceTableIndex::from_u32(resource);
    ComponentInstance::from_vmctx(vmctx, |instance| {
        Ok(match instance.resource_drop(resource, idx)? {
            Some(rep) => (u64::from(rep) << 1) | 1,
            None => 0,
        })
    })
}

unsafe fn resource_transfer_own(
    vmctx: *mut VMComponentContext,
    src_idx: u32,
    src_table: u32,
    dst_table: u32,
) -> Result<u32> {
    let src_table = TypeResourceTableIndex::from_u32(src_table);
    let dst_table = TypeResourceTableIndex::from_u32(dst_table);
    ComponentInstance::from_vmctx(vmctx, |instance| {
        instance.resource_transfer_own(src_idx, src_table, dst_table)
    })
}

unsafe fn resource_transfer_borrow(
    vmctx: *mut VMComponentContext,
    src_idx: u32,
    src_table: u32,
    dst_table: u32,
) -> Result<u32> {
    let src_table = TypeResourceTableIndex::from_u32(src_table);
    let dst_table = TypeResourceTableIndex::from_u32(dst_table);
    ComponentInstance::from_vmctx(vmctx, |instance| {
        instance.resource_transfer_borrow(src_idx, src_table, dst_table)
    })
}

unsafe fn resource_enter_call(vmctx: *mut VMComponentContext) -> Result<()> {
    ComponentInstance::from_vmctx(vmctx, |instance| Ok(instance.resource_enter_call()))
}

unsafe fn resource_exit_call(vmctx: *mut VMComponentContext) -> Result<()> {
    ComponentInstance::from_vmctx(vmctx, |instance| instance.resource_exit_call())
}

unsafe fn trap(_vmctx: *mut VMComponentContext, code: u8) -> Result<()> {
    Err(wasmtime_environ::Trap::from_u8(code).unwrap()).err2anyhow()
}
