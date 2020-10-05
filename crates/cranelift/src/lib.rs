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
use cranelift_codegen::ir::{self, ExternalName};
use cranelift_codegen::machinst::buffer::MachSrcLoc;
use cranelift_codegen::print_errors::pretty_error;
use cranelift_codegen::{binemit, isa, Context};
use cranelift_wasm::{DefinedFuncIndex, FuncIndex, FuncTranslator};
use std::convert::TryFrom;
use std::sync::Mutex;
use wasmtime_environ::{
    CompileError, CompiledFunction, Compiler, FunctionAddressMap, FunctionBodyData,
    InstructionAddressMap, ModuleTranslation, Relocation, RelocationTarget, StackMapInformation,
    TrapInformation,
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
    fn reloc_block(
        &mut self,
        _offset: binemit::CodeOffset,
        _reloc: binemit::Reloc,
        _block_offset: binemit::CodeOffset,
    ) {
        // This should use the `offsets` field of `ir::Function`.
        panic!("block headers not yet implemented");
    }
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
        source_loc: ir::SourceLoc,
        trap_code: ir::TrapCode,
    ) {
        self.traps.push(TrapInformation {
            code_offset,
            source_loc,
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
    body_len: usize,
    isa: &dyn isa::TargetIsa,
) -> FunctionAddressMap {
    let mut instructions = Vec::new();

    if let Some(ref mcr) = &context.mach_compile_result {
        // New-style backend: we have a `MachCompileResult` that will give us `MachSrcLoc` mapping
        // tuples.
        for &MachSrcLoc { start, end, loc } in mcr.buffer.get_srclocs_sorted() {
            instructions.push(InstructionAddressMap {
                srcloc: loc,
                code_offset: start as usize,
                code_len: (end - start) as usize,
            });
        }
    } else {
        // Old-style backend: we need to traverse the instruction/encoding info in the function.
        let func = &context.func;
        let mut blocks = func.layout.blocks().collect::<Vec<_>>();
        blocks.sort_by_key(|block| func.offsets[*block]); // Ensure inst offsets always increase

        let encinfo = isa.encoding_info();
        for block in blocks {
            for (offset, inst, size) in func.inst_offsets(block, &encinfo) {
                let srcloc = func.srclocs[inst];
                instructions.push(InstructionAddressMap {
                    srcloc,
                    code_offset: offset as usize,
                    code_len: size as usize,
                });
            }
        }
    }

    // Generate artificial srcloc for function start/end to identify boundary
    // within module. Similar to FuncTranslator::cur_srcloc(): it will wrap around
    // if byte code is larger than 4 GB.
    let data = data.body.get_binary_reader();
    let offset = data.original_position();
    let len = data.bytes_remaining();
    let start_srcloc = ir::SourceLoc::new(offset as u32);
    let end_srcloc = ir::SourceLoc::new((offset + len) as u32);

    FunctionAddressMap {
        instructions,
        start_srcloc,
        end_srcloc,
        body_offset: 0,
        body_len,
    }
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
    ) -> Result<CompiledFunction, CompileError> {
        let module = &translation.module;
        let tunables = &translation.tunables;
        let func_index = module.func_index(func_index);
        let mut context = Context::new();
        context.func.name = get_func_name(func_index);
        context.func.signature = module.native_func_signature(func_index).clone();
        if tunables.debug_info {
            context.func.collect_debug_info();
        }

        let mut func_env = FuncEnvironment::new(isa.frontend_config(), module, tunables);

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
        let result = func_translator.translate_body(
            &mut input.validator,
            input.body.clone(),
            &mut context.func,
            &mut func_env,
        );
        if result.is_ok() {
            self.save_translator(func_translator);
        }
        result?;

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

        let address_transform = get_function_address_map(&context, &input, code_buf.len(), isa);

        let ranges = if tunables.debug_info {
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
}
