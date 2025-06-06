use cranelift_codegen::ir::BlockArg;
use itertools::{Either, Itertools};

use cranelift_codegen::ir::condcodes::*;
use cranelift_codegen::ir::types::*;
use cranelift_codegen::ir::{self, MemFlags};
use cranelift_codegen::ir::{Block, BlockCall, InstBuilder, JumpTableData};
use cranelift_frontend::FunctionBuilder;
use wasmtime_environ::{PtrSize, TagIndex, TypeIndex, WasmResult, WasmValType};

// TODO(frank-emrich) This is the size for x64 Linux. Once we support different
// platforms for stack switching, must select appropriate value for target.
pub const CONTROL_CONTEXT_SIZE: usize = 24;

use super::control_effect::ControlEffect;
use super::fatpointer;

/// This module contains compile-time counterparts to types defined elsewhere.
pub(crate) mod stack_switching_helpers {
    use core::marker::PhantomData;
    use cranelift_codegen::ir;
    use cranelift_codegen::ir::InstBuilder;
    use cranelift_codegen::ir::condcodes::IntCC;
    use cranelift_codegen::ir::types::*;
    use cranelift_codegen::ir::{StackSlot, StackSlotKind::*};
    use cranelift_frontend::FunctionBuilder;
    use std::mem;
    use wasmtime_environ::PtrSize;

    #[derive(Copy, Clone)]
    pub struct VMContRef {
        pub address: ir::Value,
    }

    #[derive(Copy, Clone)]
    pub struct VMHostArray<T> {
        /// Base address of this object, which must be shifted by `offset` below.
        base: ir::Value,

        /// Adding this (statically) known offset gets us the overall address.
        offset: i32,

        /// The type parameter T is never used in the fields above. We still
        /// want to have it for consistency with
        /// `wasmtime_environ::Vector` and to use it in the associated
        /// functions.
        phantom: PhantomData<T>,
    }

    pub type VMPayloads = VMHostArray<u128>;

    // Actually a vector of *mut VMTagDefinition
    pub type VMHandlerList = VMHostArray<*mut u8>;

    /// Compile-time representation of wasmtime_environ::VMStackChain,
    /// consisting of two `ir::Value`s.
    pub struct VMStackChain {
        discriminant: ir::Value,
        payload: ir::Value,
    }

    pub struct VMCommonStackInformation {
        pub address: ir::Value,
    }

    /// Compile-time representation of `crate::runtime::vm::stack::VMContinuationStack`.
    pub struct VMContinuationStack {
        /// This is NOT the "top of stack" address of the stack itself. In line
        /// with how the (runtime) `FiberStack` type works, this is a pointer to
        /// the TOS address.
        tos_ptr: ir::Value,
    }

    impl VMContRef {
        pub fn new(address: ir::Value) -> VMContRef {
            VMContRef { address }
        }

        pub fn args<'a>(
            &self,
            env: &mut crate::func_environ::FuncEnvironment<'a>,
            _builder: &mut FunctionBuilder,
        ) -> VMPayloads {
            let offset = env.offsets.ptr.vmcontref_args().into();
            VMPayloads::new(self.address, offset)
        }

        pub fn values<'a>(
            &self,
            env: &mut crate::func_environ::FuncEnvironment<'a>,
            _builder: &mut FunctionBuilder,
        ) -> VMPayloads {
            let offset = env.offsets.ptr.vmcontref_values().into();
            VMPayloads::new(self.address, offset)
        }

        pub fn common_stack_information<'a>(
            &self,
            env: &mut crate::func_environ::FuncEnvironment<'a>,
            builder: &mut FunctionBuilder,
        ) -> VMCommonStackInformation {
            let offset: i64 = env.offsets.ptr.vmcontref_common_stack_information().into();
            let address = builder.ins().iadd_imm(self.address, offset);
            VMCommonStackInformation { address }
        }

        /// Stores the parent of this continuation, which may either be another
        /// continuation or the initial stack. It is therefore represented as a
        /// `VMStackChain` element.
        pub fn set_parent_stack_chain<'a>(
            &mut self,
            env: &mut crate::func_environ::FuncEnvironment<'a>,
            builder: &mut FunctionBuilder,
            new_stack_chain: &VMStackChain,
        ) {
            let offset = env.offsets.ptr.vmcontref_parent_chain().into();
            new_stack_chain.store(env, builder, self.address, offset)
        }

        /// Loads the parent of this continuation, which may either be another
        /// continuation or the initial stack. It is therefore represented as a
        /// `VMStackChain` element.
        pub fn get_parent_stack_chain<'a>(
            &self,
            env: &mut crate::func_environ::FuncEnvironment<'a>,
            builder: &mut FunctionBuilder,
        ) -> VMStackChain {
            let offset = env.offsets.ptr.vmcontref_parent_chain().into();
            VMStackChain::load(env, builder, self.address, offset, env.pointer_type())
        }

        pub fn set_last_ancestor<'a>(
            &self,
            env: &mut crate::func_environ::FuncEnvironment<'a>,
            builder: &mut FunctionBuilder,
            last_ancestor: ir::Value,
        ) {
            let offset: i32 = env.offsets.ptr.vmcontref_last_ancestor().into();
            let mem_flags = ir::MemFlags::trusted();
            builder
                .ins()
                .store(mem_flags, last_ancestor, self.address, offset);
        }

        pub fn get_last_ancestor<'a>(
            &self,
            env: &mut crate::func_environ::FuncEnvironment<'a>,
            builder: &mut FunctionBuilder,
        ) -> ir::Value {
            let offset: i32 = env.offsets.ptr.vmcontref_last_ancestor().into();
            let mem_flags = ir::MemFlags::trusted();
            builder
                .ins()
                .load(env.pointer_type(), mem_flags, self.address, offset)
        }

        /// Gets the revision counter the a given continuation
        /// reference.
        pub fn get_revision<'a>(
            &mut self,
            env: &mut crate::func_environ::FuncEnvironment<'a>,
            builder: &mut FunctionBuilder,
        ) -> ir::Value {
            let mem_flags = ir::MemFlags::trusted();
            let offset: i32 = env.offsets.ptr.vmcontref_revision().into();
            let revision = builder.ins().load(I64, mem_flags, self.address, offset);
            revision
        }

        /// Sets the revision counter on the given continuation
        /// reference to `revision + 1`.

        pub fn incr_revision<'a>(
            &mut self,
            env: &mut crate::func_environ::FuncEnvironment<'a>,
            builder: &mut FunctionBuilder,
            revision: ir::Value,
        ) -> ir::Value {
            let mem_flags = ir::MemFlags::trusted();
            let offset: i32 = env.offsets.ptr.vmcontref_revision().into();
            let revision_plus1 = builder.ins().iadd_imm(revision, 1);
            builder
                .ins()
                .store(mem_flags, revision_plus1, self.address, offset);
            revision_plus1
        }

        pub fn get_fiber_stack<'a>(
            &self,
            env: &mut crate::func_environ::FuncEnvironment<'a>,
            builder: &mut FunctionBuilder,
        ) -> VMContinuationStack {
            // The top of stack field is stored at offset 0 of the `FiberStack`.
            let offset: i64 = env.offsets.ptr.vmcontref_stack().into();
            let fiber_stack_top_of_stack_ptr = builder.ins().iadd_imm(self.address, offset);
            VMContinuationStack::new(fiber_stack_top_of_stack_ptr)
        }
    }

    impl<T> VMHostArray<T> {
        pub(crate) fn new(base: ir::Value, offset: i32) -> Self {
            Self {
                base,
                offset,
                phantom: PhantomData::default(),
            }
        }

        fn get(&self, builder: &mut FunctionBuilder, ty: ir::Type, offset: i32) -> ir::Value {
            let mem_flags = ir::MemFlags::trusted();
            builder
                .ins()
                .load(ty, mem_flags, self.base, self.offset + offset)
        }

        fn set<U>(&self, builder: &mut FunctionBuilder, offset: i32, value: ir::Value) {
            debug_assert_eq!(
                builder.func.dfg.value_type(value),
                Type::int_with_byte_size(u16::try_from(std::mem::size_of::<U>()).unwrap()).unwrap()
            );
            let mem_flags = ir::MemFlags::trusted();
            builder
                .ins()
                .store(mem_flags, value, self.base, self.offset + offset);
        }

        pub fn get_data<'a>(
            &self,
            env: &mut crate::func_environ::FuncEnvironment<'a>,
            builder: &mut FunctionBuilder,
        ) -> ir::Value {
            let offset = env.offsets.ptr.vmhostarray_data().into();
            self.get(builder, env.pointer_type(), offset)
        }

        pub fn get_length<'a>(
            &self,
            env: &mut crate::func_environ::FuncEnvironment<'a>,
            builder: &mut FunctionBuilder,
        ) -> ir::Value {
            // Array length is stored as u32.
            let offset = env.offsets.ptr.vmhostarray_length().into();
            self.get(builder, I32, offset)
        }

        fn set_length<'a>(
            &self,
            env: &mut crate::func_environ::FuncEnvironment<'a>,
            builder: &mut FunctionBuilder,
            length: ir::Value,
        ) {
            // Array length is stored as u32.
            let offset = env.offsets.ptr.vmhostarray_length().into();
            self.set::<u32>(builder, offset, length);
        }

        fn set_capacity<'a>(
            &self,
            env: &mut crate::func_environ::FuncEnvironment<'a>,
            builder: &mut FunctionBuilder,
            capacity: ir::Value,
        ) {
            // Array capacity is stored as u32.
            let offset = env.offsets.ptr.vmhostarray_capacity().into();
            self.set::<u32>(builder, offset, capacity);
        }

        fn set_data<'a>(
            &self,
            env: &mut crate::func_environ::FuncEnvironment<'a>,
            builder: &mut FunctionBuilder,
            data: ir::Value,
        ) {
            let offset = env.offsets.ptr.vmhostarray_data().into();
            self.set::<*mut T>(builder, offset, data);
        }

        /// Returns pointer to next empty slot in data buffer and marks the
        /// subsequent `arg_count` slots as occupied.
        pub fn occupy_next_slots<'a>(
            &self,
            env: &mut crate::func_environ::FuncEnvironment<'a>,
            builder: &mut FunctionBuilder,
            arg_count: i32,
        ) -> ir::Value {
            let data = self.get_data(env, builder);
            let original_length = self.get_length(env, builder);
            let new_length = builder
                .ins()
                .iadd_imm(original_length, i64::from(arg_count));
            self.set_length(env, builder, new_length);

            let value_size: i64 = mem::size_of::<T>().try_into().unwrap();
            let original_length = builder.ins().uextend(I64, original_length);
            let byte_offset = builder.ins().imul_imm(original_length, value_size);
            builder.ins().iadd(data, byte_offset)
        }

        pub fn allocate_or_reuse_stack_slot<'a>(
            &self,
            env: &mut crate::func_environ::FuncEnvironment<'a>,
            builder: &mut FunctionBuilder,
            required_capacity: u32,
            existing_slot: Option<StackSlot>,
        ) -> StackSlot {
            let align = u8::try_from(std::mem::align_of::<T>()).unwrap();
            let entry_size = u32::try_from(std::mem::size_of::<T>()).unwrap();
            let required_size = required_capacity * entry_size;

            match existing_slot {
                Some(slot) if builder.func.get_stack_slot_data(slot).size >= required_size => {
                    let slot_data = builder.func.get_stack_slot_data(slot).clone();
                    let existing_capacity = slot_data.size / entry_size;

                    let capacity_value = builder.ins().iconst(I32, i64::from(existing_capacity));
                    debug_assert!(align <= builder.func.get_stack_slot_data(slot).align_shift);
                    debug_assert_eq!(builder.func.get_stack_slot_data(slot).kind, ExplicitSlot);

                    let existing_data = builder.ins().stack_addr(env.pointer_type(), slot, 0);

                    self.set_capacity(env, builder, capacity_value);
                    self.set_data(env, builder, existing_data);

                    slot
                }
                _ => {
                    let capacity_value = builder.ins().iconst(I32, i64::from(required_capacity));
                    let slot_size = ir::StackSlotData::new(
                        ir::StackSlotKind::ExplicitSlot,
                        required_size,
                        align,
                    );
                    let slot = builder.create_sized_stack_slot(slot_size);
                    let new_data = builder.ins().stack_addr(env.pointer_type(), slot, 0);

                    self.set_capacity(env, builder, capacity_value);
                    self.set_data(env, builder, new_data);

                    slot
                }
            }
        }

        /// Loads n entries from this Vector object, where n is the length of
        /// `load_types`, which also gives the types of the values to load.
        /// Loading starts at index 0 of the Vector object.
        pub fn load_data_entries<'a>(
            &self,
            env: &mut crate::func_environ::FuncEnvironment<'a>,
            builder: &mut FunctionBuilder,
            load_types: &[ir::Type],
        ) -> Vec<ir::Value> {
            let memflags = ir::MemFlags::trusted();

            let data_start_pointer = self.get_data(env, builder);
            let mut values = vec![];
            let mut offset = 0;
            let entry_size = i32::try_from(std::mem::size_of::<T>()).unwrap();
            for valtype in load_types {
                let val = builder
                    .ins()
                    .load(*valtype, memflags, data_start_pointer, offset);
                values.push(val);
                offset += entry_size;
            }
            values
        }

        /// Stores the given `values` in this Vector object, beginning at
        /// index 0. This expects the Vector object to be empty (i.e., current
        /// length is 0), and to be of sufficient capacity to store |`values`|
        /// entries.
        /// If `allow_smaller` is true, we allow storing values whose type has a
        /// smaller size than T's. In that case, such values will be stored at
        /// the beginning of a `T`-sized slot.
        pub fn store_data_entries<'a>(
            &self,
            env: &mut crate::func_environ::FuncEnvironment<'a>,
            builder: &mut FunctionBuilder,
            values: &[ir::Value],
            allow_smaller: bool,
        ) {
            let store_count = builder
                .ins()
                .iconst(I32, i64::try_from(values.len()).unwrap());

            // TODO(posborne): allow_smaller only used in debug_assert!
            if cfg!(debug_assertions) {
                for val in values {
                    let ty = builder.func.dfg.value_type(*val);
                    let size = usize::try_from(ty.bytes()).unwrap();
                    if allow_smaller {
                        debug_assert!(size <= std::mem::size_of::<T>());
                    } else {
                        debug_assert!(size == std::mem::size_of::<T>());
                    }
                }
            }

            let memflags = ir::MemFlags::trusted();

            let data_start_pointer = self.get_data(env, builder);

            let entry_size = i32::try_from(std::mem::size_of::<T>()).unwrap();
            let mut offset = 0;
            for value in values {
                builder
                    .ins()
                    .store(memflags, *value, data_start_pointer, offset);
                offset += entry_size;
            }

            self.set_length(env, builder, store_count);
        }

        pub fn clear<'a>(
            &self,
            env: &mut crate::func_environ::FuncEnvironment<'a>,
            builder: &mut FunctionBuilder,
            discard_buffer: bool,
        ) {
            let zero32 = builder.ins().iconst(I32, 0);
            self.set_length(env, builder, zero32);

            if discard_buffer {
                let zero32 = builder.ins().iconst(I32, 0);
                self.set_capacity(env, builder, zero32);

                let zero_ptr = builder.ins().iconst(env.pointer_type(), 0);
                self.set_data(env, builder, zero_ptr);
            }
        }
    }

    impl VMStackChain {
        /// Creates a `Self` corressponding to `VMStackChain::Continuation(contref)`.
        pub fn from_continuation<'a>(
            env: &mut crate::func_environ::FuncEnvironment<'a>,
            builder: &mut FunctionBuilder,
            contref: ir::Value,
        ) -> VMStackChain {
            debug_assert_eq!(
                env.offsets.ptr.size_of_vmstack_chain(),
                2 * env.offsets.ptr.size()
            );
            let discriminant = wasmtime_environ::STACK_CHAIN_CONTINUATION_DISCRIMINANT;
            let discriminant = builder
                .ins()
                .iconst(env.pointer_type(), i64::try_from(discriminant).unwrap());
            VMStackChain {
                discriminant,
                payload: contref,
            }
        }

        /// Creates a `Self` corressponding to `VMStackChain::Absent`.
        pub fn absent<'a>(
            env: &mut crate::func_environ::FuncEnvironment<'a>,
            builder: &mut FunctionBuilder,
        ) -> VMStackChain {
            debug_assert_eq!(
                env.offsets.ptr.size_of_vmstack_chain(),
                2 * env.offsets.ptr.size()
            );
            let discriminant = wasmtime_environ::STACK_CHAIN_ABSENT_DISCRIMINANT;
            let discriminant = builder
                .ins()
                .iconst(env.pointer_type(), i64::try_from(discriminant).unwrap());
            let zero_filler = builder.ins().iconst(env.pointer_type(), 0i64);
            VMStackChain {
                discriminant,
                payload: zero_filler,
            }
        }

        pub fn is_initial_stack<'a>(
            &self,
            _env: &mut crate::func_environ::FuncEnvironment<'a>,
            builder: &mut FunctionBuilder,
        ) -> ir::Value {
            builder.ins().icmp_imm(
                IntCC::Equal,
                self.discriminant,
                i64::try_from(wasmtime_environ::STACK_CHAIN_INITIAL_STACK_DISCRIMINANT).unwrap(),
            )
        }

        /// Return the two raw `ir::Value`s that represent this VMStackChain.
        pub fn to_raw_parts(&self) -> [ir::Value; 2] {
            [self.discriminant, self.payload]
        }

        /// Construct a `Self` from two raw `ir::Value`s.
        pub fn from_raw_parts(raw_data: [ir::Value; 2]) -> VMStackChain {
            VMStackChain {
                discriminant: raw_data[0],
                payload: raw_data[1],
            }
        }

        /// Load a `VMStackChain` object from the given address.
        pub fn load<'a>(
            _env: &mut crate::func_environ::FuncEnvironment<'a>,
            builder: &mut FunctionBuilder,
            pointer: ir::Value,
            initial_offset: i32,
            pointer_type: ir::Type,
        ) -> VMStackChain {
            let memflags = ir::MemFlags::trusted();
            let mut offset = initial_offset;
            let mut data = vec![];
            for _ in 0..2 {
                data.push(builder.ins().load(pointer_type, memflags, pointer, offset));
                offset += i32::try_from(pointer_type.bytes()).unwrap();
            }
            let data = <[ir::Value; 2]>::try_from(data).unwrap();
            Self::from_raw_parts(data)
        }

        /// Store this `VMStackChain` object at the given address.
        pub fn store<'a>(
            &self,
            env: &mut crate::func_environ::FuncEnvironment<'a>,
            builder: &mut FunctionBuilder,
            target_pointer: ir::Value,
            initial_offset: i32,
        ) {
            let memflags = ir::MemFlags::trusted();
            let mut offset = initial_offset;
            let data = self.to_raw_parts();

            for value in data {
                debug_assert_eq!(builder.func.dfg.value_type(value), env.pointer_type());
                builder.ins().store(memflags, value, target_pointer, offset);
                offset += i32::try_from(env.pointer_type().bytes()).unwrap();
            }
        }

        /// Use this only if you've already checked that `self` corresponds to a `VMStackChain::Continuation`.
        pub fn unchecked_get_continuation<'a>(
            &self,
            _env: &mut crate::func_environ::FuncEnvironment<'a>,
            _builder: &mut FunctionBuilder,
        ) -> ir::Value {
            // TODO(posborne): this used to have emitted assertions but now does not do much.
            self.payload
        }

        /// Must only be called if `self` represents a `InitialStack` or
        /// `Continuation` variant. Returns a pointer to the associated
        /// `CommonStackInformation` object.
        pub fn get_common_stack_information<'a>(
            &self,
            env: &mut crate::func_environ::FuncEnvironment<'a>,
            _builder: &mut FunctionBuilder,
        ) -> VMCommonStackInformation {
            // `self` corresponds to a VMStackChain::InitialStack or
            // VMStackChain::Continuation.
            // In both cases, the payload is a pointer.
            let address = self.payload;

            // `obj` is now a pointer to the beginning of either
            // 1. A `VMContRef` struct (in the case of a
            // VMStackChain::Continuation)
            // 2. A CommonStackInformation struct (in the case of
            // VMStackChain::InitialStack)
            //
            // Since a `VMContRef` starts with an (inlined) CommonStackInformation
            // object at offset 0, we actually have in both cases that `ptr` is
            // now the address of the beginning of a VMStackLimits object.
            debug_assert_eq!(env.offsets.ptr.vmcontref_common_stack_information(), 0);
            VMCommonStackInformation { address }
        }
    }

    impl VMCommonStackInformation {
        fn get_state_ptr<'a>(
            &self,
            env: &mut crate::func_environ::FuncEnvironment<'a>,
            builder: &mut FunctionBuilder,
        ) -> ir::Value {
            let offset: i64 = env.offsets.ptr.vmcommon_stack_information_state().into();

            builder.ins().iadd_imm(self.address, offset)
        }

        fn get_stack_limits_ptr<'a>(
            &self,
            env: &mut crate::func_environ::FuncEnvironment<'a>,
            builder: &mut FunctionBuilder,
        ) -> ir::Value {
            let offset: i64 = env.offsets.ptr.vmcommon_stack_information_limits().into();

            builder.ins().iadd_imm(self.address, offset)
        }

        fn load_state<'a>(
            &self,
            env: &mut crate::func_environ::FuncEnvironment<'a>,
            builder: &mut FunctionBuilder,
        ) -> ir::Value {
            let mem_flags = ir::MemFlags::trusted();
            let state_ptr = self.get_state_ptr(env, builder);

            builder.ins().load(I32, mem_flags, state_ptr, 0)
        }

        fn set_state_no_payload<'a>(
            &self,
            env: &mut crate::func_environ::FuncEnvironment<'a>,
            builder: &mut FunctionBuilder,
            discriminant: u32,
        ) {
            let discriminant = builder.ins().iconst(I32, i64::from(discriminant));
            let mem_flags = ir::MemFlags::trusted();
            let state_ptr = self.get_state_ptr(env, builder);

            builder.ins().store(mem_flags, discriminant, state_ptr, 0);
        }

        pub fn set_state_running<'a>(
            &self,
            env: &mut crate::func_environ::FuncEnvironment<'a>,
            builder: &mut FunctionBuilder,
        ) {
            let discriminant = wasmtime_environ::STACK_STATE_RUNNING_DISCRIMINANT;
            self.set_state_no_payload(env, builder, discriminant);
        }

        pub fn set_state_parent<'a>(
            &self,
            env: &mut crate::func_environ::FuncEnvironment<'a>,
            builder: &mut FunctionBuilder,
        ) {
            let discriminant = wasmtime_environ::STACK_STATE_PARENT_DISCRIMINANT;
            self.set_state_no_payload(env, builder, discriminant);
        }

        pub fn set_state_returned<'a>(
            &self,
            env: &mut crate::func_environ::FuncEnvironment<'a>,
            builder: &mut FunctionBuilder,
        ) {
            let discriminant = wasmtime_environ::STACK_STATE_RETURNED_DISCRIMINANT;
            self.set_state_no_payload(env, builder, discriminant);
        }

        pub fn set_state_suspended<'a>(
            &self,
            env: &mut crate::func_environ::FuncEnvironment<'a>,
            builder: &mut FunctionBuilder,
        ) {
            let discriminant = wasmtime_environ::STACK_STATE_SUSPENDED_DISCRIMINANT;
            self.set_state_no_payload(env, builder, discriminant);
        }

        /// Checks whether the `VMStackState` reflects that the stack has ever been
        /// active (instead of just having been allocated, but never resumed).
        pub fn was_invoked<'a>(
            &self,
            env: &mut crate::func_environ::FuncEnvironment<'a>,
            builder: &mut FunctionBuilder,
        ) -> ir::Value {
            let actual_state = self.load_state(env, builder);
            let allocated = wasmtime_environ::STACK_STATE_FRESH_DISCRIMINANT;
            builder
                .ins()
                .icmp_imm(IntCC::NotEqual, actual_state, i64::from(allocated))
        }

        pub fn get_handler_list<'a>(
            &self,
            env: &mut crate::func_environ::FuncEnvironment<'a>,
            _builder: &mut FunctionBuilder,
        ) -> VMHandlerList {
            let offset = env.offsets.ptr.vmcommon_stack_information_handlers().into();
            VMHandlerList::new(self.address, offset)
        }

        pub fn get_first_switch_handler_index<'a>(
            &self,
            env: &mut crate::func_environ::FuncEnvironment<'a>,
            builder: &mut FunctionBuilder,
        ) -> ir::Value {
            // Field first_switch_handler_index has type u32
            let memflags = ir::MemFlags::trusted();
            let offset: i32 = env
                .offsets
                .ptr
                .vmcommon_stack_information_first_switch_handler_index()
                .into();
            builder.ins().load(I32, memflags, self.address, offset)
        }

        pub fn set_first_switch_handler_index<'a>(
            &self,
            env: &mut crate::func_environ::FuncEnvironment<'a>,
            builder: &mut FunctionBuilder,
            value: ir::Value,
        ) {
            // Field first_switch_handler_index has type u32
            let memflags = ir::MemFlags::trusted();
            let offset: i32 = env
                .offsets
                .ptr
                .vmcommon_stack_information_first_switch_handler_index()
                .into();
            builder.ins().store(memflags, value, self.address, offset);
        }

        /// Sets `last_wasm_entry_sp` and `stack_limit` fields in
        /// `VMRuntimelimits` using the values from the `VMStackLimits` of this
        /// object.
        pub fn write_limits_to_vmcontext<'a>(
            &self,
            env: &mut crate::func_environ::FuncEnvironment<'a>,
            builder: &mut FunctionBuilder,
            vmruntime_limits_ptr: ir::Value,
        ) {
            let stack_limits_ptr = self.get_stack_limits_ptr(env, builder);

            let memflags = ir::MemFlags::trusted();

            let mut copy_to_vm_runtime_limits = |our_offset, their_offset| {
                let our_value = builder.ins().load(
                    env.pointer_type(),
                    memflags,
                    stack_limits_ptr,
                    i32::from(our_offset),
                );
                builder.ins().store(
                    memflags,
                    our_value,
                    vmruntime_limits_ptr,
                    i32::from(their_offset),
                );
            };

            let pointer_size = u8::try_from(env.pointer_type().bytes()).unwrap();
            let stack_limit_offset = env.offsets.ptr.vmstack_limits_stack_limit();
            let last_wasm_entry_fp_offset = env.offsets.ptr.vmstack_limits_last_wasm_entry_fp();
            copy_to_vm_runtime_limits(
                stack_limit_offset,
                pointer_size.vmstore_context_stack_limit(),
            );
            copy_to_vm_runtime_limits(
                last_wasm_entry_fp_offset,
                pointer_size.vmstore_context_last_wasm_entry_fp(),
            );
        }

        /// Overwrites the `last_wasm_entry_fp` field of the `VMStackLimits`
        /// object in the `VMStackLimits` of this object by loading the corresponding
        /// field from the `VMRuntimeLimits`.
        /// If `load_stack_limit` is true, we do the same for the `stack_limit`
        /// field.
        pub fn load_limits_from_vmcontext<'a>(
            &self,
            env: &mut crate::func_environ::FuncEnvironment<'a>,
            builder: &mut FunctionBuilder,
            vmruntime_limits_ptr: ir::Value,
            load_stack_limit: bool,
        ) {
            let stack_limits_ptr = self.get_stack_limits_ptr(env, builder);

            let memflags = ir::MemFlags::trusted();
            let pointer_size = u8::try_from(env.pointer_type().bytes()).unwrap();

            let mut copy = |runtime_limits_offset, stack_limits_offset| {
                let from_vm_runtime_limits = builder.ins().load(
                    env.pointer_type(),
                    memflags,
                    vmruntime_limits_ptr,
                    runtime_limits_offset,
                );
                builder.ins().store(
                    memflags,
                    from_vm_runtime_limits,
                    stack_limits_ptr,
                    stack_limits_offset,
                );
            };

            let last_wasm_entry_fp_offset = env.offsets.ptr.vmstack_limits_last_wasm_entry_fp();
            copy(
                pointer_size.vmstore_context_last_wasm_entry_fp(),
                last_wasm_entry_fp_offset,
            );

            if load_stack_limit {
                let stack_limit_offset = env.offsets.ptr.vmstack_limits_stack_limit();
                copy(
                    pointer_size.vmstore_context_stack_limit(),
                    stack_limit_offset,
                );
            }
        }
    }

    impl VMContinuationStack {
        /// The parameter is NOT the "top of stack" address of the stack itself. In line
        /// with how the (runtime) `FiberStack` type works, this is a pointer to
        /// the TOS address.
        pub fn new(tos_ptr: ir::Value) -> Self {
            Self { tos_ptr }
        }

        fn load_top_of_stack<'a>(
            &self,
            _env: &mut crate::func_environ::FuncEnvironment<'a>,
            builder: &mut FunctionBuilder,
        ) -> ir::Value {
            let mem_flags = ir::MemFlags::trusted();
            builder.ins().load(I64, mem_flags, self.tos_ptr, 0)
        }

        /// Returns address of the control context stored in the stack memory,
        /// as used by stack_switch instructions.
        pub fn load_control_context<'a>(
            &self,
            env: &mut crate::func_environ::FuncEnvironment<'a>,
            builder: &mut FunctionBuilder,
        ) -> ir::Value {
            let tos = self.load_top_of_stack(env, builder);
            // Control context begins 24 bytes below top of stack (see unix.rs)
            builder.ins().iadd_imm(tos, -0x18)
        }
    }
}

use helpers::VMStackChain;
use stack_switching_helpers as helpers;

/// Stores the given arguments in the appropriate `VMPayloads` object in the
/// continuation. If the continuation was never invoked, use the `args` object.
/// Otherwise, use the `values` object.
pub(crate) fn vmcontref_store_payloads<'a>(
    env: &mut crate::func_environ::FuncEnvironment<'a>,
    builder: &mut FunctionBuilder,
    values: &[ir::Value],
    contref: ir::Value,
) {
    let count =
        i32::try_from(values.len()).expect("Number of stack switching payloads should fit in i32");
    if values.len() > 0 {
        let use_args_block = builder.create_block();
        let use_payloads_block = builder.create_block();
        let store_data_block = builder.create_block();
        builder.append_block_param(store_data_block, env.pointer_type());

        let co = helpers::VMContRef::new(contref);
        let csi = co.common_stack_information(env, builder);
        let was_invoked = csi.was_invoked(env, builder);
        builder
            .ins()
            .brif(was_invoked, use_payloads_block, &[], use_args_block, &[]);

        {
            builder.switch_to_block(use_args_block);
            builder.seal_block(use_args_block);

            let args = co.args(env, builder);
            let ptr = args.occupy_next_slots(env, builder, count);

            builder
                .ins()
                .jump(store_data_block, &[BlockArg::Value(ptr)]);
        }

        {
            builder.switch_to_block(use_payloads_block);
            builder.seal_block(use_payloads_block);

            let payloads = co.values(env, builder);

            // This also checks that the buffer is large enough to hold
            // `values.len()` more elements.
            let ptr = payloads.occupy_next_slots(env, builder, count);
            builder
                .ins()
                .jump(store_data_block, &[BlockArg::Value(ptr)]);
        }

        {
            builder.switch_to_block(store_data_block);
            builder.seal_block(store_data_block);

            let ptr = builder.block_params(store_data_block)[0];

            // Store the values.
            let memflags = ir::MemFlags::trusted();
            let mut offset = 0;
            for value in values {
                builder.ins().store(memflags, *value, ptr, offset);
                offset += i32::from(env.offsets.ptr.maximum_value_size());
            }
        }
    }
}

pub(crate) fn tag_address<'a>(
    env: &mut crate::func_environ::FuncEnvironment<'a>,
    builder: &mut FunctionBuilder,
    index: u32,
) -> ir::Value {
    let vmctx = env.vmctx_val(&mut builder.cursor());
    let tag_index = wasmtime_environ::TagIndex::from_u32(index);
    let pointer_type = env.pointer_type();
    if let Some(def_index) = env.module.defined_tag_index(tag_index) {
        let offset = i32::try_from(env.offsets.vmctx_vmtag_definition(def_index)).unwrap();
        builder.ins().iadd_imm(vmctx, i64::from(offset))
    } else {
        let offset = i32::try_from(env.offsets.vmctx_vmtag_import_from(tag_index)).unwrap();
        builder.ins().load(
            pointer_type,
            ir::MemFlags::trusted().with_readonly(),
            vmctx,
            ir::immediates::Offset32::new(offset),
        )
    }
}

/// Returns the stack chain saved in the given `VMContext`. Note that the
/// head of the list is the actively running stack (initial stack or
/// continuation).
pub fn vmctx_load_stack_chain<'a>(
    env: &mut crate::func_environ::FuncEnvironment<'a>,
    builder: &mut FunctionBuilder,
    vmctx: ir::Value,
) -> VMStackChain {
    let stack_chain_offset = env.offsets.ptr.vmstore_context_stack_chain().into();

    // First we need to get the `VMStoreContext`.
    let vm_store_context_offset = env.offsets.ptr.vmctx_store_context();
    let vm_store_context = builder.ins().load(
        env.pointer_type(),
        MemFlags::trusted(),
        vmctx,
        vm_store_context_offset,
    );

    VMStackChain::load(
        env,
        builder,
        vm_store_context,
        stack_chain_offset,
        env.pointer_type(),
    )
}

/// Stores the given stack chain saved in the `VMContext`, overwriting the
/// exsiting one.
pub fn vmctx_store_stack_chain<'a>(
    env: &mut crate::func_environ::FuncEnvironment<'a>,
    builder: &mut FunctionBuilder,
    vmctx: ir::Value,
    stack_chain: &VMStackChain,
) {
    let stack_chain_offset = env.offsets.ptr.vmstore_context_stack_chain().into();

    // First we need to get the `VMStoreContext`.
    let vm_store_context_offset = env.offsets.ptr.vmctx_store_context();
    let vm_store_context = builder.ins().load(
        env.pointer_type(),
        MemFlags::trusted(),
        vmctx,
        vm_store_context_offset,
    );

    stack_chain.store(env, builder, vm_store_context, stack_chain_offset)
}

/// Similar to `vmctx_store_stack_chain`, but instead of storing an arbitrary
/// `VMStackChain`, stores VMStackChain::Continuation(contref)`.
pub fn vmctx_set_active_continuation<'a>(
    env: &mut crate::func_environ::FuncEnvironment<'a>,
    builder: &mut FunctionBuilder,
    vmctx: ir::Value,
    contref: ir::Value,
) {
    let chain = VMStackChain::from_continuation(env, builder, contref);
    vmctx_store_stack_chain(env, builder, vmctx, &chain)
}

pub fn vmctx_load_vm_runtime_limits_ptr<'a>(
    env: &mut crate::func_environ::FuncEnvironment<'a>,
    builder: &mut FunctionBuilder,
    vmctx: ir::Value,
) -> ir::Value {
    let pointer_type = env.pointer_type();
    let offset = i32::from(env.offsets.ptr.vmctx_store_context());

    // The *pointer* to the VMRuntimeLimits does not change within the
    // same function, allowing us to set the `read_only` flag.
    let flags = ir::MemFlags::trusted().with_readonly();

    builder.ins().load(pointer_type, flags, vmctx, offset)
}

/// This function generates code that searches for a handler for `tag_address`,
/// which must be a `*mut VMTagDefinition`. The search walks up the chain of
/// continuations beginning at `start`.
///
/// The flag `search_suspend_handlers` determines whether we search for a
/// suspend or switch handler. Concretely, this influences which part of each
/// handler list we will search.
///
/// We trap if no handler was found.
///
/// The returned values are:
/// 1. The stack (continuation or initial stack, represented as a VMStackChain) in
///    whose handler list we found the tag (i.e., the stack that performed the
///    resume instruction that installed handler for the tag).
/// 2. The continuation whose parent is the stack mentioned in 1.
/// 3. The index of the handler in the handler list.
///
/// In pseudo-code, the generated code's behavior can be expressed as
/// follows:
///
/// chain_link = start
/// while !chain_link.is_initial_stack() {
///   contref = chain_link.get_contref()
///   parent_link = contref.parent
///   parent_csi = parent_link.get_common_stack_information();
///   handlers = parent_csi.handlers;
///   (begin_range, end_range) = if search_suspend_handlers {
///     (0, parent_csi.first_switch_handler_index)
///   } else {
///     (parent_csi.first_switch_handler_index, handlers.length)
///   };
///   for index in begin_range..end_range {
///     if handlers[index] == tag_address {
///       goto on_match(contref, index)
///     }
///   }
///   chain_link = parent_link
/// }
/// trap(unhandled_tag)
///
/// on_match(conref : VMContRef, handler_index : u32)
/// ... execution continues here here ...
///
fn search_handler<'a>(
    env: &mut crate::func_environ::FuncEnvironment<'a>,
    builder: &mut FunctionBuilder,
    start: &helpers::VMStackChain,
    tag_address: ir::Value,
    search_suspend_handlers: bool,
) -> (VMStackChain, ir::Value, ir::Value) {
    let handle_link = builder.create_block();
    let begin_search_handler_list = builder.create_block();
    let try_index = builder.create_block();
    let compare_tags = builder.create_block();
    let on_match = builder.create_block();
    let on_no_match = builder.create_block();
    let block_args = start.to_raw_parts().map(|v| BlockArg::Value(v));

    // Terminate previous block:
    builder.ins().jump(handle_link, &block_args);

    // Block handle_link
    let chain_link = {
        builder.append_block_param(handle_link, env.pointer_type());
        builder.append_block_param(handle_link, env.pointer_type());
        builder.switch_to_block(handle_link);

        let raw_parts = builder.block_params(handle_link);
        let chain_link = helpers::VMStackChain::from_raw_parts([raw_parts[0], raw_parts[1]]);
        let is_initial_stack = chain_link.is_initial_stack(env, builder);
        builder.ins().brif(
            is_initial_stack,
            on_no_match,
            &[],
            begin_search_handler_list,
            &[],
        );
        chain_link
    };

    // Block begin_search_handler_list
    let (contref, parent_link, handler_list_data_ptr, end_range) = {
        builder.switch_to_block(begin_search_handler_list);
        let contref = chain_link.unchecked_get_continuation(env, builder);
        let contref = helpers::VMContRef::new(contref);

        let parent_link = contref.get_parent_stack_chain(env, builder);
        let parent_csi = parent_link.get_common_stack_information(env, builder);

        let handlers = parent_csi.get_handler_list(env, builder);
        let handler_list_data_ptr = handlers.get_data(env, builder);

        let first_switch_handler_index = parent_csi.get_first_switch_handler_index(env, builder);

        // Note that these indices are inclusive-exclusive, i.e. [begin_range, end_range).
        let (begin_range, end_range) = if search_suspend_handlers {
            let zero = builder.ins().iconst(I32, 0);
            (zero, first_switch_handler_index)
        } else {
            let length = handlers.get_length(env, builder);
            (first_switch_handler_index, length)
        };

        builder
            .ins()
            .jump(try_index, &[BlockArg::Value(begin_range)]);

        (contref, parent_link, handler_list_data_ptr, end_range)
    };

    // Block try_index
    let index = {
        builder.append_block_param(try_index, I32);
        builder.switch_to_block(try_index);
        let index = builder.block_params(try_index)[0];

        let in_bounds = builder
            .ins()
            .icmp(IntCC::UnsignedLessThan, index, end_range);
        let block_args = parent_link.to_raw_parts().map(|v| BlockArg::Value(v));
        builder
            .ins()
            .brif(in_bounds, compare_tags, &[], handle_link, &block_args);
        index
    };

    // Block compare_tags
    {
        builder.switch_to_block(compare_tags);

        let base = handler_list_data_ptr;
        let entry_size = std::mem::size_of::<*mut u8>();
        let offset = builder
            .ins()
            .imul_imm(index, i64::try_from(entry_size).unwrap());
        let offset = builder.ins().uextend(I64, offset);
        let entry_address = builder.ins().iadd(base, offset);

        let memflags = ir::MemFlags::trusted();

        let handled_tag = builder
            .ins()
            .load(env.pointer_type(), memflags, entry_address, 0);

        let tags_match = builder.ins().icmp(IntCC::Equal, handled_tag, tag_address);
        let incremented_index = builder.ins().iadd_imm(index, 1);
        builder.ins().brif(
            tags_match,
            on_match,
            &[],
            try_index,
            &[BlockArg::Value(incremented_index)],
        );
    }

    // Block on_no_match
    {
        builder.switch_to_block(on_no_match);
        builder.set_cold_block(on_no_match);
        builder.ins().trap(crate::TRAP_UNHANDLED_TAG);
    }

    builder.seal_block(handle_link);
    builder.seal_block(begin_search_handler_list);
    builder.seal_block(try_index);
    builder.seal_block(compare_tags);
    builder.seal_block(on_match);
    builder.seal_block(on_no_match);

    // final block: on_match
    builder.switch_to_block(on_match);

    (parent_link, contref.address, index)
}

pub(crate) fn translate_cont_bind<'a>(
    env: &mut crate::func_environ::FuncEnvironment<'a>,
    builder: &mut FunctionBuilder,
    contobj: ir::Value,
    args: &[ir::Value],
) -> ir::Value {
    let (witness, contref) = fatpointer::deconstruct(env, &mut builder.cursor(), contobj);

    // The typing rules for cont.bind allow a null reference to be passed to it.
    builder.ins().trapz(contref, crate::TRAP_NULL_REFERENCE);

    let mut vmcontref = helpers::VMContRef::new(contref);
    let revision = vmcontref.get_revision(env, builder);
    let evidence = builder.ins().icmp(IntCC::Equal, witness, revision);
    builder
        .ins()
        .trapz(evidence, crate::TRAP_CONTINUATION_ALREADY_CONSUMED);

    vmcontref_store_payloads(env, builder, args, contref);

    let revision = vmcontref.incr_revision(env, builder, revision);
    let contobj = fatpointer::construct(env, &mut builder.cursor(), revision, contref);
    contobj
}

pub(crate) fn translate_cont_new<'a>(
    env: &mut crate::func_environ::FuncEnvironment<'a>,
    builder: &mut FunctionBuilder,
    func: ir::Value,
    arg_types: &[WasmValType],
    return_types: &[WasmValType],
) -> WasmResult<ir::Value> {
    // The typing rules for cont.new allow a null reference to be passed to it.
    builder.ins().trapz(func, crate::TRAP_NULL_REFERENCE);

    let nargs = builder
        .ins()
        .iconst(I32, i64::try_from(arg_types.len()).unwrap());
    let nreturns = builder
        .ins()
        .iconst(I32, i64::try_from(return_types.len()).unwrap());

    let cont_new_func = super::builtins::cont_new(env, &mut builder.func)?;
    let vmctx = env.vmctx_val(&mut builder.cursor());
    let call_inst = builder
        .ins()
        .call(cont_new_func, &[vmctx, func, nargs, nreturns]);
    let contref = *builder.func.dfg.inst_results(call_inst).first().unwrap();

    let tag = helpers::VMContRef::new(contref).get_revision(env, builder);
    let contobj = fatpointer::construct(env, &mut builder.cursor(), tag, contref);
    Ok(contobj)
}

pub(crate) fn translate_resume<'a>(
    env: &mut crate::func_environ::FuncEnvironment<'a>,
    builder: &mut FunctionBuilder,
    type_index: u32,
    resume_contobj: ir::Value,
    resume_args: &[ir::Value],
    resumetable: &[(u32, Option<ir::Block>)],
) -> WasmResult<Vec<ir::Value>> {
    // The resume instruction is the most involved instruction to
    // compile as it is responsible for both continuation application
    // and control tag dispatch.
    //
    // Here we translate a resume instruction into several basic
    // blocks as follows:
    //
    //        previous block
    //              |
    //              |
    //        resume_block
    //         /           \
    //        /             \
    //        |             |
    //  return_block        |
    //                suspend block
    //                      |
    //                dispatch block
    //
    // * resume_block handles continuation arguments and performs
    //   actual stack switch. On ordinary return from resume, it jumps
    //   to the `return_block`, whereas on suspension it jumps to the
    //   `suspend_block`.
    // * suspend_block is used on suspension, jumps onward to
    //   `dispatch_block`.
    // * dispatch_block uses a jump table to dispatch to actual
    //   user-defined handler blocks, based on the handler index
    //   provided on suspension. Note that we do not jump to the
    //   handler blocks directly. Instead, each handler block has a
    //   corresponding premable block, which we jump to in order to
    //   reach a particular handler block. The preamble block prepares
    //   the arguments and continuation object to be passed to the
    //   actual handler block.
    //
    let resume_block = builder.create_block();
    let return_block = builder.create_block();
    let suspend_block = builder.create_block();
    let dispatch_block = builder.create_block();

    let vmctx = env.vmctx_val(&mut builder.cursor());

    // Split the resumetable into suspend handlers (each represented by the tag
    // index and handler block) and the switch handlers (represented just by the
    // tag index). Note that we currently don't remove duplicate tags.
    let (suspend_handlers, switch_tags): (Vec<(u32, Block)>, Vec<u32>) = resumetable
        .iter()
        .partition_map(|(tag_index, block_opt)| match block_opt {
            Some(block) => Either::Left((*tag_index, *block)),
            None => Either::Right(*tag_index),
        });

    // Technically, there is no need to have a dedicated resume block, we could
    // just put all of its contents into the current block.
    builder.ins().jump(resume_block, &[]);

    // Resume block: actually resume the continuation chain ending at `resume_contref`.
    let (resume_result, vm_runtime_limits_ptr, original_stack_chain, new_stack_chain) = {
        builder.switch_to_block(resume_block);
        builder.seal_block(resume_block);

        let (witness, resume_contref) =
            fatpointer::deconstruct(env, &mut builder.cursor(), resume_contobj);

        // The typing rules for resume allow a null reference to be passed to it.
        builder
            .ins()
            .trapz(resume_contref, crate::TRAP_NULL_REFERENCE);

        let mut vmcontref = helpers::VMContRef::new(resume_contref);

        let revision = vmcontref.get_revision(env, builder);
        let evidence = builder.ins().icmp(IntCC::Equal, revision, witness);
        builder
            .ins()
            .trapz(evidence, crate::TRAP_CONTINUATION_ALREADY_CONSUMED);
        let _next_revision = vmcontref.incr_revision(env, builder, revision);

        if resume_args.len() > 0 {
            // We store the arguments in the `VMContRef` to be resumed.
            vmcontref_store_payloads(env, builder, resume_args, resume_contref);
        }

        // Splice together stack chains:
        // Connect the end of the chain starting at `resume_contref` to the currently active chain.
        let mut last_ancestor = helpers::VMContRef::new(vmcontref.get_last_ancestor(env, builder));

        // Make the currently running continuation (if any) the parent of the one we are about to resume.
        let original_stack_chain = vmctx_load_stack_chain(env, builder, vmctx);
        last_ancestor.set_parent_stack_chain(env, builder, &original_stack_chain);

        // Just for consistency: `vmcontref` is about to get state Running, so let's zero out its last_ancestor field.
        let zero = builder.ins().iconst(env.pointer_type(), 0);
        vmcontref.set_last_ancestor(env, builder, zero);

        // We mark `resume_contref` as the currently running one
        vmctx_set_active_continuation(env, builder, vmctx, resume_contref);

        // Note that the resume_contref libcall a few lines further below
        // manipulates the stack limits as follows:
        // 1. Copy stack_limit, last_wasm_entry_sp and last_wasm_exit* values from
        // VMRuntimeLimits into the currently active continuation (i.e., the
        // one that will become the parent of the to-be-resumed one)
        //
        // 2. Copy `stack_limit` and `last_wasm_entry_sp` in the
        // `VMStackLimits` of `resume_contref` into the `VMRuntimeLimits`.
        //
        // See the comment on `wasmtime_environ::VMStackChain` for a
        // description of the invariants that we maintain for the various stack
        // limits.

        // `resume_contref` is now active, and its parent is suspended.
        let resume_contref = helpers::VMContRef::new(resume_contref);
        let resume_csi = resume_contref.common_stack_information(env, builder);
        let parent_csi = original_stack_chain.get_common_stack_information(env, builder);
        resume_csi.set_state_running(env, builder);
        parent_csi.set_state_parent(env, builder);

        // We update the `VMStackLimits` of the parent of the continuation to be resumed
        // as well as the `VMRuntimeLimits`.
        // See the comment on `wasmtime_environ::VMStackChain` for a description
        // of the invariants that we maintain for the various stack limits.
        let vm_runtime_limits_ptr = vmctx_load_vm_runtime_limits_ptr(env, builder, vmctx);
        parent_csi.load_limits_from_vmcontext(env, builder, vm_runtime_limits_ptr, true);
        resume_csi.write_limits_to_vmcontext(env, builder, vm_runtime_limits_ptr);

        // Install handlers in (soon to be) parent's VMHandlerList:
        // Let the i-th handler clause be (on $tag $block).
        // Then the i-th entry of the VMHandlerList will be the address of $tag.
        let handler_list = parent_csi.get_handler_list(env, builder);

        if resumetable.len() > 0 {
            // Total number of handlers (suspend and switch).
            let handler_count = u32::try_from(resumetable.len()).unwrap();
            // Populate the Array's data ptr with a pointer to a sufficiently
            // large area on this stack.
            env.stack_switching_handler_list_buffer =
                Some(handler_list.allocate_or_reuse_stack_slot(
                    env,
                    builder,
                    handler_count,
                    env.stack_switching_handler_list_buffer,
                ));

            let suspend_handler_count = suspend_handlers.len();

            // All handlers, represented by the indices of the tags they handle.
            // All the suspend handlers come first, followed by all the switch handlers.
            let all_handlers = suspend_handlers
                .iter()
                .map(|(tag_index, _block)| *tag_index)
                .chain(switch_tags);

            // Translate all tag indices to tag addresses (i.e., the corresponding *mut VMTagDefinition).
            let all_tag_addresses: Vec<ir::Value> = all_handlers
                .map(|tag_index| tag_address(env, builder, tag_index))
                .collect();

            // Store all tag addresess in the handler list.
            handler_list.store_data_entries(env, builder, &all_tag_addresses, false);

            // To enable distinguishing switch and suspend handlers when searching the handler list:
            // Store at which index the switch handlers start.
            let first_switch_handler_index = builder
                .ins()
                .iconst(I32, i64::try_from(suspend_handler_count).unwrap());
            parent_csi.set_first_switch_handler_index(env, builder, first_switch_handler_index);
        }

        let resume_payload = ControlEffect::encode_resume(builder).to_u64();

        // Note that the control context we use for switching is not the one in
        // (the stack of) resume_contref, but in (the stack of) last_ancestor!
        let fiber_stack = last_ancestor.get_fiber_stack(env, builder);
        let control_context_ptr = fiber_stack.load_control_context(env, builder);

        let result =
            builder
                .ins()
                .stack_switch(control_context_ptr, control_context_ptr, resume_payload);

        // At this point we know nothing about the continuation that just
        // suspended or returned. In particular, it does not have to be what we
        // called `resume_contref` earlier on. We must reload the information
        // about the now active continuation from the VMContext.
        let new_stack_chain = vmctx_load_stack_chain(env, builder, vmctx);

        // Now the parent contref (or initial stack) is active again
        vmctx_store_stack_chain(env, builder, vmctx, &original_stack_chain);
        parent_csi.set_state_running(env, builder);

        // Just for consistency: Clear the handler list.
        handler_list.clear(env, builder, true);
        parent_csi.set_first_switch_handler_index(env, builder, zero);

        // Extract the result and signal bit.
        let result = ControlEffect::from_u64(result);
        let signal = result.signal(builder);

        // Jump to the return block if the result signal is 0, otherwise jump to
        // the suspend block.
        builder
            .ins()
            .brif(signal, suspend_block, &[], return_block, &[]);

        (
            result,
            vm_runtime_limits_ptr,
            original_stack_chain,
            new_stack_chain,
        )
    };

    // The suspend block: Only used when we suspended, not for returns.
    // Here we extract the index of the handler to use.
    let (handler_index, suspended_contref, suspended_contobj) = {
        builder.switch_to_block(suspend_block);
        builder.seal_block(suspend_block);

        let suspended_continuation = new_stack_chain.unchecked_get_continuation(env, builder);
        let mut suspended_continuation = helpers::VMContRef::new(suspended_continuation);
        let suspended_csi = suspended_continuation.common_stack_information(env, builder);

        // Note that at the suspend site, we already
        // 1. Set the state of suspended_continuation to Suspended
        // 2. Set suspended_continuation.last_ancestor
        // 3. Broke the continuation chain at suspended_continuation.last_ancestor

        // We store parts of the VMRuntimeLimits into the continuation that just suspended.
        suspended_csi.load_limits_from_vmcontext(env, builder, vm_runtime_limits_ptr, false);

        // Afterwards (!), restore parts of the VMRuntimeLimits from the
        // parent of the suspended continuation (which is now active).
        let parent_csi = original_stack_chain.get_common_stack_information(env, builder);
        parent_csi.write_limits_to_vmcontext(env, builder, vm_runtime_limits_ptr);

        // Extract the handler index
        let handler_index = resume_result.handler_index(builder);

        let revision = suspended_continuation.get_revision(env, builder);
        let suspended_contobj = fatpointer::construct(
            env,
            &mut builder.cursor(),
            revision,
            suspended_continuation.address,
        );

        // We need to terminate this block before being allowed to switch to
        // another one.
        builder.ins().jump(dispatch_block, &[]);

        (handler_index, suspended_continuation, suspended_contobj)
    };

    // For technical reasons, the jump table needs to have a default
    // block. In our case, it should be unreachable, since the handler
    // index we dispatch on should correspond to a an actual handler
    // block in the jump table.
    let jt_default_block = builder.create_block();
    {
        builder.switch_to_block(jt_default_block);
        builder.set_cold_block(jt_default_block);

        builder.ins().trap(crate::TRAP_UNREACHABLE);
    }

    // We create a preamble block for each of the actual handler blocks: It
    // reads the necessary arguments and passes them to the actual handler
    // block, together with the continuation object.
    let target_preamble_blocks = {
        let mut preamble_blocks = vec![];

        for &(handle_tag, target_block) in &suspend_handlers {
            let preamble_block = builder.create_block();
            preamble_blocks.push(preamble_block);
            builder.switch_to_block(preamble_block);

            let param_types = env.tag_params(TagIndex::from_u32(handle_tag));
            let param_types: Vec<ir::Type> = param_types
                .iter()
                .map(|wty| crate::value_type(env.isa(), *wty))
                .collect();

            let values = suspended_contref.values(env, builder);
            let mut suspend_args: Vec<BlockArg> = values
                .load_data_entries(env, builder, &param_types)
                .into_iter()
                .map(|v| BlockArg::Value(v))
                .collect();

            // At the suspend site, we store the suspend args in the the
            // `values` buffer of the VMContRef that was active at the time that
            // the suspend instruction was performed.
            suspend_args.push(BlockArg::Value(suspended_contobj));

            // We clear the suspend args. This is mostly for consistency. Note
            // that we don't zero out the data buffer, we still need it for the

            values.clear(env, builder, false);

            builder.ins().jump(target_block, &suspend_args);
        }

        preamble_blocks
    };

    // Dispatch block. All it does is jump to the right premable block based on
    // the handler index.
    {
        builder.switch_to_block(dispatch_block);
        builder.seal_block(dispatch_block);

        let default_bc = builder.func.dfg.block_call(jt_default_block, &[]);

        let adapter_bcs: Vec<BlockCall> = target_preamble_blocks
            .iter()
            .map(|b| builder.func.dfg.block_call(*b, &[]))
            .collect();

        let jt_data = JumpTableData::new(default_bc, &adapter_bcs);
        let jt = builder.create_jump_table(jt_data);

        builder.ins().br_table(handler_index, jt);

        for preamble_block in target_preamble_blocks {
            builder.seal_block(preamble_block);
        }
        builder.seal_block(jt_default_block);
    }

    // Return block: Jumped to by resume block if continuation
    // returned normally.
    {
        builder.switch_to_block(return_block);
        builder.seal_block(return_block);

        // If we got a return signal, a continuation must have been running.
        let returned_contref = new_stack_chain.unchecked_get_continuation(env, builder);
        let returned_contref = helpers::VMContRef::new(returned_contref);

        // Restore parts of the VMRuntimeLimits from the parent of the
        // returned continuation (which is now active).
        let parent_csi = original_stack_chain.get_common_stack_information(env, builder);
        parent_csi.write_limits_to_vmcontext(env, builder, vm_runtime_limits_ptr);

        let returned_csi = returned_contref.common_stack_information(env, builder);
        returned_csi.set_state_returned(env, builder);

        // Load the values returned by the continuation.
        let return_types: Vec<_> = env
            .continuation_returns(TypeIndex::from_u32(type_index))
            .iter()
            .map(|ty| crate::value_type(env.isa(), *ty))
            .collect();
        let payloads = returned_contref.args(env, builder);
        let return_values = payloads.load_data_entries(env, builder, &return_types);
        payloads.clear(env, builder, true);

        Ok(return_values)
    }
}

pub(crate) fn translate_suspend<'a>(
    env: &mut crate::func_environ::FuncEnvironment<'a>,
    builder: &mut FunctionBuilder,
    tag_index: u32,
    suspend_args: &[ir::Value],
    tag_return_types: &[ir::Type],
) -> Vec<ir::Value> {
    let tag_addr = tag_address(env, builder, tag_index);

    let vmctx = env.vmctx_val(&mut builder.cursor());
    let active_stack_chain = vmctx_load_stack_chain(env, builder, vmctx);

    let (_, end_of_chain_contref, handler_index) =
        search_handler(env, builder, &active_stack_chain, tag_addr, true);

    // If we get here, the search_handler logic succeeded (i.e., did not trap).
    // Thus, there is at least one parent, so we are not on the initial stack.
    // Can therefore extract continuation directly.
    let active_contref = active_stack_chain.unchecked_get_continuation(env, builder);
    let active_contref = helpers::VMContRef::new(active_contref);
    let mut end_of_chain_contref = helpers::VMContRef::new(end_of_chain_contref);

    active_contref.set_last_ancestor(env, builder, end_of_chain_contref.address);

    // In the active_contref's `values` buffer, stack-allocate enough room so that we can
    // later store the following:
    // 1. The suspend arguments
    // 2. Afterwards, the tag return values
    let values = active_contref.values(env, builder);
    let required_capacity =
        u32::try_from(std::cmp::max(suspend_args.len(), tag_return_types.len()))
            .expect("Number of stack switching payloads should fit in u32");

    if required_capacity > 0 {
        env.stack_switching_values_buffer = Some(values.allocate_or_reuse_stack_slot(
            env,
            builder,
            required_capacity,
            env.stack_switching_values_buffer,
        ));
    }

    if suspend_args.len() > 0 {
        values.store_data_entries(env, builder, suspend_args, true)
    }

    // Set current continuation to suspended and break up handler chain.
    let active_contref_csi = active_contref.common_stack_information(env, builder);
    active_contref_csi.set_state_suspended(env, builder);
    let absent_chain_link = VMStackChain::absent(env, builder);
    end_of_chain_contref.set_parent_stack_chain(env, builder, &absent_chain_link);

    let suspend_payload = ControlEffect::encode_suspend(builder, handler_index).to_u64();

    // Note that the control context we use for switching is the one
    // at the end of the chain, not the one in active_contref!
    // This also means that stack_switch saves the information about
    // the current stack in the control context located in the stack
    // of end_of_chain_contref.
    let fiber_stack = end_of_chain_contref.get_fiber_stack(env, builder);
    let control_context_ptr = fiber_stack.load_control_context(env, builder);

    builder
        .ins()
        .stack_switch(control_context_ptr, control_context_ptr, suspend_payload);

    // The return values of the suspend instruction are the tag return values, saved in the `args` buffer.
    let values = active_contref.values(env, builder);
    let return_values = values.load_data_entries(env, builder, tag_return_types);
    // We effectively consume the values and discard the stack allocated buffer.
    values.clear(env, builder, true);

    return_values
}

pub(crate) fn translate_switch<'a>(
    env: &mut crate::func_environ::FuncEnvironment<'a>,
    builder: &mut FunctionBuilder,
    tag_index: u32,
    switchee_contobj: ir::Value,
    switch_args: &[ir::Value],
    return_types: &[ir::Type],
) -> WasmResult<Vec<ir::Value>> {
    let vmctx = env.vmctx_val(&mut builder.cursor());

    // Check and increment revision on switchee continuation object (i.e., the
    // one being switched to). Logically, the switchee continuation extends from
    // `switchee_contref` to `switchee_contref.last_ancestor` (i.e., the end of
    // the parent chain starting at `switchee_contref`).
    let switchee_contref = {
        let (witness, target_contref) =
            fatpointer::deconstruct(env, &mut builder.cursor(), switchee_contobj);

        // The typing rules for switch allow a null reference to be passed to it.
        builder
            .ins()
            .trapz(target_contref, crate::TRAP_NULL_REFERENCE);

        let mut target_contref = helpers::VMContRef::new(target_contref);

        let revision = target_contref.get_revision(env, builder);
        let evidence = builder.ins().icmp(IntCC::Equal, revision, witness);
        builder
            .ins()
            .trapz(evidence, crate::TRAP_CONTINUATION_ALREADY_CONSUMED);
        let _next_revision = target_contref.incr_revision(env, builder, revision);
        target_contref
    };

    // We create the "switcher continuation" (i.e., the one executing switch)
    // from the current execution context: Logically, it extends from the
    // continuation reference executing `switch` (subsequently called
    // `switcher_contref`) to the immediate child (called
    // `switcher_contref_last_ancestor`) of the stack with the corresponding
    // handler (saved in `handler_stack_chain`).
    let (
        switcher_contref,
        switcher_contobj,
        switcher_contref_last_ancestor,
        handler_stack_chain,
        vm_runtime_limits_ptr,
    ) = {
        let tag_addr = tag_address(env, builder, tag_index);
        let active_stack_chain = vmctx_load_stack_chain(env, builder, vmctx);
        let (handler_stack_chain, last_ancestor, _handler_index) =
            search_handler(env, builder, &active_stack_chain, tag_addr, false);
        let mut last_ancestor = helpers::VMContRef::new(last_ancestor);

        // If we get here, the search_handler logic succeeded (i.e., did not trap).
        // Thus, there is at least one parent, so we are not on the initial stack.
        // Can therefore extract continuation directly.
        let switcher_contref = active_stack_chain.unchecked_get_continuation(env, builder);
        let mut switcher_contref = helpers::VMContRef::new(switcher_contref);

        switcher_contref.set_last_ancestor(env, builder, last_ancestor.address);

        // In the switcher_contref's `values` buffer, stack-allocate enough room so that we can
        // later store `tag_return_types.len()` when resuming the continuation.
        let values = switcher_contref.values(env, builder);
        let required_capacity = u32::try_from(return_types.len()).unwrap();
        if required_capacity > 0 {
            env.stack_switching_values_buffer = Some(values.allocate_or_reuse_stack_slot(
                env,
                builder,
                required_capacity,
                env.stack_switching_values_buffer,
            ));
        }

        let switcher_contref_csi = switcher_contref.common_stack_information(env, builder);
        switcher_contref_csi.set_state_suspended(env, builder);
        // We break off `switcher_contref` from the chain of active
        // continuations, by separating the link between `last_ancestor` and its
        // parent stack.
        let absent = VMStackChain::absent(env, builder);
        last_ancestor.set_parent_stack_chain(env, builder, &absent);

        // Load current runtime limits from `VMContext` and store in the
        // switcher continuation.
        let vm_runtime_limits_ptr = vmctx_load_vm_runtime_limits_ptr(env, builder, vmctx);
        switcher_contref_csi.load_limits_from_vmcontext(env, builder, vm_runtime_limits_ptr, false);

        let revision = switcher_contref.get_revision(env, builder);
        let new_contobj = fatpointer::construct(
            env,
            &mut builder.cursor(),
            revision,
            switcher_contref.address,
        );

        (
            switcher_contref,
            new_contobj,
            last_ancestor,
            handler_stack_chain,
            vm_runtime_limits_ptr,
        )
    };

    // Prepare switchee continuation:
    // - Store "ordinary" switch arguments as well as the contobj just
    //   synthesized from the current context (i.e., `switcher_contobj`) in the
    //   switchee continuation's payload buffer.
    // - Splice switchee's continuation chain with handler stack to form new
    //   overall chain of active continuations.
    let (switchee_contref_csi, switchee_contref_last_ancestor) = {
        let mut combined_payloads = switch_args.to_vec();
        combined_payloads.push(switcher_contobj);
        vmcontref_store_payloads(env, builder, &combined_payloads, switchee_contref.address);

        let switchee_contref_csi = switchee_contref.common_stack_information(env, builder);
        switchee_contref_csi.set_state_running(env, builder);

        let switchee_contref_last_ancestor = switchee_contref.get_last_ancestor(env, builder);
        let mut switchee_contref_last_ancestor =
            helpers::VMContRef::new(switchee_contref_last_ancestor);

        switchee_contref_last_ancestor.set_parent_stack_chain(env, builder, &handler_stack_chain);

        (switchee_contref_csi, switchee_contref_last_ancestor)
    };

    // Update VMContext/Store: Update active continuation and `VMRuntimeLimits`.
    {
        vmctx_set_active_continuation(env, builder, vmctx, switchee_contref.address);

        switchee_contref_csi.write_limits_to_vmcontext(env, builder, vm_runtime_limits_ptr);
    }

    // Perform actual stack switch
    {
        let switcher_last_ancestor_fs =
            switcher_contref_last_ancestor.get_fiber_stack(env, builder);
        let switcher_last_ancestor_cc =
            switcher_last_ancestor_fs.load_control_context(env, builder);

        let switchee_last_ancestor_fs =
            switchee_contref_last_ancestor.get_fiber_stack(env, builder);
        let switchee_last_ancestor_cc =
            switchee_last_ancestor_fs.load_control_context(env, builder);

        // The stack switch involves the following control contexts (e.g., IP,
        // SP, FP, ...):
        // - `switchee_last_ancestor_cc` contains the information to continue
        //    execution in the switchee/target continuation.
        // - `switcher_last_ancestor_cc` contains the information about how to
        //    continue execution once we suspend/return to the stack with the
        //    switch handler.
        //
        // In total, the following needs to happen:
        // 1. Load control context at `switchee_last_ancestor_cc` to perform
        //    stack switch.
        // 2. Move control context at `switcher_last_ancestor_cc` over to
        //    `switchee_last_ancestor_cc`.
        // 3. Upon actual switch, save current control context at
        //    `switcher_last_ancestor_cc`.
        //
        // We implement this as follows:
        // 1. We copy `switchee_last_ancestor_cc` to a temporary area on the
        //    stack (`tmp_control_context`).
        // 2. We copy `switcher_last_ancestor_cc` over to
        //    `switchee_last_ancestor_cc`.
        // 3. We invoke the stack switch instruction such that it reads from the
        //    temporary area, and writes to `switcher_last_ancestor_cc`.
        //
        // Note that the temporary area is only accessed once by the
        // `stack_switch` instruction emitted later in this block, meaning that we
        // don't have to worry about its lifetime.
        //
        // NOTE(frank-emrich) The implementation below results in one stack slot
        // being created per switch instruction, even though multiple switch
        // instructions in the same function could safely re-use the same stack
        // slot. Thus, we could implement logic for sharing the stack slot by
        // adding an appropriate field to `FuncEnvironment`.
        //
        // NOTE(frank-emrich) We could avoid the copying to a temporary area by
        // making `stack_switch` do all of the necessary moving itself. However,
        // that would be a rather ad-hoc change to how the instruction uses the
        // two pointers given to it.

        let slot_size = ir::StackSlotData::new(
            ir::StackSlotKind::ExplicitSlot,
            u32::try_from(CONTROL_CONTEXT_SIZE).unwrap(),
            u8::try_from(env.pointer_type().bytes()).unwrap(),
        );
        let slot = builder.create_sized_stack_slot(slot_size);
        let tmp_control_context = builder.ins().stack_addr(env.pointer_type(), slot, 0);

        let flags = MemFlags::trusted();
        let mut offset: i32 = 0;
        while offset < i32::try_from(CONTROL_CONTEXT_SIZE).unwrap() {
            // switchee_last_ancestor_cc -> tmp control context
            let tmp1 =
                builder
                    .ins()
                    .load(env.pointer_type(), flags, switchee_last_ancestor_cc, offset);
            builder
                .ins()
                .store(flags, tmp1, tmp_control_context, offset);

            // switcher_last_ancestor_cc -> switchee_last_ancestor_cc
            let tmp2 =
                builder
                    .ins()
                    .load(env.pointer_type(), flags, switcher_last_ancestor_cc, offset);
            builder
                .ins()
                .store(flags, tmp2, switchee_last_ancestor_cc, offset);

            offset += i32::try_from(env.pointer_type().bytes()).unwrap();
        }

        let switch_payload = ControlEffect::encode_switch(builder).to_u64();

        let _result = builder.ins().stack_switch(
            switcher_last_ancestor_cc,
            tmp_control_context,
            switch_payload,
        );
    }

    // After switching back to the original stack: Load return values, they are
    // stored on the switcher continuation.
    let return_values = {
        let payloads = switcher_contref.values(env, builder);
        let return_values = payloads.load_data_entries(env, builder, return_types);
        // We consume the values and discard the buffer (allocated on this stack)
        payloads.clear(env, builder, true);
        return_values
    };

    Ok(return_values)
}
