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
//!```cton
//!     v1 = bxor.i64 v2, v3
//! ```
//!
//! becomes:
//!
//!```cton
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
//! ```cton
//!     v4 = iadd_imm.i64 v1, 1
//! ```
//!
//! becomes, using the expanded code from above:
//!
//! ```cton
//!     v40, v5 = iadd_imm_cout.i32 v10, 1
//!     v6 = bint.i32
//!     v41 = iadd.i32 v11, v6
//!     v4 = iconcat v40, v41
//! ```
//!
//! This means that the `iconcat` instructions defining `v1` and `v4` end up with no uses, so they
//! can be trivially deleted by a dead code elimination pass.
//!
//! # EBB arguments
//!
//! If all instructions that produce an `i64` value are legalized as above, we will eventually end
//! up with no `i64` values anywhere, except for EBB arguments. We can work around this by
//! iteratively splitting EBB arguments too. That should leave us with no illegal value types
//! anywhere.
//!
//! It is possible to have circular dependencies of EBB arguments that are never used by any real
//! instructions. These loops will remain in the program.

use flowgraph::ControlFlowGraph;
use ir::{DataFlowGraph, Cursor, Value, Opcode, ValueDef, InstructionData, InstBuilder};

/// Split `value` into two values using the `isplit` semantics. Do this by reusing existing values
/// if possible.
pub fn isplit(dfg: &mut DataFlowGraph,
              _cfg: &ControlFlowGraph,
              pos: &mut Cursor,
              value: Value)
              -> (Value, Value) {
    split_value(dfg, pos, value, Opcode::Iconcat)
}

/// Split `value` into halves using the `vsplit` semantics. Do this by reusing existing values if
/// possible.
pub fn vsplit(dfg: &mut DataFlowGraph,
              _cfg: &ControlFlowGraph,
              pos: &mut Cursor,
              value: Value)
              -> (Value, Value) {
    split_value(dfg, pos, value, Opcode::Vconcat)
}

/// Split a single value using the integer or vector semantics given by the `concat` opcode.
///
/// If the value is defined by a `concat` instruction, just reuse the operand values of that
/// instruction.
///
/// Return the two new values representing the parts of `value`.
fn split_value(dfg: &mut DataFlowGraph,
               pos: &mut Cursor,
               value: Value,
               concat: Opcode)
               -> (Value, Value) {
    let value = dfg.resolve_copies(value);
    let mut reuse = None;

    match dfg.value_def(value) {
        ValueDef::Res(inst, num) => {
            // This is an instruction result. See if the value was created by a `concat`
            // instruction.
            if let InstructionData::Binary { opcode, args, .. } = dfg[inst] {
                assert_eq!(num, 0);
                if opcode == concat {
                    reuse = Some((args[0], args[1]));
                }
            }
        }
        ValueDef::Arg(_ebb, _num) => {}
    }

    // Did the code above succeed in finding values we can reuse?
    if let Some(pair) = reuse {
        pair
    } else {
        // No, we'll just have to insert the requested split instruction at `pos`.
        match concat {
            Opcode::Iconcat => dfg.ins(pos).isplit(value),
            Opcode::Vconcat => dfg.ins(pos).vsplit(value),
            _ => panic!("Unhandled concat opcode: {}", concat),
        }
    }
}
