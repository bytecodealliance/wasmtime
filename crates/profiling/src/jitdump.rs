//! Support for jitdump files which can be used by perf for profiling jitted code.
//! Spec definitions for the output format is as described here:
//! https://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git/tree/tools/perf/Documentation/jitdump-specification.txt
//!
//! Usage Example:
//!     Record
//!         sudo perf record -k 1 -e instructions:u target/debug/wasmtime -g --jitdump test.wasm
//!     Combine
//!         sudo perf inject -v -j -i perf.data -o perf.jit.data
//!     Report
//!         sudo perf report -i perf.jit.data -F+period,srcline
//! Note: For descriptive results, the WASM file being executed should contain dwarf debug data
use libc::c_int;
#[cfg(not(target_os = "windows"))]
use libc::{c_void, clock_gettime, mmap, open, sysconf};
use object::Object;
use scroll::{IOwrite, SizeWith, NATIVE};
use serde::{Deserialize, Serialize};
use std::error::Error;
#[cfg(not(target_os = "windows"))]
use std::ffi::CString;
use std::fmt::Debug;
use std::fs::File;
use std::io::Write;
#[cfg(not(target_os = "windows"))]
use std::os::unix::io::FromRawFd;
use std::{borrow, mem, process};
use target_lexicon::Architecture;

#[cfg(target_pointer_width = "64")]
use goblin::elf64 as elf;

#[cfg(target_pointer_width = "32")]
use goblin::elf32 as elf;

/// Defines jitdump record types
#[repr(u32)]
pub enum RecordId {
    /// Value 0: JIT_CODE_LOAD: record describing a jitted function
    JitCodeLoad = 0,
    /// Value 1: JIT_CODE_MOVE: record describing an already jitted function which is moved
    _JitCodeMove = 1,
    /// Value 2: JIT_CODE_DEBUG_INFO: record describing the debug information for a jitted function
    JitCodeDebugInfo = 2,
    /// Value 3: JIT_CODE_CLOSE: record marking the end of the jit runtime (optional)
    _JitCodeClose = 3,
    /// Value 4: JIT_CODE_UNWINDING_INFO: record describing a function unwinding information
    _JitCodeUnwindingInfo = 4,
}

/// Each record starts with this fixed size record header which describes the record that follows
#[derive(Serialize, Deserialize, Debug, Default, Clone, Copy, IOwrite, SizeWith)]
#[repr(C)]
pub struct RecordHeader {
    /// uint32_t id: a value identifying the record type (see below)
    id: u32,
    /// uint32_t total_size: the size in bytes of the record including the header.
    record_size: u32,
    /// uint64_t timestamp: a timestamp of when the record was created.
    timestamp: u64,
}

/// The CodeLoadRecord is used for describing jitted functions
#[derive(Serialize, Deserialize, Debug, Default, Clone, Copy, IOwrite, SizeWith)]
#[repr(C)]
pub struct CodeLoadRecord {
    /// Fixed sized header that describes this record
    header: RecordHeader,
    /// uint32_t pid: OS process id of the runtime generating the jitted code
    pid: u32,
    /// uint32_t tid: OS thread identification of the runtime thread generating the jitted code
    tid: u32,
    /// uint64_t vma: virtual address of jitted code start
    virtual_address: u64,
    /// uint64_t code_addr: code start address for the jitted code. By default vma = code_addr
    address: u64,
    /// uint64_t code_size: size in bytes of the generated jitted code
    size: u64,
    /// uint64_t code_index: unique identifier for the jitted code (see below)
    index: u64,
}

/// Describes source line information for a jitted function
#[derive(Serialize, Deserialize, Debug, Default)]
#[repr(C)]
pub struct DebugEntry {
    /// uint64_t code_addr: address of function for which the debug information is generated
    address: u64,
    /// uint32_t line: source file line number (starting at 1)
    line: u32,
    /// uint32_t discrim: column discriminator, 0 is default
    discriminator: u32,
    /// char name[n]: source file name in ASCII, including null termination
    filename: String,
}

/// Describes debug information for a jitted function. An array of debug entries are
/// appended to this record during writting. Note, this record must preceed the code
/// load record that describes the same jitted function.
#[derive(Serialize, Deserialize, Debug, Default, Clone, Copy, IOwrite, SizeWith)]
#[repr(C)]
pub struct DebugInfoRecord {
    /// Fixed sized header that describes this record
    header: RecordHeader,
    /// uint64_t code_addr: address of function for which the debug information is generated
    address: u64,
    /// uint64_t nr_entry: number of debug entries for the function appended to this record
    count: u64,
}

/// Fixed-sized header for each jitdump file
#[derive(Serialize, Deserialize, Debug, Default, IOwrite, SizeWith)]
#[repr(C)]
pub struct FileHeader {
    /// uint32_t magic: a magic number tagging the file type. The value is 4-byte long and represents the
    /// string "JiTD" in ASCII form. It is 0x4A695444 or 0x4454694a depending on the endianness. The field can
    /// be used to detect the endianness of the file
    magic: u32,
    /// uint32_t version: a 4-byte value representing the format version. It is currently set to 2
    version: u32,
    /// uint32_t total_size: size in bytes of file header
    size: u32,
    /// uint32_t elf_mach: ELF architecture encoding (ELF e_machine value as specified in /usr/include/elf.h)
    e_machine: u32,
    /// uint32_t pad1: padding. Reserved for future use
    pad1: u32,
    /// uint32_t pid: JIT runtime process identification (OS specific)
    pid: u32,
    /// uint64_t timestamp: timestamp of when the file was created
    timestamp: u64,
    /// uint64_t flags: a bitmask of flags
    flags: u64,
}

/// Interface for driving the creation of jitdump files
#[derive(Debug, Default)]
pub struct JitDumpAgent {
    /// File instance for the jit dump file
    pub jitdump_file: Option<File>,
    /// Unique identifier for jitted code
    pub code_index: u64,
    /// Flag for experimenting with dumping code load record
    /// after each function (true) or after each module. This
    /// flag is currently set to true.
    pub dump_funcs: bool,
}

impl JitDumpAgent {
    /// Intialize a JitDumpAgent and write out the header
    pub fn init(&mut self) -> Result<(), Box<dyn Error>> {
        #[cfg(target_os = "windows")]
        return Err("Target OS not supported.");
        #[cfg(not(target_os = "windows"))]
        {
            let filename = format!("./jit-{}.dump", process::id());
            let mut jitdump_file;
            unsafe {
                let filename_c = CString::new(filename)?;
                let fd = open(
                    filename_c.as_ptr(),
                    libc::O_CREAT | libc::O_TRUNC | libc::O_RDWR,
                    0666,
                );
                let pgsz = sysconf(libc::_SC_PAGESIZE) as usize;
                mmap(
                    0 as *mut c_void,
                    pgsz,
                    libc::PROT_EXEC | libc::PROT_READ,
                    libc::MAP_PRIVATE,
                    fd,
                    0,
                );
                jitdump_file = File::from_raw_fd(fd);
            }
            JitDumpAgent::write_file_header(&mut jitdump_file)?;
            self.jitdump_file = Some(jitdump_file);
            self.code_index = 0;
            self.dump_funcs = true;
            Ok(())
        }
    }

    /// Returns timestamp from a single source
    #[allow(unused_variables)]
    fn get_time_stamp(timestamp: &mut u64) -> c_int {
        #[cfg(not(target_os = "windows"))]
        {
            unsafe {
                let mut ts = mem::MaybeUninit::zeroed().assume_init();
                clock_gettime(libc::CLOCK_MONOTONIC, &mut ts);
                // TODO: What does it mean for either sec or nsec to be negative?
                *timestamp = (ts.tv_sec * 1000000000 + ts.tv_nsec) as u64;
            }
        }
        return 0;
    }

    /// Returns the ELF machine architecture.
    #[allow(dead_code)]
    fn get_e_machine() -> u32 {
        match target_lexicon::HOST.architecture {
            Architecture::X86_64 => elf::header::EM_X86_64 as u32,
            Architecture::I686 => elf::header::EM_386 as u32,
            Architecture::Arm => elf::header::EM_ARM as u32,
            Architecture::Armv4t => elf::header::EM_ARM as u32,
            Architecture::Armv5te => elf::header::EM_ARM as u32,
            Architecture::Armv7 => elf::header::EM_ARM as u32,
            Architecture::Armv7s => elf::header::EM_ARM as u32,
            Architecture::Aarch64 => elf::header::EM_AARCH64 as u32,
            _ => unimplemented!("unrecognized architecture"),
        }
    }

    #[allow(dead_code)]
    fn write_file_header(file: &mut File) -> Result<(), JitDumpError> {
        let mut header: FileHeader = Default::default();
        let mut timestamp: u64 = 0;
        JitDumpAgent::get_time_stamp(&mut timestamp);
        header.timestamp = timestamp;

        let e_machine = JitDumpAgent::get_e_machine();
        if e_machine != elf::header::EM_NONE as u32 {
            header.e_machine = e_machine;
        }

        if cfg!(target_endian = "little") {
            header.magic = 0x4A695444
        } else {
            header.magic = 0x4454694a
        }
        header.version = 1;
        header.size = mem::size_of::<FileHeader>() as u32;
        header.pad1 = 0;
        header.pid = process::id();
        header.flags = 0;

        file.iowrite_with(header, NATIVE)?;
        Ok(())
    }

    fn write_code_load_record(
        &mut self,
        record_name: &str,
        cl_record: CodeLoadRecord,
        code_buffer: &[u8],
    ) -> Result<(), JitDumpError> {
        let mut jitdump_file = self.jitdump_file.as_ref().unwrap();
        jitdump_file.iowrite_with(cl_record, NATIVE)?;
        jitdump_file.write_all(record_name.as_bytes())?;
        jitdump_file.write_all(b"\0")?;
        jitdump_file.write_all(code_buffer)?;
        Ok(())
    }

    /// Write DebugInfoRecord to open jit dump file.
    /// Must be written before the corresponding CodeLoadRecord.
    fn write_debug_info_record(&mut self, dir_record: DebugInfoRecord) -> Result<(), JitDumpError> {
        self.jitdump_file
            .as_ref()
            .unwrap()
            .iowrite_with(dir_record, NATIVE)?;
        Ok(())
    }

    /// Write DebugInfoRecord to open jit dump file.
    /// Must be written before the corresponding CodeLoadRecord.
    fn write_debug_info_entries(
        &mut self,
        die_entries: Vec<DebugEntry>,
    ) -> Result<(), JitDumpError> {
        for entry in die_entries.iter() {
            let mut jitdump_file = self.jitdump_file.as_ref().unwrap();
            jitdump_file.iowrite_with(entry.address, NATIVE)?;
            jitdump_file.iowrite_with(entry.line, NATIVE)?;
            jitdump_file.iowrite_with(entry.discriminator, NATIVE)?;
            jitdump_file.write_all(entry.filename.as_bytes())?;
            jitdump_file.write_all(b"\0")?;
        }
        Ok(())
    }

    /// Sent when a method is compiled and loaded into memory by the VM.
    pub fn module_load(
        &mut self,
        module_name: &str,
        addr: *const u8,
        len: usize,
        dbg_image: Option<&[u8]>,
    ) -> () {
        let pid = process::id();
        let tid = pid; // ThreadId does appear to track underlying thread. Using PID.

        if let Some(img) = &dbg_image {
            if let Err(err) = self.dump_from_debug_image(img, module_name, addr, len, pid, tid) {
                println!(
                    "Jitdump: module_load failed dumping from debug image: {:?}\n",
                    err
                );
            }
        } else {
            let mut timestamp: u64 = 0;
            JitDumpAgent::get_time_stamp(&mut timestamp);
            self.dump_code_load_record(module_name, addr, len, timestamp, pid, tid);
        }
    }

    fn dump_code_load_record(
        &mut self,
        method_name: &str,
        addr: *const u8,
        len: usize,
        timestamp: u64,
        pid: u32,
        tid: u32,
    ) -> () {
        let name_len = method_name.len() + 1;
        let size_limit = mem::size_of::<CodeLoadRecord>();

        let rh = RecordHeader {
            id: RecordId::JitCodeLoad as u32,
            record_size: size_limit as u32 + name_len as u32 + len as u32,
            timestamp: timestamp,
        };

        let clr = CodeLoadRecord {
            header: rh,
            pid: pid,
            tid: tid,
            virtual_address: addr as u64,
            address: addr as u64,
            size: len as u64,
            index: self.code_index,
        };
        self.code_index += 1;

        unsafe {
            let code_buffer: &[u8] = std::slice::from_raw_parts(addr, len);
            if let Err(err) = self.write_code_load_record(method_name, clr, code_buffer) {
                println!("Jitdump: write_code_load_failed_record failed: {:?}\n", err);
            }
        }
    }

    /// Attempts to dump debuginfo data structures, adding method and line level
    /// for the jitted function.
    pub fn dump_from_debug_image(
        &mut self,
        dbg_image: &[u8],
        module_name: &str,
        addr: *const u8,
        len: usize,
        pid: u32,
        tid: u32,
    ) -> Result<(), JitDumpError> {
        let file = object::File::parse(&dbg_image).unwrap();
        let endian = if file.is_little_endian() {
            gimli::RunTimeEndian::Little
        } else {
            gimli::RunTimeEndian::Big
        };

        let load_section = |id: gimli::SectionId| -> Result<borrow::Cow<[u8]>, JitDumpError> {
            Ok(file
                .section_data_by_name(id.name())
                .unwrap_or(borrow::Cow::Borrowed(&[][..])))
        };

        let load_section_sup = |_| Ok(borrow::Cow::Borrowed(&[][..]));
        let dwarf_cow = gimli::Dwarf::load(&load_section, &load_section_sup)?;
        let borrow_section: &dyn for<'a> Fn(
            &'a borrow::Cow<[u8]>,
        )
            -> gimli::EndianSlice<'a, gimli::RunTimeEndian> =
            &|section| gimli::EndianSlice::new(&*section, endian);

        let dwarf = dwarf_cow.borrow(&borrow_section);

        let mut iter = dwarf.units();
        while let Some(header) = iter.next()? {
            let unit = match dwarf.unit(header) {
                Ok(unit) => unit,
                Err(_err) => {
                    return Ok(());
                }
            };
            self.dump_entries(unit, &dwarf, module_name, addr, len, pid, tid)?;
            // TODO: Temp exit to avoid duplicate addresses being covered by only
            // processing the top unit
            break;
        }
        if !self.dump_funcs {
            let mut timestamp: u64 = 0;
            JitDumpAgent::get_time_stamp(&mut timestamp);
            self.dump_code_load_record(module_name, addr, len, timestamp, pid, tid);
        }
        Ok(())
    }

    fn dump_entries<R: Reader>(
        &mut self,
        unit: gimli::Unit<R>,
        dwarf: &gimli::Dwarf<R>,
        module_name: &str,
        addr: *const u8,
        len: usize,
        pid: u32,
        tid: u32,
    ) -> Result<(), JitDumpError> {
        let mut depth = 0;
        let mut entries = unit.entries();
        while let Some((delta_depth, entry)) = entries.next_dfs()? {
            if self.dump_funcs {
                let record_header = RecordHeader {
                    id: RecordId::JitCodeLoad as u32,
                    record_size: 0,
                    timestamp: 0,
                };

                let mut clr = CodeLoadRecord {
                    header: record_header,
                    pid: pid,
                    tid: tid,
                    virtual_address: 0,
                    address: 0,
                    size: 0,
                    index: 0,
                };
                let mut clr_name: String = String::from(module_name);
                let mut get_debug_entry = false;
                depth += delta_depth;
                assert!(depth >= 0);

                if entry.tag() == gimli::constants::DW_TAG_subprogram {
                    get_debug_entry = true;

                    let mut attrs = entry.attrs();
                    while let Some(attr) = attrs.next()? {
                        if let Some(n) = attr.name().static_string() {
                            if n == "DW_AT_low_pc" {
                                clr.address = match attr.value() {
                                    gimli::AttributeValue::Addr(address) => address,
                                    _ => 0,
                                };
                                clr.virtual_address = clr.address;
                            } else if n == "DW_AT_high_pc" {
                                clr.size = match attr.value() {
                                    gimli::AttributeValue::Udata(data) => data,
                                    _ => 0,
                                };
                            } else if n == "DW_AT_name" {
                                clr_name = match attr.value() {
                                    gimli::AttributeValue::DebugStrRef(offset) => {
                                        if let Ok(s) = dwarf.debug_str.get_str(offset) {
                                            clr_name.push_str("::");
                                            clr_name.push_str(&s.to_string_lossy()?);
                                            clr_name
                                        } else {
                                            clr_name.push_str("::");
                                            clr_name.push_str("?");
                                            clr_name
                                        }
                                    }
                                    _ => {
                                        clr_name.push_str("??");
                                        clr_name
                                    }
                                };
                            }
                        }
                    }
                }
                if get_debug_entry {
                    //  TODO: Temp check to make sure well only formed data is processed.
                    if clr.address == 0 {
                        continue;
                    }
                    //  TODO: Temp check to make sure well only formed data is processed.
                    if clr_name == "?" {
                        continue;
                    }
                    if clr.address == 0 || clr.size == 0 {
                        clr.address = addr as u64;
                        clr.virtual_address = addr as u64;
                        clr.size = len as u64;
                    }
                    clr.header.record_size = mem::size_of::<CodeLoadRecord>() as u32
                        + (clr_name.len() + 1) as u32
                        + clr.size as u32;
                    clr.index = self.code_index;
                    self.code_index += 1;
                    self.dump_debug_info(&unit, &dwarf, clr.address, clr.size, None)?;

                    let mut timestamp: u64 = 0;
                    JitDumpAgent::get_time_stamp(&mut timestamp);
                    clr.header.timestamp = timestamp;

                    unsafe {
                        let code_buffer: &[u8] =
                            std::slice::from_raw_parts(clr.address as *const u8, clr.size as usize);
                        let _ = self.write_code_load_record(&clr_name, clr, code_buffer);
                    }
                }
            } else {
                let mut func_name: String = String::from("?");
                let mut func_addr = 0;
                let mut func_size = 0;

                let mut get_debug_entry = false;
                depth += delta_depth;
                assert!(depth >= 0);
                if entry.tag() == gimli::constants::DW_TAG_subprogram {
                    get_debug_entry = true;

                    let mut attrs = entry.attrs();
                    while let Some(attr) = attrs.next()? {
                        if let Some(n) = attr.name().static_string() {
                            if n == "DW_AT_low_pc" {
                                func_addr = match attr.value() {
                                    gimli::AttributeValue::Addr(address) => address,
                                    _ => 0,
                                };
                            } else if n == "DW_AT_high_pc" {
                                func_size = match attr.value() {
                                    gimli::AttributeValue::Udata(data) => data,
                                    _ => 0,
                                };
                            } else if n == "DW_AT_name" {
                                func_name = match attr.value() {
                                    gimli::AttributeValue::DebugStrRef(offset) => {
                                        if let Ok(s) = dwarf.debug_str.get_str(offset) {
                                            func_name.clear();
                                            func_name.push_str(&s.to_string_lossy()?);
                                            func_name
                                        } else {
                                            func_name.push_str("?");
                                            func_name
                                        }
                                    }
                                    _ => {
                                        func_name.push_str("??");
                                        func_name
                                    }
                                };
                            }
                        }
                    }
                }
                if get_debug_entry {
                    //  TODO: Temp check to make sure well only formed data is processed.
                    if func_addr == 0 {
                        continue;
                    }
                    //  TODO: Temp check to make sure well only formed data is processed.
                    if func_name == "?" {
                        continue;
                    }
                    self.dump_debug_info(
                        &unit,
                        &dwarf,
                        func_addr,
                        func_size,
                        Some(func_name.as_str()),
                    )?;
                }
            }
        }
        Ok(())
    }

    fn dump_debug_info<R: Reader>(
        &mut self,
        unit: &gimli::Unit<R>,
        dwarf: &gimli::Dwarf<R>,
        address: u64,
        size: u64,
        file_suffix: Option<&str>,
    ) -> Result<(), JitDumpError> {
        let mut timestamp: u64 = 0;
        JitDumpAgent::get_time_stamp(&mut timestamp);
        if let Some(program) = unit.line_program.clone() {
            let mut debug_info_record = DebugInfoRecord {
                header: RecordHeader {
                    id: RecordId::JitCodeDebugInfo as u32,
                    record_size: 0,
                    timestamp: timestamp,
                },
                address: address,
                count: 0,
            };

            let mut debug_entries = Vec::new();
            let mut debug_entries_total_filenames_len = 0;
            let mut rows = program.rows();
            while let Some((header, row)) = rows.next_row()? {
                let row_file_index = row.file_index() - 1;
                let myfile = dwarf
                    .attr_string(
                        &unit,
                        header.file_names()[row_file_index as usize].path_name(),
                    )
                    .unwrap();
                let filename = myfile.to_string_lossy()?;
                let line = row.line().unwrap_or(0);
                let column = match row.column() {
                    gimli::ColumnType::Column(column) => column,
                    gimli::ColumnType::LeftEdge => 0,
                };

                if (row.address() < address) || (row.address() > (address + size)) {
                    continue;
                }
                let mut debug_entry = DebugEntry {
                    address: row.address(),
                    line: line as u32,
                    discriminator: column as u32,
                    filename: filename.to_string(),
                };

                if let Some(suffix) = file_suffix {
                    debug_entry.filename.push_str("::");
                    debug_entry.filename.push_str(suffix);
                }

                debug_entries_total_filenames_len += debug_entry.filename.len() + 1;
                debug_entries.push(debug_entry);
            }

            debug_info_record.count = debug_entries.len() as u64;

            let debug_entries_size = (debug_info_record.count
                * (mem::size_of::<DebugEntry>() as u64 - mem::size_of::<String>() as u64))
                + debug_entries_total_filenames_len as u64;
            debug_info_record.header.record_size =
                mem::size_of::<DebugInfoRecord>() as u32 + debug_entries_size as u32;

            let _ = self.write_debug_info_record(debug_info_record);
            let _ = self.write_debug_info_entries(debug_entries);
        }
        Ok(())
    }
}

use crate::ProfilingAgent;
impl ProfilingAgent for JitDumpAgent {
    fn module_load(
        &mut self,
        module_name: &str,
        addr: *const u8,
        len: usize,
        dbg_image: Option<&[u8]>,
    ) -> () {
        if self.jitdump_file.is_none() {
            if JitDumpAgent::init(self).ok().is_some() {
                JitDumpAgent::module_load(self, module_name, addr, len, dbg_image);
            } else {
                println!("Jitdump: Failed to initialize JitDumpAgent\n");
            }
        } else {
            JitDumpAgent::module_load(self, module_name, addr, len, dbg_image);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JitDumpError {
    GimliError(gimli::Error),
    IOError,
}

impl Error for JitDumpError {
    fn description(&self) -> &str {
        match *self {
            JitDumpError::GimliError(ref err) => err.description(),
            JitDumpError::IOError => "An I/O error occurred.",
        }
    }
}

impl std::fmt::Display for JitDumpError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::result::Result<(), std::fmt::Error> {
        Debug::fmt(self, f)
    }
}

impl From<gimli::Error> for JitDumpError {
    fn from(err: gimli::Error) -> Self {
        JitDumpError::GimliError(err)
    }
}

impl From<std::io::Error> for JitDumpError {
    fn from(_err: std::io::Error) -> Self {
        JitDumpError::IOError
    }
}

trait Reader: gimli::Reader<Offset = usize> + Send + Sync {}

impl<'input, Endian> Reader for gimli::EndianSlice<'input, Endian> where
    Endian: gimli::Endianity + Send + Sync
{
}
