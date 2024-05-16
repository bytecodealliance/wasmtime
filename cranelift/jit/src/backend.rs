//! Defines `JITModule`.

use crate::{compiled_blob::CompiledBlob, memory::BranchProtection, memory::Memory};
use cranelift_codegen::binemit::Reloc;
use cranelift_codegen::isa::{OwnedTargetIsa, TargetIsa};
use cranelift_codegen::settings::Configurable;
use cranelift_codegen::{ir, settings, FinalizedMachReloc};
use cranelift_control::ControlPlane;
use cranelift_entity::SecondaryMap;
use cranelift_module::{
    DataDescription, DataId, FuncId, Init, Linkage, Module, ModuleDeclarations, ModuleError,
    ModuleReloc, ModuleRelocTarget, ModuleResult,
};
use log::info;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::CString;
use std::io::Write;
use std::ptr;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicPtr, Ordering};
use target_lexicon::PointerWidth;

const WRITABLE_DATA_ALIGNMENT: u64 = 0x8;
const READONLY_DATA_ALIGNMENT: u64 = 0x1;

/// A builder for `JITModule`.
pub struct JITBuilder {
    isa: OwnedTargetIsa,
    symbols: HashMap<String, *const u8>,
    lookup_symbols: Vec<Box<dyn Fn(&str) -> Option<*const u8>>>,
    libcall_names: Box<dyn Fn(ir::LibCall) -> String + Send + Sync>,
    hotswap_enabled: bool,
}

impl JITBuilder {
    /// Create a new `JITBuilder`.
    ///
    /// The `libcall_names` function provides a way to translate `cranelift_codegen`'s `ir::LibCall`
    /// enum to symbols. LibCalls are inserted in the IR as part of the legalization for certain
    /// floating point instructions, and for stack probes. If you don't know what to use for this
    /// argument, use `cranelift_module::default_libcall_names()`.
    pub fn new(
        libcall_names: Box<dyn Fn(ir::LibCall) -> String + Send + Sync>,
    ) -> ModuleResult<Self> {
        Self::with_flags(&[], libcall_names)
    }

    /// Create a new `JITBuilder` with the given flags.
    ///
    /// The `libcall_names` function provides a way to translate `cranelift_codegen`'s `ir::LibCall`
    /// enum to symbols. LibCalls are inserted in the IR as part of the legalization for certain
    /// floating point instructions, and for stack probes. If you don't know what to use for this
    /// argument, use `cranelift_module::default_libcall_names()`.
    pub fn with_flags(
        flags: &[(&str, &str)],
        libcall_names: Box<dyn Fn(ir::LibCall) -> String + Send + Sync>,
    ) -> ModuleResult<Self> {
        let mut flag_builder = settings::builder();
        for (name, value) in flags {
            flag_builder.set(name, value)?;
        }

        // On at least AArch64, "colocated" calls use shorter-range relocations,
        // which might not reach all definitions; we can't handle that here, so
        // we require long-range relocation types.
        flag_builder.set("use_colocated_libcalls", "false").unwrap();
        flag_builder.set("is_pic", "true").unwrap();
        let isa_builder = cranelift_native::builder().unwrap_or_else(|msg| {
            panic!("host machine is not supported: {}", msg);
        });
        let isa = isa_builder.finish(settings::Flags::new(flag_builder))?;
        Ok(Self::with_isa(isa, libcall_names))
    }

    /// Create a new `JITBuilder` with an arbitrary target. This is mainly
    /// useful for testing.
    ///
    /// To create a `JITBuilder` for native use, use the `new` or `with_flags`
    /// constructors instead.
    ///
    /// The `libcall_names` function provides a way to translate `cranelift_codegen`'s `ir::LibCall`
    /// enum to symbols. LibCalls are inserted in the IR as part of the legalization for certain
    /// floating point instructions, and for stack probes. If you don't know what to use for this
    /// argument, use `cranelift_module::default_libcall_names()`.
    pub fn with_isa(
        isa: OwnedTargetIsa,
        libcall_names: Box<dyn Fn(ir::LibCall) -> String + Send + Sync>,
    ) -> Self {
        let symbols = HashMap::new();
        let lookup_symbols = vec![Box::new(lookup_with_dlsym) as Box<_>];
        Self {
            isa,
            symbols,
            lookup_symbols,
            libcall_names,
            hotswap_enabled: false,
        }
    }

    /// Define a symbol in the internal symbol table.
    ///
    /// The JIT will use the symbol table to resolve names that are declared,
    /// but not defined, in the module being compiled.  A common example is
    /// external functions.  With this method, functions and data can be exposed
    /// to the code being compiled which are defined by the host.
    ///
    /// If a symbol is defined more than once, the most recent definition will
    /// be retained.
    ///
    /// If the JIT fails to find a symbol in its internal table, it will fall
    /// back to a platform-specific search (this typically involves searching
    /// the current process for public symbols, followed by searching the
    /// platform's C runtime).
    pub fn symbol<K>(&mut self, name: K, ptr: *const u8) -> &mut Self
    where
        K: Into<String>,
    {
        self.symbols.insert(name.into(), ptr);
        self
    }

    /// Define multiple symbols in the internal symbol table.
    ///
    /// Using this is equivalent to calling `symbol` on each element.
    pub fn symbols<It, K>(&mut self, symbols: It) -> &mut Self
    where
        It: IntoIterator<Item = (K, *const u8)>,
        K: Into<String>,
    {
        for (name, ptr) in symbols {
            self.symbols.insert(name.into(), ptr);
        }
        self
    }

    /// Add a symbol lookup fn.
    ///
    /// Symbol lookup fn's are used to lookup symbols when they couldn't be found in the internal
    /// symbol table. Symbol lookup fn's are called in reverse of the order in which they were added.
    pub fn symbol_lookup_fn(
        &mut self,
        symbol_lookup_fn: Box<dyn Fn(&str) -> Option<*const u8>>,
    ) -> &mut Self {
        self.lookup_symbols.push(symbol_lookup_fn);
        self
    }

    /// Enable or disable hotswap support. See [`JITModule::prepare_for_function_redefine`]
    /// for more information.
    ///
    /// Enabling hotswap support requires PIC code.
    pub fn hotswap(&mut self, enabled: bool) -> &mut Self {
        self.hotswap_enabled = enabled;
        self
    }
}

/// A pending update to the GOT.
struct GotUpdate {
    /// The entry that is to be updated.
    entry: NonNull<AtomicPtr<u8>>,

    /// The new value of the entry.
    ptr: *const u8,
}

/// A `JITModule` implements `Module` and emits code and data into memory where it can be
/// directly called and accessed.
///
/// See the `JITBuilder` for a convenient way to construct `JITModule` instances.
pub struct JITModule {
    isa: OwnedTargetIsa,
    hotswap_enabled: bool,
    symbols: RefCell<HashMap<String, *const u8>>,
    lookup_symbols: Vec<Box<dyn Fn(&str) -> Option<*const u8>>>,
    libcall_names: Box<dyn Fn(ir::LibCall) -> String>,
    memory: MemoryHandle,
    declarations: ModuleDeclarations,
    function_got_entries: SecondaryMap<FuncId, Option<NonNull<AtomicPtr<u8>>>>,
    function_plt_entries: SecondaryMap<FuncId, Option<NonNull<[u8; 16]>>>,
    data_object_got_entries: SecondaryMap<DataId, Option<NonNull<AtomicPtr<u8>>>>,
    libcall_got_entries: HashMap<ir::LibCall, NonNull<AtomicPtr<u8>>>,
    libcall_plt_entries: HashMap<ir::LibCall, NonNull<[u8; 16]>>,
    compiled_functions: SecondaryMap<FuncId, Option<CompiledBlob>>,
    compiled_data_objects: SecondaryMap<DataId, Option<CompiledBlob>>,
    functions_to_finalize: Vec<FuncId>,
    data_objects_to_finalize: Vec<DataId>,

    /// Updates to the GOT awaiting relocations to be made and region protections to be set
    pending_got_updates: Vec<GotUpdate>,
}

/// A handle to allow freeing memory allocated by the `Module`.
struct MemoryHandle {
    code: Memory,
    readonly: Memory,
    writable: Memory,
}

impl JITModule {
    /// Free memory allocated for code and data segments of compiled functions.
    ///
    /// # Safety
    ///
    /// Because this function invalidates any pointers retrieved from the
    /// corresponding module, it should only be used when none of the functions
    /// from that module are currently executing and none of the `fn` pointers
    /// are called afterwards.
    pub unsafe fn free_memory(mut self) {
        self.memory.code.free_memory();
        self.memory.readonly.free_memory();
        self.memory.writable.free_memory();
    }

    fn lookup_symbol(&self, name: &str) -> Option<*const u8> {
        match self.symbols.borrow_mut().entry(name.to_owned()) {
            std::collections::hash_map::Entry::Occupied(occ) => Some(*occ.get()),
            std::collections::hash_map::Entry::Vacant(vac) => {
                let ptr = self
                    .lookup_symbols
                    .iter()
                    .rev() // Try last lookup function first
                    .find_map(|lookup| lookup(name));
                if let Some(ptr) = ptr {
                    vac.insert(ptr);
                }
                ptr
            }
        }
    }

    fn new_got_entry(&mut self, val: *const u8) -> NonNull<AtomicPtr<u8>> {
        let got_entry = self
            .memory
            .writable
            .allocate(
                std::mem::size_of::<AtomicPtr<u8>>(),
                std::mem::align_of::<AtomicPtr<u8>>().try_into().unwrap(),
            )
            .unwrap()
            .cast::<AtomicPtr<u8>>();
        unsafe {
            std::ptr::write(got_entry, AtomicPtr::new(val as *mut _));
        }
        NonNull::new(got_entry).unwrap()
    }

    fn new_plt_entry(&mut self, got_entry: NonNull<AtomicPtr<u8>>) -> NonNull<[u8; 16]> {
        let plt_entry = self
            .memory
            .code
            .allocate(
                std::mem::size_of::<[u8; 16]>(),
                self.isa
                    .symbol_alignment()
                    .max(self.isa.function_alignment().minimum as u64),
            )
            .unwrap()
            .cast::<[u8; 16]>();
        unsafe {
            Self::write_plt_entry_bytes(plt_entry, got_entry);
        }
        NonNull::new(plt_entry).unwrap()
    }

    fn new_func_plt_entry(&mut self, id: FuncId, val: *const u8) {
        let got_entry = self.new_got_entry(val);
        self.function_got_entries[id] = Some(got_entry);
        let plt_entry = self.new_plt_entry(got_entry);
        self.record_function_for_perf(
            plt_entry.as_ptr().cast(),
            std::mem::size_of::<[u8; 16]>(),
            &format!(
                "{}@plt",
                self.declarations.get_function_decl(id).linkage_name(id)
            ),
        );
        self.function_plt_entries[id] = Some(plt_entry);
    }

    fn new_data_got_entry(&mut self, id: DataId, val: *const u8) {
        let got_entry = self.new_got_entry(val);
        self.data_object_got_entries[id] = Some(got_entry);
    }

    unsafe fn write_plt_entry_bytes(plt_ptr: *mut [u8; 16], got_ptr: NonNull<AtomicPtr<u8>>) {
        assert!(
            cfg!(target_arch = "x86_64"),
            "PLT is currently only supported on x86_64"
        );
        // jmp *got_ptr; ud2; ud2; ud2; ud2; ud2
        let mut plt_val = [
            0xff, 0x25, 0, 0, 0, 0, 0x0f, 0x0b, 0x0f, 0x0b, 0x0f, 0x0b, 0x0f, 0x0b, 0x0f, 0x0b,
        ];
        let what = got_ptr.as_ptr() as isize - 4;
        let at = plt_ptr as isize + 2;
        plt_val[2..6].copy_from_slice(&i32::to_ne_bytes(i32::try_from(what - at).unwrap()));
        std::ptr::write(plt_ptr, plt_val);
    }

    fn get_address(&self, name: &ModuleRelocTarget) -> *const u8 {
        match *name {
            ModuleRelocTarget::User { .. } => {
                let (name, linkage) = if ModuleDeclarations::is_function(name) {
                    if self.hotswap_enabled {
                        return self.get_plt_address(name);
                    } else {
                        let func_id = FuncId::from_name(name);
                        match &self.compiled_functions[func_id] {
                            Some(compiled) => return compiled.ptr,
                            None => {
                                let decl = self.declarations.get_function_decl(func_id);
                                (&decl.name, decl.linkage)
                            }
                        }
                    }
                } else {
                    let data_id = DataId::from_name(name);
                    match &self.compiled_data_objects[data_id] {
                        Some(compiled) => return compiled.ptr,
                        None => {
                            let decl = self.declarations.get_data_decl(data_id);
                            (&decl.name, decl.linkage)
                        }
                    }
                };
                let name = name
                    .as_ref()
                    .expect("anonymous symbol must be defined locally");
                if let Some(ptr) = self.lookup_symbol(name) {
                    ptr
                } else if linkage == Linkage::Preemptible {
                    0 as *const u8
                } else {
                    panic!("can't resolve symbol {}", name);
                }
            }
            ModuleRelocTarget::LibCall(ref libcall) => {
                let sym = (self.libcall_names)(*libcall);
                self.lookup_symbol(&sym)
                    .unwrap_or_else(|| panic!("can't resolve libcall {}", sym))
            }
            _ => panic!("invalid name"),
        }
    }

    /// Returns the given function's entry in the Global Offset Table.
    ///
    /// Panics if there's no entry in the table for the given function.
    pub fn read_got_entry(&self, func_id: FuncId) -> *const u8 {
        let got_entry = self.function_got_entries[func_id].unwrap();
        unsafe { got_entry.as_ref() }.load(Ordering::SeqCst)
    }

    fn get_got_address(&self, name: &ModuleRelocTarget) -> NonNull<AtomicPtr<u8>> {
        match *name {
            ModuleRelocTarget::User { .. } => {
                if ModuleDeclarations::is_function(name) {
                    let func_id = FuncId::from_name(name);
                    self.function_got_entries[func_id].unwrap()
                } else {
                    let data_id = DataId::from_name(name);
                    self.data_object_got_entries[data_id].unwrap()
                }
            }
            ModuleRelocTarget::LibCall(ref libcall) => *self
                .libcall_got_entries
                .get(libcall)
                .unwrap_or_else(|| panic!("can't resolve libcall {}", libcall)),
            _ => panic!("invalid name"),
        }
    }

    fn get_plt_address(&self, name: &ModuleRelocTarget) -> *const u8 {
        match *name {
            ModuleRelocTarget::User { .. } => {
                if ModuleDeclarations::is_function(name) {
                    let func_id = FuncId::from_name(name);
                    self.function_plt_entries[func_id]
                        .unwrap()
                        .as_ptr()
                        .cast::<u8>()
                } else {
                    unreachable!("PLT relocations can only have functions as target");
                }
            }
            ModuleRelocTarget::LibCall(ref libcall) => self
                .libcall_plt_entries
                .get(libcall)
                .unwrap_or_else(|| panic!("can't resolve libcall {}", libcall))
                .as_ptr()
                .cast::<u8>(),
            _ => panic!("invalid name"),
        }
    }

    /// Returns the address of a finalized function.
    ///
    /// The pointer remains valid until either [`JITModule::free_memory`] is called or in the future
    /// some way of deallocating this individual function is used.
    pub fn get_finalized_function(&self, func_id: FuncId) -> *const u8 {
        let info = &self.compiled_functions[func_id];
        assert!(
            !self.functions_to_finalize.iter().any(|x| *x == func_id),
            "function not yet finalized"
        );
        info.as_ref()
            .expect("function must be compiled before it can be finalized")
            .ptr
    }

    /// Returns the address and size of a finalized data object.
    ///
    /// The pointer remains valid until either [`JITModule::free_memory`] is called or in the future
    /// some way of deallocating this individual data object is used.
    pub fn get_finalized_data(&self, data_id: DataId) -> (*const u8, usize) {
        let info = &self.compiled_data_objects[data_id];
        assert!(
            !self.data_objects_to_finalize.iter().any(|x| *x == data_id),
            "data object not yet finalized"
        );
        let compiled = info
            .as_ref()
            .expect("data object must be compiled before it can be finalized");

        (compiled.ptr, compiled.size)
    }

    fn record_function_for_perf(&self, ptr: *mut u8, size: usize, name: &str) {
        // The Linux perf tool supports JIT code via a /tmp/perf-$PID.map file,
        // which contains memory regions and their associated names.  If we
        // are profiling with perf and saving binaries to PERF_BUILDID_DIR
        // for post-profile analysis, write information about each function
        // we define.
        if cfg!(unix) && ::std::env::var_os("PERF_BUILDID_DIR").is_some() {
            let mut map_file = ::std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(format!("/tmp/perf-{}.map", ::std::process::id()))
                .unwrap();

            let _ = writeln!(map_file, "{:x} {:x} {}", ptr as usize, size, name);
        }
    }

    /// Finalize all functions and data objects that are defined but not yet finalized.
    /// All symbols referenced in their bodies that are declared as needing a definition
    /// must be defined by this point.
    ///
    /// Use `get_finalized_function` and `get_finalized_data` to obtain the final
    /// artifacts.
    ///
    /// Returns ModuleError in case of allocation or syscall failure
    pub fn finalize_definitions(&mut self) -> ModuleResult<()> {
        for func in std::mem::take(&mut self.functions_to_finalize) {
            let decl = self.declarations.get_function_decl(func);
            assert!(decl.linkage.is_definable());
            let func = self.compiled_functions[func]
                .as_ref()
                .expect("function must be compiled before it can be finalized");
            func.perform_relocations(
                |name| self.get_address(name),
                |name| self.get_got_address(name).as_ptr().cast(),
                |name| self.get_plt_address(name),
            );
        }

        for data in std::mem::take(&mut self.data_objects_to_finalize) {
            let decl = self.declarations.get_data_decl(data);
            assert!(decl.linkage.is_definable());
            let data = self.compiled_data_objects[data]
                .as_ref()
                .expect("data object must be compiled before it can be finalized");
            data.perform_relocations(
                |name| self.get_address(name),
                |name| self.get_got_address(name).as_ptr().cast(),
                |name| self.get_plt_address(name),
            );
        }

        // Now that we're done patching, prepare the memory for execution!
        self.memory.readonly.set_readonly()?;
        self.memory.code.set_readable_and_executable()?;

        for update in self.pending_got_updates.drain(..) {
            unsafe { update.entry.as_ref() }.store(update.ptr as *mut _, Ordering::SeqCst);
        }
        Ok(())
    }

    /// Create a new `JITModule`.
    pub fn new(builder: JITBuilder) -> Self {
        if builder.hotswap_enabled {
            assert!(
                builder.isa.flags().is_pic(),
                "Hotswapping requires PIC code"
            );
        }

        let branch_protection =
            if cfg!(target_arch = "aarch64") && use_bti(&builder.isa.isa_flags()) {
                BranchProtection::BTI
            } else {
                BranchProtection::None
            };
        let mut module = Self {
            isa: builder.isa,
            hotswap_enabled: builder.hotswap_enabled,
            symbols: RefCell::new(builder.symbols),
            lookup_symbols: builder.lookup_symbols,
            libcall_names: builder.libcall_names,
            memory: MemoryHandle {
                code: Memory::new(branch_protection),
                // Branch protection is not applicable to non-executable memory.
                readonly: Memory::new(BranchProtection::None),
                writable: Memory::new(BranchProtection::None),
            },
            declarations: ModuleDeclarations::default(),
            function_got_entries: SecondaryMap::new(),
            function_plt_entries: SecondaryMap::new(),
            data_object_got_entries: SecondaryMap::new(),
            libcall_got_entries: HashMap::new(),
            libcall_plt_entries: HashMap::new(),
            compiled_functions: SecondaryMap::new(),
            compiled_data_objects: SecondaryMap::new(),
            functions_to_finalize: Vec::new(),
            data_objects_to_finalize: Vec::new(),
            pending_got_updates: Vec::new(),
        };

        // Pre-create a GOT and PLT entry for each libcall.
        let all_libcalls = if module.isa.flags().is_pic() {
            ir::LibCall::all_libcalls()
        } else {
            &[] // Not PIC, so no GOT and PLT entries necessary
        };
        for &libcall in all_libcalls {
            let sym = (module.libcall_names)(libcall);
            let addr = if let Some(addr) = module.lookup_symbol(&sym) {
                addr
            } else {
                continue;
            };
            let got_entry = module.new_got_entry(addr);
            module.libcall_got_entries.insert(libcall, got_entry);
            let plt_entry = module.new_plt_entry(got_entry);
            module.libcall_plt_entries.insert(libcall, plt_entry);
        }

        module
    }

    /// Allow a single future `define_function` on a previously defined function. This allows for
    /// hot code swapping and lazy compilation of functions.
    ///
    /// This requires hotswap support to be enabled first using [`JITBuilder::hotswap`].
    pub fn prepare_for_function_redefine(&mut self, func_id: FuncId) -> ModuleResult<()> {
        assert!(self.hotswap_enabled, "Hotswap support is not enabled");
        let decl = self.declarations.get_function_decl(func_id);
        if !decl.linkage.is_definable() {
            return Err(ModuleError::InvalidImportDefinition(
                decl.linkage_name(func_id).into_owned(),
            ));
        }

        if self.compiled_functions[func_id].is_none() {
            return Err(ModuleError::Backend(anyhow::anyhow!(
                "Tried to redefine not yet defined function {}",
                decl.linkage_name(func_id),
            )));
        }

        self.compiled_functions[func_id] = None;

        // FIXME return some kind of handle that allows for deallocating the function

        Ok(())
    }
}

impl Module for JITModule {
    fn isa(&self) -> &dyn TargetIsa {
        &*self.isa
    }

    fn declarations(&self) -> &ModuleDeclarations {
        &self.declarations
    }

    fn declare_function(
        &mut self,
        name: &str,
        linkage: Linkage,
        signature: &ir::Signature,
    ) -> ModuleResult<FuncId> {
        let (id, linkage) = self
            .declarations
            .declare_function(name, linkage, signature)?;
        if self.function_got_entries[id].is_none() && self.isa.flags().is_pic() {
            // FIXME populate got entries with a null pointer when defined
            let val = if linkage == Linkage::Import {
                self.lookup_symbol(name).unwrap_or(std::ptr::null())
            } else {
                std::ptr::null()
            };
            self.new_func_plt_entry(id, val);
        }
        Ok(id)
    }

    fn declare_anonymous_function(&mut self, signature: &ir::Signature) -> ModuleResult<FuncId> {
        let id = self.declarations.declare_anonymous_function(signature)?;
        if self.isa.flags().is_pic() {
            self.new_func_plt_entry(id, std::ptr::null());
        }
        Ok(id)
    }

    fn declare_data(
        &mut self,
        name: &str,
        linkage: Linkage,
        writable: bool,
        tls: bool,
    ) -> ModuleResult<DataId> {
        assert!(!tls, "JIT doesn't yet support TLS");
        let (id, linkage) = self
            .declarations
            .declare_data(name, linkage, writable, tls)?;
        if self.data_object_got_entries[id].is_none() && self.isa.flags().is_pic() {
            // FIXME populate got entries with a null pointer when defined
            let val = if linkage == Linkage::Import {
                self.lookup_symbol(name).unwrap_or(std::ptr::null())
            } else {
                std::ptr::null()
            };
            self.new_data_got_entry(id, val);
        }
        Ok(id)
    }

    fn declare_anonymous_data(&mut self, writable: bool, tls: bool) -> ModuleResult<DataId> {
        assert!(!tls, "JIT doesn't yet support TLS");
        let id = self.declarations.declare_anonymous_data(writable, tls)?;
        if self.isa.flags().is_pic() {
            self.new_data_got_entry(id, std::ptr::null());
        }
        Ok(id)
    }

    fn define_function_with_control_plane(
        &mut self,
        id: FuncId,
        ctx: &mut cranelift_codegen::Context,
        ctrl_plane: &mut ControlPlane,
    ) -> ModuleResult<()> {
        info!("defining function {}: {}", id, ctx.func.display());
        let decl = self.declarations.get_function_decl(id);
        if !decl.linkage.is_definable() {
            return Err(ModuleError::InvalidImportDefinition(
                decl.linkage_name(id).into_owned(),
            ));
        }

        if !self.compiled_functions[id].is_none() {
            return Err(ModuleError::DuplicateDefinition(
                decl.linkage_name(id).into_owned(),
            ));
        }

        if self.hotswap_enabled {
            // Disable colocated if hotswapping is enabled to avoid a PLT indirection in case of
            // calls and to allow data objects to be hotswapped in the future.
            for func in ctx.func.dfg.ext_funcs.values_mut() {
                func.colocated = false;
            }

            for gv in ctx.func.global_values.values_mut() {
                match gv {
                    ir::GlobalValueData::Symbol { colocated, .. } => *colocated = false,
                    _ => {}
                }
            }
        }

        // work around borrow-checker to allow reuse of ctx below
        let res = ctx.compile(self.isa(), ctrl_plane)?;
        let alignment = res.buffer.alignment as u64;
        let compiled_code = ctx.compiled_code().unwrap();

        let size = compiled_code.code_info().total_size as usize;
        let align = alignment
            .max(self.isa.function_alignment().minimum as u64)
            .max(self.isa.symbol_alignment());
        let ptr = self
            .memory
            .code
            .allocate(size, align)
            .map_err(|e| ModuleError::Allocation {
                message: "unable to alloc function",
                err: e,
            })?;

        {
            let mem = unsafe { std::slice::from_raw_parts_mut(ptr, size) };
            mem.copy_from_slice(compiled_code.code_buffer());
        }

        let relocs = compiled_code
            .buffer
            .relocs()
            .iter()
            .map(|reloc| ModuleReloc::from_mach_reloc(reloc, &ctx.func, id))
            .collect();

        self.record_function_for_perf(ptr, size, &decl.linkage_name(id));
        self.compiled_functions[id] = Some(CompiledBlob { ptr, size, relocs });

        if self.isa.flags().is_pic() {
            self.pending_got_updates.push(GotUpdate {
                entry: self.function_got_entries[id].unwrap(),
                ptr,
            })
        }

        if self.hotswap_enabled {
            self.compiled_functions[id]
                .as_ref()
                .unwrap()
                .perform_relocations(
                    |name| match *name {
                        ModuleRelocTarget::User { .. } => {
                            unreachable!("non GOT or PLT relocation in function {} to {}", id, name)
                        }
                        ModuleRelocTarget::LibCall(ref libcall) => self
                            .libcall_plt_entries
                            .get(libcall)
                            .unwrap_or_else(|| panic!("can't resolve libcall {}", libcall))
                            .as_ptr()
                            .cast::<u8>(),
                        _ => panic!("invalid name"),
                    },
                    |name| self.get_got_address(name).as_ptr().cast(),
                    |name| self.get_plt_address(name),
                );
        } else {
            self.functions_to_finalize.push(id);
        }

        Ok(())
    }

    fn define_function_bytes(
        &mut self,
        id: FuncId,
        func: &ir::Function,
        alignment: u64,
        bytes: &[u8],
        relocs: &[FinalizedMachReloc],
    ) -> ModuleResult<()> {
        info!("defining function {} with bytes", id);
        let decl = self.declarations.get_function_decl(id);
        if !decl.linkage.is_definable() {
            return Err(ModuleError::InvalidImportDefinition(
                decl.linkage_name(id).into_owned(),
            ));
        }

        if !self.compiled_functions[id].is_none() {
            return Err(ModuleError::DuplicateDefinition(
                decl.linkage_name(id).into_owned(),
            ));
        }

        let size = bytes.len();
        let align = alignment
            .max(self.isa.function_alignment().minimum as u64)
            .max(self.isa.symbol_alignment());
        let ptr = self
            .memory
            .code
            .allocate(size, align)
            .map_err(|e| ModuleError::Allocation {
                message: "unable to alloc function bytes",
                err: e,
            })?;

        unsafe {
            ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, size);
        }

        self.record_function_for_perf(ptr, size, &decl.linkage_name(id));
        self.compiled_functions[id] = Some(CompiledBlob {
            ptr,
            size,
            relocs: relocs
                .iter()
                .map(|reloc| ModuleReloc::from_mach_reloc(reloc, func, id))
                .collect(),
        });

        if self.isa.flags().is_pic() {
            self.pending_got_updates.push(GotUpdate {
                entry: self.function_got_entries[id].unwrap(),
                ptr,
            })
        }

        if self.hotswap_enabled {
            self.compiled_functions[id]
                .as_ref()
                .unwrap()
                .perform_relocations(
                    |name| unreachable!("non GOT or PLT relocation in function {} to {}", id, name),
                    |name| self.get_got_address(name).as_ptr().cast(),
                    |name| self.get_plt_address(name),
                );
        } else {
            self.functions_to_finalize.push(id);
        }

        Ok(())
    }

    fn define_data(&mut self, id: DataId, data: &DataDescription) -> ModuleResult<()> {
        let decl = self.declarations.get_data_decl(id);
        if !decl.linkage.is_definable() {
            return Err(ModuleError::InvalidImportDefinition(
                decl.linkage_name(id).into_owned(),
            ));
        }

        if !self.compiled_data_objects[id].is_none() {
            return Err(ModuleError::DuplicateDefinition(
                decl.linkage_name(id).into_owned(),
            ));
        }

        assert!(!decl.tls, "JIT doesn't yet support TLS");

        let &DataDescription {
            ref init,
            function_decls: _,
            data_decls: _,
            function_relocs: _,
            data_relocs: _,
            custom_segment_section: _,
            align,
        } = data;

        let size = init.size();
        let ptr = if size == 0 {
            // Return a correctly aligned non-null pointer to avoid UB in write_bytes and
            // copy_nonoverlapping.
            usize::try_from(align.unwrap_or(WRITABLE_DATA_ALIGNMENT)).unwrap() as *mut u8
        } else if decl.writable {
            self.memory
                .writable
                .allocate(size, align.unwrap_or(WRITABLE_DATA_ALIGNMENT))
                .map_err(|e| ModuleError::Allocation {
                    message: "unable to alloc writable data",
                    err: e,
                })?
        } else {
            self.memory
                .readonly
                .allocate(size, align.unwrap_or(READONLY_DATA_ALIGNMENT))
                .map_err(|e| ModuleError::Allocation {
                    message: "unable to alloc readonly data",
                    err: e,
                })?
        };

        if ptr.is_null() {
            // FIXME pass a Layout to allocate and only compute the layout once.
            std::alloc::handle_alloc_error(
                std::alloc::Layout::from_size_align(
                    size,
                    align.unwrap_or(READONLY_DATA_ALIGNMENT).try_into().unwrap(),
                )
                .unwrap(),
            );
        }

        match *init {
            Init::Uninitialized => {
                panic!("data is not initialized yet");
            }
            Init::Zeros { .. } => {
                unsafe { ptr::write_bytes(ptr, 0, size) };
            }
            Init::Bytes { ref contents } => {
                let src = contents.as_ptr();
                unsafe { ptr::copy_nonoverlapping(src, ptr, size) };
            }
        }

        let pointer_reloc = match self.isa.triple().pointer_width().unwrap() {
            PointerWidth::U16 => panic!(),
            PointerWidth::U32 => Reloc::Abs4,
            PointerWidth::U64 => Reloc::Abs8,
        };
        let relocs = data.all_relocs(pointer_reloc).collect::<Vec<_>>();

        self.compiled_data_objects[id] = Some(CompiledBlob { ptr, size, relocs });
        self.data_objects_to_finalize.push(id);
        if self.isa.flags().is_pic() {
            self.pending_got_updates.push(GotUpdate {
                entry: self.data_object_got_entries[id].unwrap(),
                ptr,
            })
        }

        Ok(())
    }

    fn get_name(&self, name: &str) -> Option<cranelift_module::FuncOrDataId> {
        self.declarations().get_name(name)
    }

    fn target_config(&self) -> cranelift_codegen::isa::TargetFrontendConfig {
        self.isa().frontend_config()
    }

    fn make_context(&self) -> cranelift_codegen::Context {
        let mut ctx = cranelift_codegen::Context::new();
        ctx.func.signature.call_conv = self.isa().default_call_conv();
        ctx
    }

    fn clear_context(&self, ctx: &mut cranelift_codegen::Context) {
        ctx.clear();
        ctx.func.signature.call_conv = self.isa().default_call_conv();
    }

    fn make_signature(&self) -> ir::Signature {
        ir::Signature::new(self.isa().default_call_conv())
    }

    fn clear_signature(&self, sig: &mut ir::Signature) {
        sig.clear(self.isa().default_call_conv());
    }
}

#[cfg(not(windows))]
fn lookup_with_dlsym(name: &str) -> Option<*const u8> {
    let c_str = CString::new(name).unwrap();
    let c_str_ptr = c_str.as_ptr();
    let sym = unsafe { libc::dlsym(libc::RTLD_DEFAULT, c_str_ptr) };
    if sym.is_null() {
        None
    } else {
        Some(sym as *const u8)
    }
}

#[cfg(windows)]
fn lookup_with_dlsym(name: &str) -> Option<*const u8> {
    use std::os::windows::io::RawHandle;
    use windows_sys::Win32::Foundation::HMODULE;
    use windows_sys::Win32::System::LibraryLoader;

    const UCRTBASE: &[u8] = b"ucrtbase.dll\0";

    let c_str = CString::new(name).unwrap();
    let c_str_ptr = c_str.as_ptr();

    unsafe {
        let handles = [
            // try to find the searched symbol in the currently running executable
            ptr::null_mut(),
            // try to find the searched symbol in local c runtime
            LibraryLoader::GetModuleHandleA(UCRTBASE.as_ptr()) as RawHandle,
        ];

        for handle in &handles {
            let addr = LibraryLoader::GetProcAddress(*handle as HMODULE, c_str_ptr.cast());
            match addr {
                None => continue,
                Some(addr) => return Some(addr as *const u8),
            }
        }

        None
    }
}

fn use_bti(isa_flags: &Vec<settings::Value>) -> bool {
    isa_flags
        .iter()
        .find(|&f| f.name == "use_bti")
        .map_or(false, |f| f.as_bool().unwrap_or(false))
}
