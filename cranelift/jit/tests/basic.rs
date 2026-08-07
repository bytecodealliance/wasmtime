use cranelift_codegen::ir::*;
use cranelift_codegen::isa::{CallConv, OwnedTargetIsa};
use cranelift_codegen::settings::{self, Configurable};
use cranelift_codegen::{Context, ir::types::I16};
use cranelift_entity::EntityRef;
use cranelift_frontend::*;
use cranelift_jit::*;
use cranelift_module::*;

fn isa() -> Option<OwnedTargetIsa> {
    let mut flag_builder = settings::builder();
    flag_builder.set("use_colocated_libcalls", "false").unwrap();
    // PIC is only supported by the JIT on x86_64, where it routes symbol
    // address materialization through per-blob GOT entries.
    let is_pic = if cfg!(target_arch = "x86_64") {
        "true"
    } else {
        "false"
    };
    flag_builder.set("is_pic", is_pic).unwrap();
    let isa_builder = cranelift_native::builder().ok()?;
    isa_builder.finish(settings::Flags::new(flag_builder)).ok()
}

#[test]
fn error_on_incompatible_sig_in_declare_function() {
    let Some(isa) = isa() else {
        return;
    };
    let mut module = JITModule::new(JITBuilder::with_isa(isa, default_libcall_names()));

    let mut sig = Signature {
        params: vec![AbiParam::new(types::I64)],
        returns: vec![],
        call_conv: CallConv::SystemV,
    };
    module
        .declare_function("abc", Linkage::Local, &sig)
        .unwrap();
    sig.params[0] = AbiParam::new(types::I32);
    module
        .declare_function("abc", Linkage::Local, &sig)
        .err()
        .unwrap(); // Make sure this is an error
}

fn define_simple_function(module: &mut JITModule) -> Result<FuncId, ModuleError> {
    let sig = Signature {
        params: vec![],
        returns: vec![],
        call_conv: CallConv::SystemV,
    };

    let func_id = module.declare_function("abc", Linkage::Local, &sig)?;

    let mut ctx = Context::new();
    ctx.func = Function::with_name_signature(UserFuncName::user(0, func_id.as_u32()), sig);
    let mut func_ctx = FunctionBuilderContext::new();
    {
        let mut bcx: FunctionBuilder = FunctionBuilder::new(&mut ctx.func, &mut func_ctx);
        let block = bcx.create_block();
        bcx.switch_to_block(block);
        bcx.ins().return_(&[]);
    }

    module.define_function(func_id, &mut ctx)?;

    Ok(func_id)
}

#[test]
fn panic_on_define_after_finalize() {
    let Some(isa) = isa() else {
        return;
    };
    let mut module = JITModule::new(JITBuilder::with_isa(isa, default_libcall_names()));

    define_simple_function(&mut module).unwrap();
    define_simple_function(&mut module).err().unwrap();
}

#[test]
fn switch_error() {
    use cranelift_codegen::settings;

    let sig = Signature {
        params: vec![AbiParam::new(types::I32)],
        returns: vec![AbiParam::new(types::I32)],
        call_conv: CallConv::SystemV,
    };

    let mut func = Function::with_name_signature(UserFuncName::default(), sig);

    let mut func_ctx = FunctionBuilderContext::new();
    {
        let mut bcx: FunctionBuilder = FunctionBuilder::new(&mut func, &mut func_ctx);
        let start = bcx.create_block();
        let bb0 = bcx.create_block();
        let bb1 = bcx.create_block();
        let bb2 = bcx.create_block();
        let bb3 = bcx.create_block();
        println!("{start} {bb0} {bb1} {bb2} {bb3}");

        bcx.declare_var(types::I32);
        bcx.declare_var(types::I32);
        let in_val = bcx.append_block_param(start, types::I32);
        bcx.switch_to_block(start);
        bcx.def_var(Variable::new(0), in_val);
        bcx.ins().jump(bb0, &[]);

        bcx.switch_to_block(bb0);
        let discr = bcx.use_var(Variable::new(0));
        let mut switch = cranelift_frontend::Switch::new();
        for &(index, bb) in &[
            (9, bb1),
            (13, bb1),
            (10, bb1),
            (92, bb1),
            (39, bb1),
            (34, bb1),
        ] {
            switch.set_entry(index, bb);
        }
        switch.emit(&mut bcx, discr, bb2);

        bcx.switch_to_block(bb1);
        let v = bcx.use_var(Variable::new(0));
        bcx.def_var(Variable::new(1), v);
        bcx.ins().jump(bb3, &[]);

        bcx.switch_to_block(bb2);
        let v = bcx.use_var(Variable::new(0));
        bcx.def_var(Variable::new(1), v);
        bcx.ins().jump(bb3, &[]);

        bcx.switch_to_block(bb3);
        let r = bcx.use_var(Variable::new(1));
        bcx.ins().return_(&[r]);

        bcx.seal_all_blocks();
        bcx.finalize(cranelift_codegen::isa::TargetFrontendConfig {
            default_call_conv: CallConv::SystemV,
            pointer_width: target_lexicon::PointerWidth::U64,
            page_size_align_log2: 12,
        });
    }

    let flags = settings::Flags::new(settings::builder());
    match cranelift_codegen::verify_function(&func, &flags) {
        Ok(_) => {}
        Err(err) => {
            let pretty_error =
                cranelift_codegen::print_errors::pretty_verifier_error(&func, None, err);
            panic!("pretty_error:\n{pretty_error}");
        }
    }
}

#[test]
fn libcall_function() {
    let Some(isa) = isa() else {
        return;
    };
    let mut module = JITModule::new(JITBuilder::with_isa(isa, default_libcall_names()));

    let sig = Signature {
        params: vec![],
        returns: vec![],
        call_conv: CallConv::SystemV,
    };

    let func_id = module
        .declare_function("function", Linkage::Local, &sig)
        .unwrap();

    let mut ctx = Context::new();
    ctx.func = Function::with_name_signature(UserFuncName::user(0, func_id.as_u32()), sig);

    let mut func_ctx = FunctionBuilderContext::new();
    {
        let mut bcx: FunctionBuilder = FunctionBuilder::new(&mut ctx.func, &mut func_ctx);
        let block = bcx.create_block();
        bcx.switch_to_block(block);

        let int = module.target_config().pointer_type();
        let zero = bcx.ins().iconst(I16, 0);
        let size = bcx.ins().iconst(int, 10);

        let mut signature = module.make_signature();
        signature.params.push(AbiParam::new(int));
        signature.returns.push(AbiParam::new(int));
        let callee = module
            .declare_function("malloc", Linkage::Import, &signature)
            .expect("declare malloc function");
        let local_callee = module.declare_func_in_func(callee, &mut bcx.func);
        let argument_exprs = vec![size];
        let call = bcx.ins().call(local_callee, &argument_exprs);
        let buffer = bcx.inst_results(call)[0];

        bcx.call_memset(module.target_config(), buffer, zero, size);

        bcx.ins().return_(&[]);
    }

    module
        .define_function_with_control_plane(func_id, &mut ctx, &mut Default::default())
        .unwrap();

    module.finalize_definitions().unwrap();
}

// This used to cause UB. See https://github.com/bytecodealliance/wasmtime/issues/7918.
#[test]
fn empty_data_object() {
    let Some(isa) = isa() else {
        return;
    };
    let mut module = JITModule::new(JITBuilder::with_isa(isa, default_libcall_names()));

    let data_id = module
        .declare_data("empty", Linkage::Export, false, false)
        .unwrap();

    let mut data = DataDescription::new();
    data.define(Box::new([]));
    module.define_data(data_id, &data).unwrap();
}

#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
mod far_x86_memory {
    use std::collections::VecDeque;
    use std::io;
    use std::ptr;
    use std::sync::{Arc, Mutex};

    use cranelift_jit::{BranchProtection, JITMemoryKind, JITMemoryProvider};
    use cranelift_module::ModuleResult;

    pub(super) const VENEER_SIZE: usize = 16;

    struct ReservedAddressSpace {
        base: usize,
        len: usize,
        page_size: usize,
    }

    impl ReservedAddressSpace {
        fn new() -> Self {
            let page_size = usize::try_from(unsafe { libc::sysconf(libc::_SC_PAGESIZE) }).unwrap();
            let len = (i32::MAX as usize)
                .checked_add(16 * 1024 * 1024)
                .unwrap()
                .next_multiple_of(page_size);
            let mapping = unsafe {
                libc::mmap(
                    ptr::null_mut(),
                    len,
                    libc::PROT_NONE,
                    libc::MAP_PRIVATE | libc::MAP_ANONYMOUS | libc::MAP_NORESERVE,
                    -1,
                    0,
                )
            };
            assert_ne!(mapping, libc::MAP_FAILED);
            Self {
                base: mapping.addr(),
                len,
                page_size,
            }
        }
    }

    impl Drop for ReservedAddressSpace {
        fn drop(&mut self) {
            let result = unsafe { libc::munmap(self.base as *mut libc::c_void, self.len) };
            assert_eq!(result, 0);
        }
    }

    #[derive(Clone, Copy)]
    pub(super) enum Placement {
        Low,
        High,
    }

    #[derive(Clone, Copy)]
    enum FinalProtection {
        ReadExecute,
        ReadOnly,
        ReadWrite,
    }

    struct Allocation {
        addr: usize,
        len: usize,
        final_protection: FinalProtection,
    }

    /// Places JIT allocations at predetermined ends of a virtual address
    /// reservation so tests can deterministically exceed `rel32` range.
    pub(super) struct FarMemoryProvider {
        space: ReservedAddressSpace,
        executable_placements: VecDeque<Placement>,
        readonly_placements: VecDeque<Placement>,
        writable_placements: VecDeque<Placement>,
        low_offset: usize,
        high_offset: usize,
        allocations: Vec<Allocation>,
        requested_exec_sizes: Arc<Mutex<Vec<usize>>>,
    }

    impl FarMemoryProvider {
        pub(super) fn new(
            executable_placements: impl IntoIterator<Item = Placement>,
            readonly_placements: impl IntoIterator<Item = Placement>,
        ) -> (Self, Arc<Mutex<Vec<usize>>>) {
            let space = ReservedAddressSpace::new();
            let high_offset = space.len;
            let requested_exec_sizes = Arc::new(Mutex::new(Vec::new()));
            (
                Self {
                    space,
                    executable_placements: executable_placements.into_iter().collect(),
                    readonly_placements: readonly_placements.into_iter().collect(),
                    writable_placements: VecDeque::new(),
                    low_offset: 0,
                    high_offset,
                    allocations: Vec::new(),
                    requested_exec_sizes: Arc::clone(&requested_exec_sizes),
                },
                requested_exec_sizes,
            )
        }

        fn allocate_at(
            &mut self,
            size: usize,
            align: u64,
            placement: Placement,
            final_protection: FinalProtection,
        ) -> io::Result<*mut u8> {
            assert!(usize::try_from(align).unwrap() <= self.space.page_size);
            let len = size
                .next_multiple_of(self.space.page_size)
                .max(self.space.page_size);
            let addr = match placement {
                Placement::Low => {
                    let addr = self.space.base + self.low_offset;
                    self.low_offset += len;
                    addr
                }
                Placement::High => {
                    self.high_offset -= len;
                    self.space.base + self.high_offset
                }
            };
            assert!(self.low_offset <= self.high_offset);
            let result = unsafe {
                libc::mprotect(
                    addr as *mut libc::c_void,
                    len,
                    libc::PROT_READ | libc::PROT_WRITE,
                )
            };
            assert_eq!(result, 0);
            self.allocations.push(Allocation {
                addr,
                len,
                final_protection,
            });
            Ok(addr as *mut u8)
        }
    }

    impl JITMemoryProvider for FarMemoryProvider {
        fn allocate(
            &mut self,
            size: usize,
            align: u64,
            kind: JITMemoryKind,
        ) -> io::Result<*mut u8> {
            let (placement, final_protection) = match kind {
                JITMemoryKind::Executable => {
                    self.requested_exec_sizes.lock().unwrap().push(size);
                    (
                        self.executable_placements
                            .pop_front()
                            .expect("placement for every executable allocation"),
                        FinalProtection::ReadExecute,
                    )
                }
                JITMemoryKind::Writable => (
                    self.writable_placements
                        .pop_front()
                        .expect("placement for every writable allocation"),
                    FinalProtection::ReadWrite,
                ),
                JITMemoryKind::ReadOnly => (
                    self.readonly_placements
                        .pop_front()
                        .expect("placement for every read-only allocation"),
                    FinalProtection::ReadOnly,
                ),
            };
            self.allocate_at(size, align, placement, final_protection)
        }

        unsafe fn free_memory(&mut self) {
            // All allocations are freed together when `space` is unmapped.
            self.allocations.clear();
        }

        fn finalize(&mut self, _branch_protection: BranchProtection) -> ModuleResult<()> {
            for allocation in &self.allocations {
                let protection = match allocation.final_protection {
                    FinalProtection::ReadExecute => libc::PROT_READ | libc::PROT_EXEC,
                    FinalProtection::ReadOnly => libc::PROT_READ,
                    FinalProtection::ReadWrite => libc::PROT_READ | libc::PROT_WRITE,
                };
                let result = unsafe {
                    libc::mprotect(
                        allocation.addr as *mut libc::c_void,
                        allocation.len,
                        protection,
                    )
                };
                assert_eq!(result, 0);
            }
            Ok(())
        }
    }
}

/// Reproduces a bug where a `call` or `jmp` between two functions of the same
/// module that happen to be placed further than ±2 GiB apart panicked while
/// applying the `X86CallPCRel4` relocation, instead of routing the control
/// transfer through a veneer like AArch64 already did for `Arm64Call`.
#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
#[test]
fn far_x86_control_transfers_use_veneers() {
    use std::mem;

    use cranelift_codegen::binemit::Reloc;

    use far_x86_memory::{FarMemoryProvider, Placement, VENEER_SIZE};

    let (memory, requested_exec_sizes) = FarMemoryProvider::new(
        [
            Placement::Low,
            Placement::High,
            Placement::Low,
            Placement::High,
            Placement::Low,
            Placement::Low,
            Placement::High,
            Placement::High,
        ],
        [],
    );
    let mut builder = JITBuilder::new(default_libcall_names()).unwrap();
    builder.memory_provider(Box::new(memory));

    let mut module = JITModule::new(builder);
    let mut signature = module.make_signature();
    signature.returns.push(AbiParam::new(types::I32));
    let low_callee = module
        .declare_function("low_callee", Linkage::Local, &signature)
        .unwrap();
    let high_caller = module
        .declare_function("high_caller", Linkage::Local, &signature)
        .unwrap();
    let low_tail_caller = module
        .declare_function("low_tail_caller", Linkage::Local, &signature)
        .unwrap();
    let high_tail_callee = module
        .declare_function("high_tail_callee", Linkage::Local, &signature)
        .unwrap();

    // low_callee: mov eax, 42; ret
    module
        .define_function_bytes(low_callee, 1, &[0xb8, 42, 0, 0, 0, 0xc3], &[])
        .unwrap();

    // Compile high_caller normally so this test also checks the relocation
    // emitted for a compiler-generated direct call.
    let mut ctx = module.make_context();
    ctx.func.name = UserFuncName::user(0, high_caller.as_u32());
    ctx.func.signature = signature.clone();
    let low_callee_ref = module.declare_func_in_func(low_callee, &mut ctx.func);
    let mut func_ctx = FunctionBuilderContext::new();
    {
        let mut bcx = FunctionBuilder::new(&mut ctx.func, &mut func_ctx);
        let block = bcx.create_block();
        bcx.switch_to_block(block);
        let call = bcx.ins().call(low_callee_ref, &[]);
        let result = bcx.inst_results(call)[0];
        bcx.ins().return_(&[result]);
    }
    module.define_function(high_caller, &mut ctx).unwrap();
    let (high_caller_code_len, high_caller_reloc_offset) = {
        let compiled = ctx.compiled_code().unwrap();
        let reloc = compiled
            .buffer
            .relocs()
            .iter()
            .find(|reloc| reloc.kind == Reloc::X86CallPCRel4)
            .unwrap();
        (compiled.code_buffer().len(), reloc.offset as usize)
    };

    // low_tail_caller: jmp high_tail_callee
    let relocations = [ModuleReloc {
        offset: 1,
        kind: Reloc::X86CallPCRel4,
        name: ModuleRelocTarget::user(0, high_tail_callee.as_u32()),
        addend: -4,
    }];
    module
        .define_function_bytes(low_tail_caller, 1, &[0xe9, 0, 0, 0, 0], &relocations)
        .unwrap();

    // high_tail_callee: mov eax, 84; ret
    module
        .define_function_bytes(high_tail_callee, 1, &[0xb8, 84, 0, 0, 0, 0xc3], &[])
        .unwrap();

    // A caller with one near target and two far targets checks that only the
    // far calls use distinct, densely packed veneers.
    let mixed_caller = module
        .declare_function("mixed_caller", Linkage::Local, &signature)
        .unwrap();
    let near = module
        .declare_function("near", Linkage::Local, &signature)
        .unwrap();
    let far_a = module
        .declare_function("far_a", Linkage::Local, &signature)
        .unwrap();
    let far_b = module
        .declare_function("far_b", Linkage::Local, &signature)
        .unwrap();
    let mixed_caller_code = [
        0x48, 0x83, 0xec, 0x08, // sub rsp, 8
        0xe8, 0, 0, 0, 0, // call near
        0xe8, 0, 0, 0, 0, // call far_a
        0xe8, 0, 0, 0, 0, // call far_b
        0x48, 0x83, 0xc4, 0x08, // add rsp, 8
        0xc3, // ret
    ];
    let mixed_relocations = [
        ModuleReloc {
            offset: 5,
            kind: Reloc::X86CallPCRel4,
            name: ModuleRelocTarget::user(0, near.as_u32()),
            addend: -4,
        },
        ModuleReloc {
            offset: 10,
            kind: Reloc::X86CallPCRel4,
            name: ModuleRelocTarget::user(0, far_a.as_u32()),
            addend: -4,
        },
        ModuleReloc {
            offset: 15,
            kind: Reloc::X86CallPCRel4,
            name: ModuleRelocTarget::user(0, far_b.as_u32()),
            addend: -4,
        },
    ];
    module
        .define_function_bytes(mixed_caller, 1, &mixed_caller_code, &mixed_relocations)
        .unwrap();
    module
        .define_function_bytes(near, 1, &[0xb8, 1, 0, 0, 0, 0xc3], &[])
        .unwrap();
    module
        .define_function_bytes(far_a, 1, &[0xb8, 2, 0, 0, 0, 0xc3], &[])
        .unwrap();
    module
        .define_function_bytes(far_b, 1, &[0xb8, 42, 0, 0, 0, 0xc3], &[])
        .unwrap();

    module.finalize_definitions().unwrap();

    let low_callee_ptr = module.get_finalized_function(low_callee);
    let high_caller_ptr = module.get_finalized_function(high_caller);
    let low_tail_caller_ptr = module.get_finalized_function(low_tail_caller);
    let high_tail_callee_ptr = module.get_finalized_function(high_tail_callee);
    let mixed_caller_ptr = module.get_finalized_function(mixed_caller);
    let near_ptr = module.get_finalized_function(near);
    let far_a_ptr = module.get_finalized_function(far_a);
    let far_b_ptr = module.get_finalized_function(far_b);

    fn branch_destination(caller: *const u8, reloc_offset: usize) -> *const u8 {
        let at = caller.wrapping_byte_add(reloc_offset);
        let displacement = unsafe { at.cast::<i32>().read_unaligned() };
        at.wrapping_byte_add(4)
            .wrapping_byte_offset(displacement as isize)
    }

    fn assert_veneer(
        caller: *const u8,
        code_len: usize,
        reloc_offset: usize,
        expected_target: *const u8,
    ) -> *const u8 {
        assert!(caller.addr().abs_diff(expected_target.addr()) > i32::MAX as usize);

        let veneer = branch_destination(caller, reloc_offset);
        assert_eq!(veneer, caller.wrapping_byte_add(code_len));

        let veneer_code = unsafe { std::slice::from_raw_parts(veneer, 6) };
        assert_eq!(veneer_code, [0xff, 0x25, 0, 0, 0, 0]);
        let veneer_target = unsafe { veneer.byte_add(6).cast::<u64>().read_unaligned() };
        assert_eq!(veneer_target, expected_target.addr() as u64);
        veneer
    }

    // Check a negative-distance call and a positive-distance tail jump.
    assert!(high_caller_ptr.addr() > low_callee_ptr.addr());
    assert_veneer(
        high_caller_ptr,
        high_caller_code_len,
        high_caller_reloc_offset,
        low_callee_ptr,
    );
    let high_caller_fn: extern "C" fn() -> u32 = unsafe { mem::transmute(high_caller_ptr) };
    assert_eq!(high_caller_fn(), 42);

    assert!(low_tail_caller_ptr.addr() < high_tail_callee_ptr.addr());
    assert_veneer(low_tail_caller_ptr, 5, 1, high_tail_callee_ptr);
    let low_tail_caller_fn: extern "C" fn() -> u32 = unsafe { mem::transmute(low_tail_caller_ptr) };
    assert_eq!(low_tail_caller_fn(), 84);

    assert!(mixed_caller_ptr.addr().abs_diff(near_ptr.addr()) <= i32::MAX as usize);
    assert!(mixed_caller_ptr.addr().abs_diff(far_a_ptr.addr()) > i32::MAX as usize);
    assert!(mixed_caller_ptr.addr().abs_diff(far_b_ptr.addr()) > i32::MAX as usize);
    assert_eq!(branch_destination(mixed_caller_ptr, 5), near_ptr);
    let veneer_a = assert_veneer(mixed_caller_ptr, mixed_caller_code.len(), 10, far_a_ptr);
    let veneer_b = assert_veneer(
        mixed_caller_ptr,
        mixed_caller_code.len() + VENEER_SIZE,
        15,
        far_b_ptr,
    );
    assert_eq!(veneer_b, veneer_a.wrapping_byte_add(VENEER_SIZE));
    let mixed_caller_fn: extern "C" fn() -> u32 = unsafe { mem::transmute(mixed_caller_ptr) };
    assert_eq!(mixed_caller_fn(), 42);

    // One worst-case slot is reserved for every branch relocation, including
    // the call that ultimately resolves directly.
    let requested_exec_sizes = requested_exec_sizes.lock().unwrap();
    assert_eq!(requested_exec_sizes[1], high_caller_code_len + VENEER_SIZE);
    assert_eq!(requested_exec_sizes[2], 5 + VENEER_SIZE);
    assert_eq!(
        requested_exec_sizes[4],
        mixed_caller_code.len() + mixed_relocations.len() * VENEER_SIZE
    );
    drop(requested_exec_sizes);

    unsafe { module.free_memory() };
}

/// Reproduces the Roto failure: JIT code materializes the address of an
/// anonymous read-only data object allocated more than ±2 GiB away. The
/// GOT-relative load cannot be relaxed to a `lea` and reads the address from
/// a GOT entry at the end of the function's allocation.
#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
#[test]
fn far_x86_data_address_resolves_through_got() {
    use cranelift_codegen::binemit::Reloc;

    use far_x86_memory::{FarMemoryProvider, Placement};

    let (memory, _) = FarMemoryProvider::new([Placement::Low], [Placement::High]);
    let mut builder = JITBuilder::new(default_libcall_names()).unwrap();
    builder.memory_provider(Box::new(memory));
    let mut module = JITModule::new(builder);

    let data_id = module.declare_anonymous_data(false, false).unwrap();
    let mut data = DataDescription::new();
    data.define(vec![42].into_boxed_slice());
    module.define_data(data_id, &data).unwrap();

    let mut signature = module.make_signature();
    signature
        .returns
        .push(AbiParam::new(module.target_config().pointer_type()));
    let address_func = module
        .declare_function("data_address", Linkage::Local, &signature)
        .unwrap();
    let mut ctx = module.make_context();
    ctx.func.name = UserFuncName::user(0, address_func.as_u32());
    ctx.func.signature = signature;
    let data_ref = module.declare_data_in_func(data_id, &mut ctx.func);
    let mut func_ctx = FunctionBuilderContext::new();
    {
        let mut bcx = FunctionBuilder::new(&mut ctx.func, &mut func_ctx);
        let block = bcx.create_block();
        bcx.switch_to_block(block);
        let address = bcx
            .ins()
            .symbol_value(module.target_config().pointer_type(), data_ref);
        bcx.ins().return_(&[address]);
        bcx.seal_all_blocks();
        bcx.finalize(module.target_config());
    }
    module.define_function(address_func, &mut ctx).unwrap();

    // Before the fix, finalization panics here while applying an out-of-range
    // X86PCRel4 relocation.
    module.finalize_definitions().unwrap();

    let relocs = ctx.compiled_code().unwrap().buffer.relocs();
    assert_eq!(relocs.len(), 1);
    assert_eq!(relocs[0].kind, Reloc::X86GOTPCRel4);
    let reloc_offset = usize::try_from(relocs[0].offset).unwrap();

    let (data_ptr, _) = module.get_finalized_data(data_id);
    let address_ptr = module.get_finalized_function(address_func);
    assert!(address_ptr.addr().abs_diff(data_ptr.addr()) > i32::MAX as usize);

    // The load stays a `mov` and reads the data object's address from a GOT
    // entry at the end of the function's allocation.
    assert_eq!(
        unsafe { address_ptr.byte_add(reloc_offset - 2).read() },
        0x8b
    );
    let displacement = unsafe {
        address_ptr
            .byte_add(reloc_offset)
            .cast::<i32>()
            .read_unaligned()
    };
    let got_entry = address_ptr
        .wrapping_byte_add(reloc_offset + 4)
        .wrapping_byte_offset(displacement as isize);
    assert!(got_entry.addr() > address_ptr.addr());
    assert_eq!(
        unsafe { got_entry.cast::<u64>().read_unaligned() },
        data_ptr.addr() as u64
    );

    let get_address: extern "C" fn() -> usize = unsafe { std::mem::transmute(address_ptr) };
    assert_eq!(get_address(), data_ptr.addr());

    unsafe { module.free_memory() };
}

/// When the memory provider does place a symbol within displacement range,
/// the GOT-relative load is relaxed to a `lea` computing the address
/// directly, so near symbol accesses don't pay for a GOT indirection.
#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
#[test]
fn near_x86_data_address_relaxes_got_load_to_lea() {
    use cranelift_codegen::binemit::Reloc;

    use far_x86_memory::{FarMemoryProvider, Placement};

    let (memory, _) = FarMemoryProvider::new([Placement::Low], [Placement::Low]);
    let mut builder = JITBuilder::new(default_libcall_names()).unwrap();
    builder.memory_provider(Box::new(memory));
    let mut module = JITModule::new(builder);

    let data_id = module.declare_anonymous_data(false, false).unwrap();
    let mut data = DataDescription::new();
    data.define(vec![42].into_boxed_slice());
    module.define_data(data_id, &data).unwrap();

    let mut signature = module.make_signature();
    signature
        .returns
        .push(AbiParam::new(module.target_config().pointer_type()));
    let address_func = module
        .declare_function("data_address", Linkage::Local, &signature)
        .unwrap();
    let mut ctx = module.make_context();
    ctx.func.name = UserFuncName::user(0, address_func.as_u32());
    ctx.func.signature = signature;
    let data_ref = module.declare_data_in_func(data_id, &mut ctx.func);
    let mut func_ctx = FunctionBuilderContext::new();
    {
        let mut bcx = FunctionBuilder::new(&mut ctx.func, &mut func_ctx);
        let block = bcx.create_block();
        bcx.switch_to_block(block);
        let address = bcx
            .ins()
            .symbol_value(module.target_config().pointer_type(), data_ref);
        bcx.ins().return_(&[address]);
        bcx.seal_all_blocks();
        bcx.finalize(module.target_config());
    }
    module.define_function(address_func, &mut ctx).unwrap();
    module.finalize_definitions().unwrap();

    let relocs = ctx.compiled_code().unwrap().buffer.relocs();
    assert_eq!(relocs.len(), 1);
    assert_eq!(relocs[0].kind, Reloc::X86GOTPCRel4);
    let reloc_offset = usize::try_from(relocs[0].offset).unwrap();

    let (data_ptr, _) = module.get_finalized_data(data_id);
    let address_ptr = module.get_finalized_function(address_func);
    assert!(address_ptr.addr().abs_diff(data_ptr.addr()) <= i32::MAX as usize);

    // The load was relaxed to a `lea` pointing directly at the data object.
    assert_eq!(
        unsafe { address_ptr.byte_add(reloc_offset - 2).read() },
        0x8d
    );
    let displacement = unsafe {
        address_ptr
            .byte_add(reloc_offset)
            .cast::<i32>()
            .read_unaligned()
    };
    assert_eq!(
        address_ptr
            .wrapping_byte_add(reloc_offset + 4)
            .wrapping_byte_offset(displacement as isize),
        data_ptr
    );

    let get_address: extern "C" fn() -> usize = unsafe { std::mem::transmute(address_ptr) };
    assert_eq!(get_address(), data_ptr.addr());

    unsafe { module.free_memory() };
}

/// A materialized function pointer has the same range issue as a data pointer,
/// but direct calls to that function must remain eligible for call veneers.
/// The GOT entry holds the function's real address, preserving pointer
/// identity with `get_finalized_function`.
#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
#[test]
fn far_x86_function_address_resolves_through_got() {
    use cranelift_codegen::binemit::Reloc;

    use far_x86_memory::{FarMemoryProvider, Placement};

    let (memory, _) = FarMemoryProvider::new([Placement::High, Placement::Low], []);
    let mut builder = JITBuilder::new(default_libcall_names()).unwrap();
    builder.memory_provider(Box::new(memory));
    let mut module = JITModule::new(builder);

    let target_signature = module.make_signature();
    let target = module
        .declare_function("address_target", Linkage::Local, &target_signature)
        .unwrap();
    module
        .define_function_bytes(target, 1, &[0xc3], &[])
        .unwrap();

    let mut address_signature = module.make_signature();
    address_signature
        .returns
        .push(AbiParam::new(module.target_config().pointer_type()));
    let address_func = module
        .declare_function("function_address", Linkage::Local, &address_signature)
        .unwrap();
    let mut ctx = module.make_context();
    ctx.func.name = UserFuncName::user(0, address_func.as_u32());
    ctx.func.signature = address_signature;
    let target_ref = module.declare_func_in_func(target, &mut ctx.func);
    let mut func_ctx = FunctionBuilderContext::new();
    {
        let mut bcx = FunctionBuilder::new(&mut ctx.func, &mut func_ctx);
        let block = bcx.create_block();
        bcx.switch_to_block(block);
        bcx.ins().call(target_ref, &[]);
        let address = bcx
            .ins()
            .func_addr(module.target_config().pointer_type(), target_ref);
        bcx.ins().return_(&[address]);
        bcx.seal_all_blocks();
        bcx.finalize(module.target_config());
    }
    module.define_function(address_func, &mut ctx).unwrap();

    // Before the fix, finalization panics here while applying an out-of-range
    // X86PCRel4 relocation.
    module.finalize_definitions().unwrap();

    let relocs = ctx.compiled_code().unwrap().buffer.relocs();
    assert_eq!(relocs.len(), 2);
    assert_eq!(
        relocs
            .iter()
            .filter(|reloc| reloc.kind == Reloc::X86CallPCRel4)
            .count(),
        1
    );
    assert_eq!(
        relocs
            .iter()
            .filter(|reloc| reloc.kind == Reloc::X86GOTPCRel4)
            .count(),
        1
    );

    let target_ptr = module.get_finalized_function(target);
    let address_ptr = module.get_finalized_function(address_func);
    assert!(address_ptr.addr().abs_diff(target_ptr.addr()) > i32::MAX as usize);
    let get_address: extern "C" fn() -> usize = unsafe { std::mem::transmute(address_ptr) };
    assert_eq!(get_address(), target_ptr.addr());

    unsafe { module.free_memory() };
}
