//! The abstract operand stack `GcOps::fixup` tracks to make an op sequence valid.

use crate::generators::gc_ops::ops::GcOp;
use crate::generators::gc_ops::types::{TypeId, Types};

/// Tracks the required operand type on the abstract value stack.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum StackType {
    /// `externref`.
    ExternRef,
    /// `eqref`.
    Eq,
    /// `i31ref`.
    I31,
    /// `(ref $*)` — optionally with a concrete type index.
    Struct(Option<u32>),
    /// `(ref array)` or `(ref $t)` — optionally with a concrete type index.
    Array(Option<u32>),
}

/// Whether the dense type index `sub` is the same as, or a subtype of, `sup`.
fn is_subtype_dense(sub: u32, sup: u32, types: &Types, encoding_order: &[TypeId]) -> bool {
    let sub = encoding_order.get(usize::try_from(sub).unwrap());
    let sup = encoding_order.get(usize::try_from(sup).unwrap());
    match (sub, sup) {
        (Some(&sub), Some(&sup)) => types.is_subtype(sub, sup),
        _ => false,
    }
}

/// Whether `top` can serve as an operand of type `req`, including by subtyping.
fn satisfies(req: StackType, top: StackType, types: &Types, encoding_order: &[TypeId]) -> bool {
    use StackType::{Array, Eq, ExternRef, I31, Struct};
    match (req, top) {
        (ExternRef, ExternRef) | (I31, I31) => true,
        // struct, array and i31 are all subtypes of eq.
        (Eq, Eq | Struct(_) | Array(_) | I31) => true,
        (Struct(None), Struct(_)) | (Array(None), Array(_)) => true,
        (Struct(Some(want)), Struct(Some(have))) | (Array(Some(want)), Array(Some(have))) => {
            is_subtype_dense(have, want, types, encoding_order)
        }
        _ => false,
    }
}

/// The op that produces a value of type `req` from nothing: a null, or a fresh
/// object for a concrete type. Abstract nulls also cover the case of no types at all.
fn synthesize(req: StackType, num_types: u32) -> GcOp {
    match req {
        StackType::ExternRef => GcOp::NullExtern,
        StackType::Eq => GcOp::NullEq,
        StackType::I31 => GcOp::RefI31 { value: 0 },
        StackType::Struct(None) => GcOp::NullStruct,
        StackType::Array(None) => GcOp::NullArray,
        StackType::Struct(Some(t)) => {
            debug_assert_ne!(num_types, 0, "typed struct requirement with no types");
            GcOp::StructNew {
                type_index: StackType::clamp(t, num_types),
            }
        }
        StackType::Array(Some(t)) => {
            debug_assert_ne!(num_types, 0, "typed array requirement with no types");
            GcOp::ArrayNewDefault {
                type_index: StackType::clamp(t, num_types),
            }
        }
    }
}

impl StackType {
    /// Ensure the top of `stack` satisfies `req`, emitting a fixup op if it does
    /// not, then pop it. `None` (a `Drop` operand) takes anything.
    pub fn fixup_operand(
        req: Option<StackType>,
        stack: &mut Vec<StackType>,
        out: &mut Vec<GcOp>,
        num_types: u32,
        types: &Types,
        encoding_order: &[TypeId],
    ) {
        let mut result_types = Vec::new();
        match req {
            None => {
                if stack.is_empty() {
                    Self::emit(GcOp::NullExtern, stack, out, num_types, &mut result_types);
                }
            }
            Some(req) => {
                let ok = stack
                    .last()
                    .is_some_and(|&top| satisfies(req, top, types, encoding_order));
                if !ok {
                    let op = synthesize(req, num_types);
                    Self::emit(op, stack, out, num_types, &mut result_types);
                }
            }
        }
        let popped = stack.pop();
        log::trace!("[StackType::fixup_operand] req={req:?} popped={popped:?} stack={stack:?}");
    }

    /// Emit an opcode and update the stack.
    pub(crate) fn emit(
        op: GcOp,
        stack: &mut Vec<Self>,
        out: &mut Vec<GcOp>,
        num_types: u32,
        result_types: &mut Vec<Self>,
    ) {
        log::trace!(
            "[StackType::emit] op={op:?} stack_len_before={} num_types={num_types}",
            stack.len()
        );
        out.push(op);
        result_types.clear();
        op.result_types(result_types);
        for ty in result_types {
            let clamped_ty = match ty {
                Self::Struct(Some(t)) => Self::Struct(Some(Self::clamp(*t, num_types))),
                Self::Array(Some(t)) => Self::Array(Some(Self::clamp(*t, num_types))),
                other => *other,
            };
            log::trace!("[StackType::emit] push result {clamped_ty:?}");
            stack.push(clamped_ty);
        }
        log::trace!("[StackType::emit] leave stack={stack:?}");
    }

    /// Fixup for cast ops: ensures the sub/super type relationship actually
    /// holds. Always repairs the op rather than dropping it.
    ///
    /// For upcast the operand (sub) is on the stack, so we keep sub fixed
    /// and adjust super. For downcast the operand (super) is on the stack,
    /// so we keep super fixed and adjust sub.
    pub fn fixup_cast(op: GcOp, types: &Types, encoding_order: &[TypeId]) -> GcOp {
        match op {
            GcOp::RefCastUpward {
                sub_type_index,
                super_type_index,
            } => {
                // Operand is sub (on the stack) — keep it, fix super.
                let super_type_index = Self::find_supertype_of(
                    sub_type_index,
                    super_type_index,
                    types,
                    encoding_order,
                );
                GcOp::RefCastUpward {
                    sub_type_index,
                    super_type_index,
                }
            }
            GcOp::RefCastDownward {
                sub_type_index,
                super_type_index,
            } => {
                // Operand is super (on the stack) — keep it, fix sub.
                let sub_type_index =
                    Self::find_subtype_of(super_type_index, sub_type_index, types, encoding_order);
                GcOp::RefCastDownward {
                    sub_type_index,
                    super_type_index,
                }
            }
            // Array casts use the same index repair (subtyping is kind-agnostic).
            GcOp::ArrayRefCastUpward {
                sub_type_index,
                super_type_index,
            } => {
                let super_type_index = Self::find_supertype_of(
                    sub_type_index,
                    super_type_index,
                    types,
                    encoding_order,
                );
                GcOp::ArrayRefCastUpward {
                    sub_type_index,
                    super_type_index,
                }
            }
            GcOp::ArrayRefCastDownward {
                sub_type_index,
                super_type_index,
            } => {
                let sub_type_index =
                    Self::find_subtype_of(super_type_index, sub_type_index, types, encoding_order);
                GcOp::ArrayRefCastDownward {
                    sub_type_index,
                    super_type_index,
                }
            }
            other => other,
        }
    }

    /// Given a sub type on the stack, find a valid super_type_index such
    /// that sub <: super. Keeps sub fixed. Falls back to self-cast.
    fn find_supertype_of(
        sub_type_index: u32,
        super_type_index: u32,
        types: &Types,
        encoding_order: &[TypeId],
    ) -> u32 {
        if let (Some(&sub_tid), Some(&super_tid)) = (
            encoding_order.get(usize::try_from(sub_type_index).unwrap()),
            encoding_order.get(usize::try_from(super_type_index).unwrap()),
        ) {
            // Already valid.
            if types.is_subtype(sub_tid, super_tid) {
                return super_type_index;
            }
            // Try sub's direct supertype.
            if let Some(actual_super) = types.type_defs.get(&sub_tid).and_then(|d| d.supertype) {
                if let Some(idx) = encoding_order.iter().position(|&t| t == actual_super) {
                    return u32::try_from(idx).unwrap();
                }
            }
        }
        // Self-cast.
        sub_type_index
    }

    /// Given a super type on the stack, find a valid sub_type_index such
    /// that sub <: super. Keeps super fixed. Falls back to self-cast.
    fn find_subtype_of(
        super_type_index: u32,
        sub_type_index: u32,
        types: &Types,
        encoding_order: &[TypeId],
    ) -> u32 {
        if let (Some(&sub_tid), Some(&super_tid)) = (
            encoding_order.get(usize::try_from(sub_type_index).unwrap()),
            encoding_order.get(usize::try_from(super_type_index).unwrap()),
        ) {
            // Already valid.
            if types.is_subtype(sub_tid, super_tid) {
                return sub_type_index;
            }
            // Try to find any direct subtype of super.
            for (idx, tid) in encoding_order.iter().enumerate() {
                if let Some(def) = types.type_defs.get(tid) {
                    if def.supertype == Some(super_tid) {
                        return u32::try_from(idx).unwrap();
                    }
                }
            }
        }
        // Self-cast.
        super_type_index
    }

    /// Clamp a type index to the number of types.
    fn clamp(t: u32, n: u32) -> u32 {
        if n == 0 { 0 } else { t % n }
    }
}
