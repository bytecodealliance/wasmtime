//! Low-level compilation of an fused adapter function.
//!
//! This module is tasked with the top-level `compile` function which creates a
//! single WebAssembly function which will perform the steps of the fused
//! adapter for an `AdapterData` provided. This is the "meat" of compilation
//! where the validation of the canonical ABI or similar all happens to
//! translate arguments from one module to another.
//!
//! ## Traps and their ordering
//!
//! Currently this compiler is pretty "loose" about the ordering of precisely
//! what trap happens where. The main reason for this is that to core wasm all
//! traps are the same and for fused adapters if a trap happens no intermediate
//! side effects are visible (as designed by the canonical ABI itself). For this
//! it's important to note that some of the precise choices of control flow here
//! can be somewhat arbitrary, an intentional decision.

use crate::component::{
    InterfaceType, StringEncoding, TypeEnumIndex, TypeExpectedIndex, TypeFlagsIndex,
    TypeInterfaceIndex, TypeRecordIndex, TypeTupleIndex, TypeUnionIndex, TypeVariantIndex,
    FLAG_MAY_ENTER, FLAG_MAY_LEAVE, MAX_FLAT_PARAMS, MAX_FLAT_RESULTS,
};
use crate::fact::core_types::CoreTypes;
use crate::fact::signature::{align_to, Signature};
use crate::fact::transcode::{FixedEncoding as FE, Transcode, Transcoder, Transcoders};
use crate::fact::traps::Trap;
use crate::fact::{AdapterData, Context, Module, Options};
use crate::GlobalIndex;
use std::collections::HashMap;
use std::mem;
use std::ops::Range;
use wasm_encoder::{BlockType, Encode, Instruction, Instruction::*, MemArg, ValType};
use wasmtime_component_util::{DiscriminantSize, FlagsSize};

struct Compiler<'a, 'b> {
    /// The module that the adapter will eventually be inserted into.
    module: &'a Module<'a>,

    /// The type section of `module`
    types: &'b mut CoreTypes,

    /// Imported functions to transcode between various string encodings.
    transcoders: &'b mut Transcoders,

    /// Metadata about the adapter that is being compiled.
    adapter: &'a AdapterData,

    /// The encoded WebAssembly function body so far, not including locals.
    code: Vec<u8>,

    /// Generated locals that this function will use.
    ///
    /// The first entry in the tuple is the number of locals and the second
    /// entry is the type of those locals. This is pushed during compilation as
    /// locals become necessary.
    locals: Vec<(u32, ValType)>,

    /// Total number of locals generated so far.
    nlocals: u32,

    /// Metadata about all `unreachable` trap instructions in this function and
    /// what the trap represents. The offset within `self.code` is recorded as
    /// well.
    traps: Vec<(usize, Trap)>,

    /// The function signature of the lowered half of this trampoline, or the
    /// signature of the function that's being generated.
    lower_sig: &'a Signature,

    /// The function signature of the lifted half of this trampoline, or the
    /// signature of the function that's imported the trampoline will call.
    lift_sig: &'a Signature,
}

pub(super) fn compile(
    module: &Module<'_>,
    types: &mut CoreTypes,
    transcoders: &mut Transcoders,
    adapter: &AdapterData,
) -> (Vec<u8>, Vec<(usize, Trap)>) {
    let lower_sig = &module.signature(&adapter.lower, Context::Lower);
    let lift_sig = &module.signature(&adapter.lift, Context::Lift);
    Compiler {
        module,
        types,
        adapter,
        transcoders,
        code: Vec::new(),
        locals: Vec::new(),
        nlocals: lower_sig.params.len() as u32,
        traps: Vec::new(),
        lower_sig,
        lift_sig,
    }
    .compile()
}

/// Possible ways that a interface value is represented in the core wasm
/// canonical ABI.
enum Source<'a> {
    /// This value is stored on the "stack" in wasm locals.
    ///
    /// This could mean that it's inline from the parameters to the function or
    /// that after a function call the results were stored in locals and the
    /// locals are the inline results.
    Stack(Stack<'a>),

    /// This value is stored in linear memory described by the `Memory`
    /// structure.
    Memory(Memory<'a>),
}

/// Same as `Source` but for where values are translated into.
enum Destination<'a> {
    /// This value is destined for the WebAssembly stack which means that
    /// results are simply pushed as we go along.
    ///
    /// The types listed are the types that are expected to be on the stack at
    /// the end of translation.
    Stack(&'a [ValType], &'a Options),

    /// This value is to be placed in linear memory described by `Memory`.
    Memory(Memory<'a>),
}

struct Stack<'a> {
    /// The locals that comprise a particular value.
    ///
    /// The length of this list represents the flattened list of types that make
    /// up the component value. Each list has the index of the local being
    /// accessed as well as the type of the local itself.
    locals: &'a [(u32, ValType)],
    /// The lifting/lowering options for where this stack of values comes from
    opts: &'a Options,
}

/// Representation of where a value is going to be stored in linear memory.
struct Memory<'a> {
    /// The lifting/lowering options with memory configuration
    opts: &'a Options,
    /// The index of the local that contains the base address of where the
    /// storage is happening.
    addr_local: u32,
    /// A "static" offset that will be baked into wasm instructions for where
    /// memory loads/stores happen.
    offset: u32,
}

impl Compiler<'_, '_> {
    fn compile(&mut self) -> (Vec<u8>, Vec<(usize, Trap)>) {
        // Check the instance flags required for this trampoline.
        //
        // This inserts the initial check required by `canon_lower` that the
        // caller instance can be left and additionally checks the
        // flags on the callee if necessary whether it can be entered.
        self.trap_if_not_flag(self.adapter.lower.flags, FLAG_MAY_LEAVE, Trap::CannotLeave);
        if self.adapter.called_as_export {
            self.trap_if_not_flag(self.adapter.lift.flags, FLAG_MAY_ENTER, Trap::CannotEnter);
            self.set_flag(self.adapter.lift.flags, FLAG_MAY_ENTER, false);
        } else if self.module.debug {
            self.assert_not_flag(
                self.adapter.lift.flags,
                FLAG_MAY_ENTER,
                "may_enter should be unset",
            );
        }

        // Perform the translation of arguments. Note that `FLAG_MAY_LEAVE` is
        // cleared around this invocation for the callee as per the
        // `canon_lift` definition in the spec. Additionally note that the
        // precise ordering of traps here is not required since internal state
        // is not visible to either instance and a trap will "lock down" both
        // instances to no longer be visible. This means that we're free to
        // reorder lifts/lowers and flags and such as is necessary and
        // convenient here.
        //
        // TODO: if translation doesn't actually call any functions in either
        // instance then there's no need to set/clear the flag here and that can
        // be optimized away.
        self.set_flag(self.adapter.lift.flags, FLAG_MAY_LEAVE, false);
        let param_locals = self
            .lower_sig
            .params
            .iter()
            .enumerate()
            .map(|(i, ty)| (i as u32, *ty))
            .collect::<Vec<_>>();
        self.translate_params(&param_locals);
        self.set_flag(self.adapter.lift.flags, FLAG_MAY_LEAVE, true);

        // With all the arguments on the stack the actual target function is
        // now invoked. The core wasm results of the function are then placed
        // into locals for result translation afterwards.
        self.instruction(Call(self.adapter.callee.as_u32()));
        let mut result_locals = Vec::with_capacity(self.lift_sig.results.len());
        for ty in self.lift_sig.results.iter().rev() {
            let local = self.gen_local(*ty);
            self.instruction(LocalSet(local));
            result_locals.push((local, *ty));
        }
        result_locals.reverse();

        // Like above during the translation of results the caller cannot be
        // left (as we might invoke things like `realloc`). Again the precise
        // order of everything doesn't matter since intermediate states cannot
        // be witnessed, hence the setting of flags here to encapsulate both
        // liftings and lowerings.
        //
        // TODO: like above the management of the `MAY_LEAVE` flag can probably
        // be elided here for "simple" results.
        self.set_flag(self.adapter.lower.flags, FLAG_MAY_LEAVE, false);
        self.translate_results(&param_locals, &result_locals);
        self.set_flag(self.adapter.lower.flags, FLAG_MAY_LEAVE, true);

        // And finally post-return state is handled here once all results/etc
        // are all translated.
        if let Some(func) = self.adapter.lift.post_return {
            for (result, _) in result_locals.iter() {
                self.instruction(LocalGet(*result));
            }
            self.instruction(Call(func.as_u32()));
        }
        if self.adapter.called_as_export {
            self.set_flag(self.adapter.lift.flags, FLAG_MAY_ENTER, true);
        }

        self.finish()
    }

    fn translate_params(&mut self, param_locals: &[(u32, ValType)]) {
        let src_tys = &self.module.types[self.adapter.lower.ty].params;
        let src_tys = src_tys.iter().map(|(_, ty)| *ty).collect::<Vec<_>>();
        let dst_tys = &self.module.types[self.adapter.lift.ty].params;
        let dst_tys = dst_tys.iter().map(|(_, ty)| *ty).collect::<Vec<_>>();

        // TODO: handle subtyping
        assert_eq!(src_tys.len(), dst_tys.len());

        let src_flat = self
            .module
            .flatten_types(&self.adapter.lower, src_tys.iter().copied());
        let dst_flat = self
            .module
            .flatten_types(&self.adapter.lift, dst_tys.iter().copied());

        let src = if src_flat.len() <= MAX_FLAT_PARAMS {
            Source::Stack(Stack {
                locals: &param_locals[..src_flat.len()],
                opts: &self.adapter.lower,
            })
        } else {
            // If there are too many parameters then that means the parameters
            // are actually a tuple stored in linear memory addressed by the
            // first parameter local.
            let (addr, ty) = param_locals[0];
            assert_eq!(ty, self.adapter.lower.ptr());
            let align = src_tys
                .iter()
                .map(|t| self.module.align(&self.adapter.lower, t))
                .max()
                .unwrap_or(1);
            Source::Memory(self.memory_operand(&self.adapter.lower, addr, align))
        };

        let dst = if dst_flat.len() <= MAX_FLAT_PARAMS {
            Destination::Stack(&dst_flat, &self.adapter.lift)
        } else {
            // If there are too many parameters then space is allocated in the
            // destination module for the parameters via its `realloc` function.
            let (size, align) = self
                .module
                .record_size_align(&self.adapter.lift, dst_tys.iter());
            let size = MallocSize::Const(size);
            Destination::Memory(self.malloc(&self.adapter.lift, size, align))
        };

        let srcs = src
            .record_field_srcs(self.module, src_tys.iter().copied())
            .zip(src_tys.iter());
        let dsts = dst
            .record_field_dsts(self.module, dst_tys.iter().copied())
            .zip(dst_tys.iter());
        for ((src, src_ty), (dst, dst_ty)) in srcs.zip(dsts) {
            self.translate(&src_ty, &src, &dst_ty, &dst);
        }

        // If the destination was linear memory instead of the stack then the
        // actual parameter that we're passing is the address of the values
        // stored, so ensure that's happening in the wasm body here.
        if let Destination::Memory(mem) = dst {
            self.instruction(LocalGet(mem.addr_local));
        }
    }

    fn translate_results(
        &mut self,
        param_locals: &[(u32, ValType)],
        result_locals: &[(u32, ValType)],
    ) {
        let src_ty = self.module.types[self.adapter.lift.ty].result;
        let dst_ty = self.module.types[self.adapter.lower.ty].result;

        let src_flat = self.module.flatten_types(&self.adapter.lift, [src_ty]);
        let dst_flat = self.module.flatten_types(&self.adapter.lower, [dst_ty]);

        let src = if src_flat.len() <= MAX_FLAT_RESULTS {
            Source::Stack(Stack {
                locals: result_locals,
                opts: &self.adapter.lift,
            })
        } else {
            // The original results to read from in this case come from the
            // return value of the function itself. The imported function will
            // return a linear memory address at which the values can be read
            // from.
            let align = self.module.align(&self.adapter.lift, &src_ty);
            assert_eq!(result_locals.len(), 1);
            let (addr, ty) = result_locals[0];
            assert_eq!(ty, self.adapter.lift.ptr());
            Source::Memory(self.memory_operand(&self.adapter.lift, addr, align))
        };

        let dst = if dst_flat.len() <= MAX_FLAT_RESULTS {
            Destination::Stack(&dst_flat, &self.adapter.lower)
        } else {
            // This is slightly different than `translate_params` where the
            // return pointer was provided by the caller of this function
            // meaning the last parameter local is a pointer into linear memory.
            let align = self.module.align(&self.adapter.lower, &dst_ty);
            let (addr, ty) = *param_locals.last().expect("no retptr");
            assert_eq!(ty, self.adapter.lower.ptr());
            Destination::Memory(self.memory_operand(&self.adapter.lower, addr, align))
        };

        self.translate(&src_ty, &src, &dst_ty, &dst);
    }

    fn translate(
        &mut self,
        src_ty: &InterfaceType,
        src: &Source<'_>,
        dst_ty: &InterfaceType,
        dst: &Destination,
    ) {
        if let Source::Memory(mem) = src {
            self.assert_aligned(src_ty, mem);
        }
        if let Destination::Memory(mem) = dst {
            self.assert_aligned(dst_ty, mem);
        }
        match src_ty {
            InterfaceType::Unit => self.translate_unit(src, dst_ty, dst),
            InterfaceType::Bool => self.translate_bool(src, dst_ty, dst),
            InterfaceType::U8 => self.translate_u8(src, dst_ty, dst),
            InterfaceType::S8 => self.translate_s8(src, dst_ty, dst),
            InterfaceType::U16 => self.translate_u16(src, dst_ty, dst),
            InterfaceType::S16 => self.translate_s16(src, dst_ty, dst),
            InterfaceType::U32 => self.translate_u32(src, dst_ty, dst),
            InterfaceType::S32 => self.translate_s32(src, dst_ty, dst),
            InterfaceType::U64 => self.translate_u64(src, dst_ty, dst),
            InterfaceType::S64 => self.translate_s64(src, dst_ty, dst),
            InterfaceType::Float32 => self.translate_f32(src, dst_ty, dst),
            InterfaceType::Float64 => self.translate_f64(src, dst_ty, dst),
            InterfaceType::Char => self.translate_char(src, dst_ty, dst),
            InterfaceType::String => self.translate_string(src, dst_ty, dst),
            InterfaceType::List(t) => self.translate_list(*t, src, dst_ty, dst),
            InterfaceType::Record(t) => self.translate_record(*t, src, dst_ty, dst),
            InterfaceType::Flags(f) => self.translate_flags(*f, src, dst_ty, dst),
            InterfaceType::Tuple(t) => self.translate_tuple(*t, src, dst_ty, dst),
            InterfaceType::Variant(v) => self.translate_variant(*v, src, dst_ty, dst),
            InterfaceType::Union(u) => self.translate_union(*u, src, dst_ty, dst),
            InterfaceType::Enum(t) => self.translate_enum(*t, src, dst_ty, dst),
            InterfaceType::Option(t) => self.translate_option(*t, src, dst_ty, dst),
            InterfaceType::Expected(t) => self.translate_expected(*t, src, dst_ty, dst),
        }
    }

    fn translate_unit(&mut self, src: &Source<'_>, dst_ty: &InterfaceType, dst: &Destination) {
        // TODO: subtyping
        assert!(matches!(dst_ty, InterfaceType::Unit));
        drop((src, dst));
    }

    fn translate_bool(&mut self, src: &Source<'_>, dst_ty: &InterfaceType, dst: &Destination) {
        // TODO: subtyping
        assert!(matches!(dst_ty, InterfaceType::Bool));
        self.push_dst_addr(dst);

        // Booleans are canonicalized to 0 or 1 as they pass through the
        // component boundary, so use a `select` instruction to do so.
        self.instruction(I32Const(1));
        self.instruction(I32Const(0));
        match src {
            Source::Memory(mem) => self.i32_load8u(mem),
            Source::Stack(stack) => self.stack_get(stack, ValType::I32),
        }
        self.instruction(Select);

        match dst {
            Destination::Memory(mem) => self.i32_store8(mem),
            Destination::Stack(stack, _) => self.stack_set(stack, ValType::I32),
        }
    }

    fn translate_u8(&mut self, src: &Source<'_>, dst_ty: &InterfaceType, dst: &Destination) {
        // TODO: subtyping
        assert!(matches!(dst_ty, InterfaceType::U8));
        self.convert_u8_mask(src, dst, 0xff);
    }

    fn convert_u8_mask(&mut self, src: &Source<'_>, dst: &Destination<'_>, mask: u8) {
        self.push_dst_addr(dst);
        let mut needs_mask = true;
        match src {
            Source::Memory(mem) => {
                self.i32_load8u(mem);
                needs_mask = mask != 0xff;
            }
            Source::Stack(stack) => {
                self.stack_get(stack, ValType::I32);
            }
        }
        if needs_mask {
            self.instruction(I32Const(i32::from(mask)));
            self.instruction(I32And);
        }
        match dst {
            Destination::Memory(mem) => self.i32_store8(mem),
            Destination::Stack(stack, _) => self.stack_set(stack, ValType::I32),
        }
    }

    fn translate_s8(&mut self, src: &Source<'_>, dst_ty: &InterfaceType, dst: &Destination) {
        // TODO: subtyping
        assert!(matches!(dst_ty, InterfaceType::S8));
        self.push_dst_addr(dst);
        match src {
            Source::Memory(mem) => self.i32_load8s(mem),
            Source::Stack(stack) => {
                self.stack_get(stack, ValType::I32);
                self.instruction(I32Extend8S);
            }
        }
        match dst {
            Destination::Memory(mem) => self.i32_store8(mem),
            Destination::Stack(stack, _) => self.stack_set(stack, ValType::I32),
        }
    }

    fn translate_u16(&mut self, src: &Source<'_>, dst_ty: &InterfaceType, dst: &Destination) {
        // TODO: subtyping
        assert!(matches!(dst_ty, InterfaceType::U16));
        self.convert_u16_mask(src, dst, 0xffff);
    }

    fn convert_u16_mask(&mut self, src: &Source<'_>, dst: &Destination<'_>, mask: u16) {
        self.push_dst_addr(dst);
        let mut needs_mask = true;
        match src {
            Source::Memory(mem) => {
                self.i32_load16u(mem);
                needs_mask = mask != 0xffff;
            }
            Source::Stack(stack) => {
                self.stack_get(stack, ValType::I32);
            }
        }
        if needs_mask {
            self.instruction(I32Const(i32::from(mask)));
            self.instruction(I32And);
        }
        match dst {
            Destination::Memory(mem) => self.i32_store16(mem),
            Destination::Stack(stack, _) => self.stack_set(stack, ValType::I32),
        }
    }

    fn translate_s16(&mut self, src: &Source<'_>, dst_ty: &InterfaceType, dst: &Destination) {
        // TODO: subtyping
        assert!(matches!(dst_ty, InterfaceType::S16));
        self.push_dst_addr(dst);
        match src {
            Source::Memory(mem) => self.i32_load16s(mem),
            Source::Stack(stack) => {
                self.stack_get(stack, ValType::I32);
                self.instruction(I32Extend16S);
            }
        }
        match dst {
            Destination::Memory(mem) => self.i32_store16(mem),
            Destination::Stack(stack, _) => self.stack_set(stack, ValType::I32),
        }
    }

    fn translate_u32(&mut self, src: &Source<'_>, dst_ty: &InterfaceType, dst: &Destination) {
        // TODO: subtyping
        assert!(matches!(dst_ty, InterfaceType::U32));
        self.convert_u32_mask(src, dst, 0xffffffff)
    }

    fn convert_u32_mask(&mut self, src: &Source<'_>, dst: &Destination<'_>, mask: u32) {
        self.push_dst_addr(dst);
        match src {
            Source::Memory(mem) => self.i32_load16u(mem),
            Source::Stack(stack) => self.stack_get(stack, ValType::I32),
        }
        if mask != 0xffffffff {
            self.instruction(I32Const(mask as i32));
            self.instruction(I32And);
        }
        match dst {
            Destination::Memory(mem) => self.i32_store(mem),
            Destination::Stack(stack, _) => self.stack_set(stack, ValType::I32),
        }
    }

    fn translate_s32(&mut self, src: &Source<'_>, dst_ty: &InterfaceType, dst: &Destination) {
        // TODO: subtyping
        assert!(matches!(dst_ty, InterfaceType::S32));
        self.push_dst_addr(dst);
        match src {
            Source::Memory(mem) => self.i32_load(mem),
            Source::Stack(stack) => self.stack_get(stack, ValType::I32),
        }
        match dst {
            Destination::Memory(mem) => self.i32_store(mem),
            Destination::Stack(stack, _) => self.stack_set(stack, ValType::I32),
        }
    }

    fn translate_u64(&mut self, src: &Source<'_>, dst_ty: &InterfaceType, dst: &Destination) {
        // TODO: subtyping
        assert!(matches!(dst_ty, InterfaceType::U64));
        self.push_dst_addr(dst);
        match src {
            Source::Memory(mem) => self.i64_load(mem),
            Source::Stack(stack) => self.stack_get(stack, ValType::I64),
        }
        match dst {
            Destination::Memory(mem) => self.i64_store(mem),
            Destination::Stack(stack, _) => self.stack_set(stack, ValType::I64),
        }
    }

    fn translate_s64(&mut self, src: &Source<'_>, dst_ty: &InterfaceType, dst: &Destination) {
        // TODO: subtyping
        assert!(matches!(dst_ty, InterfaceType::S64));
        self.push_dst_addr(dst);
        match src {
            Source::Memory(mem) => self.i64_load(mem),
            Source::Stack(stack) => self.stack_get(stack, ValType::I64),
        }
        match dst {
            Destination::Memory(mem) => self.i64_store(mem),
            Destination::Stack(stack, _) => self.stack_set(stack, ValType::I64),
        }
    }

    fn translate_f32(&mut self, src: &Source<'_>, dst_ty: &InterfaceType, dst: &Destination) {
        // TODO: subtyping
        assert!(matches!(dst_ty, InterfaceType::Float32));
        self.push_dst_addr(dst);
        match src {
            Source::Memory(mem) => self.f32_load(mem),
            Source::Stack(stack) => self.stack_get(stack, ValType::F32),
        }
        match dst {
            Destination::Memory(mem) => self.f32_store(mem),
            Destination::Stack(stack, _) => self.stack_set(stack, ValType::F32),
        }
    }

    fn translate_f64(&mut self, src: &Source<'_>, dst_ty: &InterfaceType, dst: &Destination) {
        // TODO: subtyping
        assert!(matches!(dst_ty, InterfaceType::Float64));
        self.push_dst_addr(dst);
        match src {
            Source::Memory(mem) => self.f64_load(mem),
            Source::Stack(stack) => self.stack_get(stack, ValType::F64),
        }
        match dst {
            Destination::Memory(mem) => self.f64_store(mem),
            Destination::Stack(stack, _) => self.stack_set(stack, ValType::F64),
        }
    }

    fn translate_char(&mut self, src: &Source<'_>, dst_ty: &InterfaceType, dst: &Destination) {
        assert!(matches!(dst_ty, InterfaceType::Char));
        let local = self.gen_local(ValType::I32);
        match src {
            Source::Memory(mem) => self.i32_load(mem),
            Source::Stack(stack) => self.stack_get(stack, ValType::I32),
        }
        self.instruction(LocalSet(local));

        // This sequence is copied from the output of LLVM for:
        //
        //      pub extern "C" fn foo(x: u32) -> char {
        //          char::try_from(x)
        //              .unwrap_or_else(|_| std::arch::wasm32::unreachable())
        //      }
        //
        // Apparently this does what's required by the canonical ABI:
        //
        //    def i32_to_char(opts, i):
        //      trap_if(i >= 0x110000)
        //      trap_if(0xD800 <= i <= 0xDFFF)
        //      return chr(i)
        //
        // ... but I don't know how it works other than "well I trust LLVM"
        self.instruction(Block(BlockType::Empty));
        self.instruction(Block(BlockType::Empty));
        self.instruction(LocalGet(local));
        self.instruction(I32Const(0xd800));
        self.instruction(I32Xor);
        self.instruction(I32Const(-0x110000));
        self.instruction(I32Add);
        self.instruction(I32Const(-0x10f800));
        self.instruction(I32LtU);
        self.instruction(BrIf(0));
        self.instruction(LocalGet(local));
        self.instruction(I32Const(0x110000));
        self.instruction(I32Ne);
        self.instruction(BrIf(1));
        self.instruction(End);
        self.trap(Trap::InvalidChar);
        self.instruction(End);

        self.push_dst_addr(dst);
        self.instruction(LocalGet(local));
        match dst {
            Destination::Memory(mem) => {
                self.i32_store(mem);
            }
            Destination::Stack(stack, _) => self.stack_set(stack, ValType::I32),
        }
    }

    fn translate_string(&mut self, src: &Source<'_>, dst_ty: &InterfaceType, dst: &Destination) {
        assert!(matches!(dst_ty, InterfaceType::String));
        let src_opts = src.opts();
        let dst_opts = dst.opts();

        // Load the pointer/length of this string into temporary locals. These
        // will be referenced a good deal so this just makes it easier to deal
        // with them consistently below rather than trying to reload from memory
        // for example.
        let src_ptr = self.gen_local(src_opts.ptr());
        let src_len = self.gen_local(src_opts.ptr());
        match src {
            Source::Stack(s) => {
                assert_eq!(s.locals.len(), 2);
                self.stack_get(&s.slice(0..1), src_opts.ptr());
                self.instruction(LocalSet(src_ptr));
                self.stack_get(&s.slice(1..2), src_opts.ptr());
                self.instruction(LocalSet(src_len));
            }
            Source::Memory(mem) => {
                self.ptr_load(mem);
                self.instruction(LocalSet(src_ptr));
                self.ptr_load(&mem.bump(src_opts.ptr_size().into()));
                self.instruction(LocalSet(src_len));
            }
        }

        let dst_byte_len = self.gen_local(dst_opts.ptr());
        let dst_len = self.gen_local(dst_opts.ptr());

        const MAX_STRING_BYTE_LENGTH: u32 = 1 << 31;
        const UTF16_TAG: u32 = 1 << 31;

        let transcoder = |me: &mut Self, op: Transcode| {
            me.transcoders.import(
                me.types,
                Transcoder {
                    from_memory: src_opts.memory.unwrap(),
                    from_memory64: src_opts.memory64,
                    to_memory: dst_opts.memory.unwrap(),
                    to_memory64: dst_opts.memory64,
                    op,
                },
            )
        };

        let validate_string_length_u8 = |me: &mut Self, dst: u8| {
            // Check to see if the source byte length is out of bounds in
            // which case a trap is generated.
            me.instruction(LocalGet(src_len));
            let max = MAX_STRING_BYTE_LENGTH / u32::from(dst);
            me.ptr_uconst(src_opts, max);
            me.ptr_ge_u(src_opts);
            me.instruction(If(BlockType::Empty));
            me.trap(Trap::StringLengthTooBig);
            me.instruction(End);
        };

        let validate_string_length =
            |me: &mut Self, dst: FE| validate_string_length_u8(me, dst.width());

        // Corresponding function for `store_string_copy` in the spec.
        //
        // This performs a transcoding of the string with a one-pass copy from
        // the `src` encoding to the `dst` encoding. This is only possible for
        // fixed encodings where the first allocation is guaranteed to be an
        // appropriate fit so it's not suitable for all encodings.
        //
        // Imported host transcoding functions here take the src/dst pointers as
        // well as the number of code units in the source (which always matches
        // the number of code units in the destination). There is no return
        // value from the transcode function since the encoding should always
        // work on the first pass.
        let string_copy = |me: &mut Self, src: FE, dst: FE| {
            assert!(dst.width() >= src.width());
            validate_string_length(me, dst);

            // Calculate the source byte length given the size of each code
            // unit. Note that this shouldn't overflow given
            // `validate_string_length` above.
            let src_byte_len = if src.width() == 1 {
                src_len
            } else {
                assert_eq!(src.width(), 2);
                let tmp = me.gen_local(src_opts.ptr());
                me.instruction(LocalGet(src_len));
                me.ptr_uconst(src_opts, 1);
                me.ptr_shl(src_opts);
                me.instruction(LocalSet(tmp));
                tmp
            };

            // Convert the source code units length to the destination byte
            // length type.
            me.convert_src_len_to_dst(src_len, src_opts.ptr(), dst_opts.ptr());
            me.instruction(LocalTee(dst_len));
            if dst.width() > 1 {
                assert_eq!(dst.width(), 2);
                me.ptr_uconst(dst_opts, 1);
                me.ptr_shl(dst_opts);
            }
            me.instruction(LocalSet(dst_byte_len));

            // Allocate space in the destination using the calculated byte
            // length.
            let dst_mem = me.malloc(
                dst_opts,
                MallocSize::Local(dst_byte_len),
                dst.width().into(),
            );

            // Skip the `transcode` function if the byte length is zero
            // because no memory is modified and bounds checks don't matter.
            me.instruction(LocalGet(src_len));
            me.ptr_if(src_opts, BlockType::Empty);

            // Validate that `src_len + src_ptr` and
            // `dst_mem.addr_local + dst_byte_len` are both in-bounds. This
            // is done by loading the last byte of the string and if that
            // doesn't trap then it's known valid.
            me.validate_string_inbounds(src_opts, src_ptr, src_byte_len);
            me.validate_string_inbounds(dst_opts, dst_mem.addr_local, dst_byte_len);

            // If the validations pass then the host `transcode` intrinsic
            // is invoked. This will either raise a trap or otherwise succeed
            // in which case we're done.
            let op = if src == dst {
                Transcode::Copy(src)
            } else {
                assert_eq!(src, FE::Latin1);
                assert_eq!(dst, FE::Utf16);
                Transcode::Latin1ToUtf16
            };
            let transcode = transcoder(me, op);
            me.instruction(LocalGet(src_ptr));
            me.instruction(LocalGet(src_len));
            me.instruction(LocalGet(dst_mem.addr_local));
            me.instruction(Call(transcode));

            me.instruction(End); // end of "if not zero" block

            dst_mem.addr_local
        };

        // Corresponding function for `store_string_to_utf8` in the spec.
        //
        // This translation works by possibly performing a number of
        // reallocations. First a buffer of size input-code-units is used to try
        // to get the transcoding correct on the first try. If that fails the
        // maximum worst-case size is used and then that is resized down if it's
        // too large.
        //
        // The host transcoding function imported here will receive src ptr/len
        // and dst ptr/len and return how many code units were consumed on both
        // sides. The amount of code units consumed in the source dictates which
        // branches are taken in this conversion.
        let deflate_to_utf8 = |me: &mut Self, src: FE| {
            validate_string_length(me, src);

            // Optimistically assume that the code unit length of the source is
            // all that's needed in the destination. Perform that allocaiton
            // here and proceed to transcoding below.
            me.convert_src_len_to_dst(src_len, src_opts.ptr(), dst_opts.ptr());
            me.instruction(LocalTee(dst_len));
            me.instruction(LocalSet(dst_byte_len));
            let dst_mem = me.malloc(dst_opts, MallocSize::Local(dst_byte_len), 1);

            // If the string is non-empty encoding is attempted.
            me.instruction(LocalGet(src_len));
            me.ptr_if(src_opts, BlockType::Empty);

            // Ensure buffers are all in-bounds
            let src_byte_len = match src {
                FE::Latin1 => src_len,
                FE::Utf16 => {
                    let tmp = me.gen_local(src_opts.ptr());
                    me.instruction(LocalGet(src_len));
                    me.ptr_uconst(src_opts, 1);
                    me.ptr_shl(src_opts);
                    me.instruction(LocalSet(tmp));
                    tmp
                }
                FE::Utf8 => unreachable!(),
            };
            me.validate_string_inbounds(src_opts, src_ptr, src_byte_len);
            me.validate_string_inbounds(dst_opts, dst_mem.addr_local, dst_byte_len);

            // Perform the initial transcode
            let op = match src {
                FE::Latin1 => Transcode::Latin1ToUtf8,
                FE::Utf16 => Transcode::Utf16ToUtf8,
                FE::Utf8 => unreachable!(),
            };
            let transcode = transcoder(me, op);
            me.instruction(LocalGet(src_ptr));
            me.instruction(LocalGet(src_len));
            me.instruction(LocalGet(dst_mem.addr_local));
            me.instruction(LocalGet(dst_byte_len));
            me.instruction(Call(transcode));
            me.instruction(LocalSet(dst_len));
            let src_len_tmp = me.gen_local(src_opts.ptr());
            me.instruction(LocalSet(src_len_tmp));

            // Test if the source was entirely transcoded by comparing
            // `src_len_tmp`, the number of code units transcoded from the
            // source, with `src_len`, the original number of code units.
            me.instruction(LocalGet(src_len_tmp));
            me.instruction(LocalGet(src_len));
            me.ptr_ne(src_opts);
            me.instruction(If(BlockType::Empty));

            // Here a worst-case reallocation is performed to grow `dst_mem`.
            // In-line a check is also performed that the worst-case byte size
            // fits within the maximum size of strings.
            me.instruction(LocalGet(dst_mem.addr_local)); // old_ptr
            me.instruction(LocalGet(dst_byte_len)); // old_size
            me.ptr_uconst(dst_opts, 1); // align
            let factor = match src {
                FE::Latin1 => 2,
                FE::Utf16 => 3,
                _ => unreachable!(),
            };
            validate_string_length_u8(me, factor);
            me.convert_src_len_to_dst(src_len, src_opts.ptr(), dst_opts.ptr());
            me.ptr_uconst(dst_opts, factor.into());
            me.ptr_mul(dst_opts);
            me.instruction(LocalTee(dst_byte_len));
            me.instruction(Call(dst_opts.realloc.unwrap().as_u32()));
            me.instruction(LocalSet(dst_mem.addr_local));

            // Verify that the destination is still in-bounds
            me.validate_string_inbounds(dst_opts, dst_mem.addr_local, dst_byte_len);

            // Perform another round of transcoding that should be guaranteed
            // to succeed. Note that all the parameters here are offset by the
            // results of the first transcoding to only perform the remaining
            // transcode on the final units.
            me.instruction(LocalGet(src_ptr));
            me.instruction(LocalGet(src_len_tmp));
            if let FE::Utf16 = src {
                me.ptr_uconst(src_opts, 1);
                me.ptr_shl(src_opts);
            }
            me.ptr_add(src_opts);
            me.instruction(LocalGet(src_len));
            me.instruction(LocalGet(src_len_tmp));
            me.ptr_sub(src_opts);
            me.instruction(LocalGet(dst_mem.addr_local));
            me.instruction(LocalGet(dst_len));
            me.ptr_add(dst_opts);
            me.instruction(LocalGet(dst_byte_len));
            me.instruction(LocalGet(dst_len));
            me.ptr_sub(dst_opts);
            me.instruction(Call(transcode));

            // Add the second result, the amount of destination units encoded,
            // to `dst_len` so it's an accurate reflection of the final size of
            // the destination buffer.
            me.instruction(LocalGet(dst_len));
            me.ptr_add(dst_opts);
            me.instruction(LocalSet(dst_len));

            // In debug mode verify the first result consumed the entire string,
            // otherwise simply discard it.
            if me.module.debug {
                me.instruction(LocalGet(src_len));
                me.instruction(LocalGet(src_len_tmp));
                me.ptr_sub(src_opts);
                me.ptr_ne(src_opts);
                me.instruction(If(BlockType::Empty));
                me.trap(Trap::AssertFailed("should have finished encoding"));
                me.instruction(End);
            } else {
                me.instruction(Drop);
            }

            // Perform a downsizing if the worst-case size was too large
            me.instruction(LocalGet(dst_len));
            me.instruction(LocalGet(dst_byte_len));
            me.ptr_ne(dst_opts);
            me.instruction(If(BlockType::Empty));
            me.instruction(LocalGet(dst_mem.addr_local)); // old_ptr
            me.instruction(LocalGet(dst_byte_len)); // old_size
            me.ptr_uconst(dst_opts, 1); // align
            me.instruction(LocalGet(dst_len)); // new_size
            me.instruction(Call(dst_opts.realloc.unwrap().as_u32()));
            me.instruction(LocalSet(dst_mem.addr_local));
            me.instruction(End);

            // If the first transcode was enough then assert that the returned
            // amount of destination items written equals the byte size.
            if me.module.debug {
                me.instruction(Else);

                me.instruction(LocalGet(dst_len));
                me.instruction(LocalGet(dst_byte_len));
                me.ptr_ne(dst_opts);
                me.instruction(If(BlockType::Empty));
                me.trap(Trap::AssertFailed("should have finished encoding"));
                me.instruction(End);
            }

            me.instruction(End); // end of "first transcode not enough"

            me.instruction(End); // end of nonzero string length block

            dst_mem.addr_local
        };

        // Corresponds to the `store_utf8_to_utf16` function in the spec.
        //
        // When converting utf-8 to utf-16 a pessimistic allocation is
        // done which is twice the byte length of the utf-8 string.
        // The host then transcodes and returns how many code units were
        // actually used during the transcoding and if it's beneath the
        // pessimistic maximum then the buffer is reallocated down to
        // a smaller amount.
        //
        // The host-imported transcoding function takes the src/dst pointer as
        // well as the code unit size of both the source and destination. The
        // destination should always be big enough to hold the result of the
        // transcode and so the result of the host function is how many code
        // units were written to the destination.
        let utf8_to_utf16 = |me: &mut Self| {
            validate_string_length(me, FE::Utf16);
            me.convert_src_len_to_dst(src_len, src_opts.ptr(), dst_opts.ptr());
            me.ptr_uconst(dst_opts, 1);
            me.ptr_shl(dst_opts);
            me.instruction(LocalSet(dst_byte_len));
            let dst_mem = me.malloc(dst_opts, MallocSize::Local(dst_byte_len), 2);

            // If the number of original code units is non-empty then
            // the pointer/lengths are validated and then passed to the
            // host transcode.
            me.instruction(LocalGet(src_len));
            me.ptr_if(src_opts, BlockType::Empty);
            me.validate_string_inbounds(src_opts, src_ptr, src_len);
            me.validate_string_inbounds(dst_opts, dst_mem.addr_local, dst_byte_len);

            let transcode = transcoder(me, Transcode::Utf8ToUtf16);
            me.instruction(LocalGet(src_ptr));
            me.instruction(LocalGet(src_len));
            me.instruction(LocalGet(dst_mem.addr_local));
            me.instruction(Call(transcode));
            me.instruction(LocalSet(dst_len));

            // If the number of code units returned by transcode is not
            // equal to the original number of code units then
            // the buffer must be shrunk.
            //
            // Note that the byte length of the final allocation we
            // want is twice the code unit length returned by the
            // transcoding function.
            me.convert_src_len_to_dst(src_len, src_opts.ptr(), dst_opts.ptr());
            me.instruction(LocalGet(dst_len));
            me.ptr_ne(dst_opts);
            me.instruction(If(BlockType::Empty));
            me.instruction(LocalGet(dst_mem.addr_local));
            me.instruction(LocalGet(dst_byte_len));
            me.ptr_uconst(dst_opts, 2);
            me.instruction(LocalGet(dst_len));
            me.ptr_uconst(dst_opts, 1);
            me.ptr_shl(dst_opts);
            me.instruction(Call(dst_opts.realloc.unwrap().as_u32()));
            me.instruction(LocalSet(dst_mem.addr_local));
            me.instruction(End); // end of shrink-to-fit

            me.instruction(End); // end of nonzero string length
            dst_mem.addr_local
        };

        // Corresponds to `store_probably_utf16_to_latin1_or_utf16` in the spec.
        //
        // This will try to transcode the input utf16 string to utf16 in the
        // destination. If utf16 isn't needed though and latin1 could be used
        // then that's used instead and a reallocation to downsize occurs
        // afterwards.
        //
        // The host transcode function here will take the src/dst pointers as
        // well as src length. The destination byte length is twice the src code
        // unit length. The return value is the tagged length of the returned
        // string. If the upper bit is set then utf16 was used and the
        // conversion is done. If the upper bit is not set then latin1 was used
        // and a downsizing needs to happen.
        let compact_utf16_to_compact = |me: &mut Self| {
            validate_string_length(me, FE::Utf16);
            me.convert_src_len_to_dst(src_len, src_opts.ptr(), dst_opts.ptr());
            me.ptr_uconst(dst_opts, 1);
            me.ptr_shl(dst_opts);
            me.instruction(LocalSet(dst_byte_len));
            let dst_mem = me.malloc(dst_opts, MallocSize::Local(dst_byte_len), 2);

            // If the number of original code units is non-empty then
            // the pointer/lengths are validated and then passed to the
            // host transcode.
            me.instruction(LocalGet(src_len));
            me.ptr_if(src_opts, BlockType::Empty);
            me.validate_string_inbounds(src_opts, src_ptr, src_len);
            me.validate_string_inbounds(dst_opts, dst_mem.addr_local, dst_byte_len);

            let transcode = transcoder(me, Transcode::Utf16ToCompactProbablyUtf16);
            me.instruction(LocalGet(src_ptr));
            me.instruction(LocalGet(src_len));
            me.instruction(LocalGet(dst_mem.addr_local));
            me.instruction(Call(transcode));
            me.instruction(LocalSet(dst_len));

            // Assert that the untagged code unit length is the same as the
            // source code unit length.
            if me.module.debug {
                me.instruction(LocalGet(dst_len));
                me.ptr_uconst(dst_opts, !UTF16_TAG);
                me.ptr_and(dst_opts);
                me.convert_src_len_to_dst(src_len, src_opts.ptr(), dst_opts.ptr());
                me.ptr_ne(dst_opts);
                me.instruction(If(BlockType::Empty));
                me.trap(Trap::AssertFailed("expected equal code units"));
                me.instruction(End);
            }

            // If the UTF16_TAG is set then utf16 was used and the destination
            // should be appropriately sized. Bail out of the "is this string
            // empty" block and fall through otherwise to resizing.
            me.instruction(LocalGet(dst_len));
            me.ptr_uconst(dst_opts, UTF16_TAG);
            me.ptr_and(dst_opts);
            me.ptr_br_if(dst_opts, 0);

            // Here `realloc` is used to downsize the string
            me.instruction(LocalGet(dst_mem.addr_local)); // old_ptr
            me.instruction(LocalGet(dst_byte_len)); // old_size
            me.ptr_uconst(dst_opts, 2); // align
            me.instruction(LocalGet(dst_len)); // new_size
            me.instruction(Call(dst_opts.realloc.unwrap().as_u32()));
            me.instruction(LocalSet(dst_mem.addr_local));

            me.instruction(End); // end of nonzero string length
            dst_mem.addr_local
        };

        // Corresponds to `store_string_to_latin1_or_utf16` in the spec.
        //
        // This will attempt a first pass of transcoding to latin1 and on
        // failure a larger buffer is allocated for utf16 and then utf16 is
        // encoded in-place into the buffer. After either latin1 or utf16 the
        // buffer is then resized to fit the final string allocation.
        let string_to_compact = |me: &mut Self, src: FE| {
            validate_string_length(me, src);
            me.convert_src_len_to_dst(src_len, src_opts.ptr(), dst_opts.ptr());
            me.instruction(LocalTee(dst_len));
            me.instruction(LocalSet(dst_byte_len));
            let dst_mem = me.malloc(dst_opts, MallocSize::Local(dst_byte_len), 2);

            // If the number of original code units is non-empty then
            // the pointer/lengths are validated and then passed to the
            // host transcode.
            me.instruction(LocalGet(src_len));
            me.ptr_if(src_opts, BlockType::Empty);
            me.validate_string_inbounds(src_opts, src_ptr, src_len);
            me.validate_string_inbounds(dst_opts, dst_mem.addr_local, dst_byte_len);

            // Perform the initial latin1 transcode. This returns the number of
            // source code units consumed and the number of destination code
            // units (bytes) written.
            let (latin1, utf16) = match src {
                FE::Utf8 => (Transcode::Utf8ToLatin1, Transcode::Utf8ToCompactUtf16),
                FE::Utf16 => (Transcode::Utf16ToLatin1, Transcode::Utf16ToCompactUtf16),
                FE::Latin1 => unreachable!(),
            };
            let transcode_latin1 = transcoder(me, latin1);
            let transcode_utf16 = transcoder(me, utf16);
            me.instruction(LocalGet(src_ptr));
            me.instruction(LocalGet(src_len));
            me.instruction(LocalGet(dst_mem.addr_local));
            me.instruction(Call(transcode_latin1));
            me.instruction(LocalSet(dst_len));
            let src_len_tmp = me.gen_local(src_opts.ptr());
            me.instruction(LocalSet(src_len_tmp));

            // If the source was entirely consumed then the transcode completed
            // and all that's necessary is to optionally shrink the buffer.
            me.instruction(LocalGet(src_len_tmp));
            me.instruction(LocalGet(src_len));
            me.ptr_eq(src_opts);
            me.instruction(If(BlockType::Empty)); // if latin1-or-utf16 block

            // Test if the original byte length of the allocation is the same as
            // the number of written bytes, and if not then shrink the buffer
            // with a call to `realloc`.
            me.instruction(LocalGet(dst_byte_len));
            me.instruction(LocalGet(dst_len));
            me.ptr_ne(dst_opts);
            me.instruction(If(BlockType::Empty));
            me.instruction(LocalGet(dst_mem.addr_local)); // old_ptr
            me.instruction(LocalGet(dst_byte_len)); // old_size
            me.ptr_uconst(dst_opts, 2); // align
            me.instruction(LocalGet(dst_len)); // new_size
            me.instruction(Call(dst_opts.realloc.unwrap().as_u32()));
            me.instruction(LocalSet(dst_mem.addr_local));
            me.instruction(End);

            // In this block the latin1 encoding failed. The host transcode
            // returned how many units were consumed from the source and how
            // many bytes were written to the destination. Here the buffer is
            // inflated and sized and the second utf16 intrinsic is invoked to
            // perform the final inflation.
            me.instruction(Else); // else latin1-or-utf16 block

            // For utf8 validate that the inflated size is still within bounds.
            if src.width() == 1 {
                validate_string_length_u8(me, 2);
            }

            // Reallocate the buffer with twice the source code units in byte
            // size.
            me.instruction(LocalGet(dst_mem.addr_local)); // old_ptr
            me.instruction(LocalGet(dst_byte_len)); // old_size
            me.ptr_uconst(dst_opts, 2); // align
            me.convert_src_len_to_dst(src_len, src_opts.ptr(), dst_opts.ptr());
            me.ptr_uconst(dst_opts, 1);
            me.ptr_shl(dst_opts);
            me.instruction(LocalTee(dst_byte_len));
            me.instruction(Call(dst_opts.realloc.unwrap().as_u32()));
            me.instruction(LocalSet(dst_mem.addr_local));

            // Call the host utf16 transcoding function. This will inflate the
            // prior latin1 bytes and then encode the rest of the source string
            // as utf16 into the remaining space in the destination buffer.
            me.instruction(LocalGet(src_ptr));
            me.instruction(LocalGet(src_len_tmp));
            if let FE::Utf16 = src {
                me.ptr_uconst(src_opts, 1);
                me.ptr_shl(src_opts);
            }
            me.ptr_add(src_opts);
            me.instruction(LocalGet(src_len));
            me.instruction(LocalGet(src_len_tmp));
            me.ptr_sub(src_opts);
            me.instruction(LocalGet(dst_mem.addr_local));
            me.convert_src_len_to_dst(src_len, src_opts.ptr(), dst_opts.ptr());
            me.instruction(LocalGet(dst_len));
            me.instruction(Call(transcode_utf16));
            me.instruction(LocalSet(dst_len));

            // If the returned number of code units written to the destination
            // is not equal to the size of the allocation then the allocation is
            // resized down to the appropriate size.
            //
            // Note that the byte size desired is `2*dst_len` and the current
            // byte buffer size is `2*src_len` so the `2` factor isn't checked
            // here, just the lengths.
            me.instruction(LocalGet(dst_len));
            me.convert_src_len_to_dst(src_len, src_opts.ptr(), dst_opts.ptr());
            me.ptr_ne(dst_opts);
            me.instruction(If(BlockType::Empty));
            me.instruction(LocalGet(dst_mem.addr_local)); // old_ptr
            me.instruction(LocalGet(dst_byte_len)); // old_size
            me.ptr_uconst(dst_opts, 2); // align
            me.instruction(LocalGet(dst_len));
            me.ptr_uconst(dst_opts, 1);
            me.ptr_shl(dst_opts);
            me.instruction(Call(dst_opts.realloc.unwrap().as_u32()));
            me.instruction(LocalSet(dst_mem.addr_local));
            me.instruction(End);

            // Tag the returned pointer as utf16
            me.instruction(LocalGet(dst_len));
            me.ptr_uconst(dst_opts, UTF16_TAG);
            me.ptr_or(dst_opts);
            me.instruction(LocalSet(dst_len));

            me.instruction(End); // end latin1-or-utf16 block
            me.instruction(End); // end empty string block

            dst_mem.addr_local
        };

        let dst_ptr = match src_opts.string_encoding {
            StringEncoding::Utf8 => match dst_opts.string_encoding {
                StringEncoding::Utf8 => string_copy(self, FE::Utf8, FE::Utf8),
                StringEncoding::Utf16 => utf8_to_utf16(self),
                StringEncoding::CompactUtf16 => string_to_compact(self, FE::Utf8),
            },

            StringEncoding::Utf16 => {
                self.verify_aligned(src_opts, src_ptr, 2);
                match dst_opts.string_encoding {
                    StringEncoding::Utf16 => string_copy(self, FE::Utf16, FE::Utf16),
                    StringEncoding::Utf8 => deflate_to_utf8(self, FE::Utf16),
                    StringEncoding::CompactUtf16 => string_to_compact(self, FE::Utf16),
                }
            }

            StringEncoding::CompactUtf16 => {
                self.verify_aligned(src_opts, src_ptr, 2);

                // Test the tag big to see if this is a utf16 or a latin1 string
                // at runtime...
                self.instruction(LocalGet(src_len));
                self.ptr_uconst(src_opts, UTF16_TAG);
                self.ptr_and(src_opts);
                self.ptr_if(src_opts, BlockType::Empty);

                // In the utf16 block unset the upper bit from the length local
                // so further calculations have the right value. Afterwards the
                // string transcode proceeds assuming utf16.
                self.instruction(LocalGet(src_len));
                self.ptr_uconst(src_opts, UTF16_TAG);
                self.ptr_xor(src_opts);
                self.instruction(LocalSet(src_len));
                let mem1 = match dst_opts.string_encoding {
                    StringEncoding::Utf16 => string_copy(self, FE::Utf16, FE::Utf16),
                    StringEncoding::Utf8 => deflate_to_utf8(self, FE::Utf16),
                    StringEncoding::CompactUtf16 => compact_utf16_to_compact(self),
                };

                self.instruction(Else);

                // In the latin1 block the `src_len` local is already the number
                // of code units, so the string transcoding is all that needs to
                // happen.
                let mem2 = match dst_opts.string_encoding {
                    StringEncoding::Utf16 => string_copy(self, FE::Latin1, FE::Utf16),
                    StringEncoding::Utf8 => deflate_to_utf8(self, FE::Latin1),
                    StringEncoding::CompactUtf16 => string_copy(self, FE::Latin1, FE::Latin1),
                };
                // Set our `mem2` generated local to the `mem1` generated local
                // as the resulting pointer of this transcode.
                self.instruction(LocalGet(mem2));
                self.instruction(LocalSet(mem1));
                self.instruction(End);
                mem1
            }
        };

        // Store the ptr/length in the desired destination
        match dst {
            Destination::Stack(s, _) => {
                self.instruction(LocalGet(dst_ptr));
                self.stack_set(&s[..1], dst_opts.ptr());
                self.instruction(LocalGet(dst_len));
                self.stack_set(&s[1..], dst_opts.ptr());
            }
            Destination::Memory(mem) => {
                self.instruction(LocalGet(mem.addr_local));
                self.instruction(LocalGet(dst_ptr));
                self.ptr_store(mem);
                self.instruction(LocalGet(mem.addr_local));
                self.instruction(LocalGet(dst_len));
                self.ptr_store(&mem.bump(dst_opts.ptr_size().into()));
            }
        }
    }

    fn validate_string_inbounds(&mut self, opts: &Options, ptr_local: u32, len_local: u32) {
        let tmp = self.gen_local(opts.ptr());

        // Add the ptr/len and save that into a local. If the result is less
        // than the original pointer then wraparound occurred and this should
        // trap.
        self.instruction(LocalGet(ptr_local));
        self.instruction(LocalGet(len_local));
        self.ptr_add(opts);
        self.instruction(LocalTee(tmp));
        self.instruction(LocalGet(ptr_local));
        self.ptr_lt_u(opts);
        self.instruction(If(BlockType::Empty));
        self.trap(Trap::StringLengthOverflow);
        self.instruction(End);

        // If we didn't wrap around then load the final byte of the string which
        // will trap if the string is itself out of bounds.
        self.instruction(LocalGet(tmp));
        self.ptr_iconst(opts, -1);
        self.ptr_add(opts);
        self.instruction(I32Load8_U(MemArg {
            offset: 0,
            align: 0,
            memory_index: opts.memory.unwrap().as_u32(),
        }));
        self.instruction(Drop);
    }

    fn translate_list(
        &mut self,
        src_ty: TypeInterfaceIndex,
        src: &Source<'_>,
        dst_ty: &InterfaceType,
        dst: &Destination,
    ) {
        let src_element_ty = &self.module.types[src_ty];
        let dst_element_ty = match dst_ty {
            InterfaceType::List(r) => &self.module.types[*r],
            _ => panic!("expected a list"),
        };
        let src_opts = src.opts();
        let dst_opts = dst.opts();
        let (src_size, src_align) = self.module.size_align(src_opts, src_element_ty);
        let (dst_size, dst_align) = self.module.size_align(dst_opts, dst_element_ty);

        // Load the pointer/length of this list into temporary locals. These
        // will be referenced a good deal so this just makes it easier to deal
        // with them consistently below rather than trying to reload from memory
        // for example.
        let src_ptr = self.gen_local(src_opts.ptr());
        let src_len = self.gen_local(src_opts.ptr());
        match src {
            Source::Stack(s) => {
                assert_eq!(s.locals.len(), 2);
                self.stack_get(&s.slice(0..1), src_opts.ptr());
                self.instruction(LocalSet(src_ptr));
                self.stack_get(&s.slice(1..2), src_opts.ptr());
                self.instruction(LocalSet(src_len));
            }
            Source::Memory(mem) => {
                self.ptr_load(mem);
                self.instruction(LocalSet(src_ptr));
                self.ptr_load(&mem.bump(src_opts.ptr_size().into()));
                self.instruction(LocalSet(src_len));
            }
        }

        // Create a `Memory` operand which will internally assert that the
        // `src_ptr` value is properly aligned.
        let src_mem = self.memory_operand(src_opts, src_ptr, src_align);

        // Next the byte size of the allocation in the destination is
        // determined. Note that this is pretty tricky because pointer widths
        // could be changing and otherwise everything must stay within the
        // 32-bit size-space. This internally will ensure that `src_len *
        // dst_size` doesn't overflow 32-bits and will place the final result in
        // `dst_byte_len` where `dst_byte_len` has the appropriate type for the
        // destination.
        let dst_byte_len = self.gen_local(dst_opts.ptr());
        self.calculate_dst_byte_len(
            src_len,
            dst_byte_len,
            src_opts.ptr(),
            dst_opts.ptr(),
            dst_size,
        );

        // Here `realloc` is invoked (in a `malloc`-like fashion) to allocate
        // space for the list in the destination memory. This will also
        // internally insert checks that the returned pointer is aligned
        // correctly for the destination.
        let dst_mem = self.malloc(dst_opts, MallocSize::Local(dst_byte_len), dst_align);

        // At this point we have aligned pointers, a length, and a byte length
        // for the destination. The spec also requires this translation to
        // ensure that the range of memory within the source and destination
        // memories are valid. Currently though this attempts to optimize that
        // somewhat at least. The thinking is that if we hit an out-of-bounds
        // memory access during translation that's the same as a trap up-front.
        // This means we can generally minimize up-front checks in favor of
        // simply trying to load out-of-bounds memory.
        //
        // This doesn't mean we can avoid a check entirely though. One major
        // worry here is integer overflow of the pointers in linear memory as
        // they're incremented to move to the next element as part of
        // translation. For example if the entire 32-bit address space were
        // valid and the base pointer was `0xffff_fff0` where the size was 17
        // that should not be a valid list but "simply defer to the loop below"
        // would cause a wraparound to occur and no trap would be detected.
        //
        // To solve this a check is inserted here that the `base + byte_len`
        // calculation doesn't overflow the 32-bit address space. Note though
        // that this is only done for 32-bit memories, not 64-bit memories.
        // Given the iteration of the loop below the only worry is when the
        // address space is 100% mapped and wraparound is possible. Otherwise if
        // anything in the address space is unmapped then we're guaranteed to
        // hit a trap as we march from the base pointer to the end of the array.
        // It's assumed that it's impossible for a 64-bit memory to have the
        // entire address space mapped, so this isn't a concern for 64-bit
        // memories.
        //
        // Technically this is only a concern for 32-bit memories if the entire
        // address space is mapped, so `memory.size` could be used to skip most
        // of the check here but it's assume that the `memory.size` check is
        // probably more expensive than just checking for 32-bit overflow by
        // using 64-bit arithmetic. This should hypothetically be tested though!
        //
        // TODO: the most-optimal thing here is to probably, once per adapter,
        // call `memory.size` and put that in a local. If that is not the
        // maximum for a 32-bit memory then this entire bounds-check here can be
        // skipped.
        if !src_opts.memory64 && src_size > 0 {
            self.instruction(LocalGet(src_mem.addr_local));
            self.instruction(I64ExtendI32U);
            if src_size < dst_size {
                // If the source byte size is less than the destination size
                // then we can leverage the fact that `dst_byte_len` was already
                // calculated and didn't overflow so this is also guaranteed to
                // not overflow.
                self.instruction(LocalGet(src_len));
                self.instruction(I64ExtendI32U);
                if src_size != 1 {
                    self.instruction(I64Const(i64::try_from(src_size).unwrap()));
                    self.instruction(I64Mul);
                }
            } else if src_size == dst_size {
                // If the source byte size is the same as the destination byte
                // size then that can be reused. Note that the destination byte
                // size is already guaranteed to fit in 32 bits, even if it's
                // store in a 64-bit local.
                self.instruction(LocalGet(dst_byte_len));
                if dst_opts.ptr() == ValType::I32 {
                    self.instruction(I64ExtendI32U);
                }
            } else {
                // Otherwise if the source byte size is larger than the
                // destination byte size then the source byte size needs to be
                // calculated fresh here. Note, though, that the result of this
                // multiplication is not checked for overflow. The reason for
                // that is that the result here flows into the check below about
                // overflow and if this computation overflows it should be
                // guaranteed to overflow the next computation.
                //
                // In general what's being checked here is:
                //
                //      src_mem.addr_local + src_len * src_size
                //
                // These three values are all 32-bits originally and if they're
                // all assumed to be `u32::MAX` then:
                //
                //      let max = u64::from(u32::MAX);
                //      let result = max + max * max;
                //      assert_eq!(result, 0xffffffff00000000);
                //
                // This means that once an upper bit is set it's guaranteed to
                // stay set as part of this computation, so the multiplication
                // here is left unchecked to fall through into the addition
                // below.
                self.instruction(LocalGet(src_len));
                self.instruction(I64ExtendI32U);
                self.instruction(I64Const(i64::try_from(src_size).unwrap()));
                self.instruction(I64Mul);
            }
            self.instruction(I64Add);
            self.instruction(I64Const(32));
            self.instruction(I64ShrU);
            self.instruction(I32WrapI64);
            self.instruction(If(BlockType::Empty));
            self.trap(Trap::ListByteLengthOverflow);
            self.instruction(End);
        }

        // If the destination is a 32-bit memory then its overflow check is
        // relatively simple since we've already calculated the byte length of
        // the destination above and can reuse that in this check.
        if !dst_opts.memory64 && dst_size > 0 {
            self.instruction(LocalGet(dst_mem.addr_local));
            self.instruction(I64ExtendI32U);
            self.instruction(LocalGet(dst_byte_len));
            self.instruction(I64ExtendI32U);
            self.instruction(I64Add);
            self.instruction(I64Const(32));
            self.instruction(I64ShrU);
            self.instruction(I32WrapI64);
            self.instruction(If(BlockType::Empty));
            self.trap(Trap::ListByteLengthOverflow);
            self.instruction(End);
        }

        // This is the main body of the loop to actually translate list types.
        // Note that if both element sizes are 0 then this won't actually do
        // anything so the loop is removed entirely.
        if src_size > 0 || dst_size > 0 {
            let cur_dst_ptr = self.gen_local(dst_opts.ptr());
            let cur_src_ptr = self.gen_local(src_opts.ptr());
            let remaining = self.gen_local(src_opts.ptr());

            // This block encompasses the entire loop and is use to exit before even
            // entering the loop if the list size is zero.
            self.instruction(Block(BlockType::Empty));

            // Set the `remaining` local and only continue if it's > 0
            self.instruction(LocalGet(src_len));
            self.instruction(LocalTee(remaining));
            self.ptr_eqz(src_opts);
            self.instruction(BrIf(0));

            // Initialize the two destination pointers to their initial values
            self.instruction(LocalGet(src_mem.addr_local));
            self.instruction(LocalSet(cur_src_ptr));
            self.instruction(LocalGet(dst_mem.addr_local));
            self.instruction(LocalSet(cur_dst_ptr));

            self.instruction(Loop(BlockType::Empty));

            // Translate the next element in the list
            let element_src = Source::Memory(Memory {
                opts: src_opts,
                offset: 0,
                addr_local: cur_src_ptr,
            });
            let element_dst = Destination::Memory(Memory {
                opts: dst_opts,
                offset: 0,
                addr_local: cur_dst_ptr,
            });
            self.translate(src_element_ty, &element_src, dst_element_ty, &element_dst);

            // Update the two loop pointers
            if src_size > 0 {
                self.instruction(LocalGet(cur_src_ptr));
                self.ptr_uconst(src_opts, u32::try_from(src_size).unwrap());
                self.ptr_add(src_opts);
                self.instruction(LocalSet(cur_src_ptr));
            }
            if dst_size > 0 {
                self.instruction(LocalGet(cur_dst_ptr));
                self.ptr_uconst(dst_opts, u32::try_from(dst_size).unwrap());
                self.ptr_add(dst_opts);
                self.instruction(LocalSet(cur_dst_ptr));
            }

            // Update the remaining count, falling through to break out if it's zero
            // now.
            self.instruction(LocalGet(remaining));
            self.ptr_iconst(src_opts, -1);
            self.ptr_add(src_opts);
            self.instruction(LocalTee(remaining));
            self.ptr_br_if(src_opts, 0);
            self.instruction(End); // end of loop
            self.instruction(End); // end of block
        }

        // Store the ptr/length in the desired destination
        match dst {
            Destination::Stack(s, _) => {
                self.instruction(LocalGet(dst_mem.addr_local));
                self.stack_set(&s[..1], dst_opts.ptr());
                self.convert_src_len_to_dst(src_len, src_opts.ptr(), dst_opts.ptr());
                self.stack_set(&s[1..], dst_opts.ptr());
            }
            Destination::Memory(mem) => {
                self.instruction(LocalGet(mem.addr_local));
                self.instruction(LocalGet(dst_mem.addr_local));
                self.ptr_store(mem);
                self.instruction(LocalGet(mem.addr_local));
                self.convert_src_len_to_dst(src_len, src_opts.ptr(), dst_opts.ptr());
                self.ptr_store(&mem.bump(dst_opts.ptr_size().into()));
            }
        }
    }

    fn calculate_dst_byte_len(
        &mut self,
        src_len_local: u32,
        dst_len_local: u32,
        src_ptr_ty: ValType,
        dst_ptr_ty: ValType,
        dst_elt_size: usize,
    ) {
        // Zero-size types are easy to handle here because the byte size of the
        // destination is always zero.
        if dst_elt_size == 0 {
            if dst_ptr_ty == ValType::I64 {
                self.instruction(I64Const(0));
            } else {
                self.instruction(I32Const(0));
            }
            self.instruction(LocalSet(dst_len_local));
            return;
        }

        // For one-byte elements in the destination the check here can be a bit
        // more optimal than the general case below. In these situations if the
        // source pointer type is 32-bit then we're guaranteed to not overflow,
        // so the source length is simply casted to the destination's type.
        //
        // If the source is 64-bit then all that needs to be checked is to
        // ensure that it does not have the upper 32-bits set.
        if dst_elt_size == 1 {
            if let ValType::I64 = src_ptr_ty {
                self.instruction(LocalGet(src_len_local));
                self.instruction(I64Const(32));
                self.instruction(I64ShrU);
                self.instruction(I32WrapI64);
                self.instruction(If(BlockType::Empty));
                self.trap(Trap::ListByteLengthOverflow);
                self.instruction(End);
            }
            self.convert_src_len_to_dst(src_len_local, src_ptr_ty, dst_ptr_ty);
            self.instruction(LocalSet(dst_len_local));
            return;
        }

        // The main check implemented by this function is to verify that
        // `src_len_local` does not exceed the 32-bit range. Byte sizes for
        // lists must always fit in 32-bits to get transferred to 32-bit
        // memories.
        self.instruction(Block(BlockType::Empty));
        self.instruction(Block(BlockType::Empty));
        self.instruction(LocalGet(src_len_local));
        match src_ptr_ty {
            // The source's list length is guaranteed to be less than 32-bits
            // so simply extend it up to a 64-bit type for the multiplication
            // below.
            ValType::I32 => self.instruction(I64ExtendI32U),

            // If the source is a 64-bit memory then if the item length doesn't
            // fit in 32-bits the byte length definitly won't, so generate a
            // branch to our overflow trap here if any of the upper 32-bits are set.
            ValType::I64 => {
                self.instruction(I64Const(32));
                self.instruction(I64ShrU);
                self.instruction(I32WrapI64);
                self.instruction(BrIf(0));
                self.instruction(LocalGet(src_len_local));
            }

            _ => unreachable!(),
        }

        // Next perform a 64-bit multiplication with the element byte size that
        // is itself guaranteed to fit in 32-bits. The result is then checked
        // to see if we overflowed the 32-bit space. The two input operands to
        // the multiplication are guaranteed to be 32-bits at most which means
        // that this multiplication shouldn't overflow.
        //
        // The result of the multiplication is saved into a local as well to
        // get the result afterwards.
        let tmp = if dst_ptr_ty != ValType::I64 {
            self.gen_local(ValType::I64)
        } else {
            dst_len_local
        };
        self.instruction(I64Const(u32::try_from(dst_elt_size).unwrap().into()));
        self.instruction(I64Mul);
        self.instruction(LocalTee(tmp));
        // Branch to success if the upper 32-bits are zero, otherwise
        // fall-through to the trap.
        self.instruction(I64Const(32));
        self.instruction(I64ShrU);
        self.instruction(I64Eqz);
        self.instruction(BrIf(1));
        self.instruction(End);
        self.trap(Trap::ListByteLengthOverflow);
        self.instruction(End);

        // If a fresh local was used to store the result of the multiplication
        // then convert it down to 32-bits which should be guaranteed to not
        // lose information at this point.
        if dst_ptr_ty != ValType::I64 {
            self.instruction(LocalGet(tmp));
            self.instruction(I32WrapI64);
            self.instruction(LocalSet(dst_len_local));
        }
    }

    fn convert_src_len_to_dst(
        &mut self,
        src_len_local: u32,
        src_ptr_ty: ValType,
        dst_ptr_ty: ValType,
    ) {
        self.instruction(LocalGet(src_len_local));
        match (src_ptr_ty, dst_ptr_ty) {
            (ValType::I32, ValType::I64) => self.instruction(I64ExtendI32U),
            (ValType::I64, ValType::I32) => self.instruction(I32WrapI64),
            (src, dst) => assert_eq!(src, dst),
        }
    }

    fn translate_record(
        &mut self,
        src_ty: TypeRecordIndex,
        src: &Source<'_>,
        dst_ty: &InterfaceType,
        dst: &Destination,
    ) {
        let src_ty = &self.module.types[src_ty];
        let dst_ty = match dst_ty {
            InterfaceType::Record(r) => &self.module.types[*r],
            _ => panic!("expected a record"),
        };

        // TODO: subtyping
        assert_eq!(src_ty.fields.len(), dst_ty.fields.len());

        // First a map is made of the source fields to where they're coming
        // from (e.g. which offset or which locals). This map is keyed by the
        // fields' names
        let mut src_fields = HashMap::new();
        for (i, src) in src
            .record_field_srcs(self.module, src_ty.fields.iter().map(|f| f.ty))
            .enumerate()
        {
            let field = &src_ty.fields[i];
            src_fields.insert(&field.name, (src, &field.ty));
        }

        // .. and next translation is performed in the order of the destination
        // fields in case the destination is the stack to ensure that the stack
        // has the fields all in the right order.
        //
        // Note that the lookup in `src_fields` is an infallible lookup which
        // will panic if the field isn't found.
        //
        // TODO: should that lookup be fallible with subtyping?
        for (i, dst) in dst
            .record_field_dsts(self.module, dst_ty.fields.iter().map(|f| f.ty))
            .enumerate()
        {
            let field = &dst_ty.fields[i];
            let (src, src_ty) = &src_fields[&field.name];
            self.translate(src_ty, src, &field.ty, &dst);
        }
    }

    fn translate_flags(
        &mut self,
        src_ty: TypeFlagsIndex,
        src: &Source<'_>,
        dst_ty: &InterfaceType,
        dst: &Destination,
    ) {
        let src_ty = &self.module.types[src_ty];
        let dst_ty = match dst_ty {
            InterfaceType::Flags(r) => &self.module.types[*r],
            _ => panic!("expected a record"),
        };

        // TODO: subtyping
        //
        // Notably this implementation does not support reordering flags from
        // the source to the destination nor having more flags in the
        // destination. Currently this is a copy from source to destination
        // in-bulk. Otherwise reordering indices would have to have some sort of
        // fancy bit twiddling tricks or something like that.
        assert_eq!(src_ty.names, dst_ty.names);
        let cnt = src_ty.names.len();
        match FlagsSize::from_count(cnt) {
            FlagsSize::Size0 => {}
            FlagsSize::Size1 => {
                let mask = if cnt == 8 { 0xff } else { (1 << cnt) - 1 };
                self.convert_u8_mask(src, dst, mask);
            }
            FlagsSize::Size2 => {
                let mask = if cnt == 16 { 0xffff } else { (1 << cnt) - 1 };
                self.convert_u16_mask(src, dst, mask);
            }
            FlagsSize::Size4Plus(n) => {
                let srcs = src.record_field_srcs(self.module, (0..n).map(|_| InterfaceType::U32));
                let dsts = dst.record_field_dsts(self.module, (0..n).map(|_| InterfaceType::U32));
                for (i, (src, dst)) in srcs.zip(dsts).enumerate() {
                    let mask = if i == n - 1 && (cnt % 32 != 0) {
                        (1 << (cnt % 32)) - 1
                    } else {
                        0xffffffff
                    };
                    self.convert_u32_mask(&src, &dst, mask);
                }
            }
        }
    }

    fn translate_tuple(
        &mut self,
        src_ty: TypeTupleIndex,
        src: &Source<'_>,
        dst_ty: &InterfaceType,
        dst: &Destination,
    ) {
        let src_ty = &self.module.types[src_ty];
        let dst_ty = match dst_ty {
            InterfaceType::Tuple(t) => &self.module.types[*t],
            _ => panic!("expected a tuple"),
        };

        // TODO: subtyping
        assert_eq!(src_ty.types.len(), dst_ty.types.len());

        let srcs = src
            .record_field_srcs(self.module, src_ty.types.iter().copied())
            .zip(src_ty.types.iter());
        let dsts = dst
            .record_field_dsts(self.module, dst_ty.types.iter().copied())
            .zip(dst_ty.types.iter());
        for ((src, src_ty), (dst, dst_ty)) in srcs.zip(dsts) {
            self.translate(src_ty, &src, dst_ty, &dst);
        }
    }

    fn translate_variant(
        &mut self,
        src_ty: TypeVariantIndex,
        src: &Source<'_>,
        dst_ty: &InterfaceType,
        dst: &Destination,
    ) {
        let src_ty = &self.module.types[src_ty];
        let dst_ty = match dst_ty {
            InterfaceType::Variant(t) => &self.module.types[*t],
            _ => panic!("expected a variant"),
        };

        let src_disc_size = DiscriminantSize::from_count(src_ty.cases.len()).unwrap();
        let dst_disc_size = DiscriminantSize::from_count(dst_ty.cases.len()).unwrap();

        let iter = src_ty.cases.iter().enumerate().map(|(src_i, src_case)| {
            let dst_i = dst_ty
                .cases
                .iter()
                .position(|c| c.name == src_case.name)
                .unwrap();
            let dst_case = &dst_ty.cases[dst_i];
            let src_i = u32::try_from(src_i).unwrap();
            let dst_i = u32::try_from(dst_i).unwrap();
            VariantCase {
                src_i,
                src_ty: &src_case.ty,
                dst_i,
                dst_ty: &dst_case.ty,
            }
        });
        self.convert_variant(src, src_disc_size, dst, dst_disc_size, iter);
    }

    fn translate_union(
        &mut self,
        src_ty: TypeUnionIndex,
        src: &Source<'_>,
        dst_ty: &InterfaceType,
        dst: &Destination,
    ) {
        let src_ty = &self.module.types[src_ty];
        let dst_ty = match dst_ty {
            InterfaceType::Union(t) => &self.module.types[*t],
            _ => panic!("expected an option"),
        };
        assert_eq!(src_ty.types.len(), dst_ty.types.len());

        self.convert_variant(
            src,
            DiscriminantSize::Size1,
            dst,
            DiscriminantSize::Size1,
            src_ty
                .types
                .iter()
                .zip(dst_ty.types.iter())
                .enumerate()
                .map(|(i, (src_ty, dst_ty))| {
                    let i = u32::try_from(i).unwrap();
                    VariantCase {
                        src_i: i,
                        dst_i: i,
                        src_ty,
                        dst_ty,
                    }
                }),
        );
    }

    fn translate_enum(
        &mut self,
        src_ty: TypeEnumIndex,
        src: &Source<'_>,
        dst_ty: &InterfaceType,
        dst: &Destination,
    ) {
        let src_ty = &self.module.types[src_ty];
        let dst_ty = match dst_ty {
            InterfaceType::Enum(t) => &self.module.types[*t],
            _ => panic!("expected an option"),
        };

        let unit = &InterfaceType::Unit;
        self.convert_variant(
            src,
            DiscriminantSize::from_count(src_ty.names.len()).unwrap(),
            dst,
            DiscriminantSize::from_count(dst_ty.names.len()).unwrap(),
            src_ty.names.iter().enumerate().map(|(src_i, src_name)| {
                let dst_i = dst_ty.names.iter().position(|n| n == src_name).unwrap();
                let src_i = u32::try_from(src_i).unwrap();
                let dst_i = u32::try_from(dst_i).unwrap();
                VariantCase {
                    src_i,
                    dst_i,
                    src_ty: unit,
                    dst_ty: unit,
                }
            }),
        );
    }

    fn translate_option(
        &mut self,
        src_ty: TypeInterfaceIndex,
        src: &Source<'_>,
        dst_ty: &InterfaceType,
        dst: &Destination,
    ) {
        let src_ty = &self.module.types[src_ty];
        let dst_ty = match dst_ty {
            InterfaceType::Option(t) => &self.module.types[*t],
            _ => panic!("expected an option"),
        };

        self.convert_variant(
            src,
            DiscriminantSize::Size1,
            dst,
            DiscriminantSize::Size1,
            [
                VariantCase {
                    src_i: 0,
                    dst_i: 0,
                    src_ty: &InterfaceType::Unit,
                    dst_ty: &InterfaceType::Unit,
                },
                VariantCase {
                    src_i: 1,
                    dst_i: 1,
                    src_ty,
                    dst_ty,
                },
            ]
            .into_iter(),
        );
    }

    fn translate_expected(
        &mut self,
        src_ty: TypeExpectedIndex,
        src: &Source<'_>,
        dst_ty: &InterfaceType,
        dst: &Destination,
    ) {
        let src_ty = &self.module.types[src_ty];
        let dst_ty = match dst_ty {
            InterfaceType::Expected(t) => &self.module.types[*t],
            _ => panic!("expected an expected"),
        };

        self.convert_variant(
            src,
            DiscriminantSize::Size1,
            dst,
            DiscriminantSize::Size1,
            [
                VariantCase {
                    src_i: 0,
                    dst_i: 0,
                    src_ty: &src_ty.ok,
                    dst_ty: &dst_ty.ok,
                },
                VariantCase {
                    src_i: 1,
                    dst_i: 1,
                    src_ty: &src_ty.err,
                    dst_ty: &dst_ty.err,
                },
            ]
            .into_iter(),
        );
    }

    fn convert_variant<'a>(
        &mut self,
        src: &Source<'_>,
        src_disc_size: DiscriminantSize,
        dst: &Destination,
        dst_disc_size: DiscriminantSize,
        src_cases: impl ExactSizeIterator<Item = VariantCase<'a>>,
    ) {
        // The outermost block is special since it has the result type of the
        // translation here. That will depend on the `dst`.
        let outer_block_ty = match dst {
            Destination::Stack(dst_flat, _) => match dst_flat.len() {
                0 => BlockType::Empty,
                1 => BlockType::Result(dst_flat[0]),
                _ => {
                    let ty = self.types.function(&[], &dst_flat);
                    BlockType::FunctionType(ty)
                }
            },
            Destination::Memory(_) => BlockType::Empty,
        };
        self.instruction(Block(outer_block_ty));

        // After the outermost block generate a new block for each of the
        // remaining cases.
        let src_cases_len = src_cases.len();
        for _ in 0..src_cases_len - 1 {
            self.instruction(Block(BlockType::Empty));
        }

        // Generate a block for an invalid variant discriminant
        self.instruction(Block(BlockType::Empty));

        // And generate one final block that we'll be jumping out of with the
        // `br_table`
        self.instruction(Block(BlockType::Empty));

        // Load the discriminant
        match src {
            Source::Stack(s) => self.stack_get(&s.slice(0..1), ValType::I32),
            Source::Memory(mem) => match src_disc_size {
                DiscriminantSize::Size1 => self.i32_load8u(mem),
                DiscriminantSize::Size2 => self.i32_load16u(mem),
                DiscriminantSize::Size4 => self.i32_load(mem),
            },
        }

        // Generate the `br_table` for the discriminant. Each case has an
        // offset of 1 to skip the trapping block.
        let mut targets = Vec::new();
        for i in 0..src_cases_len {
            targets.push((i + 1) as u32);
        }
        self.instruction(BrTable(targets[..].into(), 0));
        self.instruction(End); // end the `br_table` block

        self.trap(Trap::InvalidDiscriminant);
        self.instruction(End); // end the "invalid discriminant" block

        // Translate each case individually within its own block. Note that the
        // iteration order here places the first case in the innermost block
        // and the last case in the outermost block. This matches the order
        // of the jump targets in the `br_table` instruction.
        let src_cases_len = u32::try_from(src_cases_len).unwrap();
        for case in src_cases {
            let VariantCase {
                src_i,
                src_ty,
                dst_i,
                dst_ty,
            } = case;

            // Translate the discriminant here, noting that `dst_i` may be
            // different than `src_i`.
            self.push_dst_addr(dst);
            self.instruction(I32Const(dst_i as i32));
            match dst {
                Destination::Stack(stack, _) => self.stack_set(&stack[..1], ValType::I32),
                Destination::Memory(mem) => match dst_disc_size {
                    DiscriminantSize::Size1 => self.i32_store8(mem),
                    DiscriminantSize::Size2 => self.i32_store16(mem),
                    DiscriminantSize::Size4 => self.i32_store(mem),
                },
            }

            // Translate the payload of this case using the various types from
            // the dst/src.
            let src_payload = src.payload_src(self.module, src_disc_size, src_ty);
            let dst_payload = dst.payload_dst(self.module, dst_disc_size, dst_ty);
            self.translate(src_ty, &src_payload, dst_ty, &dst_payload);

            // If the results of this translation were placed on the stack then
            // the stack values may need to be padded with more zeros due to
            // this particular case being possibly smaller than the entire
            // variant. That's handled here by pushing remaining zeros after
            // accounting for the discriminant pushed as well as the results of
            // this individual payload.
            if let Destination::Stack(payload_results, _) = dst_payload {
                if let Destination::Stack(dst_results, _) = dst {
                    let remaining = &dst_results[1..][payload_results.len()..];
                    for ty in remaining {
                        match ty {
                            ValType::I32 => self.instruction(I32Const(0)),
                            ValType::I64 => self.instruction(I64Const(0)),
                            ValType::F32 => self.instruction(F32Const(0.0)),
                            ValType::F64 => self.instruction(F64Const(0.0)),
                            _ => unreachable!(),
                        }
                    }
                }
            }

            // Branch to the outermost block. Note that this isn't needed for
            // the outermost case since it simply falls through.
            if src_i != src_cases_len - 1 {
                self.instruction(Br(src_cases_len - src_i - 1));
            }
            self.instruction(End); // end this case's block
        }
    }

    fn trap_if_not_flag(&mut self, flags_global: GlobalIndex, flag_to_test: i32, trap: Trap) {
        self.instruction(GlobalGet(flags_global.as_u32()));
        self.instruction(I32Const(flag_to_test));
        self.instruction(I32And);
        self.instruction(I32Eqz);
        self.instruction(If(BlockType::Empty));
        self.trap(trap);
        self.instruction(End);
    }

    fn assert_not_flag(&mut self, flags_global: GlobalIndex, flag_to_test: i32, msg: &'static str) {
        self.instruction(GlobalGet(flags_global.as_u32()));
        self.instruction(I32Const(flag_to_test));
        self.instruction(I32And);
        self.instruction(If(BlockType::Empty));
        self.trap(Trap::AssertFailed(msg));
        self.instruction(End);
    }

    fn set_flag(&mut self, flags_global: GlobalIndex, flag_to_set: i32, value: bool) {
        self.instruction(GlobalGet(flags_global.as_u32()));
        if value {
            self.instruction(I32Const(flag_to_set));
            self.instruction(I32Or);
        } else {
            self.instruction(I32Const(!flag_to_set));
            self.instruction(I32And);
        }
        self.instruction(GlobalSet(flags_global.as_u32()));
    }

    fn verify_aligned(&mut self, opts: &Options, addr_local: u32, align: usize) {
        // If the alignment is 1 then everything is trivially aligned and the
        // check can be omitted.
        if align == 1 {
            return;
        }
        self.instruction(LocalGet(addr_local));
        assert!(align.is_power_of_two());
        self.ptr_uconst(opts, u32::try_from(align - 1).unwrap());
        self.ptr_and(opts);
        self.ptr_if(opts, BlockType::Empty);
        self.trap(Trap::UnalignedPointer);
        self.instruction(End);
    }

    fn assert_aligned(&mut self, ty: &InterfaceType, mem: &Memory) {
        if !self.module.debug {
            return;
        }
        let align = self.module.align(mem.opts, ty);
        if align == 1 {
            return;
        }
        assert!(align.is_power_of_two());
        self.instruction(LocalGet(mem.addr_local));
        self.ptr_uconst(mem.opts, mem.offset);
        self.ptr_add(mem.opts);
        self.ptr_uconst(mem.opts, u32::try_from(align - 1).unwrap());
        self.ptr_and(mem.opts);
        self.ptr_if(mem.opts, BlockType::Empty);
        self.trap(Trap::AssertFailed("pointer not aligned"));
        self.instruction(End);
    }

    fn malloc<'a>(&mut self, opts: &'a Options, size: MallocSize, align: usize) -> Memory<'a> {
        let addr_local = self.gen_local(opts.ptr());
        let realloc = opts.realloc.unwrap();
        self.ptr_uconst(opts, 0);
        self.ptr_uconst(opts, 0);
        self.ptr_uconst(opts, u32::try_from(align).unwrap());
        match size {
            MallocSize::Const(size) => self.ptr_uconst(opts, u32::try_from(size).unwrap()),
            MallocSize::Local(idx) => self.instruction(LocalGet(idx)),
        }
        self.instruction(Call(realloc.as_u32()));
        self.instruction(LocalSet(addr_local));
        self.memory_operand(opts, addr_local, align)
    }

    fn memory_operand<'a>(
        &mut self,
        opts: &'a Options,
        addr_local: u32,
        align: usize,
    ) -> Memory<'a> {
        let ret = Memory {
            addr_local,
            offset: 0,
            opts,
        };
        self.verify_aligned(opts, ret.addr_local, align);
        ret
    }

    fn gen_local(&mut self, ty: ValType) -> u32 {
        // TODO: see if local reuse is necessary, right now this always
        // generates a new local.
        match self.locals.last_mut() {
            Some((cnt, prev_ty)) if ty == *prev_ty => *cnt += 1,
            _ => self.locals.push((1, ty)),
        }
        self.nlocals += 1;
        self.nlocals - 1
    }

    fn instruction(&mut self, instr: Instruction) {
        instr.encode(&mut self.code);
    }

    fn trap(&mut self, trap: Trap) {
        self.traps.push((self.code.len(), trap));
        self.instruction(Unreachable);
    }

    fn finish(&mut self) -> (Vec<u8>, Vec<(usize, Trap)>) {
        self.instruction(End);

        let mut bytes = Vec::new();

        // Encode all locals used for this function
        self.locals.len().encode(&mut bytes);
        for (count, ty) in self.locals.iter() {
            count.encode(&mut bytes);
            ty.encode(&mut bytes);
        }

        // Factor in the size of the encodings of locals into the offsets of
        // traps.
        for (offset, _) in self.traps.iter_mut() {
            *offset += bytes.len();
        }

        // Then append the function we built and return
        bytes.extend_from_slice(&self.code);
        (bytes, mem::take(&mut self.traps))
    }

    /// Fetches the value contained with the local specified by `stack` and
    /// converts it to `dst_ty`.
    ///
    /// This is only intended for use in primitive operations where `stack` is
    /// guaranteed to have only one local. The type of the local on the stack is
    /// then converted to `dst_ty` appropriately. Note that the types may be
    /// different due to the "flattening" of variant types.
    fn stack_get(&mut self, stack: &Stack<'_>, dst_ty: ValType) {
        assert_eq!(stack.locals.len(), 1);
        let (idx, src_ty) = stack.locals[0];
        self.instruction(LocalGet(idx));
        match (src_ty, dst_ty) {
            (ValType::I32, ValType::I32)
            | (ValType::I64, ValType::I64)
            | (ValType::F32, ValType::F32)
            | (ValType::F64, ValType::F64) => {}

            (ValType::I32, ValType::F32) => self.instruction(F32ReinterpretI32),
            (ValType::I64, ValType::I32) => self.instruction(I32WrapI64),
            (ValType::I64, ValType::F64) => self.instruction(F64ReinterpretI64),
            (ValType::F64, ValType::F32) => self.instruction(F32DemoteF64),
            (ValType::I64, ValType::F32) => {
                self.instruction(F64ReinterpretI64);
                self.instruction(F32DemoteF64);
            }

            // should not be possible given the `join` function for variants
            (ValType::I32, ValType::I64)
            | (ValType::I32, ValType::F64)
            | (ValType::F32, ValType::I32)
            | (ValType::F32, ValType::I64)
            | (ValType::F32, ValType::F64)
            | (ValType::F64, ValType::I32)
            | (ValType::F64, ValType::I64)

            // not used in the component model
            | (ValType::ExternRef, _)
            | (_, ValType::ExternRef)
            | (ValType::FuncRef, _)
            | (_, ValType::FuncRef)
            | (ValType::V128, _)
            | (_, ValType::V128) => {
                panic!("cannot get {dst_ty:?} from {src_ty:?} local");
            }
        }
    }

    /// Converts the top value on the WebAssembly stack which has type
    /// `src_ty` to `dst_tys[0]`.
    ///
    /// This is only intended for conversion of primitives where the `dst_tys`
    /// list is known to be of length 1.
    fn stack_set(&mut self, dst_tys: &[ValType], src_ty: ValType) {
        assert_eq!(dst_tys.len(), 1);
        let dst_ty = dst_tys[0];
        match (src_ty, dst_ty) {
            (ValType::I32, ValType::I32)
            | (ValType::I64, ValType::I64)
            | (ValType::F32, ValType::F32)
            | (ValType::F64, ValType::F64) => {}

            (ValType::F32, ValType::I32) => self.instruction(I32ReinterpretF32),
            (ValType::I32, ValType::I64) => self.instruction(I64ExtendI32U),
            (ValType::F64, ValType::I64) => self.instruction(I64ReinterpretF64),
            (ValType::F32, ValType::F64) => self.instruction(F64PromoteF32),
            (ValType::F32, ValType::I64) => {
                self.instruction(F64PromoteF32);
                self.instruction(I64ReinterpretF64);
            }

            // should not be possible given the `join` function for variants
            (ValType::I64, ValType::I32)
            | (ValType::F64, ValType::I32)
            | (ValType::I32, ValType::F32)
            | (ValType::I64, ValType::F32)
            | (ValType::F64, ValType::F32)
            | (ValType::I32, ValType::F64)
            | (ValType::I64, ValType::F64)

            // not used in the component model
            | (ValType::ExternRef, _)
            | (_, ValType::ExternRef)
            | (ValType::FuncRef, _)
            | (_, ValType::FuncRef)
            | (ValType::V128, _)
            | (_, ValType::V128) => {
                panic!("cannot get {dst_ty:?} from {src_ty:?} local");
            }
        }
    }

    fn i32_load8u(&mut self, mem: &Memory) {
        self.instruction(LocalGet(mem.addr_local));
        self.instruction(I32Load8_U(mem.memarg(0)));
    }

    fn i32_load8s(&mut self, mem: &Memory) {
        self.instruction(LocalGet(mem.addr_local));
        self.instruction(I32Load8_S(mem.memarg(0)));
    }

    fn i32_load16u(&mut self, mem: &Memory) {
        self.instruction(LocalGet(mem.addr_local));
        self.instruction(I32Load16_U(mem.memarg(1)));
    }

    fn i32_load16s(&mut self, mem: &Memory) {
        self.instruction(LocalGet(mem.addr_local));
        self.instruction(I32Load16_S(mem.memarg(1)));
    }

    fn i32_load(&mut self, mem: &Memory) {
        self.instruction(LocalGet(mem.addr_local));
        self.instruction(I32Load(mem.memarg(2)));
    }

    fn i64_load(&mut self, mem: &Memory) {
        self.instruction(LocalGet(mem.addr_local));
        self.instruction(I64Load(mem.memarg(3)));
    }

    fn ptr_load(&mut self, mem: &Memory) {
        if mem.opts.memory64 {
            self.i64_load(mem);
        } else {
            self.i32_load(mem);
        }
    }

    fn ptr_add(&mut self, opts: &Options) {
        if opts.memory64 {
            self.instruction(I64Add);
        } else {
            self.instruction(I32Add);
        }
    }

    fn ptr_sub(&mut self, opts: &Options) {
        if opts.memory64 {
            self.instruction(I64Sub);
        } else {
            self.instruction(I32Sub);
        }
    }

    fn ptr_mul(&mut self, opts: &Options) {
        if opts.memory64 {
            self.instruction(I64Mul);
        } else {
            self.instruction(I32Mul);
        }
    }

    fn ptr_ge_u(&mut self, opts: &Options) {
        if opts.memory64 {
            self.instruction(I64GeU);
        } else {
            self.instruction(I32GeU);
        }
    }

    fn ptr_lt_u(&mut self, opts: &Options) {
        if opts.memory64 {
            self.instruction(I64LtU);
        } else {
            self.instruction(I32LtU);
        }
    }

    fn ptr_shl(&mut self, opts: &Options) {
        if opts.memory64 {
            self.instruction(I64Shl);
        } else {
            self.instruction(I32Shl);
        }
    }

    fn ptr_eqz(&mut self, opts: &Options) {
        if opts.memory64 {
            self.instruction(I64Eqz);
        } else {
            self.instruction(I32Eqz);
        }
    }

    fn ptr_uconst(&mut self, opts: &Options, val: u32) {
        if opts.memory64 {
            self.instruction(I64Const(val.into()));
        } else {
            self.instruction(I32Const(val as i32));
        }
    }

    fn ptr_iconst(&mut self, opts: &Options, val: i32) {
        if opts.memory64 {
            self.instruction(I64Const(val.into()));
        } else {
            self.instruction(I32Const(val));
        }
    }

    fn ptr_eq(&mut self, opts: &Options) {
        if opts.memory64 {
            self.instruction(I64Eq);
        } else {
            self.instruction(I32Eq);
        }
    }

    fn ptr_ne(&mut self, opts: &Options) {
        if opts.memory64 {
            self.instruction(I64Ne);
        } else {
            self.instruction(I32Ne);
        }
    }

    fn ptr_and(&mut self, opts: &Options) {
        if opts.memory64 {
            self.instruction(I64And);
        } else {
            self.instruction(I32And);
        }
    }

    fn ptr_or(&mut self, opts: &Options) {
        if opts.memory64 {
            self.instruction(I64Or);
        } else {
            self.instruction(I32Or);
        }
    }

    fn ptr_xor(&mut self, opts: &Options) {
        if opts.memory64 {
            self.instruction(I64Xor);
        } else {
            self.instruction(I32Xor);
        }
    }

    fn ptr_if(&mut self, opts: &Options, ty: BlockType) {
        if opts.memory64 {
            self.instruction(I64Const(0));
            self.instruction(I64Ne);
        }
        self.instruction(If(ty));
    }

    fn ptr_br_if(&mut self, opts: &Options, depth: u32) {
        if opts.memory64 {
            self.instruction(I64Const(0));
            self.instruction(I64Ne);
        }
        self.instruction(BrIf(depth));
    }

    fn f32_load(&mut self, mem: &Memory) {
        self.instruction(LocalGet(mem.addr_local));
        self.instruction(F32Load(mem.memarg(2)));
    }

    fn f64_load(&mut self, mem: &Memory) {
        self.instruction(LocalGet(mem.addr_local));
        self.instruction(F64Load(mem.memarg(3)));
    }

    fn push_dst_addr(&mut self, dst: &Destination) {
        if let Destination::Memory(mem) = dst {
            self.instruction(LocalGet(mem.addr_local));
        }
    }

    fn i32_store8(&mut self, mem: &Memory) {
        self.instruction(I32Store8(mem.memarg(0)));
    }

    fn i32_store16(&mut self, mem: &Memory) {
        self.instruction(I32Store16(mem.memarg(1)));
    }

    fn i32_store(&mut self, mem: &Memory) {
        self.instruction(I32Store(mem.memarg(2)));
    }

    fn i64_store(&mut self, mem: &Memory) {
        self.instruction(I64Store(mem.memarg(3)));
    }

    fn ptr_store(&mut self, mem: &Memory) {
        if mem.opts.memory64 {
            self.i64_store(mem);
        } else {
            self.i32_store(mem);
        }
    }

    fn f32_store(&mut self, mem: &Memory) {
        self.instruction(F32Store(mem.memarg(2)));
    }

    fn f64_store(&mut self, mem: &Memory) {
        self.instruction(F64Store(mem.memarg(3)));
    }
}

impl<'a> Source<'a> {
    /// Given this `Source` returns an iterator over the `Source` for each of
    /// the component `fields` specified.
    ///
    /// This will automatically slice stack-based locals to the appropriate
    /// width for each component type and additionally calculate the appropriate
    /// offset for each memory-based type.
    fn record_field_srcs<'b>(
        &'b self,
        module: &'b Module,
        fields: impl IntoIterator<Item = InterfaceType> + 'b,
    ) -> impl Iterator<Item = Source<'a>> + 'b
    where
        'a: 'b,
    {
        let mut offset = 0;
        fields.into_iter().map(move |ty| match self {
            Source::Memory(mem) => {
                let mem = next_field_offset(&mut offset, module, &ty, mem);
                Source::Memory(mem)
            }
            Source::Stack(stack) => {
                let cnt = module.flatten_types(stack.opts, [ty]).len();
                offset += cnt;
                Source::Stack(stack.slice(offset - cnt..offset))
            }
        })
    }

    /// Returns the corresponding discriminant source and payload source f
    fn payload_src(
        &self,
        module: &Module,
        size: DiscriminantSize,
        case: &InterfaceType,
    ) -> Source<'a> {
        match self {
            Source::Stack(s) => {
                let flat_len = module.flatten_types(s.opts, [*case]).len();
                Source::Stack(s.slice(1..s.locals.len()).slice(0..flat_len))
            }
            Source::Memory(mem) => {
                let mem = payload_offset(size, module, case, mem);
                Source::Memory(mem)
            }
        }
    }

    fn opts(&self) -> &'a Options {
        match self {
            Source::Stack(s) => s.opts,
            Source::Memory(mem) => mem.opts,
        }
    }
}

impl<'a> Destination<'a> {
    /// Same as `Source::record_field_srcs` but for destinations.
    fn record_field_dsts<'b>(
        &'b self,
        module: &'b Module,
        fields: impl IntoIterator<Item = InterfaceType> + 'b,
    ) -> impl Iterator<Item = Destination> + 'b
    where
        'a: 'b,
    {
        let mut offset = 0;
        fields.into_iter().map(move |ty| match self {
            Destination::Memory(mem) => {
                let mem = next_field_offset(&mut offset, module, &ty, mem);
                Destination::Memory(mem)
            }
            Destination::Stack(s, opts) => {
                let cnt = module.flatten_types(opts, [ty]).len();
                offset += cnt;
                Destination::Stack(&s[offset - cnt..offset], opts)
            }
        })
    }

    /// Returns the corresponding discriminant source and payload source f
    fn payload_dst(
        &self,
        module: &Module,
        size: DiscriminantSize,
        case: &InterfaceType,
    ) -> Destination {
        match self {
            Destination::Stack(s, opts) => {
                let flat_len = module.flatten_types(opts, [*case]).len();
                Destination::Stack(&s[1..][..flat_len], opts)
            }
            Destination::Memory(mem) => {
                let mem = payload_offset(size, module, case, mem);
                Destination::Memory(mem)
            }
        }
    }

    fn opts(&self) -> &'a Options {
        match self {
            Destination::Stack(_, opts) => opts,
            Destination::Memory(mem) => mem.opts,
        }
    }
}

fn next_field_offset<'a>(
    offset: &mut usize,
    module: &Module,
    field: &InterfaceType,
    mem: &Memory<'a>,
) -> Memory<'a> {
    let (size, align) = module.size_align(mem.opts, field);
    *offset = align_to(*offset, align) + size;
    mem.bump(*offset - size)
}

fn payload_offset<'a>(
    disc_size: DiscriminantSize,
    module: &Module,
    case: &InterfaceType,
    mem: &Memory<'a>,
) -> Memory<'a> {
    let align = module.align(mem.opts, case);
    mem.bump(align_to(disc_size.into(), align))
}

impl<'a> Memory<'a> {
    fn memarg(&self, align: u32) -> MemArg {
        MemArg {
            offset: u64::from(self.offset),
            align,
            memory_index: self.opts.memory.unwrap().as_u32(),
        }
    }

    fn bump(&self, offset: usize) -> Memory<'a> {
        Memory {
            opts: self.opts,
            addr_local: self.addr_local,
            offset: self.offset + u32::try_from(offset).unwrap(),
        }
    }
}

impl<'a> Stack<'a> {
    fn slice(&self, range: Range<usize>) -> Stack<'a> {
        Stack {
            locals: &self.locals[range],
            opts: self.opts,
        }
    }
}

struct VariantCase<'a> {
    src_i: u32,
    src_ty: &'a InterfaceType,
    dst_i: u32,
    dst_ty: &'a InterfaceType,
}

enum MallocSize {
    Const(usize),
    Local(u32),
}
