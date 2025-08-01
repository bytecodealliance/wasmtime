//! Module for Linux pagemap based tracking of dirty pages.
//!
//! For other platforms, a no-op implementation is provided.

use self::ioctl::{Categories, PageMapScanBuilder};
use crate::prelude::*;
use crate::runtime::vm::{HostAlignedByteCount, host_page_size};
use rustix::ioctl::ioctl;
use std::fs::File;
use std::mem::MaybeUninit;
use std::ptr;

#[derive(Debug)]
pub struct PageMap(File);

impl PageMap {
    pub fn new() -> Option<PageMap> {
        let file = File::open("/proc/self/pagemap").ok()?;
        // Check if the `pagemap_scan` ioctl is supported.
        let mut regions = vec![MaybeUninit::uninit(); 1];
        let pm_scan = PageMapScanBuilder::new(ptr::slice_from_raw_parts(ptr::null_mut(), 0))
            .max_pages(1)
            .return_mask(Categories::empty())
            .category_mask(Categories::all())
            .build(&mut regions);

        // SAFETY: we did our best in the `ioctl` code below to model this ioctl
        // safely, and it's safe to issue the ioctl on `/proc/self/pagemap`.
        unsafe {
            ioctl(&file, pm_scan).ok()?;
        }
        Some(PageMap(file))
    }
}

/// Resets `ptr` for `len` bytes.
///
/// # Safety
///
/// Requires that `ptr` is valid to read and write for `len` bytes.
pub unsafe fn reset_with_pagemap(
    pagemap: Option<&PageMap>,
    ptr: *mut u8,
    len: HostAlignedByteCount,
    mut keep_resident: HostAlignedByteCount,
    mut reset_manually: impl FnMut(&mut [u8]),
    mut decommit: impl FnMut(*mut u8, usize),
) {
    keep_resident = keep_resident.min(len);
    let host_page_size = host_page_size();

    let pagemap = match pagemap {
        // The `pagemap_scan` ioctl interprets max_pages == 0 as "no limit",
        // whereas we want to interpret it as "don't scan any pages", so only
        // continue further on if we're keeping some bytes resident.
        //
        // Additionally fall back to the default behavior if the `keep_resident`
        // value is just one page of host memory.
        //
        Some(pagemap)
            if keep_resident.byte_count() > 0 && keep_resident.byte_count() > host_page_size =>
        {
            pagemap
        }

        // SAFETY: the safety requirement of
        // `pagemap_disabled::reset_with_pagemap` is the same as this function.
        _ => unsafe {
            return crate::runtime::vm::pagemap_disabled::reset_with_pagemap(
                None,
                ptr,
                len,
                keep_resident,
                reset_manually,
                decommit,
            );
        },
    };

    // For now use a fixed set of regions on the stack, but in the future this
    // may want to use a dynamically allocated vector for more regions for
    // example.
    const MAX_REGIONS: usize = 32;
    let mut storage = [MaybeUninit::uninit(); MAX_REGIONS];

    let scan_arg = PageMapScanBuilder::new(ptr::slice_from_raw_parts(ptr, len.byte_count()))
        .max_pages(keep_resident.byte_count() / host_page_size)
        // We specifically want pages that are NOT backed by the zero page or
        // backed by files. Such pages mean that they haven't changed from their
        // original contents, so they're inverted.
        .category_inverted(Categories::PFNZERO | Categories::FILE)
        // Search for pages that are written and present as those are the dirty
        // pages. Additionally search for the zero page/file page as those are
        // inverted above meaning we're searching for pages that specifically
        // don't have those flags.
        .category_mask(
            Categories::WRITTEN | Categories::PRESENT | Categories::PFNZERO | Categories::FILE,
        )
        // Don't return any categories back. This helps group regions together
        // since the reported set of categories is always empty and we otherwise
        // aren't looking for anything in particular.
        .return_mask(Categories::empty())
        // Not used, but keep the helper below "used" in case it's needed in the
        // future.
        .category_anyof_mask(Categories::empty())
        .build(&mut storage);

    // SAFETY: this should be a safe ioctl as we control the fd we're operating
    // on plus all of `scan_arg`, but this relies on `Ioctl` below being the
    // correct implementation and such.
    let result = match unsafe { ioctl(&pagemap.0, scan_arg) } {
        Ok(result) => result,

        // SAFETY: the safety requirement of
        // `pagemap_disabled::reset_with_pagemap` is the same as this function.
        Err(err) => unsafe {
            log::warn!("failed pagemap scan {err}");
            return crate::runtime::vm::pagemap_disabled::reset_with_pagemap(
                None,
                ptr,
                len,
                keep_resident,
                reset_manually,
                decommit,
            );
        },
    };

    // For all regions that were written in the scan reset them manually, then
    // afterwards decommit everything else.
    for region in result.regions() {
        // not used at this time, but keep it alive as a helper from the `ioctl`
        // module in case it's needed in the future.
        let _ = region.categories();

        // SAFETY: we're relying on Linux to pass in valid region ranges within
        // the `ptr/len` we specified to the original syscall.
        unsafe {
            reset_manually(&mut *region.region().cast_mut());
        }
    }
    let scan_size = result.walk_end().addr() - ptr.addr();
    decommit(result.walk_end().cast_mut(), len.byte_count() - scan_size);
}

mod ioctl {
    use rustix::ioctl::*;
    use std::ffi::c_void;
    use std::fmt;
    use std::marker;
    use std::mem::MaybeUninit;
    use std::ptr;

    bitflags::bitflags! {
        /// Categories that can be filtered with [`PageMapScan`]
        #[derive(Copy, Clone, PartialEq, Eq)]
        #[repr(transparent)]
        pub struct Categories: u64 {
            /// The page has asynchronous write-protection enabled.
            const WPALLOWED = 1 << 0;
            /// The page has been written to from the time it was write protected.
            const WRITTEN = 1 << 1;
            /// The page is file backed.
            const FILE = 1 << 2;
            /// The page is present in the memory.
            const PRESENT = 1 << 3;
            /// The page is swapped.
            const SWAPPED = 1 << 4;
            /// The page has zero PFN.
            const PFNZERO = 1 << 5;
            /// The page is THP or Hugetlb backed.
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

    /// Builder-style structure for building up a [`PageMapScan`] `ioctl` call.
    pub struct PageMapScanBuilder {
        pm_scan_arg: pm_scan_arg,
    }

    impl PageMapScanBuilder {
        /// Creates a new page map scan that will scan the provided range of memory.
        pub fn new(region: *const [u8]) -> PageMapScanBuilder {
            PageMapScanBuilder {
                pm_scan_arg: pm_scan_arg {
                    size: size_of::<pm_scan_arg>() as u64,
                    flags: 0,
                    start: region.cast::<u8>().addr() as u64,
                    end: region.cast::<u8>().addr().wrapping_add(region.len()) as u64,
                    walk_end: 0,
                    vec: 0,
                    vec_len: 0,
                    max_pages: 0,
                    category_inverted: Categories::empty(),
                    category_anyof_mask: Categories::empty(),
                    category_mask: Categories::empty(),
                    return_mask: Categories::empty(),
                },
            }
        }

        /// Configures the maximum number of returned pages in the output regions.
        ///
        /// Setting this to 0 disables this maximum.
        pub fn max_pages(&mut self, max: usize) -> &mut PageMapScanBuilder {
            self.pm_scan_arg.max_pages = max.try_into().unwrap();
            self
        }

        /// Configures categories which values must match if 0 instead of 1.
        ///
        /// Note that this is a mask which is xor'd to the page's true
        /// categories before testing for `category_mask`. That means that if a
        /// bit needs to be zero then it additionally must be specified in one
        /// of `category_mask` or `category_anyof_mask`.
        pub fn category_inverted(&mut self, flags: Categories) -> &mut PageMapScanBuilder {
            self.pm_scan_arg.category_inverted = flags;
            self
        }

        /// Skip pages for which any category doesn't match.
        ///
        /// This mask is applied after `category_inverted` is used to flip bits
        /// in a page's categories. Only pages which match all bits in `flags`
        /// will be considered.
        pub fn category_mask(&mut self, flags: Categories) -> &mut PageMapScanBuilder {
            self.pm_scan_arg.category_mask = flags;
            self
        }

        /// Skip pages for which no category matches.
        ///
        /// Like `category_mask` this is applied after pages have had their category
        /// bits inverted by `category_inverted`.
        pub fn category_anyof_mask(&mut self, flags: Categories) -> &mut PageMapScanBuilder {
            self.pm_scan_arg.category_anyof_mask = flags;
            self
        }

        /// Categories that are to be reported in the regions returned
        pub fn return_mask(&mut self, flags: Categories) -> &mut PageMapScanBuilder {
            self.pm_scan_arg.return_mask = flags;
            self
        }

        /// Finishes this configuration and flags that the scan results will be
        /// placed within `dst`. The returned object can be used to perform the
        /// pagemap scan ioctl.
        pub fn build<'a>(&self, dst: &'a mut [MaybeUninit<PageRegion>]) -> PageMapScan<'a> {
            let mut ret = PageMapScan {
                pm_scan_arg: self.pm_scan_arg,
                _marker: marker::PhantomData,
            };
            ret.pm_scan_arg.vec = dst.as_ptr() as u64;
            ret.pm_scan_arg.vec_len = dst.len() as u64;
            return ret;
        }
    }

    /// Return result of [`PageMapScanBuilder::build`] used to perform an `ioctl`.
    #[repr(transparent)]
    pub struct PageMapScan<'a> {
        pm_scan_arg: pm_scan_arg,
        _marker: marker::PhantomData<&'a mut [MaybeUninit<PageRegion>]>,
    }

    #[derive(Copy, Clone)]
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

    /// Return result of a [`PageMapScan`] `ioctl`.
    ///
    /// This reports where the kernel stopped walking with
    /// [`PageMapScanResult::walk_end`] and the description of regions found in
    /// [`PageMapScanResult::regions`].
    #[derive(Debug)]
    pub struct PageMapScanResult<'a> {
        walk_end: *const u8,
        regions: &'a mut [PageRegion],
    }

    impl PageMapScanResult<'_> {
        /// Where the kernel stopped walking pages, which may be earlier than the
        /// end of the requested region
        pub fn walk_end(&self) -> *const u8 {
            self.walk_end
        }

        /// Regions the kernel reported back with categories and such.
        pub fn regions(&self) -> &[PageRegion] {
            self.regions
        }
    }

    /// Return value of [`PageMapScan`], description of regions in the original scan
    /// with the categories queried.
    #[repr(transparent)]
    #[derive(Copy, Clone)]
    pub struct PageRegion(page_region);

    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    struct page_region {
        start: u64,
        end: u64,
        categories: Categories,
    }

    impl PageRegion {
        /// Returns the region of memory this represents as `*const [u8]`
        #[inline]
        pub fn region(&self) -> *const [u8] {
            ptr::slice_from_raw_parts(self.start(), self.len())
        }

        /// Returns the base pointer into memory this region represents.
        #[inline]
        pub fn start(&self) -> *const u8 {
            self.0.start as *const u8
        }

        /// Returns the byte length that this region represents.
        #[inline]
        pub fn len(&self) -> usize {
            usize::try_from(self.0.end - self.0.start).unwrap()
        }

        /// Returns the category flags associated with this region.
        #[inline]
        pub fn categories(&self) -> Categories {
            self.0.categories
        }
    }

    impl fmt::Debug for PageRegion {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("PageRegion")
                .field("start", &self.start())
                .field("len", &self.len())
                .field("categories", &self.0.categories)
                .finish()
        }
    }

    // SAFETY: this implementation should uphold the various requirements that
    // this trait has, such as `IS_MUTATING` is right, it's only used on the
    // right platform with the right files, etc.
    unsafe impl<'a> Ioctl for PageMapScan<'a> {
        type Output = PageMapScanResult<'a>;

        const IS_MUTATING: bool = true;

        fn opcode(&self) -> Opcode {
            opcode::read_write::<pm_scan_arg>(b'f', 16)
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
            // SAFETY: it's a requirement of this method that
            // `extract_output` is safe to read an indeed a `pm_scan_arg`.
            // Additionally the slice returned here originated from a slice
            // provided to `PageMapScanBuilder::build` threaded through the
            // `vec` field and it should be safe to thread that back out through
            // to the result.
            let regions = unsafe {
                assert!((len as u64) <= (*extract_output).vec_len);
                std::slice::from_raw_parts_mut((*extract_output).vec as *mut PageRegion, len)
            };
            Ok(PageMapScanResult {
                regions,
                // SAFETY: it's a requirement of this method that
                // `extract_output` is safe to read an indeed a `pm_scan_arg`.
                walk_end: unsafe { (*extract_output).walk_end as *const u8 },
            })
        }
    }
}
#[cfg(test)]
mod tests {
    use super::ioctl::*;
    use crate::prelude::*;
    use rustix::ioctl::*;
    use rustix::mm::*;
    use std::fs::File;
    use std::ptr;

    struct MmapAnonymous {
        ptr: *mut std::ffi::c_void,
        len: usize,
    }

    impl MmapAnonymous {
        fn new(pages: usize) -> MmapAnonymous {
            let len = pages * rustix::param::page_size();
            let ptr = unsafe {
                mmap_anonymous(
                    ptr::null_mut(),
                    len,
                    ProtFlags::READ | ProtFlags::WRITE,
                    MapFlags::PRIVATE,
                )
                .unwrap()
            };
            MmapAnonymous { ptr, len }
        }

        fn read(&self, page: usize) {
            unsafe {
                let offset = page * rustix::param::page_size();
                assert!(offset < self.len);
                std::ptr::read_volatile(self.ptr.cast::<u8>().add(offset));
            }
        }

        fn write(&self, page: usize) {
            unsafe {
                let offset = page * rustix::param::page_size();
                assert!(offset < self.len);
                std::ptr::write_volatile(self.ptr.cast::<u8>().add(offset), 1);
            }
        }

        fn region(&self) -> *const [u8] {
            ptr::slice_from_raw_parts(self.ptr.cast(), self.len)
        }

        fn page_region(&self, pages: std::ops::Range<usize>) -> *const [u8] {
            ptr::slice_from_raw_parts(
                self.ptr
                    .cast::<u8>()
                    .wrapping_add(pages.start * rustix::param::page_size()),
                (pages.end - pages.start) * rustix::param::page_size(),
            )
        }

        fn end(&self) -> *const u8 {
            self.ptr.cast::<u8>().wrapping_add(self.len)
        }

        fn page_end(&self, page: usize) -> *const u8 {
            self.ptr
                .cast::<u8>()
                .wrapping_add((page + 1) * rustix::param::page_size())
        }
    }

    impl Drop for MmapAnonymous {
        fn drop(&mut self) {
            unsafe {
                munmap(self.ptr, self.len).unwrap();
            }
        }
    }

    #[test]
    fn no_pages_returned() {
        let mmap = MmapAnonymous::new(10);
        let mut results = Vec::with_capacity(10);
        let fd = File::open("/proc/self/pagemap").unwrap();

        let result = unsafe {
            ioctl(
                &fd,
                PageMapScanBuilder::new(mmap.region())
                    .category_mask(Categories::WRITTEN)
                    .return_mask(Categories::all())
                    .build(results.spare_capacity_mut()),
            )
            .unwrap()
        };
        assert!(result.regions().is_empty());
        assert_eq!(result.walk_end(), mmap.end());
    }

    #[test]
    fn empty_region() {
        let mut results = Vec::with_capacity(10);
        let fd = File::open("/proc/self/pagemap").unwrap();

        let empty_region = ptr::slice_from_raw_parts(rustix::param::page_size() as *const u8, 0);
        let result = unsafe {
            ioctl(
                &fd,
                PageMapScanBuilder::new(empty_region)
                    .return_mask(Categories::all())
                    .build(results.spare_capacity_mut()),
            )
            .unwrap()
        };
        assert!(result.regions().is_empty());
    }

    #[test]
    fn basic_page_flags() {
        let mmap = MmapAnonymous::new(10);
        let mut results = Vec::with_capacity(10);
        let fd = File::open("/proc/self/pagemap").unwrap();

        mmap.read(0);
        mmap.write(1);
        mmap.write(2);
        mmap.read(3);

        mmap.read(5);
        mmap.read(6);

        let result = unsafe {
            ioctl(
                &fd,
                PageMapScanBuilder::new(mmap.region())
                    .category_mask(Categories::WRITTEN)
                    .return_mask(Categories::WRITTEN | Categories::PRESENT | Categories::PFNZERO)
                    .build(results.spare_capacity_mut()),
            )
            .unwrap()
        };
        assert_eq!(result.regions().len(), 4);
        assert_eq!(result.walk_end(), mmap.end());
        assert_eq!(result.regions()[0].region(), mmap.page_region(0..1));
        assert_eq!(
            result.regions()[0].categories(),
            Categories::WRITTEN | Categories::PRESENT | Categories::PFNZERO
        );

        assert_eq!(result.regions()[1].region(), mmap.page_region(1..3));
        assert_eq!(
            result.regions()[1].categories(),
            Categories::WRITTEN | Categories::PRESENT
        );

        assert_eq!(result.regions()[2].region(), mmap.page_region(3..4));
        assert_eq!(
            result.regions()[2].categories(),
            Categories::WRITTEN | Categories::PRESENT | Categories::PFNZERO
        );

        assert_eq!(result.regions()[3].region(), mmap.page_region(5..7));
        assert_eq!(
            result.regions()[3].categories(),
            Categories::WRITTEN | Categories::PRESENT | Categories::PFNZERO
        );
    }

    #[test]
    fn only_written_pages() {
        let mmap = MmapAnonymous::new(10);
        let mut results = Vec::with_capacity(10);
        let fd = File::open("/proc/self/pagemap").unwrap();

        mmap.read(0);
        mmap.write(1);
        mmap.write(2);
        mmap.read(3);

        mmap.read(5);
        mmap.read(6);

        let result = unsafe {
            ioctl(
                &fd,
                PageMapScanBuilder::new(mmap.region())
                    .category_inverted(Categories::PFNZERO)
                    .category_mask(Categories::WRITTEN | Categories::PFNZERO)
                    .return_mask(Categories::WRITTEN | Categories::PRESENT | Categories::PFNZERO)
                    .build(results.spare_capacity_mut()),
            )
            .unwrap()
        };
        assert_eq!(result.regions().len(), 1);
        assert_eq!(result.walk_end(), mmap.end());

        assert_eq!(result.regions()[0].region(), mmap.page_region(1..3));
        assert_eq!(
            result.regions()[0].categories(),
            Categories::WRITTEN | Categories::PRESENT
        );
    }

    #[test]
    fn region_limit() {
        let mmap = MmapAnonymous::new(10);
        let mut results = Vec::with_capacity(1);
        let fd = File::open("/proc/self/pagemap").unwrap();

        mmap.read(0);
        mmap.write(1);
        mmap.read(2);
        mmap.write(3);

        // Ask for written|pfnzero meaning only-read pages. This should return only
        // a single region of the first page.
        let result = unsafe {
            ioctl(
                &fd,
                PageMapScanBuilder::new(mmap.region())
                    .return_mask(Categories::WRITTEN | Categories::PFNZERO)
                    .build(results.spare_capacity_mut()),
            )
            .unwrap()
        };
        assert_eq!(result.regions().len(), 1);
        assert_eq!(result.walk_end(), mmap.page_end(0));

        assert_eq!(result.regions()[0].region(), mmap.page_region(0..1));
        assert_eq!(
            result.regions()[0].categories(),
            Categories::WRITTEN | Categories::PFNZERO
        );

        // If we ask for written pages though (which seems synonymous with
        // present?) then everything should be in one region.
        let result = unsafe {
            ioctl(
                &fd,
                PageMapScanBuilder::new(mmap.region())
                    .return_mask(Categories::WRITTEN)
                    .build(results.spare_capacity_mut()),
            )
            .unwrap()
        };
        assert_eq!(result.regions().len(), 1);
        assert_eq!(result.walk_end(), mmap.page_end(3));

        assert_eq!(result.regions()[0].region(), mmap.page_region(0..4));
        assert_eq!(result.regions()[0].categories(), Categories::WRITTEN);
    }

    #[test]
    fn page_limit() {
        let mmap = MmapAnonymous::new(10);
        let mut results = Vec::with_capacity(10);
        let fd = File::open("/proc/self/pagemap").unwrap();

        mmap.read(0);
        mmap.read(1);
        mmap.read(2);
        mmap.read(3);

        // Ask for written|pfnzero meaning only-read pages. This should return only
        // a single region of the first page.
        let result = unsafe {
            ioctl(
                &fd,
                PageMapScanBuilder::new(mmap.region())
                    .return_mask(Categories::WRITTEN | Categories::PFNZERO)
                    .max_pages(2)
                    .build(results.spare_capacity_mut()),
            )
            .unwrap()
        };
        assert_eq!(result.regions().len(), 1);
        assert_eq!(result.walk_end(), mmap.page_end(1));

        assert_eq!(result.regions()[0].region(), mmap.page_region(0..2));
        assert_eq!(
            result.regions()[0].categories(),
            Categories::WRITTEN | Categories::PFNZERO
        );
    }

    #[test]
    fn page_limit_with_hole() {
        let mmap = MmapAnonymous::new(10);
        let mut results = Vec::with_capacity(10);
        let fd = File::open("/proc/self/pagemap").unwrap();

        mmap.read(0);
        mmap.read(2);
        mmap.read(3);

        // Ask for written|pfnzero meaning only-read pages. This should return only
        // a single region of the first page.
        let result = unsafe {
            ioctl(
                &fd,
                PageMapScanBuilder::new(mmap.region())
                    .category_mask(Categories::WRITTEN)
                    .return_mask(Categories::WRITTEN | Categories::PFNZERO)
                    .max_pages(2)
                    .build(results.spare_capacity_mut()),
            )
            .unwrap()
        };
        assert_eq!(result.regions().len(), 2);
        assert_eq!(result.walk_end(), mmap.page_end(2));

        assert_eq!(result.regions()[0].region(), mmap.page_region(0..1));
        assert_eq!(
            result.regions()[0].categories(),
            Categories::WRITTEN | Categories::PFNZERO
        );
        assert_eq!(result.regions()[1].region(), mmap.page_region(2..3));
        assert_eq!(
            result.regions()[1].categories(),
            Categories::WRITTEN | Categories::PFNZERO
        );
    }
}
