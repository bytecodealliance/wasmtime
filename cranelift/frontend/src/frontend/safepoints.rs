//! Safepoints and stack maps.

use core::ops::{Index, IndexMut};

use super::*;

#[derive(Clone, Copy)]
#[repr(u8)]
enum SlotSize {
    Size8 = 0,
    Size16 = 1,
    Size32 = 2,
    Size64 = 3,
    Size128 = 4,
    // If adding support for more slot sizes, update `SLOT_SIZE_LEN` below.
}
const SLOT_SIZE_LEN: usize = 5;

impl TryFrom<ir::Type> for SlotSize {
    type Error = &'static str;

    fn try_from(ty: ir::Type) -> Result<Self, Self::Error> {
        Self::new(ty.bytes()).ok_or("type is not supported in stack maps")
    }
}

impl SlotSize {
    fn new(bytes: u32) -> Option<Self> {
        match bytes {
            1 => Some(Self::Size8),
            2 => Some(Self::Size16),
            4 => Some(Self::Size32),
            8 => Some(Self::Size64),
            16 => Some(Self::Size128),
            _ => None,
        }
    }

    fn unwrap_new(bytes: u32) -> Self {
        Self::new(bytes).unwrap_or_else(|| panic!("cannot create a `SlotSize` for {bytes} bytes"))
    }
}

/// A map from every `SlotSize` to a `T`.
struct SlotSizeMap<T>([T; SLOT_SIZE_LEN]);

impl<T> Index<SlotSize> for SlotSizeMap<T> {
    type Output = T;
    fn index(&self, index: SlotSize) -> &Self::Output {
        self.get(index)
    }
}

impl<T> IndexMut<SlotSize> for SlotSizeMap<T> {
    fn index_mut(&mut self, index: SlotSize) -> &mut Self::Output {
        self.get_mut(index)
    }
}

impl<T> SlotSizeMap<T> {
    fn new() -> Self
    where
        T: Default,
    {
        Self([
            T::default(),
            T::default(),
            T::default(),
            T::default(),
            T::default(),
        ])
    }

    fn get(&self, size: SlotSize) -> &T {
        let index = size as u8 as usize;
        &self.0[index]
    }

    fn get_mut(&mut self, size: SlotSize) -> &mut T {
        let index = size as u8 as usize;
        &mut self.0[index]
    }
}

impl FunctionBuilder<'_> {
    /// Insert spills for every value that needs to be in a stack map at every
    /// safepoint.
    ///
    /// We begin with a very simple, imprecise, and overapproximating liveness
    /// analysis. This considers any use (regardless if that use produces side
    /// effects or flows into another instruction that produces side effects!)
    /// of a needs-stack-map value to keep the value live. This allows us to do
    /// this liveness analysis in a single post-order traversal of the IR,
    /// without any fixed-point loop. The result of this analysis is the mapping
    /// from each needs-stack-map value that is live across a safepoint to its
    /// associated stack slot.
    ///
    /// Finally, we spill each of the needs-stack-map values that are live
    /// across a safepoint to their associated stack slot upon definition, and
    /// insert reloads from that stack slot at each use of the value.
    pub(super) fn insert_safepoint_spills(&mut self) {
        log::trace!(
            "before inserting safepoint spills and reloads:\n{}",
            self.func.display()
        );

        let stack_slots = self.find_live_stack_map_values_at_each_safepoint();
        self.insert_safepoint_spills_and_reloads(&stack_slots);

        log::trace!(
            "after inserting safepoint spills and reloads:\n{}",
            self.func.display()
        );
    }

    /// Find the live GC references for each safepoint instruction in this
    /// function.
    ///
    /// Returns a map from each safepoint instruction to the set of GC
    /// references that are live across it
    fn find_live_stack_map_values_at_each_safepoint(
        &mut self,
    ) -> crate::HashMap<ir::Value, ir::StackSlot> {
        // A mapping from each needs-stack-map value that is live across some
        // safepoint to the stack slot that it resides within. Note that if a
        // needs-stack-map value is never live across a safepoint, then we won't
        // ever add it to this map, it can remain in a virtual register for the
        // duration of its lifetime, and we won't replace all its uses with
        // reloads and all that stuff.
        let mut stack_slots: crate::HashMap<ir::Value, ir::StackSlot> = Default::default();

        // A map from slot size to free stack slots that are not being used
        // anymore. This allows us to reuse stack slots across multiple values
        // helps cut down on the ultimate size of our stack frames.
        let mut free_stack_slots = SlotSizeMap::<SmallVec<[ir::StackSlot; 4]>>::new();

        // The set of needs-stack-maps values that are currently live in our
        // traversal.
        //
        // NB: use a `BTreeSet` so that iteration is deterministic, as we will
        // insert spills an order derived from this collection's iteration
        // order.
        let mut live = BTreeSet::new();

        // Do our single-pass liveness analysis.
        //
        // Use a post-order traversal, traversing the IR backwards from uses to
        // defs, because liveness is a backwards analysis.
        //
        // 1. The definition of a value removes it from our `live` set. Values
        //    are not live before they are defined.
        //
        // 2. When we see any instruction that is a safepoint (aka non-tail
        //    calls) we record the current live set of needs-stack-map values.
        //
        //    We ignore tail calls because this caller and its frame won't exist
        //    by the time the callee is executing and potentially triggers a GC;
        //    nothing is live in the function after it exits!
        //
        //    Note that this step should actually happen *before* adding uses to
        //    the `live` set below, in order to avoid holding GC objects alive
        //    longer than necessary, because arguments to the call that are not
        //    live afterwards should need not be prevented from reclamation by
        //    the GC for us, and therefore need not appear in this stack map. It
        //    is the callee's responsibility to record such arguments in its
        //    stack maps if it keeps them alive across some call that might
        //    trigger GC.
        //
        // 3. Any use of a needs-stack-map value adds it to our `live` set.
        //
        //    Note: we do not flow liveness from block parameters back to branch
        //    arguments, and instead always consider branch arguments live. That
        //    additional precision would require a fixed-point loop in the
        //    presence of back edges.
        //
        //    Furthermore, we do not differentiate between uses of a
        //    needs-stack-map value that ultimately flow into a side-effecting
        //    operation versus uses that themselves are not live. This could be
        //    tightened up in the future, but we're starting with the easiest,
        //    simplest thing. Besides, none of our mid-end optimization passes
        //    have run at this point in time yet, so there probably isn't much,
        //    if any, dead code.

        // Helper for (1)
        let process_def = |func: &Function,
                           stack_slots: &crate::HashMap<_, _>,
                           free_stack_slots: &mut SlotSizeMap<SmallVec<_>>,
                           live: &mut BTreeSet<ir::Value>,
                           val: ir::Value| {
            log::trace!("liveness:   defining {val:?}, removing it from the live set");
            live.remove(&val);

            // This value's stack slot, if any, is now available for reuse.
            if let Some(slot) = stack_slots.get(&val) {
                log::trace!("liveness:     returning {slot:?} to the free list");
                let ty = func.dfg.value_type(val);
                free_stack_slots[SlotSize::try_from(ty).unwrap()].push(*slot);
            }
        };

        // Helper for (2)
        let process_safepoint = |func: &mut Function,
                                 stack_slots: &mut crate::HashMap<Value, StackSlot>,
                                 free_stack_slots: &mut SlotSizeMap<SmallVec<_>>,
                                 live: &BTreeSet<_>,
                                 inst: Inst| {
            log::trace!(
                "liveness:   found safepoint: {inst:?}: {}",
                func.dfg.display_inst(inst)
            );
            log::trace!("liveness:     live set = {live:?}");

            for val in live {
                let ty = func.dfg.value_type(*val);
                let slot = *stack_slots.entry(*val).or_insert_with(|| {
                    log::trace!("liveness:     {val:?} needs a stack slot");
                    let size = func.dfg.value_type(*val).bytes();
                    match free_stack_slots[SlotSize::unwrap_new(size)].pop() {
                        Some(slot) => {
                            log::trace!(
                                "liveness:       reusing free stack slot {slot:?} for {val:?}"
                            );
                            slot
                        }
                        None => {
                            debug_assert!(size.is_power_of_two());
                            let log2_size = size.ilog2();
                            let slot = func.create_sized_stack_slot(ir::StackSlotData::new(
                                ir::StackSlotKind::ExplicitSlot,
                                size,
                                log2_size.try_into().unwrap(),
                            ));
                            log::trace!(
                                "liveness:       created new stack slot {slot:?} for {val:?}"
                            );
                            slot
                        }
                    }
                });
                func.dfg.append_user_stack_map_entry(
                    inst,
                    ir::UserStackMapEntry {
                        ty,
                        slot,
                        offset: 0,
                    },
                );
            }
        };

        // Helper for (3)
        let process_use = |func: &Function, live: &mut BTreeSet<_>, inst: Inst, val: Value| {
            if live.insert(val) {
                log::trace!(
                    "liveness:   found use of {val:?}, marking it live: {inst:?}: {}",
                    func.dfg.display_inst(inst)
                );
            }
        };

        for block in self
            .func_ctx
            .dfs
            .post_order_iter(&self.func)
            // We have to `collect` here to release the borrow on `self.func` so
            // we can add the stack map entries below.
            .collect::<Vec<_>>()
        {
            log::trace!("liveness: traversing {block:?}");
            let mut option_inst = self.func.layout.last_inst(block);
            while let Some(inst) = option_inst {
                // (1) Remove values defined by this instruction from the `live`
                // set.
                for val in self.func.dfg.inst_results(inst) {
                    process_def(
                        &self.func,
                        &stack_slots,
                        &mut free_stack_slots,
                        &mut live,
                        *val,
                    );
                }

                // (2) If this instruction is a safepoint, then we need to add
                // stack map entries to record the values in `live`.
                let opcode = self.func.dfg.insts[inst].opcode();
                if opcode.is_call() && !opcode.is_return() {
                    process_safepoint(
                        &mut self.func,
                        &mut stack_slots,
                        &mut free_stack_slots,
                        &live,
                        inst,
                    );
                }

                // (3) Add all needs-stack-map values that are operands to this
                // instruction to the live set. This includes branch arguments,
                // as mentioned above.
                for val in self.func.dfg.inst_values(inst) {
                    let val = self.func.dfg.resolve_aliases(val);
                    if self.func_ctx.stack_map_values.contains(val) {
                        process_use(&self.func, &mut live, inst, val);
                    }
                }

                option_inst = self.func.layout.prev_inst(inst);
            }

            // After we've processed this block's instructions, remove its
            // parameters from the live set. This is part of step (1).
            for val in self.func.dfg.block_params(block) {
                process_def(
                    &self.func,
                    &stack_slots,
                    &mut free_stack_slots,
                    &mut live,
                    *val,
                );
            }
        }

        stack_slots
    }

    /// This function does a forwards pass over the IR and does two things:
    ///
    /// 1. Insert spills to a needs-stack-map value's associated stack slot just
    ///    after its definition.
    ///
    /// 2. Replace all uses of the needs-stack-map value with loads from that
    ///    stack slot. This will introduce many redundant loads, but the alias
    ///    analysis pass in the mid-end should clean most of these up when not
    ///    actually needed.
    fn insert_safepoint_spills_and_reloads(
        &mut self,
        stack_slots: &crate::HashMap<ir::Value, ir::StackSlot>,
    ) {
        let mut pos = FuncCursor::new(self.func);
        let mut vals: SmallVec<[_; 8]> = Default::default();

        while let Some(block) = pos.next_block() {
            // Spill needs-stack-map values defined by block parameters to their
            // associated stack slot.
            vals.extend_from_slice(pos.func.dfg.block_params(block));
            pos.next_inst();
            let mut spilled_any = false;
            for val in vals.drain(..) {
                if let Some(slot) = stack_slots.get(&val) {
                    pos.ins().stack_store(val, *slot, 0);
                    spilled_any = true;
                }
            }

            // The cursor needs to point just *before* the first original
            // instruction in the block that we didn't introduce just above, so
            // that when we loop over `next_inst()` below we are processing the
            // block's original instructions. If we inserted spills, then it
            // already does point there. If we did not insert spills, then it is
            // currently pointing at the first original instruction, but that
            // means that the upcoming `pos.next_inst()` call will skip over the
            // first original instruction, so in this case we need to back up
            // the cursor.
            if !spilled_any {
                pos = pos.at_position(CursorPosition::Before(block));
            }

            while let Some(mut inst) = pos.next_inst() {
                // Replace all uses of needs-stack-map values with loads from
                // the value's associated stack slot.
                vals.extend(pos.func.dfg.inst_values(inst));
                let mut replaced_any = false;
                for val in &mut vals {
                    if let Some(slot) = stack_slots.get(val) {
                        replaced_any = true;
                        let ty = pos.func.dfg.value_type(*val);
                        *val = pos.ins().stack_load(ty, *slot, 0);
                    }
                }
                if replaced_any {
                    pos.func.dfg.overwrite_inst_values(inst, vals.drain(..));
                } else {
                    vals.clear();
                }

                // If this instruction defines a needs-stack-map value, then
                // spill it to its stack slot.
                pos = pos.after_inst(inst);
                vals.extend_from_slice(pos.func.dfg.inst_results(inst));
                for val in vals.drain(..) {
                    if let Some(slot) = stack_slots.get(&val) {
                        inst = pos.ins().stack_store(val, *slot, 0);
                    }
                }

                pos = pos.at_inst(inst);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;
    use cranelift_codegen::isa::CallConv;

    #[test]
    fn needs_stack_map_and_loop() {
        let mut sig = Signature::new(CallConv::SystemV);
        sig.params.push(AbiParam::new(ir::types::I32));
        sig.params.push(AbiParam::new(ir::types::I32));

        let mut fn_ctx = FunctionBuilderContext::new();
        let mut func = Function::with_name_signature(ir::UserFuncName::testcase("sample"), sig);
        let mut builder = FunctionBuilder::new(&mut func, &mut fn_ctx);

        let name = builder
            .func
            .declare_imported_user_function(ir::UserExternalName {
                namespace: 0,
                index: 0,
            });
        let mut sig = Signature::new(CallConv::SystemV);
        sig.params.push(AbiParam::new(ir::types::I32));
        let signature = builder.func.import_signature(sig);
        let func_ref = builder.import_function(ir::ExtFuncData {
            name: ir::ExternalName::user(name),
            signature,
            colocated: true,
        });

        // Here the value `v1` is technically not live but our single-pass liveness
        // analysis treats every branch argument to a block as live to avoid
        // needing to do a fixed-point loop.
        //
        //     block0(v0, v1):
        //       call $foo(v0)
        //       jump block0(v0, v1)
        let block0 = builder.create_block();
        builder.append_block_params_for_function_params(block0);
        let a = builder.func.dfg.block_params(block0)[0];
        let b = builder.func.dfg.block_params(block0)[1];
        builder.declare_value_needs_stack_map(a);
        builder.declare_value_needs_stack_map(b);
        builder.switch_to_block(block0);
        builder.ins().call(func_ref, &[a]);
        builder.ins().jump(block0, &[a, b]);
        builder.seal_all_blocks();
        builder.finalize();

        eprintln!("Actual = {}", func.display());
        assert_eq!(
            func.display().to_string().trim(),
            r#"
function %sample(i32, i32) system_v {
    ss0 = explicit_slot 4, align = 4
    ss1 = explicit_slot 4, align = 4
    sig0 = (i32) system_v
    fn0 = colocated u0:0 sig0

block0(v0: i32, v1: i32):
    stack_store v0, ss0
    stack_store v1, ss1
    call fn0(v0), stack_map=[i32 @ ss0+0, i32 @ ss1+0]
    v2 = stack_load.i32 ss0
    v3 = stack_load.i32 ss1
    jump block0(v2, v3)
}            "#
                .trim()
        );
    }

    #[test]
    fn needs_stack_map_simple() {
        let sig = Signature::new(CallConv::SystemV);

        let mut fn_ctx = FunctionBuilderContext::new();
        let mut func = Function::with_name_signature(ir::UserFuncName::testcase("sample"), sig);
        let mut builder = FunctionBuilder::new(&mut func, &mut fn_ctx);

        let name = builder
            .func
            .declare_imported_user_function(ir::UserExternalName {
                namespace: 0,
                index: 0,
            });
        let mut sig = Signature::new(CallConv::SystemV);
        sig.params.push(AbiParam::new(ir::types::I32));
        let signature = builder.func.import_signature(sig);
        let func_ref = builder.import_function(ir::ExtFuncData {
            name: ir::ExternalName::user(name),
            signature,
            colocated: true,
        });

        // At each `call` we are losing one more value as no longer live, so
        // each stack map should be one smaller than the last. `v3` is never
        // live across a safepoint, so should never appear in a stack map. Note
        // that a value that is an argument to the call, but is not live after
        // the call, should not appear in the stack map. This is why `v0`
        // appears in the first call's stack map, but not the second call's
        // stack map.
        //
        //     block0:
        //       v0 = needs stack map
        //       v1 = needs stack map
        //       v2 = needs stack map
        //       v3 = needs stack map
        //       call $foo(v3)
        //       call $foo(v0)
        //       call $foo(v1)
        //       call $foo(v2)
        //       return
        let block0 = builder.create_block();
        builder.append_block_params_for_function_params(block0);
        builder.switch_to_block(block0);
        let v0 = builder.ins().iconst(ir::types::I32, 0);
        builder.declare_value_needs_stack_map(v0);
        let v1 = builder.ins().iconst(ir::types::I32, 1);
        builder.declare_value_needs_stack_map(v1);
        let v2 = builder.ins().iconst(ir::types::I32, 2);
        builder.declare_value_needs_stack_map(v2);
        let v3 = builder.ins().iconst(ir::types::I32, 3);
        builder.declare_value_needs_stack_map(v3);
        builder.ins().call(func_ref, &[v3]);
        builder.ins().call(func_ref, &[v0]);
        builder.ins().call(func_ref, &[v1]);
        builder.ins().call(func_ref, &[v2]);
        builder.ins().return_(&[]);
        builder.seal_all_blocks();
        builder.finalize();

        eprintln!("Actual = {}", func.display());
        assert_eq!(
            func.display().to_string().trim(),
            r#"
function %sample() system_v {
    ss0 = explicit_slot 4, align = 4
    ss1 = explicit_slot 4, align = 4
    ss2 = explicit_slot 4, align = 4
    sig0 = (i32) system_v
    fn0 = colocated u0:0 sig0

block0:
    v0 = iconst.i32 0
    stack_store v0, ss2  ; v0 = 0
    v1 = iconst.i32 1
    stack_store v1, ss1  ; v1 = 1
    v2 = iconst.i32 2
    stack_store v2, ss0  ; v2 = 2
    v3 = iconst.i32 3
    call fn0(v3), stack_map=[i32 @ ss2+0, i32 @ ss1+0, i32 @ ss0+0]  ; v3 = 3
    v4 = stack_load.i32 ss2
    call fn0(v4), stack_map=[i32 @ ss1+0, i32 @ ss0+0]
    v5 = stack_load.i32 ss1
    call fn0(v5), stack_map=[i32 @ ss0+0]
    v6 = stack_load.i32 ss0
    call fn0(v6)
    return
}
            "#
            .trim()
        );
    }

    #[test]
    fn needs_stack_map_and_post_order_early_return() {
        let mut sig = Signature::new(CallConv::SystemV);
        sig.params.push(AbiParam::new(ir::types::I32));

        let mut fn_ctx = FunctionBuilderContext::new();
        let mut func = Function::with_name_signature(ir::UserFuncName::testcase("sample"), sig);
        let mut builder = FunctionBuilder::new(&mut func, &mut fn_ctx);

        let name = builder
            .func
            .declare_imported_user_function(ir::UserExternalName {
                namespace: 0,
                index: 0,
            });
        let signature = builder
            .func
            .import_signature(Signature::new(CallConv::SystemV));
        let func_ref = builder.import_function(ir::ExtFuncData {
            name: ir::ExternalName::user(name),
            signature,
            colocated: true,
        });

        // Here we rely on the post-order to make sure that we never visit block
        // 4 and add `v1` to our live set, then visit block 2 and add `v1` to
        // its stack map even though `v1` is not in scope. Thanksfully, that
        // sequence is impossible because it would be an invalid post-order
        // traversal. The only valid post-order traversals are [3, 1, 2, 0] and
        // [2, 3, 1, 0].
        //
        //     block0(v0):
        //       brif v0, block1, block2
        //
        //     block1:
        //       <stuff>
        //       v1 = get some gc ref
        //       jump block3
        //
        //     block2:
        //       call $needs_safepoint_accidentally
        //       return
        //
        //     block3:
        //       stuff keeping v1 live
        //       return
        let block0 = builder.create_block();
        let block1 = builder.create_block();
        let block2 = builder.create_block();
        let block3 = builder.create_block();
        builder.append_block_params_for_function_params(block0);

        builder.switch_to_block(block0);
        let v0 = builder.func.dfg.block_params(block0)[0];
        builder.ins().brif(v0, block1, &[], block2, &[]);

        builder.switch_to_block(block1);
        let v1 = builder.ins().iconst(ir::types::I64, 0x12345678);
        builder.declare_value_needs_stack_map(v1);
        builder.ins().jump(block3, &[]);

        builder.switch_to_block(block2);
        builder.ins().call(func_ref, &[]);
        builder.ins().return_(&[]);

        builder.switch_to_block(block3);
        // NB: Our simplistic liveness analysis conservatively treats any use of
        // a value as keeping it live, regardless if the use has side effects or
        // is otherwise itself live, so an `iadd_imm` suffices to keep `v1` live
        // here.
        builder.ins().iadd_imm(v1, 0);
        builder.ins().return_(&[]);

        builder.seal_all_blocks();
        builder.finalize();

        eprintln!("Actual = {}", func.display());
        assert_eq!(
            func.display().to_string().trim(),
            r#"
function %sample(i32) system_v {
    sig0 = () system_v
    fn0 = colocated u0:0 sig0

block0(v0: i32):
    brif v0, block1, block2

block1:
    v1 = iconst.i64 0x1234_5678
    jump block3

block2:
    call fn0()
    return

block3:
    v2 = iadd_imm.i64 v1, 0  ; v1 = 0x1234_5678
    return
}
            "#
            .trim()
        );
    }

    #[test]
    fn needs_stack_map_conditional_branches_and_liveness() {
        let mut sig = Signature::new(CallConv::SystemV);
        sig.params.push(AbiParam::new(ir::types::I32));

        let mut fn_ctx = FunctionBuilderContext::new();
        let mut func = Function::with_name_signature(ir::UserFuncName::testcase("sample"), sig);
        let mut builder = FunctionBuilder::new(&mut func, &mut fn_ctx);

        let name = builder
            .func
            .declare_imported_user_function(ir::UserExternalName {
                namespace: 0,
                index: 0,
            });
        let signature = builder
            .func
            .import_signature(Signature::new(CallConv::SystemV));
        let func_ref = builder.import_function(ir::ExtFuncData {
            name: ir::ExternalName::user(name),
            signature,
            colocated: true,
        });

        // Depending on which post-order traversal we take, we might consider
        // `v1` live inside `block1` and emit unnecessary safepoint
        // spills. That's not great, but ultimately fine, we are trading away
        // precision for a single-pass analysis.
        //
        //     block0(v0):
        //       v1 = needs stack map
        //       brif v0, block1, block2
        //
        //     block1:
        //       call $foo()
        //       return
        //
        //     block2:
        //       keep v1 alive
        //       return
        let block0 = builder.create_block();
        let block1 = builder.create_block();
        let block2 = builder.create_block();
        builder.append_block_params_for_function_params(block0);

        builder.switch_to_block(block0);
        let v0 = builder.func.dfg.block_params(block0)[0];
        let v1 = builder.ins().iconst(ir::types::I64, 0x12345678);
        builder.declare_value_needs_stack_map(v1);
        builder.ins().brif(v0, block1, &[], block2, &[]);

        builder.switch_to_block(block1);
        builder.ins().call(func_ref, &[]);
        builder.ins().return_(&[]);

        builder.switch_to_block(block2);
        // NB: Our simplistic liveness analysis conservatively treats any use of
        // a value as keeping it live, regardless if the use has side effects or
        // is otherwise itself live, so an `iadd_imm` suffices to keep `v1` live
        // here.
        builder.ins().iadd_imm(v1, 0);
        builder.ins().return_(&[]);

        builder.seal_all_blocks();
        builder.finalize();

        eprintln!("Actual = {}", func.display());
        assert_eq!(
            func.display().to_string().trim(),
            r#"
function %sample(i32) system_v {
    sig0 = () system_v
    fn0 = colocated u0:0 sig0

block0(v0: i32):
    v1 = iconst.i64 0x1234_5678
    brif v0, block1, block2

block1:
    call fn0()
    return

block2:
    v2 = iadd_imm.i64 v1, 0  ; v1 = 0x1234_5678
    return
}
            "#
            .trim()
        );

        // Now Do the same test but with block 1 and 2 swapped so that we
        // exercise what we are trying to exercise, regardless of which
        // post-order traversal we happen to take.
        func.clear();
        fn_ctx.clear();

        let mut sig = Signature::new(CallConv::SystemV);
        sig.params.push(AbiParam::new(ir::types::I32));

        func.signature = sig;
        let mut builder = FunctionBuilder::new(&mut func, &mut fn_ctx);

        let name = builder
            .func
            .declare_imported_user_function(ir::UserExternalName {
                namespace: 0,
                index: 0,
            });
        let signature = builder
            .func
            .import_signature(Signature::new(CallConv::SystemV));
        let func_ref = builder.import_function(ir::ExtFuncData {
            name: ir::ExternalName::user(name),
            signature,
            colocated: true,
        });

        let block0 = builder.create_block();
        let block1 = builder.create_block();
        let block2 = builder.create_block();
        builder.append_block_params_for_function_params(block0);

        builder.switch_to_block(block0);
        let v0 = builder.func.dfg.block_params(block0)[0];
        let v1 = builder.ins().iconst(ir::types::I64, 0x12345678);
        builder.declare_value_needs_stack_map(v1);
        builder.ins().brif(v0, block1, &[], block2, &[]);

        builder.switch_to_block(block1);
        builder.ins().iadd_imm(v1, 0);
        builder.ins().return_(&[]);

        builder.switch_to_block(block2);
        builder.ins().call(func_ref, &[]);
        builder.ins().return_(&[]);

        builder.seal_all_blocks();
        builder.finalize();

        eprintln!("Actual = {}", func.display());
        assert_eq!(
            func.display().to_string().trim(),
            r#"
function u0:0(i32) system_v {
    ss0 = explicit_slot 8, align = 8
    sig0 = () system_v
    fn0 = colocated u0:0 sig0

block0(v0: i32):
    v1 = iconst.i64 0x1234_5678
    stack_store v1, ss0  ; v1 = 0x1234_5678
    brif v0, block1, block2

block1:
    v3 = stack_load.i64 ss0
    v2 = iadd_imm v3, 0
    return

block2:
    call fn0(), stack_map=[i64 @ ss0+0]
    return
}
            "#
            .trim()
        );
    }

    #[test]
    fn needs_stack_map_and_tail_calls() {
        let mut sig = Signature::new(CallConv::SystemV);
        sig.params.push(AbiParam::new(ir::types::I32));

        let mut fn_ctx = FunctionBuilderContext::new();
        let mut func = Function::with_name_signature(ir::UserFuncName::testcase("sample"), sig);
        let mut builder = FunctionBuilder::new(&mut func, &mut fn_ctx);

        let name = builder
            .func
            .declare_imported_user_function(ir::UserExternalName {
                namespace: 0,
                index: 0,
            });
        let signature = builder
            .func
            .import_signature(Signature::new(CallConv::SystemV));
        let func_ref = builder.import_function(ir::ExtFuncData {
            name: ir::ExternalName::user(name),
            signature,
            colocated: true,
        });

        // Depending on which post-order traversal we take, we might consider
        // `v1` live inside `block1`. But nothing is live after a tail call so
        // we shouldn't spill `v1` either way here.
        //
        //     block0(v0):
        //       v1 = needs stack map
        //       brif v0, block1, block2
        //
        //     block1:
        //       return_call $foo()
        //
        //     block2:
        //       keep v1 alive
        //       return
        let block0 = builder.create_block();
        let block1 = builder.create_block();
        let block2 = builder.create_block();
        builder.append_block_params_for_function_params(block0);

        builder.switch_to_block(block0);
        let v0 = builder.func.dfg.block_params(block0)[0];
        let v1 = builder.ins().iconst(ir::types::I64, 0x12345678);
        builder.declare_value_needs_stack_map(v1);
        builder.ins().brif(v0, block1, &[], block2, &[]);

        builder.switch_to_block(block1);
        builder.ins().return_call(func_ref, &[]);

        builder.switch_to_block(block2);
        // NB: Our simplistic liveness analysis conservatively treats any use of
        // a value as keeping it live, regardless if the use has side effects or
        // is otherwise itself live, so an `iadd_imm` suffices to keep `v1` live
        // here.
        builder.ins().iadd_imm(v1, 0);
        builder.ins().return_(&[]);

        builder.seal_all_blocks();
        builder.finalize();

        eprintln!("Actual = {}", func.display());
        assert_eq!(
            func.display().to_string().trim(),
            r#"
function %sample(i32) system_v {
    sig0 = () system_v
    fn0 = colocated u0:0 sig0

block0(v0: i32):
    v1 = iconst.i64 0x1234_5678
    brif v0, block1, block2

block1:
    return_call fn0()

block2:
    v2 = iadd_imm.i64 v1, 0  ; v1 = 0x1234_5678
    return
}
            "#
            .trim()
        );

        // Do the same test but with block 1 and 2 swapped so that we exercise
        // what we are trying to exercise, regardless of which post-order
        // traversal we happen to take.
        func.clear();
        fn_ctx.clear();

        let mut sig = Signature::new(CallConv::SystemV);
        sig.params.push(AbiParam::new(ir::types::I32));
        func.signature = sig;

        let mut builder = FunctionBuilder::new(&mut func, &mut fn_ctx);

        let name = builder
            .func
            .declare_imported_user_function(ir::UserExternalName {
                namespace: 0,
                index: 0,
            });
        let signature = builder
            .func
            .import_signature(Signature::new(CallConv::SystemV));
        let func_ref = builder.import_function(ir::ExtFuncData {
            name: ir::ExternalName::user(name),
            signature,
            colocated: true,
        });

        let block0 = builder.create_block();
        let block1 = builder.create_block();
        let block2 = builder.create_block();
        builder.append_block_params_for_function_params(block0);

        builder.switch_to_block(block0);
        let v0 = builder.func.dfg.block_params(block0)[0];
        let v1 = builder.ins().iconst(ir::types::I64, 0x12345678);
        builder.declare_value_needs_stack_map(v1);
        builder.ins().brif(v0, block1, &[], block2, &[]);

        builder.switch_to_block(block1);
        builder.ins().iadd_imm(v1, 0);
        builder.ins().return_(&[]);

        builder.switch_to_block(block2);
        builder.ins().return_call(func_ref, &[]);

        builder.seal_all_blocks();
        builder.finalize();

        eprintln!("Actual = {}", func.display());
        assert_eq!(
            func.display().to_string().trim(),
            r#"
function u0:0(i32) system_v {
    sig0 = () system_v
    fn0 = colocated u0:0 sig0

block0(v0: i32):
    v1 = iconst.i64 0x1234_5678
    brif v0, block1, block2

block1:
    v2 = iadd_imm.i64 v1, 0  ; v1 = 0x1234_5678
    return

block2:
    return_call fn0()
}
            "#
            .trim()
        );
    }

    #[test]
    fn needs_stack_map_and_cfg_diamond() {
        let mut sig = Signature::new(CallConv::SystemV);
        sig.params.push(AbiParam::new(ir::types::I32));

        let mut fn_ctx = FunctionBuilderContext::new();
        let mut func = Function::with_name_signature(ir::UserFuncName::testcase("sample"), sig);
        let mut builder = FunctionBuilder::new(&mut func, &mut fn_ctx);

        let name = builder
            .func
            .declare_imported_user_function(ir::UserExternalName {
                namespace: 0,
                index: 0,
            });
        let signature = builder
            .func
            .import_signature(Signature::new(CallConv::SystemV));
        let func_ref = builder.import_function(ir::ExtFuncData {
            name: ir::ExternalName::user(name),
            signature,
            colocated: true,
        });

        // Create an if/else CFG diamond that and check that various things get
        // spilled as needed.
        //
        //     block0(v0):
        //       brif v0, block1, block2
        //
        //     block1:
        //       v1 = needs stack map
        //       v2 = needs stack map
        //       call $foo()
        //       jump block3(v1, v2)
        //
        //     block2:
        //       v3 = needs stack map
        //       v4 = needs stack map
        //       call $foo()
        //       jump block3(v3, v3)  ;; Note: v4 is not live
        //
        //     block3(v5, v6):
        //       call $foo()
        //       keep v5 alive, but not v6
        let block0 = builder.create_block();
        let block1 = builder.create_block();
        let block2 = builder.create_block();
        let block3 = builder.create_block();
        builder.append_block_params_for_function_params(block0);

        builder.switch_to_block(block0);
        let v0 = builder.func.dfg.block_params(block0)[0];
        builder.ins().brif(v0, block1, &[], block2, &[]);

        builder.switch_to_block(block1);
        let v1 = builder.ins().iconst(ir::types::I64, 1);
        builder.declare_value_needs_stack_map(v1);
        let v2 = builder.ins().iconst(ir::types::I64, 2);
        builder.declare_value_needs_stack_map(v2);
        builder.ins().call(func_ref, &[]);
        builder.ins().jump(block3, &[v1, v2]);

        builder.switch_to_block(block2);
        let v3 = builder.ins().iconst(ir::types::I64, 3);
        builder.declare_value_needs_stack_map(v3);
        let v4 = builder.ins().iconst(ir::types::I64, 4);
        builder.declare_value_needs_stack_map(v4);
        builder.ins().call(func_ref, &[]);
        builder.ins().jump(block3, &[v3, v3]);

        builder.switch_to_block(block3);
        builder.append_block_param(block3, ir::types::I64);
        builder.append_block_param(block3, ir::types::I64);
        builder.ins().call(func_ref, &[]);
        // NB: Our simplistic liveness analysis conservatively treats any use of
        // a value as keeping it live, regardless if the use has side effects or
        // is otherwise itself live, so an `iadd_imm` suffices to keep `v1` live
        // here.
        builder.ins().iadd_imm(v1, 0);
        builder.ins().return_(&[]);

        builder.seal_all_blocks();
        builder.finalize();

        eprintln!("Actual = {}", func.display());
        assert_eq!(
            func.display().to_string().trim(),
            r#"
function %sample(i32) system_v {
    ss0 = explicit_slot 8, align = 8
    ss1 = explicit_slot 8, align = 8
    sig0 = () system_v
    fn0 = colocated u0:0 sig0

block0(v0: i32):
    brif v0, block1, block2

block1:
    v1 = iconst.i64 1
    stack_store v1, ss0  ; v1 = 1
    v2 = iconst.i64 2
    stack_store v2, ss1  ; v2 = 2
    call fn0(), stack_map=[i64 @ ss0+0, i64 @ ss1+0]
    v8 = stack_load.i64 ss0
    v9 = stack_load.i64 ss1
    jump block3(v8, v9)

block2:
    v3 = iconst.i64 3
    stack_store v3, ss0  ; v3 = 3
    v4 = iconst.i64 4
    call fn0(), stack_map=[i64 @ ss0+0]
    v10 = stack_load.i64 ss0
    v11 = stack_load.i64 ss0
    jump block3(v10, v11)

block3(v5: i64, v6: i64):
    call fn0(), stack_map=[i64 @ ss0+0]
    v12 = stack_load.i64 ss0
    v7 = iadd_imm v12, 0
    return
}
            "#
            .trim()
        );
    }

    #[test]
    fn needs_stack_map_and_heterogeneous_types() {
        let mut sig = Signature::new(CallConv::SystemV);
        for ty in [
            ir::types::I8,
            ir::types::I16,
            ir::types::I32,
            ir::types::I64,
            ir::types::I128,
            ir::types::F32,
            ir::types::F64,
            ir::types::I8X16,
            ir::types::I16X8,
        ] {
            sig.params.push(AbiParam::new(ty));
            sig.returns.push(AbiParam::new(ty));
        }

        let mut fn_ctx = FunctionBuilderContext::new();
        let mut func = Function::with_name_signature(ir::UserFuncName::testcase("sample"), sig);
        let mut builder = FunctionBuilder::new(&mut func, &mut fn_ctx);

        let name = builder
            .func
            .declare_imported_user_function(ir::UserExternalName {
                namespace: 0,
                index: 0,
            });
        let signature = builder
            .func
            .import_signature(Signature::new(CallConv::SystemV));
        let func_ref = builder.import_function(ir::ExtFuncData {
            name: ir::ExternalName::user(name),
            signature,
            colocated: true,
        });

        // Test that we support stack maps of heterogeneous types and properly
        // coalesce types into stack slots based on their size.
        //
        //     block0(v0, v1, v2, v3, v4, v5, v6, v7, v8):
        //       call $foo()
        //       return v0, v1, v2, v3, v4, v5, v6, v7, v8
        let block0 = builder.create_block();
        builder.append_block_params_for_function_params(block0);

        builder.switch_to_block(block0);
        let params = builder.func.dfg.block_params(block0).to_vec();
        for val in &params {
            builder.declare_value_needs_stack_map(*val);
        }
        builder.ins().call(func_ref, &[]);
        builder.ins().return_(&params);

        builder.seal_all_blocks();
        builder.finalize();

        eprintln!("Actual = {}", func.display());
        assert_eq!(
            func.display().to_string().trim(),
            r#"
function %sample(i8, i16, i32, i64, i128, f32, f64, i8x16, i16x8) -> i8, i16, i32, i64, i128, f32, f64, i8x16, i16x8 system_v {
    ss0 = explicit_slot 1
    ss1 = explicit_slot 2, align = 2
    ss2 = explicit_slot 4, align = 4
    ss3 = explicit_slot 8, align = 8
    ss4 = explicit_slot 16, align = 16
    ss5 = explicit_slot 4, align = 4
    ss6 = explicit_slot 8, align = 8
    ss7 = explicit_slot 16, align = 16
    ss8 = explicit_slot 16, align = 16
    sig0 = () system_v
    fn0 = colocated u0:0 sig0

block0(v0: i8, v1: i16, v2: i32, v3: i64, v4: i128, v5: f32, v6: f64, v7: i8x16, v8: i16x8):
    stack_store v0, ss0
    stack_store v1, ss1
    stack_store v2, ss2
    stack_store v3, ss3
    stack_store v4, ss4
    stack_store v5, ss5
    stack_store v6, ss6
    stack_store v7, ss7
    stack_store v8, ss8
    call fn0(), stack_map=[i8 @ ss0+0, i16 @ ss1+0, i32 @ ss2+0, i64 @ ss3+0, i128 @ ss4+0, f32 @ ss5+0, f64 @ ss6+0, i8x16 @ ss7+0, i16x8 @ ss8+0]
    v9 = stack_load.i8 ss0
    v10 = stack_load.i16 ss1
    v11 = stack_load.i32 ss2
    v12 = stack_load.i64 ss3
    v13 = stack_load.i128 ss4
    v14 = stack_load.f32 ss5
    v15 = stack_load.f64 ss6
    v16 = stack_load.i8x16 ss7
    v17 = stack_load.i16x8 ss8
    return v9, v10, v11, v12, v13, v14, v15, v16, v17
}
            "#
            .trim()
        );
    }

    #[test]
    fn series_of_non_overlapping_live_ranges_needs_stack_map() {
        let sig = Signature::new(CallConv::SystemV);

        let mut fn_ctx = FunctionBuilderContext::new();
        let mut func = Function::with_name_signature(ir::UserFuncName::testcase("sample"), sig);
        let mut builder = FunctionBuilder::new(&mut func, &mut fn_ctx);

        let name = builder
            .func
            .declare_imported_user_function(ir::UserExternalName {
                namespace: 0,
                index: 0,
            });
        let signature = builder
            .func
            .import_signature(Signature::new(CallConv::SystemV));
        let foo_func_ref = builder.import_function(ir::ExtFuncData {
            name: ir::ExternalName::user(name),
            signature,
            colocated: true,
        });

        let name = builder
            .func
            .declare_imported_user_function(ir::UserExternalName {
                namespace: 0,
                index: 1,
            });
        let mut sig = Signature::new(CallConv::SystemV);
        sig.params.push(AbiParam::new(ir::types::I32));
        let signature = builder.func.import_signature(sig);
        let consume_func_ref = builder.import_function(ir::ExtFuncData {
            name: ir::ExternalName::user(name),
            signature,
            colocated: true,
        });

        // Create a series of needs-stack-map values that do not have
        // overlapping live ranges, but which do appear in stack maps for calls
        // to `$foo`:
        //
        //     block0:
        //       v0 = needs stack map
        //       call $foo()
        //       call consume(v0)
        //       v1 = needs stack map
        //       call $foo()
        //       call consume(v1)
        //       v2 = needs stack map
        //       call $foo()
        //       call consume(v2)
        //       v3 = needs stack map
        //       call $foo()
        //       call consume(v3)
        //       return
        let block0 = builder.create_block();
        builder.append_block_params_for_function_params(block0);
        builder.switch_to_block(block0);
        let v0 = builder.ins().iconst(ir::types::I32, 0);
        builder.declare_value_needs_stack_map(v0);
        builder.ins().call(foo_func_ref, &[]);
        builder.ins().call(consume_func_ref, &[v0]);
        let v1 = builder.ins().iconst(ir::types::I32, 1);
        builder.declare_value_needs_stack_map(v1);
        builder.ins().call(foo_func_ref, &[]);
        builder.ins().call(consume_func_ref, &[v1]);
        let v2 = builder.ins().iconst(ir::types::I32, 2);
        builder.declare_value_needs_stack_map(v2);
        builder.ins().call(foo_func_ref, &[]);
        builder.ins().call(consume_func_ref, &[v2]);
        let v3 = builder.ins().iconst(ir::types::I32, 3);
        builder.declare_value_needs_stack_map(v3);
        builder.ins().call(foo_func_ref, &[]);
        builder.ins().call(consume_func_ref, &[v3]);
        builder.ins().return_(&[]);
        builder.seal_all_blocks();
        builder.finalize();

        eprintln!("Actual = {}", func.display());
        assert_eq!(
            func.display().to_string().trim(),
            r#"
function %sample() system_v {
    ss0 = explicit_slot 4, align = 4
    sig0 = () system_v
    sig1 = (i32) system_v
    fn0 = colocated u0:0 sig0
    fn1 = colocated u0:1 sig1

block0:
    v0 = iconst.i32 0
    stack_store v0, ss0  ; v0 = 0
    call fn0(), stack_map=[i32 @ ss0+0]
    v4 = stack_load.i32 ss0
    call fn1(v4)
    v1 = iconst.i32 1
    stack_store v1, ss0  ; v1 = 1
    call fn0(), stack_map=[i32 @ ss0+0]
    v5 = stack_load.i32 ss0
    call fn1(v5)
    v2 = iconst.i32 2
    stack_store v2, ss0  ; v2 = 2
    call fn0(), stack_map=[i32 @ ss0+0]
    v6 = stack_load.i32 ss0
    call fn1(v6)
    v3 = iconst.i32 3
    stack_store v3, ss0  ; v3 = 3
    call fn0(), stack_map=[i32 @ ss0+0]
    v7 = stack_load.i32 ss0
    call fn1(v7)
    return
}
            "#
            .trim()
        );
    }

    #[test]
    fn vars_block_params_and_needs_stack_map() {
        let _ = env_logger::try_init();

        let mut sig = Signature::new(CallConv::SystemV);
        sig.params.push(AbiParam::new(ir::types::I32));
        sig.returns.push(AbiParam::new(ir::types::I32));

        let mut fn_ctx = FunctionBuilderContext::new();
        let mut func = Function::with_name_signature(ir::UserFuncName::testcase("sample"), sig);
        let mut builder = FunctionBuilder::new(&mut func, &mut fn_ctx);

        let name = builder
            .func
            .declare_imported_user_function(ir::UserExternalName {
                namespace: 0,
                index: 0,
            });
        let mut sig = Signature::new(CallConv::SystemV);
        sig.params.push(AbiParam::new(ir::types::I32));
        let signature = builder.func.import_signature(sig);
        let func_ref = builder.import_function(ir::ExtFuncData {
            name: ir::ExternalName::user(name),
            signature,
            colocated: true,
        });

        // Use a variable, create a control flow diamond so that the variable
        // forces a block parameter on the control join point, and make sure
        // that we get stack maps for all the appropriate uses of the variable
        // in all blocks, as well as that we are reusing stack slots for each of
        // the values.
        //
        //                        block0:
        //                          x := needs stack map
        //                          call $foo(x)
        //                          br_if v0, block1, block2
        //
        //
        //     block1:                                     block2:
        //       call $foo(x)                                call $foo(x)
        //       call $foo(x)                                call $foo(x)
        //       x := new needs stack map                    x := new needs stack map
        //       call $foo(x)                                call $foo(x)
        //       jump block3                                 jump block3
        //
        //
        //                        block3:
        //                          call $foo(x)
        //                          return x

        let x = Variable::from_u32(0);
        builder.declare_var(x, ir::types::I32);
        builder.declare_var_needs_stack_map(x);

        let block0 = builder.create_block();
        let block1 = builder.create_block();
        let block2 = builder.create_block();
        let block3 = builder.create_block();

        builder.append_block_params_for_function_params(block0);
        builder.switch_to_block(block0);
        let v0 = builder.func.dfg.block_params(block0)[0];
        let val = builder.ins().iconst(ir::types::I32, 42);
        builder.def_var(x, val);
        {
            let x = builder.use_var(x);
            builder.ins().call(func_ref, &[x]);
        }
        builder.ins().brif(v0, block1, &[], block2, &[]);

        builder.switch_to_block(block1);
        {
            let x = builder.use_var(x);
            builder.ins().call(func_ref, &[x]);
            builder.ins().call(func_ref, &[x]);
        }
        let val = builder.ins().iconst(ir::types::I32, 36);
        builder.def_var(x, val);
        {
            let x = builder.use_var(x);
            builder.ins().call(func_ref, &[x]);
        }
        builder.ins().jump(block3, &[]);

        builder.switch_to_block(block2);
        {
            let x = builder.use_var(x);
            builder.ins().call(func_ref, &[x]);
            builder.ins().call(func_ref, &[x]);
        }
        let val = builder.ins().iconst(ir::types::I32, 36);
        builder.def_var(x, val);
        {
            let x = builder.use_var(x);
            builder.ins().call(func_ref, &[x]);
        }
        builder.ins().jump(block3, &[]);

        builder.switch_to_block(block3);
        let x = builder.use_var(x);
        builder.ins().call(func_ref, &[x]);
        builder.ins().return_(&[x]);

        builder.seal_all_blocks();
        builder.finalize();

        eprintln!("Actual = {}", func.display());

        // Because our liveness analysis is very simple, and visit blocks in the
        // order 3->1->2->0, we see uses of `v2` in block1 and mark it live
        // across all of block2 because we haven't reached the def in block0
        // yet, even though it isn't technically live out of block2, only live
        // in. This means that it shows up in the stack map for block2's second
        // call to `foo()` when it technically needn't, and additionally means
        // that we have two stack slots instead of a single one below. This
        // could all be improved and cleaned up by improving the liveness
        // analysis.
        assert_eq!(
            func.display().to_string().trim(),
            r#"
function %sample(i32) -> i32 system_v {
    ss0 = explicit_slot 4, align = 4
    ss1 = explicit_slot 4, align = 4
    sig0 = (i32) system_v
    fn0 = colocated u0:0 sig0

block0(v0: i32):
    v1 = iconst.i32 42
    v2 -> v1
    v4 -> v1
    stack_store v1, ss0  ; v1 = 42
    v7 = stack_load.i32 ss0
    call fn0(v7), stack_map=[i32 @ ss0+0]
    brif v0, block1, block2

block1:
    call fn0(v2), stack_map=[i32 @ ss0+0]  ; v2 = 42
    call fn0(v2)  ; v2 = 42
    v3 = iconst.i32 36
    stack_store v3, ss0  ; v3 = 36
    v8 = stack_load.i32 ss0
    call fn0(v8), stack_map=[i32 @ ss0+0]
    v9 = stack_load.i32 ss0
    jump block3(v9)

block2:
    call fn0(v4), stack_map=[i32 @ ss0+0]  ; v4 = 42
    call fn0(v4), stack_map=[i32 @ ss0+0]  ; v4 = 42
    v5 = iconst.i32 36
    stack_store v5, ss1  ; v5 = 36
    v10 = stack_load.i32 ss1
    call fn0(v10), stack_map=[i32 @ ss0+0, i32 @ ss1+0]
    v11 = stack_load.i32 ss1
    jump block3(v11)

block3(v6: i32):
    stack_store v6, ss0
    call fn0(v6), stack_map=[i32 @ ss0+0]
    v12 = stack_load.i32 ss0
    return v12
}
            "#
            .trim()
        );
    }

    #[test]
    fn var_needs_stack_map() {
        let mut sig = Signature::new(CallConv::SystemV);
        sig.params
            .push(AbiParam::new(cranelift_codegen::ir::types::I32));
        sig.returns
            .push(AbiParam::new(cranelift_codegen::ir::types::I32));

        let mut fn_ctx = FunctionBuilderContext::new();
        let mut func = Function::with_name_signature(ir::UserFuncName::testcase("sample"), sig);
        let mut builder = FunctionBuilder::new(&mut func, &mut fn_ctx);

        let var = Variable::from_u32(0);
        builder.declare_var(var, cranelift_codegen::ir::types::I32);
        builder.declare_var_needs_stack_map(var);

        let name = builder
            .func
            .declare_imported_user_function(ir::UserExternalName {
                namespace: 0,
                index: 0,
            });
        let signature = builder
            .func
            .import_signature(Signature::new(CallConv::SystemV));
        let func_ref = builder.import_function(ir::ExtFuncData {
            name: ir::ExternalName::user(name),
            signature,
            colocated: true,
        });

        let block0 = builder.create_block();
        builder.append_block_params_for_function_params(block0);
        builder.switch_to_block(block0);

        let arg = builder.func.dfg.block_params(block0)[0];
        builder.def_var(var, arg);

        builder.ins().call(func_ref, &[]);

        let val = builder.use_var(var);
        builder.ins().return_(&[val]);

        builder.seal_all_blocks();
        builder.finalize();

        eprintln!("Actual = {}", func.display());
        assert_eq!(
            func.display().to_string().trim(),
            r#"
function %sample(i32) -> i32 system_v {
    ss0 = explicit_slot 4, align = 4
    sig0 = () system_v
    fn0 = colocated u0:0 sig0

block0(v0: i32):
    stack_store v0, ss0
    call fn0(), stack_map=[i32 @ ss0+0]
    v1 = stack_load.i32 ss0
    return v1
}
            "#
            .trim()
        );
    }
}
