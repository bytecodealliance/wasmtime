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
    InterfaceType, TypeEnumIndex, TypeExpectedIndex, TypeFlagsIndex, TypeInterfaceIndex,
    TypeRecordIndex, TypeTupleIndex, TypeUnionIndex, TypeVariantIndex, FLAG_MAY_ENTER,
    FLAG_MAY_LEAVE, MAX_FLAT_PARAMS, MAX_FLAT_RESULTS,
};
use crate::fact::core_types::CoreTypes;
use crate::fact::signature::{align_to, Signature};
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
    adapter: &AdapterData,
) -> (Vec<u8>, Vec<(usize, Trap)>) {
    let lower_sig = &module.signature(&adapter.lower, Context::Lower);
    let lift_sig = &module.signature(&adapter.lift, Context::Lift);
    Compiler {
        module,
        types,
        adapter,
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
    Memory(Memory),
}

/// Same as `Source` but for where values are translated into.
enum Destination<'a> {
    /// This value is destined for the WebAssembly stack which means that
    /// results are simply pushed as we go along.
    ///
    /// The types listed are the types that are expected to be on the stack at
    /// the end of translation.
    Stack(&'a [ValType]),

    /// This value is to be placed in linear memory described by `Memory`.
    Memory(Memory),
}

struct Stack<'a> {
    /// The locals that comprise a particular value.
    ///
    /// The length of this list represents the flattened list of types that make
    /// up the component value. Each list has the index of the local being
    /// accessed as well as the type of the local itself.
    locals: &'a [(u32, ValType)],
}

/// Representation of where a value is going to be stored in linear memory.
struct Memory {
    /// Whether or not the `addr_local` is a 64-bit type.
    memory64: bool,
    /// The index of the local that contains the base address of where the
    /// storage is happening.
    addr_local: u32,
    /// A "static" offset that will be baked into wasm instructions for where
    /// memory loads/stores happen.
    offset: u32,
    /// The index of memory in the wasm module memory index space that this
    /// memory is referring to.
    memory_idx: u32,
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

        let src_flat = self.module.flatten_types(src_tys.iter().copied());
        let dst_flat = self.module.flatten_types(dst_tys.iter().copied());

        let src = if src_flat.len() <= MAX_FLAT_PARAMS {
            Source::Stack(Stack {
                locals: &param_locals[..src_flat.len()],
            })
        } else {
            // If there are too many parameters then that means the parameters
            // are actually a tuple stored in linear memory addressed by the
            // first parameter local.
            let (addr, ty) = param_locals[0];
            assert_eq!(ty, self.adapter.lower.ptr());
            let align = src_tys
                .iter()
                .map(|t| self.module.align(t))
                .max()
                .unwrap_or(1);
            Source::Memory(self.memory_operand(&self.adapter.lower, addr, align))
        };

        let dst = if dst_flat.len() <= MAX_FLAT_PARAMS {
            Destination::Stack(&dst_flat)
        } else {
            // If there are too many parameters then space is allocated in the
            // destination module for the parameters via its `realloc` function.
            let (size, align) = self.module.record_size_align(dst_tys.iter());
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

        let src_flat = self.module.flatten_types([src_ty]);
        let dst_flat = self.module.flatten_types([dst_ty]);

        let src = if src_flat.len() <= MAX_FLAT_RESULTS {
            Source::Stack(Stack {
                locals: result_locals,
            })
        } else {
            // The original results to read from in this case come from the
            // return value of the function itself. The imported function will
            // return a linear memory address at which the values can be read
            // from.
            let align = self.module.align(&src_ty);
            assert_eq!(result_locals.len(), 1);
            let (addr, ty) = result_locals[0];
            assert_eq!(ty, self.adapter.lift.ptr());
            Source::Memory(self.memory_operand(&self.adapter.lift, addr, align))
        };

        let dst = if dst_flat.len() <= MAX_FLAT_RESULTS {
            Destination::Stack(&dst_flat)
        } else {
            // This is slightly different than `translate_params` where the
            // return pointer was provided by the caller of this function
            // meaning the last parameter local is a pointer into linear memory.
            let align = self.module.align(&dst_ty);
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
            InterfaceType::Record(t) => self.translate_record(*t, src, dst_ty, dst),
            InterfaceType::Flags(f) => self.translate_flags(*f, src, dst_ty, dst),
            InterfaceType::Tuple(t) => self.translate_tuple(*t, src, dst_ty, dst),
            InterfaceType::Variant(v) => self.translate_variant(*v, src, dst_ty, dst),
            InterfaceType::Union(u) => self.translate_union(*u, src, dst_ty, dst),
            InterfaceType::Enum(t) => self.translate_enum(*t, src, dst_ty, dst),
            InterfaceType::Option(t) => self.translate_option(*t, src, dst_ty, dst),
            InterfaceType::Expected(t) => self.translate_expected(*t, src, dst_ty, dst),

            InterfaceType::String => {
                // consider this field used for now until this is fully
                // implemented.
                drop(&self.adapter.lift.string_encoding);
                unimplemented!("don't know how to translate strings")
            }

            // TODO: this needs to be filled out for all the other interface
            // types.
            ty => unimplemented!("don't know how to translate {ty:?}"),
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
            Destination::Stack(stack) => self.stack_set(stack, ValType::I32),
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
            Destination::Stack(stack) => self.stack_set(stack, ValType::I32),
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
            Destination::Stack(stack) => self.stack_set(stack, ValType::I32),
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
            Destination::Stack(stack) => self.stack_set(stack, ValType::I32),
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
            Destination::Stack(stack) => self.stack_set(stack, ValType::I32),
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
            Destination::Stack(stack) => self.stack_set(stack, ValType::I32),
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
            Destination::Stack(stack) => self.stack_set(stack, ValType::I32),
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
            Destination::Stack(stack) => self.stack_set(stack, ValType::I64),
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
            Destination::Stack(stack) => self.stack_set(stack, ValType::I64),
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
            Destination::Stack(stack) => self.stack_set(stack, ValType::F32),
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
            Destination::Stack(stack) => self.stack_set(stack, ValType::F64),
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
            Destination::Stack(stack) => self.stack_set(stack, ValType::I32),
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
            Destination::Stack(dst_flat) => match dst_flat.len() {
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
                Destination::Stack(stack) => self.stack_set(&stack[..1], ValType::I32),
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
            if let Destination::Stack(payload_results) = dst_payload {
                if let Destination::Stack(dst_results) = dst {
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

    fn verify_aligned(&mut self, memory: &Memory, align: usize) {
        // If the alignment is 1 then everything is trivially aligned and the
        // check can be omitted.
        if align == 1 {
            return;
        }
        self.instruction(LocalGet(memory.addr_local));
        assert!(align.is_power_of_two());
        if memory.memory64 {
            let mask = i64::try_from(align - 1).unwrap();
            self.instruction(I64Const(mask));
            self.instruction(I64And);
            self.instruction(I64Const(0));
            self.instruction(I64Ne);
        } else {
            let mask = i32::try_from(align - 1).unwrap();
            self.instruction(I32Const(mask));
            self.instruction(I32And);
        }
        self.instruction(If(BlockType::Empty));
        self.trap(Trap::UnalignedPointer);
        self.instruction(End);
    }

    fn assert_aligned(&mut self, ty: &InterfaceType, mem: &Memory) {
        if !self.module.debug {
            return;
        }
        let align = self.module.align(ty);
        if align == 1 {
            return;
        }
        assert!(align.is_power_of_two());
        self.instruction(LocalGet(mem.addr_local));
        if mem.memory64 {
            self.instruction(I64Const(i64::from(mem.offset)));
            self.instruction(I64Add);
            let mask = i64::try_from(align - 1).unwrap();
            self.instruction(I64Const(mask));
            self.instruction(I64And);
            self.instruction(I64Const(0));
            self.instruction(I64Ne);
        } else {
            self.instruction(I32Const(mem.i32_offset()));
            self.instruction(I32Add);
            let mask = i32::try_from(align - 1).unwrap();
            self.instruction(I32Const(mask));
            self.instruction(I32And);
        }
        self.instruction(If(BlockType::Empty));
        self.trap(Trap::AssertFailed("pointer not aligned"));
        self.instruction(End);
    }

    fn malloc(&mut self, opts: &Options, size: usize, align: usize) -> Memory {
        let addr_local = self.gen_local(opts.ptr());
        let realloc = opts.realloc.unwrap();
        if opts.memory64 {
            self.instruction(I64Const(0));
            self.instruction(I64Const(0));
            self.instruction(I64Const(i64::try_from(align).unwrap()));
            self.instruction(I64Const(i64::try_from(size).unwrap()));
        } else {
            self.instruction(I32Const(0));
            self.instruction(I32Const(0));
            self.instruction(I32Const(i32::try_from(align).unwrap()));
            self.instruction(I32Const(i32::try_from(size).unwrap()));
        }
        self.instruction(Call(realloc.as_u32()));
        self.instruction(LocalSet(addr_local));
        self.memory_operand(opts, addr_local, align)
    }

    fn memory_operand(&mut self, opts: &Options, addr_local: u32, align: usize) -> Memory {
        let memory = opts.memory.unwrap();
        let ret = Memory {
            memory64: opts.memory64,
            addr_local,
            offset: 0,
            memory_idx: memory.as_u32(),
        };
        self.verify_aligned(&ret, align);
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
                let cnt = module.flatten_types([ty]).len();
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
                let flat_len = module.flatten_types([*case]).len();
                Source::Stack(s.slice(1..s.locals.len()).slice(0..flat_len))
            }
            Source::Memory(mem) => {
                let mem = payload_offset(size, module, case, mem);
                Source::Memory(mem)
            }
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
            Destination::Stack(s) => {
                let cnt = module.flatten_types([ty]).len();
                offset += cnt;
                Destination::Stack(&s[offset - cnt..offset])
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
            Destination::Stack(s) => {
                let flat_len = module.flatten_types([*case]).len();
                Destination::Stack(&s[1..][..flat_len])
            }
            Destination::Memory(mem) => {
                let mem = payload_offset(size, module, case, mem);
                Destination::Memory(mem)
            }
        }
    }
}

fn next_field_offset(
    offset: &mut usize,
    module: &Module,
    field: &InterfaceType,
    mem: &Memory,
) -> Memory {
    let (size, align) = module.size_align(field);
    *offset = align_to(*offset, align) + size;
    mem.bump(*offset - size)
}

fn payload_offset(
    disc_size: DiscriminantSize,
    module: &Module,
    case: &InterfaceType,
    mem: &Memory,
) -> Memory {
    let align = module.align(case);
    mem.bump(align_to(disc_size.into(), align))
}

impl Memory {
    fn i32_offset(&self) -> i32 {
        self.offset as i32
    }

    fn memarg(&self, align: u32) -> MemArg {
        MemArg {
            offset: u64::from(self.offset),
            align,
            memory_index: self.memory_idx,
        }
    }

    fn bump(&self, offset: usize) -> Memory {
        Memory {
            memory64: self.memory64,
            addr_local: self.addr_local,
            memory_idx: self.memory_idx,
            offset: self.offset + u32::try_from(offset).unwrap(),
        }
    }
}

impl<'a> Stack<'a> {
    fn slice(&self, range: Range<usize>) -> Stack<'a> {
        Stack {
            locals: &self.locals[range],
        }
    }
}

struct VariantCase<'a> {
    src_i: u32,
    src_ty: &'a InterfaceType,
    dst_i: u32,
    dst_ty: &'a InterfaceType,
}
