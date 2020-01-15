//! Support for compiling with Cranelift.

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

use crate::address_map::{FunctionAddressMap, InstructionAddressMap};
use crate::cache::{ModuleCacheDataTupleType, ModuleCacheEntry};
use crate::compilation::{
    Compilation, CompileError, CompiledFunction, Relocation, RelocationTarget, TrapInformation,
};
use crate::func_environ::{get_func_name, FuncEnvironment};
use crate::{CacheConfig, FunctionBodyData, ModuleLocal, ModuleTranslation, Tunables};
use cranelift_codegen::ir::{self, ExternalName};
use cranelift_codegen::machinst::sections::MachSrcLoc;
use cranelift_codegen::print_errors::pretty_error;
use cranelift_codegen::{binemit, isa, Context};
use cranelift_entity::PrimaryMap;
use cranelift_wasm::{DefinedFuncIndex, FuncIndex, FuncTranslator, ModuleTranslationState};
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use std::convert::TryFrom;
use std::hash::{Hash, Hasher};

/// Implementation of a relocation sink that just saves all the information for later
pub struct RelocSink {
    /// Current function index.
    func_index: FuncIndex,

    /// Relocations recorded for the function.
    pub func_relocs: Vec<Relocation>,
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
    pub fn new(func_index: FuncIndex) -> Self {
        Self {
            func_index,
            func_relocs: Vec::new(),
        }
    }
}

/// Implementation of a trap sink that simply stores all trap info in-memory
#[derive(Default)]
pub struct TrapSink {
    /// The in-memory vector of trap info
    pub traps: Vec<TrapInformation>,
}

impl TrapSink {
    /// Create a new `TrapSink`
    pub fn new() -> Self {
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
        for &MachSrcLoc { start, end, loc } in mcr.sections.get_srclocs_sorted() {
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
    let start_srcloc = ir::SourceLoc::new(data.module_offset as u32);
    let end_srcloc = ir::SourceLoc::new((data.module_offset + data.data.len()) as u32);

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
pub struct Cranelift;

impl crate::compilation::Compiler for Cranelift {
    /// Compile the module using Cranelift, producing a compilation result with
    /// associated relocations.
    fn compile_module(
        translation: &ModuleTranslation,
        isa: &dyn isa::TargetIsa,
        cache_config: &CacheConfig,
    ) -> Result<ModuleCacheDataTupleType, CompileError> {
        let cache_entry = ModuleCacheEntry::new("cranelift", cache_config);

        let data = cache_entry.get_data(
            CompileEnv {
                local: &translation.module.local,
                module_translation: HashedModuleTranslationState(
                    translation.module_translation.as_ref().unwrap(),
                ),
                function_body_inputs: &translation.function_body_inputs,
                isa: Isa(isa),
                tunables: &translation.tunables,
            },
            compile,
        )?;
        Ok(data.into_tuple())
    }
}

fn compile(env: CompileEnv<'_>) -> Result<ModuleCacheDataTupleType, CompileError> {
    let Isa(isa) = env.isa;
    let mut functions = PrimaryMap::with_capacity(env.function_body_inputs.len());
    let mut relocations = PrimaryMap::with_capacity(env.function_body_inputs.len());
    let mut address_transforms = PrimaryMap::with_capacity(env.function_body_inputs.len());
    let mut value_ranges = PrimaryMap::with_capacity(env.function_body_inputs.len());
    let mut stack_slots = PrimaryMap::with_capacity(env.function_body_inputs.len());
    let mut traps = PrimaryMap::with_capacity(env.function_body_inputs.len());

    env.function_body_inputs
        .into_iter()
        .collect::<Vec<(DefinedFuncIndex, &FunctionBodyData<'_>)>>()
        .par_iter()
        .map_init(FuncTranslator::new, |func_translator, (i, input)| {
            let func_index = env.local.func_index(*i);
            let mut context = Context::new();
            context.func.name = get_func_name(func_index);
            context.func.signature = env.local.func_signature(func_index).clone();
            if env.tunables.debug_info {
                context.func.collect_debug_info();
            }

            let mut func_env = FuncEnvironment::new(isa.frontend_config(), env.local, env.tunables);

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
            func_translator.translate(
                env.module_translation.0,
                input.data,
                input.module_offset,
                &mut context.func,
                &mut func_env,
            )?;

            let mut code_buf: Vec<u8> = Vec::new();
            let mut reloc_sink = RelocSink::new(func_index);
            let mut trap_sink = TrapSink::new();
            let mut stackmap_sink = binemit::NullStackmapSink {};
            context
                .compile_and_emit(
                    isa,
                    &mut code_buf,
                    &mut reloc_sink,
                    &mut trap_sink,
                    &mut stackmap_sink,
                )
                .map_err(|error| {
                    CompileError::Codegen(pretty_error(&context.func, Some(isa), error))
                })?;

            let unwind_info = context.create_unwind_info(isa).map_err(|error| {
                CompileError::Codegen(pretty_error(&context.func, Some(isa), error))
            })?;

            let address_transform = get_function_address_map(&context, input, code_buf.len(), isa);

            let ranges = if env.tunables.debug_info {
                let ranges = context.build_value_labels_ranges(isa).map_err(|error| {
                    CompileError::Codegen(pretty_error(&context.func, Some(isa), error))
                })?;
                Some(ranges)
            } else {
                None
            };

            Ok((
                code_buf,
                context.func.jt_offsets,
                reloc_sink.func_relocs,
                address_transform,
                ranges,
                context.func.stack_slots,
                trap_sink.traps,
                unwind_info,
            ))
        })
        .collect::<Result<Vec<_>, CompileError>>()?
        .into_iter()
        .for_each(
            |(
                function,
                func_jt_offsets,
                relocs,
                address_transform,
                ranges,
                sss,
                function_traps,
                unwind_info,
            )| {
                functions.push(CompiledFunction {
                    body: function,
                    jt_offsets: func_jt_offsets,
                    unwind_info,
                });
                relocations.push(relocs);
                address_transforms.push(address_transform);
                value_ranges.push(ranges.unwrap_or_default());
                stack_slots.push(sss);
                traps.push(function_traps);
            },
        );

    // TODO: Reorganize where we create the Vec for the resolved imports.

    Ok((
        Compilation::new(functions),
        relocations,
        address_transforms,
        value_ranges,
        stack_slots,
        traps,
    ))
}

#[derive(Hash)]
struct CompileEnv<'a> {
    local: &'a ModuleLocal,
    module_translation: HashedModuleTranslationState<'a>,
    function_body_inputs: &'a PrimaryMap<DefinedFuncIndex, FunctionBodyData<'a>>,
    isa: Isa<'a, 'a>,
    tunables: &'a Tunables,
}

/// This is a wrapper struct to hash the specific bits of `TargetIsa` that
/// affect the output we care about. The trait itself can't implement `Hash`
/// (it's not object safe) so we have to implement our own hashing.
struct Isa<'a, 'b>(&'a (dyn isa::TargetIsa + 'b));

impl Hash for Isa<'_, '_> {
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        self.0.triple().hash(hasher);
        self.0.frontend_config().hash(hasher);

        // TODO: if this `to_string()` is too expensive then we should upstream
        // a native hashing ability of flags into cranelift itself, but
        // compilation and/or cache loading is relatively expensive so seems
        // unlikely.
        self.0.flags().to_string().hash(hasher);

        // TODO: ... and should we hash anything else? There's a lot of stuff in
        // `TargetIsa`, like registers/encodings/etc. Should we be hashing that
        // too? It seems like wasmtime doesn't configure it too too much, but
        // this may become an issue at some point.
    }
}

/// A wrapper struct around cranelift's `ModuleTranslationState` to implement
/// `Hash` since it's not `Hash` upstream yet.
///
/// TODO: we should upstream a `Hash` implementation, it would be very small! At
/// this moment though based on the definition it should be fine to not hash it
/// since we'll re-hash the signatures later.
struct HashedModuleTranslationState<'a>(&'a ModuleTranslationState);

impl Hash for HashedModuleTranslationState<'_> {
    fn hash<H: Hasher>(&self, _hasher: &mut H) {
        // nothing to hash right now
    }
}
