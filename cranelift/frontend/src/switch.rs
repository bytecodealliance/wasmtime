use super::HashMap;
use crate::frontend::FunctionBuilder;
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::*;
use log::debug;
use std::vec::Vec;

type EntryIndex = u64;

/// Unlike with `br_table`, `Switch` cases may be sparse or non-0-based.
/// They emit efficient code using branches, jump tables, or a combination of both.
///
/// # Example
///
/// ```rust
/// # use cranelift_codegen::ir::types::*;
/// # use cranelift_codegen::ir::{ExternalName, Function, Signature, InstBuilder};
/// # use cranelift_codegen::isa::CallConv;
/// # use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Switch};
/// #
/// # let mut sig = Signature::new(CallConv::SystemV);
/// # let mut fn_builder_ctx = FunctionBuilderContext::new();
/// # let mut func = Function::with_name_signature(ExternalName::user(0, 0), sig);
/// # let mut builder = FunctionBuilder::new(&mut func, &mut fn_builder_ctx);
/// #
/// # let entry = builder.create_ebb();
/// # builder.switch_to_block(entry);
/// #
/// let block0 = builder.create_ebb();
/// let block1 = builder.create_ebb();
/// let block2 = builder.create_ebb();
/// let fallback = builder.create_ebb();
///
/// let val = builder.ins().iconst(I32, 1);
///
/// let mut switch = Switch::new();
/// switch.set_entry(0, block0);
/// switch.set_entry(1, block1);
/// switch.set_entry(7, block2);
/// switch.emit(&mut builder, val, fallback);
/// ```
#[derive(Debug, Default)]
pub struct Switch {
    cases: HashMap<EntryIndex, Ebb>,
}

impl Switch {
    /// Create a new empty switch
    pub fn new() -> Self {
        Self {
            cases: HashMap::new(),
        }
    }

    /// Set a switch entry
    pub fn set_entry(&mut self, index: EntryIndex, ebb: Ebb) {
        let prev = self.cases.insert(index, ebb);
        assert!(
            prev.is_none(),
            "Tried to set the same entry {} twice",
            index
        );
    }

    /// Turn the `cases` `HashMap` into a list of `ContiguousCaseRange`s.
    ///
    /// # Postconditions
    ///
    /// * Every entry will be represented.
    /// * The `ContiguousCaseRange`s will not overlap.
    /// * Between two `ContiguousCaseRange`s there will be at least one entry index.
    /// * No `ContiguousCaseRange`s will be empty.
    fn collect_contiguous_case_ranges(self) -> Vec<ContiguousCaseRange> {
        debug!("build_contiguous_case_ranges before: {:#?}", self.cases);
        let mut cases = self.cases.into_iter().collect::<Vec<(_, _)>>();
        cases.sort_by_key(|&(index, _)| index);

        let mut contiguous_case_ranges: Vec<ContiguousCaseRange> = vec![];
        let mut last_index = None;
        for (index, ebb) in cases {
            match last_index {
                None => contiguous_case_ranges.push(ContiguousCaseRange::new(index)),
                Some(last_index) => {
                    if index > last_index + 1 {
                        contiguous_case_ranges.push(ContiguousCaseRange::new(index));
                    }
                }
            }
            contiguous_case_ranges.last_mut().unwrap().ebbs.push(ebb);
            last_index = Some(index);
        }

        debug!(
            "build_contiguous_case_ranges after: {:#?}",
            contiguous_case_ranges
        );

        contiguous_case_ranges
    }

    /// Binary search for the right `ContiguousCaseRange`.
    fn build_search_tree(
        bx: &mut FunctionBuilder,
        val: Value,
        otherwise: Ebb,
        contiguous_case_ranges: Vec<ContiguousCaseRange>,
    ) -> Vec<(EntryIndex, Ebb, Vec<Ebb>)> {
        let mut cases_and_jt_ebbs = Vec::new();

        // Avoid allocation in the common case
        if contiguous_case_ranges.len() <= 3 {
            Self::build_search_branches(
                bx,
                val,
                otherwise,
                contiguous_case_ranges,
                &mut cases_and_jt_ebbs,
            );
            return cases_and_jt_ebbs;
        }

        let mut stack: Vec<(Option<Ebb>, Vec<ContiguousCaseRange>)> = Vec::new();
        stack.push((None, contiguous_case_ranges));

        while let Some((ebb, contiguous_case_ranges)) = stack.pop() {
            if let Some(ebb) = ebb {
                bx.switch_to_block(ebb);
            }

            if contiguous_case_ranges.len() <= 3 {
                Self::build_search_branches(
                    bx,
                    val,
                    otherwise,
                    contiguous_case_ranges,
                    &mut cases_and_jt_ebbs,
                );
            } else {
                let split_point = contiguous_case_ranges.len() / 2;
                let mut left = contiguous_case_ranges;
                let right = left.split_off(split_point);

                let left_ebb = bx.create_ebb();
                let right_ebb = bx.create_ebb();

                let should_take_right_side = bx.ins().icmp_imm(
                    IntCC::UnsignedGreaterThanOrEqual,
                    val,
                    right[0].first_index as i64,
                );
                bx.ins().brnz(should_take_right_side, right_ebb, &[]);
                bx.ins().jump(left_ebb, &[]);

                stack.push((Some(left_ebb), left));
                stack.push((Some(right_ebb), right));
            }
        }

        cases_and_jt_ebbs
    }

    /// Linear search for the right `ContiguousCaseRange`.
    fn build_search_branches(
        bx: &mut FunctionBuilder,
        val: Value,
        otherwise: Ebb,
        contiguous_case_ranges: Vec<ContiguousCaseRange>,
        cases_and_jt_ebbs: &mut Vec<(EntryIndex, Ebb, Vec<Ebb>)>,
    ) {
        let mut was_branch = false;
        let ins_fallthrough_jump = |was_branch: bool, bx: &mut FunctionBuilder| {
            if was_branch {
                let ebb = bx.create_ebb();
                bx.ins().jump(ebb, &[]);
                bx.switch_to_block(ebb);
            }
        };
        for ContiguousCaseRange { first_index, ebbs } in contiguous_case_ranges.into_iter().rev() {
            match (ebbs.len(), first_index) {
                (1, 0) => {
                    ins_fallthrough_jump(was_branch, bx);
                    bx.ins().brz(val, ebbs[0], &[]);
                }
                (1, _) => {
                    ins_fallthrough_jump(was_branch, bx);
                    let is_good_val = bx.ins().icmp_imm(IntCC::Equal, val, first_index as i64);
                    bx.ins().brnz(is_good_val, ebbs[0], &[]);
                }
                (_, 0) => {
                    // if `first_index` is 0, then `icmp_imm uge val, first_index` is trivially true
                    let jt_ebb = bx.create_ebb();
                    bx.ins().jump(jt_ebb, &[]);
                    cases_and_jt_ebbs.push((first_index, jt_ebb, ebbs));
                    // `jump otherwise` below must not be hit, because the current block has been
                    // filled above. This is the last iteration anyway, as 0 is the smallest
                    // unsigned int, so just return here.
                    return;
                }
                (_, _) => {
                    ins_fallthrough_jump(was_branch, bx);
                    let jt_ebb = bx.create_ebb();
                    let is_good_val = bx.ins().icmp_imm(
                        IntCC::UnsignedGreaterThanOrEqual,
                        val,
                        first_index as i64,
                    );
                    bx.ins().brnz(is_good_val, jt_ebb, &[]);
                    cases_and_jt_ebbs.push((first_index, jt_ebb, ebbs));
                }
            }
            was_branch = true;
        }

        bx.ins().jump(otherwise, &[]);
    }

    /// For every item in `cases_and_jt_ebbs` this will create a jump table in the specified ebb.
    fn build_jump_tables(
        bx: &mut FunctionBuilder,
        val: Value,
        otherwise: Ebb,
        cases_and_jt_ebbs: Vec<(EntryIndex, Ebb, Vec<Ebb>)>,
    ) {
        for (first_index, jt_ebb, ebbs) in cases_and_jt_ebbs.into_iter().rev() {
            let mut jt_data = JumpTableData::new();
            for ebb in ebbs {
                jt_data.push_entry(ebb);
            }
            let jump_table = bx.create_jump_table(jt_data);

            bx.switch_to_block(jt_ebb);
            let discr = if first_index == 0 {
                val
            } else {
                bx.ins().iadd_imm(val, (first_index as i64).wrapping_neg())
            };
            bx.ins().br_table(discr, otherwise, jump_table);
        }
    }

    /// Build the switch
    ///
    /// # Arguments
    ///
    /// * The function builder to emit to
    /// * The value to switch on
    /// * The default ebb
    pub fn emit(self, bx: &mut FunctionBuilder, val: Value, otherwise: Ebb) {
        // FIXME icmp(_imm) doesn't have encodings for i8 and i16 on x86(_64) yet
        let val = match bx.func.dfg.value_type(val) {
            types::I8 | types::I16 => bx.ins().uextend(types::I32, val),
            _ => val,
        };

        let contiguous_case_ranges = self.collect_contiguous_case_ranges();
        let cases_and_jt_ebbs = Self::build_search_tree(bx, val, otherwise, contiguous_case_ranges);
        Self::build_jump_tables(bx, val, otherwise, cases_and_jt_ebbs);
    }
}

/// This represents a contiguous range of cases to switch on.
///
/// For example 10 => ebb1, 11 => ebb2, 12 => ebb7 will be represented as:
///
/// ```plain
/// ContiguousCaseRange {
///     first_index: 10,
///     ebbs: vec![Ebb::from_u32(1), Ebb::from_u32(2), Ebb::from_u32(7)]
/// }
/// ```
#[derive(Debug)]
struct ContiguousCaseRange {
    /// The entry index of the first case. Eg. 10 when the entry indexes are 10, 11, 12 and 13.
    first_index: EntryIndex,

    /// The ebbs to jump to sorted in ascending order of entry index.
    ebbs: Vec<Ebb>,
}

impl ContiguousCaseRange {
    fn new(first_index: EntryIndex) -> Self {
        Self {
            first_index,
            ebbs: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::FunctionBuilderContext;
    use cranelift_codegen::ir::Function;
    use std::string::ToString;

    macro_rules! setup {
        ($default:expr, [$($index:expr,)*]) => {{
            let mut func = Function::new();
            let mut func_ctx = FunctionBuilderContext::new();
            {
                let mut bx = FunctionBuilder::new(&mut func, &mut func_ctx);
                let ebb = bx.create_ebb();
                bx.switch_to_block(ebb);
                let val = bx.ins().iconst(types::I8, 0);
                let mut switch = Switch::new();
                $(
                    let ebb = bx.create_ebb();
                    switch.set_entry($index, ebb);
                )*
                switch.emit(&mut bx, val, Ebb::with_number($default).unwrap());
            }
            func
                .to_string()
                .trim_start_matches("function u0:0() fast {\n")
                .trim_end_matches("\n}\n")
                .to_string()
        }};
    }

    #[test]
    fn switch_zero() {
        let func = setup!(0, [0,]);
        assert_eq!(
            func,
            "ebb0:
    v0 = iconst.i8 0
    v1 = uextend.i32 v0
    brz v1, ebb1
    jump ebb0"
        );
    }

    #[test]
    fn switch_single() {
        let func = setup!(0, [1,]);
        assert_eq!(
            func,
            "ebb0:
    v0 = iconst.i8 0
    v1 = uextend.i32 v0
    v2 = icmp_imm eq v1, 1
    brnz v2, ebb1
    jump ebb0"
        );
    }

    #[test]
    fn switch_bool() {
        let func = setup!(0, [0, 1,]);
        assert_eq!(
            func,
            "    jt0 = jump_table [ebb1, ebb2]

ebb0:
    v0 = iconst.i8 0
    v1 = uextend.i32 v0
    jump ebb3

ebb3:
    br_table.i32 v1, ebb0, jt0"
        );
    }

    #[test]
    fn switch_two_gap() {
        let func = setup!(0, [0, 2,]);
        assert_eq!(
            func,
            "ebb0:
    v0 = iconst.i8 0
    v1 = uextend.i32 v0
    v2 = icmp_imm eq v1, 2
    brnz v2, ebb2
    jump ebb3

ebb3:
    brz.i32 v1, ebb1
    jump ebb0"
        );
    }

    #[test]
    fn switch_many() {
        let func = setup!(0, [0, 1, 5, 7, 10, 11, 12,]);
        assert_eq!(
            func,
            "    jt0 = jump_table [ebb1, ebb2]
    jt1 = jump_table [ebb5, ebb6, ebb7]

ebb0:
    v0 = iconst.i8 0
    v1 = uextend.i32 v0
    v2 = icmp_imm uge v1, 7
    brnz v2, ebb9
    jump ebb8

ebb9:
    v3 = icmp_imm.i32 uge v1, 10
    brnz v3, ebb10
    jump ebb11

ebb11:
    v4 = icmp_imm.i32 eq v1, 7
    brnz v4, ebb4
    jump ebb0

ebb8:
    v5 = icmp_imm.i32 eq v1, 5
    brnz v5, ebb3
    jump ebb12

ebb12:
    br_table.i32 v1, ebb0, jt0

ebb10:
    v6 = iadd_imm.i32 v1, -10
    br_table v6, ebb0, jt1"
        );
    }

    #[test]
    fn switch_min_index_value() {
        let func = setup!(0, [::core::i64::MIN as u64, 1,]);
        assert_eq!(
            func,
            "ebb0:
    v0 = iconst.i8 0
    v1 = uextend.i32 v0
    v2 = icmp_imm eq v1, 0x8000_0000_0000_0000
    brnz v2, ebb1
    jump ebb3

ebb3:
    v3 = icmp_imm.i32 eq v1, 1
    brnz v3, ebb2
    jump ebb0"
        );
    }

    #[test]
    fn switch_max_index_value() {
        let func = setup!(0, [::core::i64::MAX as u64, 1,]);
        assert_eq!(
            func,
            "ebb0:
    v0 = iconst.i8 0
    v1 = uextend.i32 v0
    v2 = icmp_imm eq v1, 0x7fff_ffff_ffff_ffff
    brnz v2, ebb1
    jump ebb3

ebb3:
    v3 = icmp_imm.i32 eq v1, 1
    brnz v3, ebb2
    jump ebb0"
        )
    }

    #[test]
    fn switch_optimal_codegen() {
        let func = setup!(0, [-1i64 as u64, 0, 1,]);
        assert_eq!(
            func,
            "    jt0 = jump_table [ebb2, ebb3]

ebb0:
    v0 = iconst.i8 0
    v1 = uextend.i32 v0
    v2 = icmp_imm eq v1, -1
    brnz v2, ebb1
    jump ebb4

ebb4:
    br_table.i32 v1, ebb0, jt0"
        );
    }
}
