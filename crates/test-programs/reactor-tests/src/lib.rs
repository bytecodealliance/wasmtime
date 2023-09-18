wit_bindgen::generate!({
    world: "test-reactor",
    path: "../../wasi/wit",
    exports: {
        world: T,
    }
});

struct T;
use wasi::io::poll;

static mut STATE: Vec<String> = Vec::new();

impl Guest for T {
    fn add_strings(ss: Vec<String>) -> u32 {
        for s in ss {
            match s.split_once("$") {
                Some((prefix, var)) if prefix.is_empty() => match std::env::var(var) {
                    Ok(val) => unsafe { STATE.push(val) },
                    Err(_) => unsafe { STATE.push("undefined".to_owned()) },
                },
                _ => unsafe { STATE.push(s) },
            }
        }
        unsafe { STATE.len() as u32 }
    }
    fn get_strings() -> Vec<String> {
        unsafe { STATE.clone() }
    }

    fn write_strings_to(o: OutputStream) -> Result<(), ()> {
        let pollable = o.subscribe();
        unsafe {
            for s in STATE.iter() {
                let mut out = s.as_bytes();
                while !out.is_empty() {
                    crate::poll_list(&[&pollable]);
                    let n = match o.check_write() {
                        Ok(n) => n,
                        Err(_) => return Err(()),
                    };

                    let len = (n as usize).min(out.len());
                    match o.write(&out[..len]) {
                        Ok(_) => out = &out[len..],
                        Err(_) => return Err(()),
                    }
                }
            }

            match o.flush() {
                Ok(_) => {}
                Err(_) => return Err(()),
            }

            crate::poll_list(&[&pollable]);
            match o.check_write() {
                Ok(_) => {}
                Err(_) => return Err(()),
            }

            Ok(())
        }
    }
    fn pass_an_imported_record(stat: wasi::filesystem::types::DescriptorStat) -> String {
        format!("{stat:?}")
    }
}

/// FIXME: This is a copy of the `poll_list` bindings generated with a
/// wit-bindgen with this fix:
/// <https://github.com/bytecodealliance/wit-bindgen/pull/670>
///
/// One that PR is in a published release, delete this code and use the
/// bindings.
///
/// Poll for completion on a set of pollables.
///
/// This function takes a list of pollables, which identify I/O sources of
/// interest, and waits until one or more of the events is ready for I/O.
///
/// The result `list<u32>` contains one or more indices of handles in the
/// argument list that is ready for I/O.
///
/// If the list contains more elements than can be indexed with a `u32`
/// value, this function traps.
///
/// A timeout can be implemented by adding a pollable from the
/// wasi-clocks API to the list.
///
/// This function does not return a `result`; polling in itself does not
/// do any I/O so it doesn't fail. If any of the I/O sources identified by
/// the pollables has an error, it is indicated by marking the source as
/// being reaedy for I/O.
pub fn poll_list(in_: &[&poll::Pollable]) -> wit_bindgen::rt::vec::Vec<u32> {
    #[allow(unused_imports)]
    use wit_bindgen::rt::{alloc, string::String, vec::Vec};
    unsafe {
        #[repr(align(4))]
        struct RetArea([u8; 8]);
        let mut ret_area = ::core::mem::MaybeUninit::<RetArea>::uninit();
        let vec0 = in_;
        let len0 = vec0.len() as i32;
        let layout0 = alloc::Layout::from_size_align_unchecked(vec0.len() * 4, 4);
        let result0 = if layout0.size() != 0 {
            let ptr = alloc::alloc(layout0);
            if ptr.is_null() {
                alloc::handle_alloc_error(layout0);
            }
            ptr
        } else {
            ::core::ptr::null_mut()
        };
        for (i, e) in vec0.into_iter().enumerate() {
            let base = result0 as i32 + (i as i32) * 4;
            {
                *((base + 0) as *mut i32) = (e).handle() as i32;
            }
        }
        let ptr1 = ret_area.as_mut_ptr() as i32;
        #[cfg(target_arch = "wasm32")]
        #[link(wasm_import_module = "wasi:io/poll")]
        extern "C" {
            #[link_name = "poll-list"]
            fn wit_import(_: i32, _: i32, _: i32);
        }

        #[cfg(not(target_arch = "wasm32"))]
        fn wit_import(_: i32, _: i32, _: i32) {
            unreachable!()
        }
        wit_import(result0 as i32, len0, ptr1);
        let l2 = *((ptr1 + 0) as *const i32);
        let l3 = *((ptr1 + 4) as *const i32);
        let len4 = l3 as usize;
        if layout0.size() != 0 {
            alloc::dealloc(result0, layout0);
        }
        Vec::from_raw_parts(l2 as *mut _, len4, len4)
    }
}
