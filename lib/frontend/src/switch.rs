use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::*;
use frontend::FunctionBuilder;
use std::collections::HashMap;
use std::vec::Vec;

type EntryIndex = u64;

/// Unlike with `br_table`, `Switch` cases may be sparse or non-0-based.
/// They emit efficient code using branches, jump tables, or a combination of both.
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

    fn build_search_branches(
        bx: &mut FunctionBuilder,
        val: Value,
        otherwise: Ebb,
        contiguous_case_ranges: Vec<ContiguousCaseRange>,
        cases_and_jt_ebbs: &mut Vec<(EntryIndex, Ebb, Vec<Ebb>)>,
    ) {
        for ContiguousCaseRange { first_index, ebbs } in contiguous_case_ranges.into_iter().rev() {
            if ebbs.len() == 1 {
                let is_good_val = bx.ins().icmp_imm(IntCC::Equal, val, first_index as i64);
                bx.ins().brnz(is_good_val, ebbs[0], &[]);
            } else {
                let jt_ebb = bx.create_ebb();
                let is_good_val =
                    bx.ins()
                        .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, val, first_index as i64);
                bx.ins().brnz(is_good_val, jt_ebb, &[]);
                cases_and_jt_ebbs.push((first_index, jt_ebb, ebbs));
            }
        }

        bx.ins().jump(otherwise, &[]);
    }

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
            let discr = bx.ins().iadd_imm(val, (first_index as i64).wrapping_neg());
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

#[derive(Debug)]
struct ContiguousCaseRange {
    first_index: EntryIndex,
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
    use cranelift_codegen::ir::Function;
    use frontend::FunctionBuilderContext;
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
                .trim_left_matches("function u0:0() fast {\n")
                .trim_right_matches("\n}\n")
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
    v2 = icmp_imm eq v1, 0
    brnz v2, ebb1
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
    v2 = icmp_imm uge v1, 0
    brnz v2, ebb3
    jump ebb0

ebb3:
    v3 = iadd_imm.i32 v1, 0
    br_table v3, ebb0, jt0"
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
    v3 = icmp_imm eq v1, 0
    brnz v3, ebb1
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
    v4 = icmp_imm.i32 eq v1, 7
    brnz v4, ebb4
    jump ebb0

ebb8:
    v5 = icmp_imm.i32 eq v1, 5
    brnz v5, ebb3
    v6 = icmp_imm.i32 uge v1, 0
    brnz v6, ebb11
    jump ebb0

ebb11:
    v7 = iadd_imm.i32 v1, 0
    br_table v7, ebb0, jt0

ebb10:
    v8 = iadd_imm.i32 v1, -10
    br_table v8, ebb0, jt1"
        );
    }

    #[test]
    fn switch_min_index_value() {
        let func = setup!(0, [::std::i64::MIN as u64, 1,]);
        assert_eq!(
            func,
            "ebb0:
    v0 = iconst.i8 0
    v1 = uextend.i32 v0
    v2 = icmp_imm eq v1, 0x8000_0000_0000_0000
    brnz v2, ebb1
    v3 = icmp_imm eq v1, 1
    brnz v3, ebb2
    jump ebb0"
        );
    }

    #[test]
    fn switch_max_index_value() {
        let func = setup!(0, [::std::i64::MAX as u64, 1,]);
        assert_eq!(
            func,
            "ebb0:
    v0 = iconst.i8 0
    v1 = uextend.i32 v0
    v2 = icmp_imm eq v1, 0x7fff_ffff_ffff_ffff
    brnz v2, ebb1
    v3 = icmp_imm eq v1, 1
    brnz v3, ebb2
    jump ebb0"
        )
    }
}
