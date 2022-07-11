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
    InterfaceType, TypeRecordIndex, TypeTupleIndex, FLAG_MAY_ENTER, FLAG_MAY_LEAVE,
    MAX_FLAT_PARAMS, MAX_FLAT_RESULTS,
};
use crate::fact::signature::{align_to, Signature};
use crate::fact::traps::Trap;
use crate::fact::{AdapterData, Context, Module, Options};
use crate::GlobalIndex;
use std::collections::HashMap;
use std::mem;
use std::ops::Range;
use wasm_encoder::{BlockType, Encode, Instruction, Instruction::*, MemArg, ValType};

struct Compiler<'a> {
    /// The module that the adapter will eventually be inserted into.
    module: &'a Module<'a>,

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

pub(super) fn compile(module: &Module<'_>, adapter: &AdapterData) -> (Vec<u8>, Vec<(usize, Trap)>) {
    let lower_sig = &module.signature(adapter.lower.ty, Context::Lower);
    let lift_sig = &module.signature(adapter.lift.ty, Context::Lift);
    Compiler {
        module,
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
enum Destination {
    /// This value is destined for the WebAssembly stack which means that
    /// results are simply pushed as we go along.
    Stack,

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

impl Compiler<'_> {
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
            Destination::Stack
        } else {
            // If there are too many parameters then space is allocated in the
            // destination module for the parameters via its `realloc` function.
            let (size, align) = self.module.record_size_align(dst_tys.iter());
            Destination::Memory(self.malloc(&self.adapter.lift, size, align))
        };

        let srcs = src
            .record_field_sources(self.module, src_tys.iter().copied())
            .zip(src_tys.iter());
        let dsts = dst
            .record_field_sources(self.module, dst_tys.iter().copied())
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
            Destination::Stack
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
            InterfaceType::U32 => self.translate_u32(src, dst_ty, dst),
            InterfaceType::Record(t) => self.translate_record(*t, src, dst_ty, dst),
            InterfaceType::Tuple(t) => self.translate_tuple(*t, src, dst_ty, dst),

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
            Destination::Stack => {}
        }
    }

    fn translate_u8(&mut self, src: &Source<'_>, dst_ty: &InterfaceType, dst: &Destination) {
        // TODO: subtyping
        assert!(matches!(dst_ty, InterfaceType::U8));
        self.push_dst_addr(dst);
        match src {
            Source::Memory(mem) => self.i32_load8u(mem),
            Source::Stack(stack) => self.stack_get(stack, ValType::I32),
        }
        match dst {
            Destination::Memory(mem) => self.i32_store8(mem),
            Destination::Stack => {}
        }
    }

    fn translate_u32(&mut self, src: &Source<'_>, dst_ty: &InterfaceType, dst: &Destination) {
        // TODO: subtyping
        assert!(matches!(dst_ty, InterfaceType::U32));
        self.push_dst_addr(dst);
        match src {
            Source::Memory(mem) => self.i32_load(mem),
            Source::Stack(stack) => self.stack_get(stack, ValType::I32),
        }
        match dst {
            Destination::Memory(mem) => self.i32_store(mem),
            Destination::Stack => {}
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
            .record_field_sources(self.module, src_ty.fields.iter().map(|f| f.ty))
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
            .record_field_sources(self.module, dst_ty.fields.iter().map(|f| f.ty))
            .enumerate()
        {
            let field = &dst_ty.fields[i];
            let (src, src_ty) = &src_fields[&field.name];
            self.translate(src_ty, src, &field.ty, &dst);
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
            .record_field_sources(self.module, src_ty.types.iter().copied())
            .zip(src_ty.types.iter());
        let dsts = dst
            .record_field_sources(self.module, dst_ty.types.iter().copied())
            .zip(dst_ty.types.iter());
        for ((src, src_ty), (dst, dst_ty)) in srcs.zip(dsts) {
            self.translate(src_ty, &src, dst_ty, &dst);
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

    fn verify_aligned(&mut self, local: u32, align: usize) {
        // If the alignment is 1 then everything is trivially aligned and the
        // check can be omitted.
        if align == 1 {
            return;
        }
        self.instruction(LocalGet(local));
        assert!(align.is_power_of_two());
        let mask = i32::try_from(align - 1).unwrap();
        self.instruction(I32Const(mask));
        self.instruction(I32And);
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
        self.instruction(I32Const(mem.i32_offset()));
        self.instruction(I32Add);
        let mask = i32::try_from(align - 1).unwrap();
        self.instruction(I32Const(mask));
        self.instruction(I32And);
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
        self.verify_aligned(addr_local, align);
        Memory {
            addr_local,
            offset: 0,
            memory_idx: memory.as_u32(),
        }
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

    fn i32_load8u(&mut self, mem: &Memory) {
        self.instruction(LocalGet(mem.addr_local));
        self.instruction(I32Load8_U(mem.memarg(0)));
    }

    fn i32_load(&mut self, mem: &Memory) {
        self.instruction(LocalGet(mem.addr_local));
        self.instruction(I32Load(mem.memarg(2)));
    }

    fn push_dst_addr(&mut self, dst: &Destination) {
        if let Destination::Memory(mem) = dst {
            self.instruction(LocalGet(mem.addr_local));
        }
    }

    fn i32_store8(&mut self, mem: &Memory) {
        self.instruction(I32Store8(mem.memarg(0)));
    }

    fn i32_store(&mut self, mem: &Memory) {
        self.instruction(I32Store(mem.memarg(2)));
    }
}

impl<'a> Source<'a> {
    /// Given this `Source` returns an iterator over the `Source` for each of
    /// the component `fields` specified.
    ///
    /// This will automatically slice stack-based locals to the appropriate
    /// width for each component type and additionally calculate the appropriate
    /// offset for each memory-based type.
    fn record_field_sources<'b>(
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
                let (size, align) = module.size_align(&ty);
                offset = align_to(offset, align) + size;
                Source::Memory(mem.bump(offset - size))
            }
            Source::Stack(stack) => {
                let cnt = module.flatten_types([ty]).len();
                offset += cnt;
                Source::Stack(stack.slice(offset - cnt..offset))
            }
        })
    }
}

impl Destination {
    /// Same as `Source::record_field_sources` but for destinations.
    fn record_field_sources<'a>(
        &'a self,
        module: &'a Module,
        fields: impl IntoIterator<Item = InterfaceType> + 'a,
    ) -> impl Iterator<Item = Destination> + 'a {
        let mut offset = 0;
        fields.into_iter().map(move |ty| match self {
            // TODO: dedupe with above?
            Destination::Memory(mem) => {
                let (size, align) = module.size_align(&ty);
                offset = align_to(offset, align) + size;
                Destination::Memory(mem.bump(offset - size))
            }
            Destination::Stack => Destination::Stack,
        })
    }
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

impl Options {
    fn ptr(&self) -> ValType {
        if self.memory64 {
            ValType::I64
        } else {
            ValType::I32
        }
    }
}
