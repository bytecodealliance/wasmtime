//! Support for compiling with Cranelift.
//!
//! This crate provides an implementation of [`Compiler`] in the form of
//! [`Cranelift`].

// # How does Wasmtime prevent stack overflow?
//
// A few locations throughout the codebase link to this file to explain
// interrupts and stack overflow. To start off, let's take a look at stack
// overflow. Wasm code is well-defined to have stack overflow being recoverable
// and raising a trap, so we need to handle this somehow! There's also an added
// constraint where as an embedder you frequently are running host-provided
// code called from wasm. WebAssembly and native code currently share the same
// call stack, so you want to make sure that your host-provided code will have
// enough call-stack available to it.
//
// Given all that, the way that stack overflow is handled is by adding a
// prologue check to all JIT functions for how much native stack is remaining.
// The `VMContext` pointer is the first argument to all functions, and the first
// field of this structure is `*const VMInterrupts` and the first field of that
// is the stack limit. Note that the stack limit in this case means "if the
// stack pointer goes below this, trap". Each JIT function which consumes stack
// space or isn't a leaf function starts off by loading the stack limit,
// checking it against the stack pointer, and optionally traps.
//
// This manual check allows the embedder (us) to give wasm a relatively precise
// amount of stack allocation. Using this scheme we reserve a chunk of stack
// for wasm code relative from where wasm code was called. This ensures that
// native code called by wasm should have native stack space to run, and the
// numbers of stack spaces here should all be configurable for various
// embeddings.
//
// Note that we do not consider each thread's stack guard page here. It's
// considered that if you hit that you still abort the whole program. This
// shouldn't happen most of the time because wasm is always stack-bound and
// it's up to the embedder to bound its own native stack.
//
// So all-in-all, that's how we implement stack checks. Note that stack checks
// cannot be disabled because it's a feature of core wasm semantics. This means
// that all functions almost always have a stack check prologue, and it's up to
// us to optimize away that cost as much as we can.
//
// For more information about the tricky bits of managing the reserved stack
// size of wasm, see the implementation in `traphandlers.rs` in the
// `update_stack_limit` function.
//
// # How is Wasmtime interrupted?
//
// Ok so given all that background of stack checks, the next thing we want to
// build on top of this is the ability to *interrupt* executing wasm code. This
// is useful to ensure that wasm always executes within a particular time slice
// or otherwise doesn't consume all CPU resources on a system. There are two
// major ways that interrupts are required:
//
// * Loops - likely immediately apparent but it's easy to write an infinite
//   loop in wasm, so we need the ability to interrupt loops.
// * Function entries - somewhat more subtle, but imagine a module where each
//   function calls the next function twice. This creates 2^n calls pretty
//   quickly, so a pretty small module can export a function with no loops
//   that takes an extremely long time to call.
//
// In many cases if an interrupt comes in you want to interrupt host code as
// well, but we're explicitly not considering that here. We're hoping that
// interrupting host code is largely left to the embedder (e.g. figuring out
// how to interrupt blocking syscalls) and they can figure that out. The purpose
// of this feature is to basically only give the ability to interrupt
// currently-executing wasm code (or triggering an interrupt as soon as wasm
// reenters itself).
//
// To implement interruption of loops we insert code at the head of all loops
// which checks the stack limit counter. If the counter matches a magical
// sentinel value that's impossible to be the real stack limit, then we
// interrupt the loop and trap. To implement interrupts of functions, we
// actually do the same thing where the magical sentinel value we use here is
// automatically considered as considering all stack pointer values as "you ran
// over your stack". This means that with a write of a magical value to one
// location we can interrupt both loops and function bodies.
//
// The "magical value" here is `usize::max_value() - N`. We reserve
// `usize::max_value()` for "the stack limit isn't set yet" and so -N is
// then used for "you got interrupted". We do a bit of patching afterwards to
// translate a stack overflow into an interrupt trap if we see that an
// interrupt happened. Note that `N` here is a medium-size-ish nonzero value
// chosen in coordination with the cranelift backend. Currently it's 32k. The
// value of N is basically a threshold in the backend for "anything less than
// this requires only one branch in the prologue, any stack size bigger requires
// two branches". Naturally we want most functions to have one branch, but we
// also need to actually catch stack overflow, so for now 32k is chosen and it's
// assume no valid stack pointer will ever be `usize::max_value() - 32k`.

use crate::func_environ::{get_func_name, FuncEnvironment};
use cranelift_codegen::ir::{self, ExternalName, InstBuilder, MemFlags};
use cranelift_codegen::isa::{CallConv, TargetIsa};
use cranelift_codegen::print_errors::pretty_error;
use cranelift_codegen::MachSrcLoc;
use cranelift_codegen::{binemit, isa, Context};
use cranelift_frontend::FunctionBuilder;
use cranelift_wasm::{DefinedFuncIndex, FuncIndex, FuncTranslator, WasmFuncType, WasmType};
use std::cmp;
use std::convert::TryFrom;
use std::mem;
use std::sync::Mutex;
use target_lexicon::CallingConvention;
use wasmtime_environ::{
    CompileError, CompiledFunction, Compiler, FunctionAddressMap, FunctionBodyData,
    InstructionAddressMap, Module, ModuleTranslation, Relocation, RelocationTarget,
    StackMapInformation, TrapInformation, Tunables, TypeTables,
};

mod func_environ;

/// Implementation of a relocation sink that just saves all the information for later
struct RelocSink {
    /// Current function index.
    func_index: FuncIndex,

    /// Relocations recorded for the function.
    func_relocs: Vec<Relocation>,
}

impl binemit::RelocSink for RelocSink {
    fn reloc_external(
        &mut self,
        offset: binemit::CodeOffset,
        _srcloc: ir::SourceLoc,
        reloc: binemit::Reloc,
        name: &ExternalName,
        addend: binemit::Addend,
    ) {
        let reloc_target = if let ExternalName::User { namespace, index } = *name {
            debug_assert_eq!(namespace, 0);
            RelocationTarget::UserFunc(FuncIndex::from_u32(index))
        } else if let ExternalName::LibCall(libcall) = *name {
            RelocationTarget::LibCall(libcall)
        } else {
            panic!("unrecognized external name")
        };
        self.func_relocs.push(Relocation {
            reloc,
            reloc_target,
            offset,
            addend,
        });
    }

    fn reloc_constant(
        &mut self,
        _code_offset: binemit::CodeOffset,
        _reloc: binemit::Reloc,
        _constant_offset: ir::ConstantOffset,
    ) {
        // Do nothing for now: cranelift emits constant data after the function code and also emits
        // function code with correct relative offsets to the constant data.
    }

    fn reloc_jt(&mut self, offset: binemit::CodeOffset, reloc: binemit::Reloc, jt: ir::JumpTable) {
        self.func_relocs.push(Relocation {
            reloc,
            reloc_target: RelocationTarget::JumpTable(self.func_index, jt),
            offset,
            addend: 0,
        });
    }
}

impl RelocSink {
    /// Return a new `RelocSink` instance.
    fn new(func_index: FuncIndex) -> Self {
        Self {
            func_index,
            func_relocs: Vec::new(),
        }
    }
}

/// Implementation of a trap sink that simply stores all trap info in-memory
#[derive(Default)]
struct TrapSink {
    /// The in-memory vector of trap info
    traps: Vec<TrapInformation>,
}

impl TrapSink {
    /// Create a new `TrapSink`
    fn new() -> Self {
        Self::default()
    }
}

impl binemit::TrapSink for TrapSink {
    fn trap(
        &mut self,
        code_offset: binemit::CodeOffset,
        _source_loc: ir::SourceLoc,
        trap_code: ir::TrapCode,
    ) {
        self.traps.push(TrapInformation {
            code_offset,
            trap_code,
        });
    }
}

#[derive(Default)]
struct StackMapSink {
    infos: Vec<StackMapInformation>,
}

impl binemit::StackMapSink for StackMapSink {
    fn add_stack_map(&mut self, code_offset: binemit::CodeOffset, stack_map: binemit::StackMap) {
        self.infos.push(StackMapInformation {
            code_offset,
            stack_map,
        });
    }
}

impl StackMapSink {
    fn finish(mut self) -> Vec<StackMapInformation> {
        self.infos.sort_unstable_by_key(|info| info.code_offset);
        self.infos
    }
}

fn get_function_address_map<'data>(
    context: &Context,
    data: &FunctionBodyData<'data>,
    body_len: u32,
    isa: &dyn isa::TargetIsa,
) -> FunctionAddressMap {
    // Generate artificial srcloc for function start/end to identify boundary
    // within module.
    let data = data.body.get_binary_reader();
    let offset = data.original_position();
    let len = data.bytes_remaining();
    assert!((offset + len) <= u32::max_value() as usize);
    let start_srcloc = ir::SourceLoc::new(offset as u32);
    let end_srcloc = ir::SourceLoc::new((offset + len) as u32);

    let instructions = if let Some(ref mcr) = &context.mach_compile_result {
        // New-style backend: we have a `MachCompileResult` that will give us `MachSrcLoc` mapping
        // tuples.
        collect_address_maps(
            body_len,
            mcr.buffer
                .get_srclocs_sorted()
                .into_iter()
                .map(|&MachSrcLoc { start, end, loc }| (loc, start, (end - start))),
        )
    } else {
        // Old-style backend: we need to traverse the instruction/encoding info in the function.
        let func = &context.func;
        let mut blocks = func.layout.blocks().collect::<Vec<_>>();
        blocks.sort_by_key(|block| func.offsets[*block]); // Ensure inst offsets always increase

        let encinfo = isa.encoding_info();
        collect_address_maps(
            body_len,
            blocks
                .into_iter()
                .flat_map(|block| func.inst_offsets(block, &encinfo))
                .map(|(offset, inst, size)| (func.srclocs[inst], offset, size)),
        )
    };

    FunctionAddressMap {
        instructions: instructions.into(),
        start_srcloc,
        end_srcloc,
        body_offset: 0,
        body_len,
    }
}

// Collects an iterator of `InstructionAddressMap` into a `Vec` for insertion
// into a `FunctionAddressMap`. This will automatically coalesce adjacent
// instructions which map to the same original source position.
fn collect_address_maps(
    code_size: u32,
    iter: impl IntoIterator<Item = (ir::SourceLoc, u32, u32)>,
) -> Vec<InstructionAddressMap> {
    let mut iter = iter.into_iter();
    let (mut cur_loc, mut cur_offset, mut cur_len) = match iter.next() {
        Some(i) => i,
        None => return Vec::new(),
    };
    let mut ret = Vec::new();
    for (loc, offset, len) in iter {
        // If this instruction is adjacent to the previous and has the same
        // source location then we can "coalesce" it with the current
        // instruction.
        if cur_offset + cur_len == offset && loc == cur_loc {
            cur_len += len;
            continue;
        }

        // Push an entry for the previous source item.
        ret.push(InstructionAddressMap {
            srcloc: cur_loc,
            code_offset: cur_offset,
        });
        // And push a "dummy" entry if necessary to cover the span of ranges,
        // if any, between the previous source offset and this one.
        if cur_offset + cur_len != offset {
            ret.push(InstructionAddressMap {
                srcloc: ir::SourceLoc::default(),
                code_offset: cur_offset + cur_len,
            });
        }
        // Update our current location to get extended later or pushed on at
        // the end.
        cur_loc = loc;
        cur_offset = offset;
        cur_len = len;
    }
    ret.push(InstructionAddressMap {
        srcloc: cur_loc,
        code_offset: cur_offset,
    });
    if cur_offset + cur_len != code_size {
        ret.push(InstructionAddressMap {
            srcloc: ir::SourceLoc::default(),
            code_offset: cur_offset + cur_len,
        });
    }

    return ret;
}

/// A compiler that compiles a WebAssembly module with Cranelift, translating the Wasm to Cranelift IR,
/// optimizing it and then translating to assembly.
#[derive(Default)]
pub struct Cranelift {
    translators: Mutex<Vec<FuncTranslator>>,
}

impl Cranelift {
    fn take_translator(&self) -> FuncTranslator {
        let candidate = self.translators.lock().unwrap().pop();
        candidate.unwrap_or_else(FuncTranslator::new)
    }

    fn save_translator(&self, translator: FuncTranslator) {
        self.translators.lock().unwrap().push(translator);
    }
}

impl Compiler for Cranelift {
    fn compile_function(
        &self,
        translation: &ModuleTranslation<'_>,
        func_index: DefinedFuncIndex,
        mut input: FunctionBodyData<'_>,
        isa: &dyn isa::TargetIsa,
        tunables: &Tunables,
        types: &TypeTables,
    ) -> Result<CompiledFunction, CompileError> {
        let module = &translation.module;
        let func_index = module.func_index(func_index);
        let mut context = Context::new();
        context.func.name = get_func_name(func_index);
        context.func.signature = func_signature(isa, module, types, func_index);
        if tunables.generate_native_debuginfo {
            context.func.collect_debug_info();
        }

        let mut func_env = FuncEnvironment::new(isa, module, types, tunables);

        // We use these as constant offsets below in
        // `stack_limit_from_arguments`, so assert their values here. This
        // allows the closure below to get coerced to a function pointer, as
        // needed by `ir::Function`.
        //
        // Otherwise our stack limit is specially calculated from the vmctx
        // argument, where we need to load the `*const VMInterrupts`
        // pointer, and then from that pointer we need to load the stack
        // limit itself. Note that manual register allocation is needed here
        // too due to how late in the process this codegen happens.
        //
        // For more information about interrupts and stack checks, see the
        // top of this file.
        let vmctx = context
            .func
            .create_global_value(ir::GlobalValueData::VMContext);
        let interrupts_ptr = context.func.create_global_value(ir::GlobalValueData::Load {
            base: vmctx,
            offset: i32::try_from(func_env.offsets.vmctx_interrupts())
                .unwrap()
                .into(),
            global_type: isa.pointer_type(),
            readonly: true,
        });
        let stack_limit = context.func.create_global_value(ir::GlobalValueData::Load {
            base: interrupts_ptr,
            offset: i32::try_from(func_env.offsets.vminterrupts_stack_limit())
                .unwrap()
                .into(),
            global_type: isa.pointer_type(),
            readonly: false,
        });
        context.func.stack_limit = Some(stack_limit);
        let mut func_translator = self.take_translator();
        func_translator.translate_body(
            &mut input.validator,
            input.body.clone(),
            &mut context.func,
            &mut func_env,
        )?;
        self.save_translator(func_translator);

        let mut code_buf: Vec<u8> = Vec::new();
        let mut reloc_sink = RelocSink::new(func_index);
        let mut trap_sink = TrapSink::new();
        let mut stack_map_sink = StackMapSink::default();
        context
            .compile_and_emit(
                isa,
                &mut code_buf,
                &mut reloc_sink,
                &mut trap_sink,
                &mut stack_map_sink,
            )
            .map_err(|error| {
                CompileError::Codegen(pretty_error(&context.func, Some(isa), error))
            })?;

        let unwind_info = context.create_unwind_info(isa).map_err(|error| {
            CompileError::Codegen(pretty_error(&context.func, Some(isa), error))
        })?;

        let address_transform =
            get_function_address_map(&context, &input, code_buf.len() as u32, isa);

        let ranges = if tunables.generate_native_debuginfo {
            let ranges = context.build_value_labels_ranges(isa).map_err(|error| {
                CompileError::Codegen(pretty_error(&context.func, Some(isa), error))
            })?;
            Some(ranges)
        } else {
            None
        };

        Ok(CompiledFunction {
            body: code_buf,
            jt_offsets: context.func.jt_offsets,
            relocations: reloc_sink.func_relocs,
            address_map: address_transform,
            value_labels_ranges: ranges.unwrap_or(Default::default()),
            stack_slots: context.func.stack_slots,
            traps: trap_sink.traps,
            unwind_info,
            stack_maps: stack_map_sink.finish(),
        })
    }

    fn host_to_wasm_trampoline(
        &self,
        isa: &dyn isa::TargetIsa,
        ty: &WasmFuncType,
    ) -> Result<CompiledFunction, CompileError> {
        let value_size = mem::size_of::<u128>();
        let pointer_type = isa.pointer_type();

        // The wasm signature we're calling in this trampoline has the actual
        // ABI of the function signature described by `ty`
        let wasm_signature = indirect_signature(isa, ty);

        // The host signature has the `VMTrampoline` signature where the ABI is
        // fixed.
        let mut host_signature = blank_sig(isa, wasmtime_call_conv(isa));
        host_signature.params.push(ir::AbiParam::new(pointer_type));
        host_signature.params.push(ir::AbiParam::new(pointer_type));

        let mut func_translator = self.take_translator();
        let mut context = Context::new();
        context.func = ir::Function::with_name_signature(ExternalName::user(0, 0), host_signature);

        // This trampoline will load all the parameters from the `values_vec`
        // that is passed in and then call the real function (also passed
        // indirectly) with the specified ABI.
        //
        // All the results are then stored into the same `values_vec`.
        let mut builder = FunctionBuilder::new(&mut context.func, func_translator.context());
        let block0 = builder.create_block();

        builder.append_block_params_for_function_params(block0);
        builder.switch_to_block(block0);
        builder.seal_block(block0);

        let (vmctx_ptr_val, caller_vmctx_ptr_val, callee_value, values_vec_ptr_val) = {
            let params = builder.func.dfg.block_params(block0);
            (params[0], params[1], params[2], params[3])
        };

        // Load the argument values out of `values_vec`.
        let mflags = ir::MemFlags::trusted();
        let callee_args = wasm_signature
            .params
            .iter()
            .enumerate()
            .map(|(i, r)| {
                match i {
                    0 => vmctx_ptr_val,
                    1 => caller_vmctx_ptr_val,
                    _ =>
                    // i - 2 because vmctx and caller vmctx aren't passed through `values_vec`.
                    {
                        builder.ins().load(
                            r.value_type,
                            mflags,
                            values_vec_ptr_val,
                            ((i - 2) * value_size) as i32,
                        )
                    }
                }
            })
            .collect::<Vec<_>>();

        // Call the indirect function pointer we were given
        let new_sig = builder.import_signature(wasm_signature);
        let call = builder
            .ins()
            .call_indirect(new_sig, callee_value, &callee_args);
        let results = builder.func.dfg.inst_results(call).to_vec();

        // Store the return values into `values_vec`.
        let mflags = ir::MemFlags::trusted();
        for (i, r) in results.iter().enumerate() {
            builder
                .ins()
                .store(mflags, *r, values_vec_ptr_val, (i * value_size) as i32);
        }
        builder.ins().return_(&[]);
        builder.finalize();

        let func = self.finish_trampoline(context, isa)?;
        self.save_translator(func_translator);
        Ok(func)
    }

    fn wasm_to_host_trampoline(
        &self,
        isa: &dyn isa::TargetIsa,
        ty: &WasmFuncType,
        host_fn: usize,
    ) -> Result<CompiledFunction, CompileError> {
        let pointer_type = isa.pointer_type();
        let wasm_signature = indirect_signature(isa, ty);
        // The host signature has an added parameter for the `values_vec` input
        // and output.
        let mut host_signature = blank_sig(isa, wasmtime_call_conv(isa));
        host_signature.params.push(ir::AbiParam::new(pointer_type));

        // Compute the size of the values vector. The vmctx and caller vmctx are passed separately.
        let value_size = mem::size_of::<u128>();
        let values_vec_len = (value_size * cmp::max(ty.params.len(), ty.returns.len())) as u32;

        let mut context = Context::new();
        context.func =
            ir::Function::with_name_signature(ir::ExternalName::user(0, 0), wasm_signature);

        let ss = context.func.create_stack_slot(ir::StackSlotData::new(
            ir::StackSlotKind::ExplicitSlot,
            values_vec_len,
        ));

        let mut func_translator = self.take_translator();
        let mut builder = FunctionBuilder::new(&mut context.func, func_translator.context());
        let block0 = builder.create_block();

        builder.append_block_params_for_function_params(block0);
        builder.switch_to_block(block0);
        builder.seal_block(block0);

        let values_vec_ptr_val = builder.ins().stack_addr(pointer_type, ss, 0);
        let mflags = MemFlags::trusted();
        for i in 0..ty.params.len() {
            let val = builder.func.dfg.block_params(block0)[i + 2];
            builder
                .ins()
                .store(mflags, val, values_vec_ptr_val, (i * value_size) as i32);
        }

        let block_params = builder.func.dfg.block_params(block0);
        let vmctx_ptr_val = block_params[0];
        let caller_vmctx_ptr_val = block_params[1];

        let callee_args = vec![vmctx_ptr_val, caller_vmctx_ptr_val, values_vec_ptr_val];

        let new_sig = builder.import_signature(host_signature);

        let callee_value = builder.ins().iconst(pointer_type, host_fn as i64);
        builder
            .ins()
            .call_indirect(new_sig, callee_value, &callee_args);

        let mflags = MemFlags::trusted();
        let mut results = Vec::new();
        for (i, r) in ty.returns.iter().enumerate() {
            let load = builder.ins().load(
                value_type(isa, *r),
                mflags,
                values_vec_ptr_val,
                (i * value_size) as i32,
            );
            results.push(load);
        }
        builder.ins().return_(&results);
        builder.finalize();

        let func = self.finish_trampoline(context, isa)?;
        self.save_translator(func_translator);
        Ok(func)
    }
}

impl Cranelift {
    fn finish_trampoline(
        &self,
        mut context: Context,
        isa: &dyn TargetIsa,
    ) -> Result<CompiledFunction, CompileError> {
        let mut code_buf = Vec::new();
        let mut reloc_sink = TrampolineRelocSink::default();
        let mut trap_sink = binemit::NullTrapSink {};
        let mut stack_map_sink = binemit::NullStackMapSink {};
        context
            .compile_and_emit(
                isa,
                &mut code_buf,
                &mut reloc_sink,
                &mut trap_sink,
                &mut stack_map_sink,
            )
            .map_err(|error| {
                CompileError::Codegen(pretty_error(&context.func, Some(isa), error))
            })?;

        let unwind_info = context.create_unwind_info(isa).map_err(|error| {
            CompileError::Codegen(pretty_error(&context.func, Some(isa), error))
        })?;

        Ok(CompiledFunction {
            body: code_buf,
            jt_offsets: context.func.jt_offsets,
            unwind_info,
            relocations: reloc_sink.relocs,
            stack_maps: Default::default(),
            stack_slots: Default::default(),
            traps: Default::default(),
            value_labels_ranges: Default::default(),
            address_map: Default::default(),
        })
    }
}

pub fn blank_sig(isa: &dyn TargetIsa, call_conv: CallConv) -> ir::Signature {
    let pointer_type = isa.pointer_type();
    let mut sig = ir::Signature::new(call_conv);
    // Add the caller/callee `vmctx` parameters.
    sig.params.push(ir::AbiParam::special(
        pointer_type,
        ir::ArgumentPurpose::VMContext,
    ));
    sig.params.push(ir::AbiParam::new(pointer_type));
    return sig;
}

pub fn wasmtime_call_conv(isa: &dyn TargetIsa) -> CallConv {
    match isa.triple().default_calling_convention() {
        Ok(CallingConvention::AppleAarch64) => CallConv::WasmtimeAppleAarch64,
        Ok(CallingConvention::SystemV) | Err(()) => CallConv::WasmtimeSystemV,
        Ok(CallingConvention::WindowsFastcall) => CallConv::WasmtimeFastcall,
        Ok(unimp) => unimplemented!("calling convention: {:?}", unimp),
    }
}

pub fn push_types(isa: &dyn TargetIsa, sig: &mut ir::Signature, wasm: &WasmFuncType) {
    let cvt = |ty: &WasmType| ir::AbiParam::new(value_type(isa, *ty));
    sig.params.extend(wasm.params.iter().map(&cvt));
    sig.returns.extend(wasm.returns.iter().map(&cvt));
}

fn value_type(isa: &dyn TargetIsa, ty: WasmType) -> ir::types::Type {
    match ty {
        WasmType::I32 => ir::types::I32,
        WasmType::I64 => ir::types::I64,
        WasmType::F32 => ir::types::F32,
        WasmType::F64 => ir::types::F64,
        WasmType::V128 => ir::types::I8X16,
        WasmType::FuncRef | WasmType::ExternRef => {
            wasmtime_environ::reference_type(ty, isa.pointer_type())
        }
        WasmType::ExnRef => unimplemented!(),
    }
}

pub fn indirect_signature(isa: &dyn TargetIsa, wasm: &WasmFuncType) -> ir::Signature {
    let mut sig = blank_sig(isa, wasmtime_call_conv(isa));
    push_types(isa, &mut sig, wasm);
    return sig;
}

pub fn func_signature(
    isa: &dyn TargetIsa,
    module: &Module,
    types: &TypeTables,
    index: FuncIndex,
) -> ir::Signature {
    let call_conv = match module.defined_func_index(index) {
        // If this is a defined function in the module and it's never possibly
        // exported, then we can optimize this function to use the fastest
        // calling convention since it's purely an internal implementation
        // detail of the module itself.
        Some(idx) if !module.possibly_exported_funcs.contains(&idx) => CallConv::Fast,

        // ... otherwise if it's an imported function or if it's a possibly
        // exported function then we use the default ABI wasmtime would
        // otherwise select.
        _ => wasmtime_call_conv(isa),
    };
    let mut sig = blank_sig(isa, call_conv);
    push_types(
        isa,
        &mut sig,
        &types.wasm_signatures[module.functions[index]],
    );
    return sig;
}

/// We don't expect trampoline compilation to produce many relocations, so
/// this `RelocSink` just asserts that it doesn't recieve most of them, but
/// handles libcall ones.
#[derive(Default)]
struct TrampolineRelocSink {
    relocs: Vec<Relocation>,
}

impl binemit::RelocSink for TrampolineRelocSink {
    fn reloc_external(
        &mut self,
        offset: binemit::CodeOffset,
        _srcloc: ir::SourceLoc,
        reloc: binemit::Reloc,
        name: &ir::ExternalName,
        addend: binemit::Addend,
    ) {
        let reloc_target = if let ir::ExternalName::LibCall(libcall) = *name {
            RelocationTarget::LibCall(libcall)
        } else {
            panic!("unrecognized external name")
        };
        self.relocs.push(Relocation {
            reloc,
            reloc_target,
            offset,
            addend,
        });
    }
    fn reloc_constant(
        &mut self,
        _code_offset: binemit::CodeOffset,
        _reloc: binemit::Reloc,
        _constant_offset: ir::ConstantOffset,
    ) {
        panic!("trampoline compilation should not produce constant relocs");
    }
    fn reloc_jt(
        &mut self,
        _offset: binemit::CodeOffset,
        _reloc: binemit::Reloc,
        _jt: ir::JumpTable,
    ) {
        panic!("trampoline compilation should not produce jump table relocs");
    }
}
