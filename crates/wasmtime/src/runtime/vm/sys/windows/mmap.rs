use crate::prelude::*;
use crate::runtime::vm::SendSyncPtr;
use std::fs::{File, OpenOptions};
use std::io;
use std::ops::Range;
use std::os::windows::prelude::*;
use std::path::Path;
use std::ptr::{self, NonNull};
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Storage::FileSystem::*;
use windows_sys::Win32::System::Memory::*;

#[derive(Debug)]
pub struct Mmap {
    memory: SendSyncPtr<[u8]>,
    is_file: bool,
}

impl Mmap {
    pub fn new_empty() -> Mmap {
        Mmap {
            memory: SendSyncPtr::from(&mut [][..]),
            is_file: false,
        }
    }

    pub fn new(size: usize) -> Result<Self> {
        let ptr = unsafe {
            VirtualAlloc(
                ptr::null_mut(),
                size,
                MEM_RESERVE | MEM_COMMIT,
                PAGE_READWRITE,
            )
        };
        if ptr.is_null() {
            bail!(io::Error::last_os_error())
        }

        let memory = std::ptr::slice_from_raw_parts_mut(ptr.cast(), size);
        let memory = SendSyncPtr::new(NonNull::new(memory).unwrap());
        Ok(Self {
            memory,
            is_file: false,
        })
    }

    pub fn reserve(size: usize) -> Result<Self> {
        let ptr = unsafe { VirtualAlloc(ptr::null_mut(), size, MEM_RESERVE, PAGE_NOACCESS) };
        if ptr.is_null() {
            bail!(io::Error::last_os_error())
        }
        let memory = std::ptr::slice_from_raw_parts_mut(ptr.cast(), size);
        let memory = SendSyncPtr::new(NonNull::new(memory).unwrap());
        Ok(Self {
            memory,
            is_file: false,
        })
    }

    pub fn from_file(path: &Path) -> Result<(Self, File)> {
        unsafe {
            // Open the file with read/execute access and only share for
            // read. This will enable us to perform the proper mmap below
            // while also disallowing other processes modifying the file
            // and having those modifications show up in our address space.
            let file = OpenOptions::new()
                .read(true)
                .access_mode(FILE_GENERIC_READ | FILE_GENERIC_EXECUTE)
                .share_mode(FILE_SHARE_READ)
                .open(path)
                .err2anyhow()
                .context("failed to open file")?;

            let len = file
                .metadata()
                .err2anyhow()
                .context("failed to get file metadata")?
                .len();
            let len = usize::try_from(len).map_err(|_| anyhow!("file too large to map"))?;

            // Create a file mapping that allows PAGE_EXECUTE_WRITECOPY.
            // This enables up-to these permissions but we won't leave all
            // of these permissions active at all times. Execution is
            // necessary for the generated code from Cranelift and the
            // WRITECOPY part is needed for possibly resolving relocations,
            // but otherwise writes don't happen.
            let mapping = CreateFileMappingW(
                file.as_raw_handle() as isize,
                ptr::null_mut(),
                PAGE_EXECUTE_WRITECOPY,
                0,
                0,
                ptr::null(),
            );
            if mapping == 0 {
                return Err(io::Error::last_os_error().into_anyhow())
                    .context("failed to create file mapping");
            }

            // Create a view for the entire file using all our requisite
            // permissions so that we can change the virtual permissions
            // later on.
            let ptr = MapViewOfFile(
                mapping,
                FILE_MAP_READ | FILE_MAP_EXECUTE | FILE_MAP_COPY,
                0,
                0,
                len,
            )
            .Value;
            let err = io::Error::last_os_error();
            CloseHandle(mapping);
            if ptr.is_null() {
                return Err(err.into_anyhow())
                    .context(format!("failed to create map view of {:#x} bytes", len));
            }

            let memory = std::ptr::slice_from_raw_parts_mut(ptr.cast(), len);
            let memory = SendSyncPtr::new(NonNull::new(memory).unwrap());
            let mut ret = Self {
                memory,
                is_file: true,
            };

            // Protect the entire file as PAGE_WRITECOPY to start (i.e.
            // remove the execute bit)
            let mut old = 0;
            if VirtualProtect(ret.as_mut_ptr().cast(), ret.len(), PAGE_WRITECOPY, &mut old) == 0 {
                return Err(io::Error::last_os_error().into_anyhow())
                    .context("failed change pages to `PAGE_READONLY`");
            }

            Ok((ret, file))
        }
    }

    pub fn make_accessible(&mut self, start: usize, len: usize) -> Result<()> {
        if unsafe {
            VirtualAlloc(
                self.as_ptr().add(start) as _,
                len,
                MEM_COMMIT,
                PAGE_READWRITE,
            )
        }
        .is_null()
        {
            bail!(io::Error::last_os_error())
        }

        Ok(())
    }

    #[inline]
    pub fn as_ptr(&self) -> *const u8 {
        self.memory.as_ptr() as *const u8
    }

    #[inline]
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.memory.as_ptr().cast()
    }

    #[inline]
    pub fn len(&self) -> usize {
        unsafe { (*self.memory.as_ptr()).len() }
    }

    pub unsafe fn make_executable(
        &self,
        range: Range<usize>,
        enable_branch_protection: bool,
    ) -> Result<()> {
        let flags = if enable_branch_protection {
            // TODO: We use this check to avoid an unused variable warning,
            // but some of the CFG-related flags might be applicable
            PAGE_EXECUTE_READ
        } else {
            PAGE_EXECUTE_READ
        };
        let mut old = 0;
        let base = self.as_ptr().add(range.start);
        let result = VirtualProtect(base as _, range.end - range.start, flags, &mut old);
        if result == 0 {
            bail!(io::Error::last_os_error());
        }
        Ok(())
    }

    pub unsafe fn make_readonly(&self, range: Range<usize>) -> Result<()> {
        let mut old = 0;
        let base = self.as_ptr().add(range.start);
        let result = VirtualProtect(base as _, range.end - range.start, PAGE_READONLY, &mut old);
        if result == 0 {
            bail!(io::Error::last_os_error());
        }
        Ok(())
    }
}

impl Drop for Mmap {
    fn drop(&mut self) {
        if self.len() == 0 {
            return;
        }

        if self.is_file {
            let r = unsafe {
                UnmapViewOfFile(MEMORY_MAPPED_VIEW_ADDRESS {
                    Value: self.as_mut_ptr().cast(),
                })
            };
            assert_ne!(r, 0);
        } else {
            let r = unsafe { VirtualFree(self.as_mut_ptr().cast(), 0, MEM_RELEASE) };
            assert_ne!(r, 0);
        }
    }
}
