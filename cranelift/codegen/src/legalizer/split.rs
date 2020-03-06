//! Value splitting.
//!
//! Some value types are too large to fit in registers, so they need to be split into smaller parts
//! that the ISA can operate on. There's two dimensions of splitting, represented by two
//! complementary instruction pairs:
//!
//! - `isplit` and `iconcat` for splitting integer types into smaller integers.
//! - `vsplit` and `vconcat` for splitting vector types into smaller vector types with the same
//!   lane types.
//!
//! There is no floating point splitting. If an ISA doesn't support `f64` values, they probably
//! have to be bit-cast to `i64` and possibly split into two `i32` values that fit in registers.
//! This breakdown is handled by the ABI lowering.
//!
//! When legalizing a single instruction, it is wrapped in splits and concatenations:
//!
//! ```clif
//!     v1 = bxor.i64 v2, v3
//! ```
//!
//! becomes:
//!
//! ```clif
//!     v20, v21 = isplit v2
//!     v30, v31 = isplit v3
//!     v10 = bxor.i32 v20, v30
//!     v11 = bxor.i32 v21, v31
//!     v1 = iconcat v10, v11
//! ```
//!
//! This local expansion approach still leaves the original `i64` values in the code as operands on
//! the `split` and `concat` instructions. It also creates a lot of redundant code to clean up as
//! values are constantly split and concatenated.
//!
//! # Optimized splitting
//!
//! We can eliminate a lot of the splitting code quite easily. Whenever we need to split a value,
//! first check if the value is defined by the corresponding concatenation. If so, then just use
//! the two concatenation inputs directly:
//!
//! ```clif
//!     v4 = iadd_imm.i64 v1, 1
//! ```
//!
//! becomes, using the expanded code from above:
//!
//! ```clif
//!     v40, v5 = iadd_imm_cout.i32 v10, 1
//!     v6 = bint.i32
//!     v41 = iadd.i32 v11, v6
//!     v4 = iconcat v40, v41
//! ```
//!
//! This means that the `iconcat` instructions defining `v1` and `v4` end up with no uses, so they
//! can be trivially deleted by a dead code elimination pass.
//!
//! # block arguments
//!
//! If all instructions that produce an `i64` value are legalized as above, we will eventually end
//! up with no `i64` values anywhere, except for block arguments. We can work around this by
//! iteratively splitting block arguments too. That should leave us with no illegal value types
//! anywhere.
//!
//! It is possible to have circular dependencies of block arguments that are never used by any real
//! instructions. These loops will remain in the program.

use crate::cursor::{Cursor, CursorPosition, FuncCursor};
use crate::flowgraph::{BlockPredecessor, ControlFlowGraph};
use crate::ir::{self, Block, Inst, InstBuilder, InstructionData, Opcode, Type, Value, ValueDef};
use alloc::vec::Vec;
use core::iter;
use smallvec::SmallVec;

/// Split `value` into two values using the `isplit` semantics. Do this by reusing existing values
/// if possible.
pub fn isplit(
    func: &mut ir::Function,
    cfg: &ControlFlowGraph,
    pos: CursorPosition,
    srcloc: ir::SourceLoc,
    value: Value,
) -> (Value, Value) {
    split_any(func, cfg, pos, srcloc, value, Opcode::Iconcat)
}

/// Split `value` into halves using the `vsplit` semantics. Do this by reusing existing values if
/// possible.
pub fn vsplit(
    func: &mut ir::Function,
    cfg: &ControlFlowGraph,
    pos: CursorPosition,
    srcloc: ir::SourceLoc,
    value: Value,
) -> (Value, Value) {
    split_any(func, cfg, pos, srcloc, value, Opcode::Vconcat)
}

/// After splitting a block argument, we need to go back and fix up all of the predecessor
/// instructions. This is potentially a recursive operation, but we don't implement it recursively
/// since that could use up too muck stack.
///
/// Instead, the repairs are deferred and placed on a work list in stack form.
struct Repair {
    concat: Opcode,
    // The argument type after splitting.
    split_type: Type,
    // The destination block whose arguments have been split.
    block: Block,
    // Number of the original block argument which has been replaced by the low part.
    num: usize,
    // Number of the new block argument which represents the high part after the split.
    hi_num: usize,
}

/// Generic version of `isplit` and `vsplit` controlled by the `concat` opcode.
fn split_any(
    func: &mut ir::Function,
    cfg: &ControlFlowGraph,
    pos: CursorPosition,
    srcloc: ir::SourceLoc,
    value: Value,
    concat: Opcode,
) -> (Value, Value) {
    let mut repairs = Vec::new();
    let pos = &mut FuncCursor::new(func).at_position(pos).with_srcloc(srcloc);
    let result = split_value(pos, value, concat, &mut repairs);

    perform_repairs(pos, cfg, repairs);

    result
}

pub fn split_block_params(func: &mut ir::Function, cfg: &ControlFlowGraph, block: Block) {
    let pos = &mut FuncCursor::new(func).at_top(block);
    let block_params = pos.func.dfg.block_params(block);

    // Add further splittable types here.
    fn type_requires_splitting(ty: Type) -> bool {
        ty == ir::types::I128
    }

    // A shortcut.  If none of the param types require splitting, exit now.  This helps because
    // the loop below necessarily has to copy the block params into a new vector, so it's better to
    // avoid doing so when possible.
    if !block_params
        .iter()
        .any(|block_param| type_requires_splitting(pos.func.dfg.value_type(*block_param)))
    {
        return;
    }

    let mut repairs = Vec::new();
    for (num, block_param) in block_params.to_vec().into_iter().enumerate() {
        if !type_requires_splitting(pos.func.dfg.value_type(block_param)) {
            continue;
        }

        split_block_param(pos, block, num, block_param, Opcode::Iconcat, &mut repairs);
    }

    perform_repairs(pos, cfg, repairs);
}

fn perform_repairs(pos: &mut FuncCursor, cfg: &ControlFlowGraph, mut repairs: Vec<Repair>) {
    // We have split the value requested, and now we may need to fix some block predecessors.
    while let Some(repair) = repairs.pop() {
        for BlockPredecessor { inst, .. } in cfg.pred_iter(repair.block) {
            let branch_opc = pos.func.dfg[inst].opcode();
            debug_assert!(
                branch_opc.is_branch(),
                "Predecessor not a branch: {}",
                pos.func.dfg.display_inst(inst, None)
            );
            let num_fixed_args = branch_opc.constraints().num_fixed_value_arguments();
            let mut args = pos.func.dfg[inst]
                .take_value_list()
                .expect("Branches must have value lists.");
            let num_args = args.len(&pos.func.dfg.value_lists);
            // Get the old value passed to the block argument we're repairing.
            let old_arg = args
                .get(num_fixed_args + repair.num, &pos.func.dfg.value_lists)
                .expect("Too few branch arguments");

            // It's possible that the CFG's predecessor list has duplicates. Detect them here.
            if pos.func.dfg.value_type(old_arg) == repair.split_type {
                pos.func.dfg[inst].put_value_list(args);
                continue;
            }

            // Split the old argument, possibly causing more repairs to be scheduled.
            pos.goto_inst(inst);

            let inst_block = pos.func.layout.inst_block(inst).expect("inst in block");

            // Insert split values prior to the terminal branch group.
            let canonical = pos
                .func
                .layout
                .canonical_branch_inst(&pos.func.dfg, inst_block);
            if let Some(first_branch) = canonical {
                pos.goto_inst(first_branch);
            }

            let (lo, hi) = split_value(pos, old_arg, repair.concat, &mut repairs);

            // The `lo` part replaces the original argument.
            *args
                .get_mut(num_fixed_args + repair.num, &mut pos.func.dfg.value_lists)
                .unwrap() = lo;

            // The `hi` part goes at the end. Since multiple repairs may have been scheduled to the
            // same block, there could be multiple arguments missing.
            if num_args > num_fixed_args + repair.hi_num {
                *args
                    .get_mut(
                        num_fixed_args + repair.hi_num,
                        &mut pos.func.dfg.value_lists,
                    )
                    .unwrap() = hi;
            } else {
                // We need to append one or more arguments. If we're adding more than one argument,
                // there must be pending repairs on the stack that will fill in the correct values
                // instead of `hi`.
                args.extend(
                    iter::repeat(hi).take(1 + num_fixed_args + repair.hi_num - num_args),
                    &mut pos.func.dfg.value_lists,
                );
            }

            // Put the value list back after manipulating it.
            pos.func.dfg[inst].put_value_list(args);
        }
    }
}

/// Split a single value using the integer or vector semantics given by the `concat` opcode.
///
/// If the value is defined by a `concat` instruction, just reuse the operand values of that
/// instruction.
///
/// Return the two new values representing the parts of `value`.
fn split_value(
    pos: &mut FuncCursor,
    value: Value,
    concat: Opcode,
    repairs: &mut Vec<Repair>,
) -> (Value, Value) {
    let value = pos.func.dfg.resolve_aliases(value);
    let mut reuse = None;

    match pos.func.dfg.value_def(value) {
        ValueDef::Result(inst, num) => {
            // This is an instruction result. See if the value was created by a `concat`
            // instruction.
            if let InstructionData::Binary { opcode, args, .. } = pos.func.dfg[inst] {
                debug_assert_eq!(num, 0);
                if opcode == concat {
                    reuse = Some((args[0], args[1]));
                }
            }
        }
        ValueDef::Param(block, num) => {
            // This is a block parameter.
            // We can split the parameter value unless this is the entry block.
            if pos.func.layout.entry_block() != Some(block) {
                reuse = Some(split_block_param(pos, block, num, value, concat, repairs));
            }
        }
    }

    // Did the code above succeed in finding values we can reuse?
    if let Some(pair) = reuse {
        pair
    } else {
        // No, we'll just have to insert the requested split instruction at `pos`. Note that `pos`
        // has not been moved by the block argument code above when `reuse` is `None`.
        match concat {
            Opcode::Iconcat => pos.ins().isplit(value),
            Opcode::Vconcat => pos.ins().vsplit(value),
            _ => panic!("Unhandled concat opcode: {}", concat),
        }
    }
}

fn split_block_param(
    pos: &mut FuncCursor,
    block: Block,
    param_num: usize,
    value: Value,
    concat: Opcode,
    repairs: &mut Vec<Repair>,
) -> (Value, Value) {
    // We are going to replace the parameter at `num` with two new arguments.
    // Determine the new value types.
    let ty = pos.func.dfg.value_type(value);
    let split_type = match concat {
        Opcode::Iconcat => ty.half_width().expect("Invalid type for isplit"),
        Opcode::Vconcat => ty.half_vector().expect("Invalid type for vsplit"),
        _ => panic!("Unhandled concat opcode: {}", concat),
    };

    // Since the `repairs` stack potentially contains other parameter numbers for
    // `block`, avoid shifting and renumbering block parameters. It could invalidate other
    // `repairs` entries.
    //
    // Replace the original `value` with the low part, and append the high part at the
    // end of the argument list.
    let lo = pos.func.dfg.replace_block_param(value, split_type);
    let hi_num = pos.func.dfg.num_block_params(block);
    let hi = pos.func.dfg.append_block_param(block, split_type);

    // Now the original value is dangling. Insert a concatenation instruction that can
    // compute it from the two new parameters. This also serves as a record of what we
    // did so a future call to this function doesn't have to redo the work.
    //
    // Note that it is safe to move `pos` here since `reuse` was set above, so we don't
    // need to insert a split instruction before returning.
    pos.goto_first_inst(block);
    pos.ins()
        .with_result(value)
        .Binary(concat, split_type, lo, hi);

    // Finally, splitting the block parameter is not enough. We also have to repair all
    // of the predecessor instructions that branch here.
    add_repair(concat, split_type, block, param_num, hi_num, repairs);

    (lo, hi)
}

// Add a repair entry to the work list.
fn add_repair(
    concat: Opcode,
    split_type: Type,
    block: Block,
    num: usize,
    hi_num: usize,
    repairs: &mut Vec<Repair>,
) {
    repairs.push(Repair {
        concat,
        split_type,
        block,
        num,
        hi_num,
    });
}

/// Strip concat-split chains. Return a simpler way of computing the same value.
///
/// Given this input:
///
/// ```clif
///     v10 = iconcat v1, v2
///     v11, v12 = isplit v10
/// ```
///
/// This function resolves `v11` to `v1` and `v12` to `v2`.
fn resolve_splits(dfg: &ir::DataFlowGraph, value: Value) -> Value {
    let value = dfg.resolve_aliases(value);

    // Deconstruct a split instruction.
    let split_res;
    let concat_opc;
    let split_arg;
    if let ValueDef::Result(inst, num) = dfg.value_def(value) {
        split_res = num;
        concat_opc = match dfg[inst].opcode() {
            Opcode::Isplit => Opcode::Iconcat,
            Opcode::Vsplit => Opcode::Vconcat,
            _ => return value,
        };
        split_arg = dfg.inst_args(inst)[0];
    } else {
        return value;
    }

    // See if split_arg is defined by a concatenation instruction.
    if let ValueDef::Result(inst, _) = dfg.value_def(split_arg) {
        if dfg[inst].opcode() == concat_opc {
            return dfg.inst_args(inst)[split_res];
        }
    }

    value
}

/// Simplify the arguments to a branch *after* the instructions leading up to the branch have been
/// legalized.
///
/// The branch argument repairs performed by `split_any()` above may be performed on branches that
/// have not yet been legalized. The repaired arguments can be defined by actual split
/// instructions in that case.
///
/// After legalizing the instructions computing the value that was split, it is likely that we can
/// avoid depending on the split instruction. Its input probably comes from a concatenation.
pub fn simplify_branch_arguments(dfg: &mut ir::DataFlowGraph, branch: Inst) {
    let mut new_args = SmallVec::<[Value; 32]>::new();

    for &arg in dfg.inst_args(branch) {
        let new_arg = resolve_splits(dfg, arg);
        new_args.push(new_arg);
    }

    dfg.inst_args_mut(branch).copy_from_slice(&new_args);
}
