//! This crate provides utilities for instruction cache maintenance for JIT authors.
//!
//! In self modifying codes such as when writing a JIT, special care must be taken when marking the
//! code as ready for execution. On fully coherent architectures (X86, S390X) the data cache (D-Cache)
//! and the instruction cache (I-Cache) are always in sync. However this is not guaranteed for all
//! architectures such as AArch64 where these caches are not coherent with each other.
//!
//! When writing new code there may be a I-cache entry for that same address which causes the
//! processor to execute whatever was in the cache instead of the new code.
//!
//! See the [ARM Community - Caches and Self-Modifying Code] blog post that contains a great explanation of the above.
//!
//! ## Usage
//!
//! You should call [clear_cache] on any pages that you write with the new code that you're intending
//! to execute. You can do this at any point in the code from the moment that you write the page up to
//! the moment where the code is executed.
//!
//! You also need to call [pipeline_flush] to ensure that there isn't any invalid instruction currently
//! in the pipeline.
//!
//! You can call this in a different order but you should only execute the new code after calling both.
//!
//! ### Example:
//! ```
//! # use std::ffi::c_void;
//! # use std::io;
//! # use jit_icache_coherence::*;
//! #
//! # struct Page {
//! #   addr: *const c_void,
//! #   len: usize,
//! # }
//! #
//! # fn main() -> io::Result<()> {
//! #
//! # let run_code = || {};
//! # let code = vec![0u8; 64];
//! # let newly_written_pages = vec![Page {
//! #    addr: &code[0] as *const u8 as *const c_void,
//! #    len: code.len(),
//! # }];
//! #
//! // Invalidate the cache for all the newly written pages where we wrote our new code.
//! for page in newly_written_pages {
//!     clear_cache(page.addr, page.len)?;
//! }
//!
//! // Once those are invalidated we also need to flush the pipeline
//! pipeline_flush()?;
//!
//! // We can now safely execute our new code.
//! run_code();
//!
//! # Ok(())
//! # }
//! ```
//!
//! <div class="example-wrap" style="display:inline-block"><pre class="compile_fail" style="white-space:normal;font:inherit;">
//!
//!  **Warning**: In order to correctly use this interface you *must* always call both
//! [clear_cache] and [pipeline_flush].
//!
//! </pre></div>
//!
//! [ARM Community - Caches and Self-Modifying Code]: https://community.arm.com/arm-community-blogs/b/architectures-and-processors-blog/posts/caches-and-self-modifying-code

#[cfg(all(
    not(target_os = "windows"),
    not(any(feature = "libc", feature = "rustix"))
))]
compile_error!("Either feature = \"libc\" or feature = \"rustix\" must be enabled!");

#[cfg(all(feature = "libc", feature = "rustix"))]
compile_error!("feature = \"libc\" and feature = \"rustix\" are mutually exclusive!");

#[cfg(all(not(target_os = "windows"), feature = "libc"))]
mod libc;
#[cfg(all(not(target_os = "windows"), feature = "rustix"))]
mod rustix;
#[cfg(target_os = "windows")]
mod win;

use std::ffi::c_void;
use std::io::Result;

/// Flushes instructions in the processor pipeline
///
/// This pipeline flush is broadcast to all processors in the same coherence domain.
///
/// If the architecture does not require a pipeline flush, this function does nothing.
pub fn pipeline_flush() -> Result<()> {
    #[cfg(target_os = "windows")]
    win::pipeline_flush()?;
    #[cfg(all(not(target_os = "windows"), feature = "libc"))]
    libc::pipeline_flush()?;
    #[cfg(all(not(target_os = "windows"), feature = "rustix"))]
    rustix::pipeline_flush()?;

    Ok(())
}

/// Flushes the instruction cache for a region of memory.
///
/// If the architecture does not require an instruction cash flush, this function does nothing.
pub fn clear_cache(ptr: *const c_void, len: usize) -> Result<()> {
    #[cfg(target_os = "windows")]
    win::clear_cache(ptr, len)?;
    #[cfg(all(not(target_os = "windows"), feature = "libc"))]
    libc::clear_cache(ptr, len)?;
    #[cfg(all(not(target_os = "windows"), feature = "rustix"))]
    rustix::clear_cache(ptr, len)?;

    Ok(())
}
