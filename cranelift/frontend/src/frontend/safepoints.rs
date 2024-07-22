//! Safepoints and stack maps.

use super::*;

impl FunctionBuilder<'_> {
    /// Insert spills for every value that needs to be in a stack map at every
    /// safepoint.
    ///
    /// First, we do a very simple, imprecise, and overapproximating liveness
    /// analysis. This considers any use (regardless if that use produces side
    /// effects or flows into another instruction that produces side effects!)
    /// of a needs-stack-map value to keep the value live. This allows us to do
    /// this liveness analysis in a single post-order traversal of the IR,
    /// without any fixed-point loop. The result of this analysis is the set of
    /// live needs-stack-map values at each instruction that must be a safepoint
    /// (currently this is just non-tail calls).
    ///
    /// Second, take those results, add stack slots so we have a place to spill
    /// to.
    ///
    /// Third, and finally, we spill the live needs-stack-map values at each
    /// safepoint into those stack slots.
    pub(super) fn insert_safepoint_spills(&mut self) {
        log::trace!(
            "before inserting safepoint spills and reloads:\n{}",
            self.func.display()
        );

        // Find all the GC values that are live across each safepoint and add
        // stack map entries for them.
        let stack_slots = self.find_live_stack_map_values_at_each_safepoint();

        // Insert spills to our new stack slots before each safepoint
        // instruction.
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

        // A map from size/align to free stack slots that are not being used
        // anymore. This allows us to reuse stack slots across multiple values
        // helps cut down on the ultimate size of our stack frames.
        let mut free_stack_slots: crate::HashMap<u32, SmallVec<[ir::StackSlot; 4]>> =
            Default::default();

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
                           free_stack_slots: &mut crate::HashMap<u32, SmallVec<_>>,
                           live: &mut BTreeSet<ir::Value>,
                           val: ir::Value| {
            log::trace!("liveness:   defining {val:?}, removing it from the live set");
            live.remove(&val);

            // This value's stack slot, if any, is now available for reuse.
            if let Some(slot) = stack_slots.get(&val) {
                log::trace!("liveness:     returning {slot:?} to the free list");
                let ty = func.dfg.value_type(val);
                free_stack_slots.entry(ty.bytes()).or_default().push(*slot);
            }
        };

        // Helper for (2)
        let process_safepoint = |func: &mut Function,
                                 stack_slots: &mut crate::HashMap<Value, StackSlot>,
                                 free_stack_slots: &mut crate::HashMap<u32, SmallVec<_>>,
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
                    free_stack_slots
                        .get_mut(&size)
                        .and_then(|list| list.pop().inspect(|slot| {
                            log::trace!(
                                "liveness:       reusing free stack slot {slot:?} for {val:?}"
                            )
                        }))
                        .unwrap_or_else(|| {
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
                        })
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
