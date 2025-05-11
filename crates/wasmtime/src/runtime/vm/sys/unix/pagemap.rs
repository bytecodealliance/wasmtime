//! Module for Linux pagemap based tracking of dirty pages.
//!
//! For other platforms, a no-op implementation is provided.

use crate::prelude::*;
pub use internal::dirty_pages_in_region;
use std::fmt;
use std::mem::MaybeUninit;

#[allow(dead_code)]
#[derive(Debug)]
pub struct DirtyPages<'a> {
    /// Slice into the initialized portion of region_storage
    pub regions: &'a [PageRegion],
    /// The number of bytes checked in the pagemap. Might be less than `len`, in which case
    /// the pages beyond `checked_bytes` should be treated as potentially dirty.
    pub checked_bytes: usize,
    // We hold the storage here to keep it alive while regions points into it
    region_storage: Vec<MaybeUninit<PageRegion>>,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct PageRegion {
    pub start: u64,
    pub end: u64,
    categories: Categories,
}

bitflags::bitflags! {
    #[derive(Copy, Clone)]
    #[repr(transparent)]
    struct Categories: u64 {
        const WPALLOWED = 1 << 0;
        const WRITTEN = 1 << 1;
        const FILE = 1 << 2;
        const PRESENT = 1 << 3;
        const SWAPPED = 1 << 4;
        const PFNZERO = 1 << 5;
        const HUGE = 1 << 6;
        const SOFT_DIRTY = 1 << 7;
    }
}

impl fmt::Debug for Categories {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        bitflags::parser::to_writer(self, f)
    }
}
impl fmt::Display for Categories {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        bitflags::parser::to_writer(self, f)
    }
}

#[cfg(not(target_os = "linux"))]
mod internal {
    use super::DirtyPages;
    use crate::prelude::*;
    use crate::runtime::vm::HostAlignedByteCount;
    #[allow(unused_variables)]
    pub fn dirty_pages_in_region<'a>(
        base: *const u8,
        len: HostAlignedByteCount,
        max_bytes: HostAlignedByteCount,
    ) -> Result<DirtyPages<'a>> {
        Err(anyhow!("pagemap_scan ioctl not supported on this platform"))
    }
}

#[cfg(target_os = "linux")]
mod internal {
    use super::{Categories, DirtyPages, PageRegion};
    use crate::prelude::*;
    use crate::runtime::vm::{host_page_size, HostAlignedByteCount};
    use rustix::ioctl::{ioctl, opcode, Ioctl, IoctlOutput, Opcode};
    use std::fs::File;
    use std::mem::MaybeUninit;
    use std::os::raw::c_void;
    use std::sync::LazyLock;
    use std::{fmt, ptr};

    pub fn dirty_pages_in_region<'a>(
        base: *const u8,
        len: HostAlignedByteCount,
        max_bytes: HostAlignedByteCount,
    ) -> Result<DirtyPages<'a>> {
        let pagemap = match &*PAGEMAP {
            Some(pagemap) => pagemap,
            None => return Err(anyhow!("pagemap_scan ioctl not supported")),
        };

        let max_pages = max_bytes.byte_count() / host_page_size();
        let mut storage = vec![MaybeUninit::uninit(); max_pages];
        let scan_arg = PageMapScan::new(
            ptr::slice_from_raw_parts(base, len.byte_count()),
            &mut storage,
            max_pages,
        );
        let result = unsafe { ioctl(pagemap, scan_arg) };
        match result {
            Ok(result) => {
                let regions = unsafe {
                    std::slice::from_raw_parts(
                        storage.as_ptr() as *const PageRegion,
                        result.regions_count,
                    )
                };
                Ok(DirtyPages {
                    regions,
                    checked_bytes: result.walk_end - base as usize,
                    region_storage: storage,
                })
            }
            Err(_) => Err(anyhow!("pagemap_scan ioctl failed")),
        }
    }

    bitflags::bitflags! {
        #[derive(Debug)]
        struct PageMapBits: u64 {
            const PRESENT = 1 << 63;
            const SWAPPED = 1 << 62;
            const FILE = 1 << 61;
            const GUARD = 1 << 58;
            const WP = 1 << 57;
            const EXCL = 1 << 56;
            const SOFT_DIRTY = 1 << 55;
        }
    }

    impl fmt::Display for PageMapBits {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            bitflags::parser::to_writer(self, f)
        }
    }

    struct PageMapScan<'a> {
        pm_scan_arg: pm_scan_arg,
        _regions: &'a mut [MaybeUninit<PageRegion>],
    }

    impl<'a> PageMapScan<'a> {
        fn new(
            region: *const [u8],
            regions: &'a mut [MaybeUninit<PageRegion>],
            max_pages: usize,
        ) -> PageMapScan<'a> {
            PageMapScan {
                pm_scan_arg: pm_scan_arg {
                    size: size_of::<pm_scan_arg>() as u64,
                    flags: 0,
                    start: unsafe { (*region).as_ptr() as u64 },
                    end: unsafe { (*region).as_ptr().wrapping_add((*region).len()) as u64 },
                    walk_end: 0,
                    vec: regions.as_mut_ptr() as u64,
                    vec_len: regions.len() as u64,
                    max_pages: max_pages as u64,
                    category_inverted: Categories::FILE | Categories::PFNZERO,
                    category_anyof_mask: Categories::empty(),
                    category_mask: Categories::WRITTEN | Categories::FILE | Categories::PFNZERO,
                    return_mask: Categories::all(),
                },
                _regions: regions,
            }
        }
    }

    #[derive(Debug)]
    #[allow(dead_code)]
    struct PageMapScanResult {
        walk_end: usize,
        regions_count: usize,
    }

    #[repr(C)]
    struct pm_scan_arg {
        size: u64,
        flags: u64,
        start: u64,
        end: u64,
        walk_end: u64,
        vec: u64,
        vec_len: u64,
        max_pages: u64,
        category_inverted: Categories,
        category_mask: Categories,
        category_anyof_mask: Categories,
        return_mask: Categories,
    }

    const PAGEMAP_SCAN: Opcode = opcode::read_write::<pm_scan_arg>(b'f', 16);

    unsafe impl<'a> Ioctl for PageMapScan<'a> {
        type Output = PageMapScanResult;

        const IS_MUTATING: bool = false;

        fn opcode(&self) -> Opcode {
            PAGEMAP_SCAN
        }

        fn as_ptr(&mut self) -> *mut c_void {
            (&raw mut self.pm_scan_arg).cast()
        }

        unsafe fn output_from_ptr(
            out: IoctlOutput,
            extract_output: *mut c_void,
        ) -> rustix::io::Result<Self::Output> {
            let extract_output = extract_output.cast::<pm_scan_arg>();
            let len = usize::try_from(out).unwrap();
            Ok(PageMapScanResult {
                regions_count: len,
                walk_end: unsafe { (*extract_output).walk_end.try_into().unwrap() },
            })
        }
    }

    /// A static reference to the `/proc/self/pagemap` file. `None` if the file
    /// can't be opened, or if the `pagemap_scan` ioctl is not supported.
    static PAGEMAP: LazyLock<Option<File>> = LazyLock::new(|| {
        let file = File::open("/proc/self/pagemap");
        if file.is_err() {
            return None;
        }
        let file = file.unwrap();
        // Check if the `pagemap_scan` ioctl is supported.
        let mut regions = vec![MaybeUninit::<PageRegion>::uninit(); 0];
        let pm_scan = PageMapScan::new(ptr::slice_from_raw_parts(ptr::null(), 0), &mut regions, 0);
        match unsafe { ioctl(&file, pm_scan) } {
            Ok(_) => Some(file),
            Err(_) => None,
        }
    });
}
