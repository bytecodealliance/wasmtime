//! Memory management for executable code.

use core::{cmp, mem};
use region;
use std::boxed::Box;
use std::string::String;
use std::vec::Vec;
use wasmtime_runtime::{Mmap, VMFunctionBody};

/// Memory manager for executable code.
pub(crate) struct CodeMemory {
    current: Mmap,
    mmaps: Vec<Mmap>,
    position: usize,
    published: usize,
}

impl CodeMemory {
    /// Create a new `CodeMemory` instance.
    pub fn new() -> Self {
        Self {
            current: Mmap::new(),
            mmaps: Vec::new(),
            position: 0,
            published: 0,
        }
    }

    /// Allocate `size` bytes of memory which can be made executable later by
    /// calling `publish()`. Note that we allocate the memory as writeable so
    /// that it can be written to and patched, though we make it readonly before
    /// actually executing from it.
    ///
    /// TODO: Add an alignment flag.
    fn allocate(&mut self, size: usize) -> Result<&mut [u8], String> {
        if self.current.len() - self.position < size {
            // For every mapping on Windows, we need an extra information for structured
            // exception handling. We use the same handler for every function, so just
            // one record for single mmap is fine.
            let size = if cfg!(all(target_os = "windows", target_pointer_width = "64")) {
                size + region::page::size()
            } else {
                size
            };
            self.mmaps.push(mem::replace(
                &mut self.current,
                Mmap::with_at_least(cmp::max(0x10000, size))?,
            ));
            self.position = 0;
            if cfg!(all(target_os = "windows", target_pointer_width = "64")) {
                host_impl::register_executable_memory(&mut self.current);
                self.position += region::page::size();
            }
        }
        let old_position = self.position;
        self.position += size;
        Ok(&mut self.current.as_mut_slice()[old_position..self.position])
    }

    /// Convert mut a slice from u8 to VMFunctionBody.
    fn view_as_mut_vmfunc_slice(slice: &mut [u8]) -> &mut [VMFunctionBody] {
        let byte_ptr: *mut [u8] = slice;
        let body_ptr = byte_ptr as *mut [VMFunctionBody];
        unsafe { &mut *body_ptr }
    }

    /// Allocate enough memory to hold a copy of `slice` and copy the data into it.
    /// TODO: Reorganize the code that calls this to emit code directly into the
    /// mmap region rather than into a Vec that we need to copy in.
    pub fn allocate_copy_of_byte_slice(
        &mut self,
        slice: &[u8],
    ) -> Result<&mut [VMFunctionBody], String> {
        let new = self.allocate(slice.len())?;
        new.copy_from_slice(slice);
        Ok(Self::view_as_mut_vmfunc_slice(new))
    }

    /// Allocate enough continuous memory block for multiple code blocks. See also
    /// allocate_copy_of_byte_slice.
    pub fn allocate_copy_of_byte_slices(
        &mut self,
        slices: &[&[u8]],
    ) -> Result<Box<[&mut [VMFunctionBody]]>, String> {
        let total_len = slices.into_iter().fold(0, |acc, slice| acc + slice.len());
        let new = self.allocate(total_len)?;
        let mut tail = new;
        let mut result = Vec::with_capacity(slices.len());
        for slice in slices {
            let (block, next_tail) = tail.split_at_mut(slice.len());
            block.copy_from_slice(slice);
            tail = next_tail;
            result.push(Self::view_as_mut_vmfunc_slice(block));
        }
        Ok(result.into_boxed_slice())
    }

    /// Make all allocated memory executable.
    pub fn publish(&mut self) {
        self.mmaps
            .push(mem::replace(&mut self.current, Mmap::new()));
        self.position = 0;

        for m in &mut self.mmaps[self.published..] {
            if m.len() != 0 {
                unsafe {
                    region::protect(m.as_mut_ptr(), m.len(), region::Protection::ReadExecute)
                }
                .expect("unable to make memory readonly and executable");
            }
        }
        self.published = self.mmaps.len();
    }
}

#[cfg(all(target_os = "windows", target_pointer_width = "64"))]
mod host_impl {
    // Docs:
    //    https://docs.microsoft.com/en-us/cpp/build/exception-handling-x64?view=vs-2019
    // SpiderMonkey impl:
    //    https://searchfox.org/mozilla-central/source/js/src/jit/ProcessExecutableMemory.cpp#139-227
    // Note:
    //    ARM requires different treatment (not implemented)

    use region;
    use std::convert::TryFrom;
    use std::ptr;
    use wasmtime_runtime::Mmap;
    use winapi::shared::basetsd::{DWORD64, ULONG64};
    use winapi::shared::minwindef::{BYTE, ULONG};
    use winapi::shared::ntdef::FALSE;
    use winapi::um::winnt::RtlInstallFunctionTableCallback;
    use winapi::um::winnt::{
        EXCEPTION_POINTERS, LONG, PCONTEXT, PDISPATCHER_CONTEXT, PEXCEPTION_POINTERS,
        PEXCEPTION_RECORD, PRUNTIME_FUNCTION, PVOID, RUNTIME_FUNCTION, UNW_FLAG_EHANDLER,
    };
    use winapi::vc::excpt::EXCEPTION_DISPOSITION;

    // todo probably we want to have a function to set a callback
    // todo WINAPI calling convention
    // todo even if WasmTrapHandler is a static function, it still compiles!
    extern "C" {
        fn WasmTrapHandler(_: PEXCEPTION_POINTERS) -> LONG;
    }

    #[repr(C)]
    struct ExceptionHandlerRecord {
        runtime_function: RUNTIME_FUNCTION,
        unwind_info: UnwindInfo,
        thunk: [u8; 12],
    }

    // Note: this is a bitfield in WinAPI, so some fields are actually merged below
    #[cfg(not(target_arch = "arm"))]
    #[repr(C)]
    struct UnwindInfo {
        version_and_flags: BYTE,
        size_of_prologue: BYTE,
        count_of_unwind_codes: BYTE,
        frame_register_and_offset: BYTE,
        exception_handler: ULONG,
    }
    #[cfg(not(target_arch = "arm"))]
    static FLAGS_BIT_OFFSET: u8 = 3;

    macro_rules! offsetof {
        ($class:ident, $field:ident) => { unsafe {
            (&(*(ptr::null::<$class>())).$field) as *const _
        } as usize };
    }

    #[cfg(not(target_arch = "arm"))]
    pub fn register_executable_memory(mmap: &mut Mmap) {
        let r = unsafe { (mmap.as_mut_ptr() as *mut ExceptionHandlerRecord).as_mut() }.unwrap();
        r.runtime_function.BeginAddress = u32::try_from(region::page::size()).unwrap();
        r.runtime_function.EndAddress = u32::try_from(mmap.len()).unwrap();
        *unsafe { r.runtime_function.u.UnwindInfoAddress_mut() } =
            u32::try_from(offsetof!(ExceptionHandlerRecord, unwind_info)).unwrap();

        r.unwind_info.version_and_flags = 1; // version
        r.unwind_info.version_and_flags |=
            u8::try_from(UNW_FLAG_EHANDLER << FLAGS_BIT_OFFSET).unwrap(); // flags
        r.unwind_info.size_of_prologue = 0;
        r.unwind_info.count_of_unwind_codes = 0;
        r.unwind_info.frame_register_and_offset = 0;
        r.unwind_info.exception_handler =
            u32::try_from(offsetof!(ExceptionHandlerRecord, thunk)).unwrap();

        // mov imm64, rax
        r.thunk[0] = 0x48;
        r.thunk[1] = 0xb8;
        unsafe {
            ptr::write_unaligned::<usize>(
                &mut r.thunk[2] as *mut _ as *mut usize,
                &exception_handler as *const _ as usize,
            )
        };

        // jmp rax
        r.thunk[10] = 0xff;
        r.thunk[11] = 0xe0;

        let res = unsafe {
            RtlInstallFunctionTableCallback(
                u64::try_from(mmap.as_ptr() as usize).unwrap() | 0x3,
                u64::try_from(mmap.as_ptr() as usize).unwrap(),
                u32::try_from(mmap.len()).unwrap(),
                Some(runtime_function_callback),
                mmap.as_mut_ptr() as *mut _, // user data ptr
                ptr::null_mut(),
            )
        };
        if res == FALSE {
            panic!("RtlInstallFunctionTableCallback() failed");
        }

        // Note: our section needs to have read & execute rights, and publish() will do it.
        //       It needs to be called before calling jitted code, so everything's fine.
        // TODO: is above true? Or do we have to call region::protect() before RtlInstallFunctionTableCallback()?
    }

    // What MSDN docs say (https://docs.microsoft.com/en-us/cpp/build/exception-handling-x64?view=vs-2019#language-specific-handler)
    // and what's in SpiderMonkey (https://searchfox.org/mozilla-central/source/js/src/jit/ProcessExecutableMemory.cpp#124-133)
    // doesn't match for all arguments, but at least they match for the two pointers that are actually used.
    unsafe extern "C" fn exception_handler(
        exception_record: PEXCEPTION_RECORD,
        _establisher_frame: ULONG64,
        context_record: PCONTEXT,
        _dispatcher_context: PDISPATCHER_CONTEXT,
    ) -> EXCEPTION_DISPOSITION {
        let mut exc_ptrs = EXCEPTION_POINTERS {
            ExceptionRecord: exception_record,
            ContextRecord: context_record,
        };
        WasmTrapHandler(&mut exc_ptrs) as EXCEPTION_DISPOSITION
    }

    unsafe extern "C" fn runtime_function_callback(
        _control_pc: DWORD64,
        context: PVOID,
    ) -> PRUNTIME_FUNCTION {
        // context (user data ptr) is a pointer to the first page of mmap where the needed structure lies
        context as *mut _
    }
}
