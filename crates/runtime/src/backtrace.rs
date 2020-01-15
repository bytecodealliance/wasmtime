//! Backtrace object and utilities.

use crate::jit_function_registry;
use std::sync::Arc;

/// Information about backtrace frame.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct BacktraceFrame {
    pc: usize,
}

impl Default for BacktraceFrame {
    fn default() -> Self {
        Self { pc: 0 }
    }
}

impl BacktraceFrame {
    /// Current PC or IP pointer for the frame.
    pub fn pc(&self) -> usize {
        self.pc
    }
    /// Additinal frame information.
    pub fn tag(&self) -> Option<Arc<jit_function_registry::JITFunctionTag>> {
        jit_function_registry::find(self.pc)
    }
}

const BACKTRACE_LIMIT: usize = 32;

/// Backtrace during WebAssembly trap.
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Backtrace {
    len: usize,
    frames: [BacktraceFrame; BACKTRACE_LIMIT],
}

impl Backtrace {
    fn new() -> Self {
        Self {
            len: 0,
            frames: [Default::default(); BACKTRACE_LIMIT],
        }
    }
    fn full(&self) -> bool {
        self.len >= BACKTRACE_LIMIT
    }
    fn push(&mut self, frame: BacktraceFrame) {
        assert!(self.len < BACKTRACE_LIMIT);
        self.frames[self.len] = frame;
        self.len += 1;
    }
    /// Amount of the backtrace frames.
    pub fn len(&self) -> usize {
        self.len
    }
}

impl std::ops::Index<usize> for Backtrace {
    type Output = BacktraceFrame;
    fn index(&self, index: usize) -> &Self::Output {
        assert!(index < self.len);
        &self.frames[index]
    }
}

impl std::fmt::Debug for Backtrace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Backtrace![")?;
        for i in 0..self.len() {
            let frame = &self.frames[i];
            writeln!(f, "  {:x}: {:?}", frame.pc(), frame.tag())?;
        }
        write!(f, "]")?;
        Ok(())
    }
}

#[cfg(not(all(target_os = "windows", target_arch = "x86_64")))]
fn capture_stack<F>(mut f: F)
where
    F: FnMut(usize) -> bool,
{
    use backtrace::trace;
    trace(|frame| {
        let pc = frame.ip() as usize;
        f(pc)
    });
}

#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
fn capture_stack<F>(mut f: F)
where
    F: FnMut(usize) -> bool,
{
    use std::mem::MaybeUninit;
    use std::ptr;
    use winapi::um::winnt::{
        RtlCaptureContext, RtlLookupFunctionEntry, RtlVirtualUnwind, CONTEXT, UNW_FLAG_NHANDLER,
    };

    #[repr(C, align(16))]
    struct WrappedContext(CONTEXT);

    unsafe {
        let mut ctx = WrappedContext(MaybeUninit::uninit().assume_init());
        RtlCaptureContext(&mut ctx.0);
        let mut unwind_history_table = MaybeUninit::zeroed().assume_init();
        while ctx.0.Rip != 0 {
            let cont = f(ctx.0.Rip as usize);
            if !cont {
                break;
            }

            let mut image_base: u64 = 0;
            let mut handler_data: *mut core::ffi::c_void = ptr::null_mut();
            let mut establisher_frame: u64 = 0;
            let rf = RtlLookupFunctionEntry(ctx.0.Rip, &mut image_base, &mut unwind_history_table);
            if rf.is_null() {
                ctx.0.Rip = ptr::read(ctx.0.Rsp as *const u64);
                ctx.0.Rsp += 8;
            } else {
                RtlVirtualUnwind(
                    UNW_FLAG_NHANDLER,
                    image_base,
                    ctx.0.Rip,
                    rf,
                    &mut ctx.0,
                    &mut handler_data,
                    &mut establisher_frame,
                    ptr::null_mut(),
                );
            }
        }
    }
}

/// Returns current backtrace. Only registered wasmtime functions will be listed.
pub fn get_backtrace() -> Backtrace {
    let mut frames = Backtrace::new();
    capture_stack(|pc| {
        if let Some(_) = jit_function_registry::find(pc) {
            frames.push(BacktraceFrame { pc });
        }
        !frames.full()
    });
    frames
}
