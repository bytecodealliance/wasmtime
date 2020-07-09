//! Generate transformations to legalize instructions without encodings.
use crate::cdsl::ast::{Def, DefPool, Expr, VarPool};
use crate::cdsl::isa::TargetIsa;
use crate::cdsl::operands::Operand;
use crate::cdsl::type_inference::Constraint;
use crate::cdsl::typevar::{TypeSet, TypeVar};
use crate::cdsl::xform::{Transform, TransformGroup, TransformGroups};

use crate::error;
use crate::gen_inst::gen_typesets_table;
use crate::srcgen::Formatter;
use crate::unique_table::UniqueTable;

use std::collections::{HashMap, HashSet};
use std::iter::FromIterator;

/// Given a `Def` node, emit code that extracts all the instruction fields from
/// `pos.func.dfg[iref]`.
///
/// Create local variables named after the `Var` instances in `node`.
///
/// Also create a local variable named `predicate` with the value of the evaluated instruction
/// predicate, or `true` if the node has no predicate.
fn unwrap_inst(transform: &Transform, fmt: &mut Formatter) -> bool {
    let var_pool = &transform.var_pool;
    let def_pool = &transform.def_pool;

    let def = def_pool.get(transform.src);
    let apply = &def.apply;
    let inst = &apply.inst;
    let iform = &inst.format;

    fmt.comment(format!(
        "Unwrap fields from instruction format {}",
        def.to_comment_string(&transform.var_pool)
    ));

    // Extract the Var arguments.
    let arg_names = apply
        .args
        .iter()
        .enumerate()
        .filter(|(arg_num, _)| {
            // Variable args are specially handled after extracting args.
            !inst.operands_in[*arg_num].is_varargs()
        })
        .map(|(arg_num, arg)| match &arg {
            Expr::Var(var_index) => var_pool.get(*var_index).name.as_ref(),
            Expr::Literal(_) => {
                let n = inst.imm_opnums.iter().position(|&i| i == arg_num).unwrap();
                iform.imm_fields[n].member
            }
        })
        .collect::<Vec<_>>()
        .join(", ");

    // May we need "args" in the values consumed by predicates?
    let emit_args = iform.num_value_operands >= 1 || iform.has_value_list;

    // We need a tuple:
    // - if there's at least one value operand, then we emit a variable for the value, and the
    // value list as args.
    // - otherwise, if there's the count of immediate operands added to the presence of a value list exceeds one.
    let need_tuple = if iform.num_value_operands >= 1 {
        true
    } else {
        let mut imm_and_varargs = inst
            .operands_in
            .iter()
            .filter(|op| op.is_immediate_or_entityref())
            .count();
        if iform.has_value_list {
            imm_and_varargs += 1;
        }
        imm_and_varargs > 1
    };

    let maybe_args = if emit_args { ", args" } else { "" };
    let defined_values = format!("{}{}", arg_names, maybe_args);

    let tuple_or_value = if need_tuple {
        format!("({})", defined_values)
    } else {
        defined_values
    };

    fmtln!(
        fmt,
        "let {} = if let ir::InstructionData::{} {{",
        tuple_or_value,
        iform.name
    );

    fmt.indent(|fmt| {
        // Fields are encoded directly.
        for field in &iform.imm_fields {
            fmtln!(fmt, "{},", field.member);
        }

        if iform.has_value_list || iform.num_value_operands > 1 {
            fmt.line("ref args,");
        } else if iform.num_value_operands == 1 {
            fmt.line("arg,");
        }

        fmt.line("..");
        fmt.outdented_line("} = pos.func.dfg[inst] {");

        if iform.has_value_list {
            fmt.line("let args = args.as_slice(&pos.func.dfg.value_lists);");
        } else if iform.num_value_operands == 1 {
            fmt.line("let args = [arg];")
        }

        // Generate the values for the tuple.
        let emit_one_value =
            |fmt: &mut Formatter, needs_comma: bool, op_num: usize, op: &Operand| {
                let comma = if needs_comma { "," } else { "" };
                if op.is_immediate_or_entityref() {
                    let n = inst.imm_opnums.iter().position(|&i| i == op_num).unwrap();
                    fmtln!(fmt, "{}{}", iform.imm_fields[n].member, comma);
                } else if op.is_value() {
                    let n = inst.value_opnums.iter().position(|&i| i == op_num).unwrap();
                    fmtln!(fmt, "pos.func.dfg.resolve_aliases(args[{}]),", n);
                } else {
                    // This is a value list argument or a varargs.
                    assert!(iform.has_value_list || op.is_varargs());
                }
            };

        if need_tuple {
            fmt.line("(");
            fmt.indent(|fmt| {
                for (op_num, op) in inst.operands_in.iter().enumerate() {
                    let needs_comma = emit_args || op_num + 1 < inst.operands_in.len();
                    emit_one_value(fmt, needs_comma, op_num, op);
                }
                if emit_args {
                    fmt.line("args");
                }
            });
            fmt.line(")");
        } else {
            // Only one of these can be true at the same time, otherwise we'd need a tuple.
            emit_one_value(fmt, false, 0, &inst.operands_in[0]);
            if emit_args {
                fmt.line("args");
            }
        }

        fmt.outdented_line("} else {");
        fmt.line(r#"unreachable!("bad instruction format")"#);
    });
    fmtln!(fmt, "};");
    fmt.empty_line();

    assert_eq!(inst.operands_in.len(), apply.args.len());
    for (i, op) in inst.operands_in.iter().enumerate() {
        if op.is_varargs() {
            let name = &var_pool
                .get(apply.args[i].maybe_var().expect("vararg without name"))
                .name;
            let n = inst
                .imm_opnums
                .iter()
                .chain(inst.value_opnums.iter())
                .max()
                .copied()
                .unwrap_or(0);
            fmtln!(fmt, "let {} = &Vec::from(&args[{}..]);", name, n);
        }
    }

    for &op_num in &inst.value_opnums {
        let arg = &apply.args[op_num];
        if let Some(var_index) = arg.maybe_var() {
            let var = var_pool.get(var_index);
            if var.has_free_typevar() {
                fmtln!(
                    fmt,
                    "let typeof_{} = pos.func.dfg.value_type({});",
                    var.name,
                    var.name
                );
            }
        }
    }

    // If the definition creates results, detach the values and place them in locals.
    let mut replace_inst = false;
    if !def.defined_vars.is_empty() {
        if def.defined_vars
            == def_pool
                .get(var_pool.get(def.defined_vars[0]).dst_def.unwrap())
                .defined_vars
        {
            // Special case: The instruction replacing node defines the exact same values.
            fmt.comment(format!(
                "Results handled by {}.",
                def_pool
                    .get(var_pool.get(def.defined_vars[0]).dst_def.unwrap())
                    .to_comment_string(var_pool)
            ));

            fmt.line("let r = pos.func.dfg.inst_results(inst);");
            for (i, &var_index) in def.defined_vars.iter().enumerate() {
                let var = var_pool.get(var_index);
                fmtln!(fmt, "let {} = &r[{}];", var.name, i);
                fmtln!(
                    fmt,
                    "let typeof_{} = pos.func.dfg.value_type(*{});",
                    var.name,
                    var.name
                );
            }

            replace_inst = true;
        } else {
            // Boring case: Detach the result values, capture them in locals.
            for &var_index in &def.defined_vars {
                fmtln!(fmt, "let {};", var_pool.get(var_index).name);
            }

            fmt.line("{");
            fmt.indent(|fmt| {
                fmt.line("let r = pos.func.dfg.inst_results(inst);");
                for i in 0..def.defined_vars.len() {
                    let var = var_pool.get(def.defined_vars[i]);
                    fmtln!(fmt, "{} = r[{}];", var.name, i);
                }
            });
            fmt.line("}");

            for &var_index in &def.defined_vars {
                let var = var_pool.get(var_index);
                if var.has_free_typevar() {
                    fmtln!(
                        fmt,
                        "let typeof_{} = pos.func.dfg.value_type({});",
                        var.name,
                        var.name
                    );
                }
            }
        }
    }
    replace_inst
}

fn build_derived_expr(tv: &TypeVar) -> String {
    let base = match &tv.base {
        Some(base) => base,
        None => {
            assert!(tv.name.starts_with("typeof_"));
            return format!("Some({})", tv.name);
        }
    };
    let base_expr = build_derived_expr(&base.type_var);
    format!(
        "{}.map(|t: crate::ir::Type| t.{}())",
        base_expr,
        base.derived_func.name()
    )
}

/// Emit rust code for the given check.
///
/// The emitted code is a statement redefining the `predicate` variable like this:
///     let predicate = predicate && ...
fn emit_runtime_typecheck<'a>(
    constraint: &'a Constraint,
    type_sets: &mut UniqueTable<'a, TypeSet>,
    fmt: &mut Formatter,
) {
    match constraint {
        Constraint::InTypeset(tv, ts) => {
            let ts_index = type_sets.add(&ts);
            fmt.comment(format!(
                "{} must belong to {:?}",
                tv.name,
                type_sets.get(ts_index)
            ));
            fmtln!(
                fmt,
                "let predicate = predicate && TYPE_SETS[{}].contains({});",
                ts_index,
                tv.name
            );
        }
        Constraint::Eq(tv1, tv2) => {
            fmtln!(
                fmt,
                "let predicate = predicate && match ({}, {}) {{",
                build_derived_expr(tv1),
                build_derived_expr(tv2)
            );
            fmt.indent(|fmt| {
                fmt.line("(Some(a), Some(b)) => a == b,");
                fmt.comment("On overflow, constraint doesn\'t apply");
                fmt.line("_ => false,");
            });
            fmtln!(fmt, "};");
        }
        Constraint::WiderOrEq(tv1, tv2) => {
            fmtln!(
                fmt,
                "let predicate = predicate && match ({}, {}) {{",
                build_derived_expr(tv1),
                build_derived_expr(tv2)
            );
            fmt.indent(|fmt| {
                fmt.line("(Some(a), Some(b)) => a.wider_or_equal(b),");
                fmt.comment("On overflow, constraint doesn\'t apply");
                fmt.line("_ => false,");
            });
            fmtln!(fmt, "};");
        }
    }
}

/// Determine if `node` represents one of the value splitting instructions: `isplit` or `vsplit.
/// These instructions are lowered specially by the `legalize::split` module.
fn is_value_split(def: &Def) -> bool {
    let name = &def.apply.inst.name;
    name == "isplit" || name == "vsplit"
}

fn emit_dst_inst(def: &Def, def_pool: &DefPool, var_pool: &VarPool, fmt: &mut Formatter) {
    let defined_vars = {
        let vars = def
            .defined_vars
            .iter()
            .map(|&var_index| var_pool.get(var_index).name.as_ref())
            .collect::<Vec<&str>>();
        if vars.len() == 1 {
            vars[0].to_string()
        } else {
            format!("({})", vars.join(", "))
        }
    };

    if is_value_split(def) {
        // Split instructions are not emitted with the builder, but by calling special functions in
        // the `legalizer::split` module. These functions will eliminate concat-split patterns.
        fmt.line("let curpos = pos.position();");
        fmt.line("let srcloc = pos.srcloc();");
        fmtln!(
            fmt,
            "let {} = split::{}(pos.func, cfg, curpos, srcloc, {});",
            defined_vars,
            def.apply.inst.snake_name(),
            def.apply.args[0].to_rust_code(var_pool)
        );
        return;
    }

    if def.defined_vars.is_empty() {
        // This node doesn't define any values, so just insert the new instruction.
        fmtln!(
            fmt,
            "pos.ins().{};",
            def.apply.rust_builder(&def.defined_vars, var_pool)
        );
        return;
    }

    if let Some(src_def0) = var_pool.get(def.defined_vars[0]).src_def {
        if def.defined_vars == def_pool.get(src_def0).defined_vars {
            // The replacement instruction defines the exact same values as the source pattern.
            // Unwrapping would have left the results intact.  Replace the whole instruction.
            fmtln!(
                fmt,
                "let {} = pos.func.dfg.replace(inst).{};",
                defined_vars,
                def.apply.rust_builder(&def.defined_vars, var_pool)
            );

            // We need to bump the cursor so following instructions are inserted *after* the
            // replaced instruction.
            fmt.line("if pos.current_inst() == Some(inst) {");
            fmt.indent(|fmt| {
                fmt.line("pos.next_inst();");
            });
            fmt.line("}");
            return;
        }
    }

    // Insert a new instruction.
    let mut builder = format!("let {} = pos.ins()", defined_vars);

    if def.defined_vars.len() == 1 && var_pool.get(def.defined_vars[0]).is_output() {
        // Reuse the single source result value.
        builder = format!(
            "{}.with_result({})",
            builder,
            var_pool.get(def.defined_vars[0]).to_rust_code()
        );
    } else if def
        .defined_vars
        .iter()
        .any(|&var_index| var_pool.get(var_index).is_output())
    {
        // There are more than one output values that can be reused.
        let array = def
            .defined_vars
            .iter()
            .map(|&var_index| {
                let var = var_pool.get(var_index);
                if var.is_output() {
                    format!("Some({})", var.name)
                } else {
                    "None".into()
                }
            })
            .collect::<Vec<_>>()
            .join(", ");
        builder = format!("{}.with_results([{}])", builder, array);
    }

    fmtln!(
        fmt,
        "{}.{};",
        builder,
        def.apply.rust_builder(&def.defined_vars, var_pool)
    );
}

/// Emit code for `transform`, assuming that the opcode of transform's root instruction
/// has already been matched.
///
/// `inst: Inst` is the variable to be replaced. It is pointed to by `pos: Cursor`.
/// `dfg: DataFlowGraph` is available and mutable.
fn gen_transform<'a>(
    replace_inst: bool,
    transform: &'a Transform,
    type_sets: &mut UniqueTable<'a, TypeSet>,
    fmt: &mut Formatter,
) {
    // Evaluate the instruction predicate if any.
    let apply = &transform.def_pool.get(transform.src).apply;

    let inst_predicate = apply
        .inst_predicate_with_ctrl_typevar(&transform.var_pool)
        .rust_predicate("pos.func");

    let has_extra_constraints = !transform.type_env.constraints.is_empty();
    if has_extra_constraints {
        // Extra constraints rely on the predicate being a variable that we can rebind as we add
        // more constraint predicates.
        if let Some(pred) = &inst_predicate {
            fmt.multi_line(&format!("let predicate = {};", pred));
        } else {
            fmt.line("let predicate = true;");
        }
    }

    // Emit any runtime checks; these will rebind `predicate` emitted right above.
    for constraint in &transform.type_env.constraints {
        emit_runtime_typecheck(constraint, type_sets, fmt);
    }

    let do_expand = |fmt: &mut Formatter| {
        // Emit any constants that must be created before use.
        for (name, value) in transform.const_pool.iter() {
            fmtln!(
                fmt,
                "let {} = pos.func.dfg.constants.insert(vec!{:?}.into());",
                name,
                value
            );
        }

        // If we are adding some blocks, we need to recall the original block, such that we can
        // recompute it.
        if !transform.block_pool.is_empty() {
            fmt.line("let orig_block = pos.current_block().unwrap();");
        }

        // If we're going to delete `inst`, we need to detach its results first so they can be
        // reattached during pattern expansion.
        if !replace_inst {
            fmt.line("pos.func.dfg.clear_results(inst);");
        }

        // Emit new block creation.
        for block in &transform.block_pool {
            let var = transform.var_pool.get(block.name);
            fmtln!(fmt, "let {} = pos.func.dfg.make_block();", var.name);
        }

        // Emit the destination pattern.
        for &def_index in &transform.dst {
            if let Some(block) = transform.block_pool.get(def_index) {
                let var = transform.var_pool.get(block.name);
                fmtln!(fmt, "pos.insert_block({});", var.name);
            }
            emit_dst_inst(
                transform.def_pool.get(def_index),
                &transform.def_pool,
                &transform.var_pool,
                fmt,
            );
        }

        // Insert a new block after the last instruction, if needed.
        let def_next_index = transform.def_pool.next_index();
        if let Some(block) = transform.block_pool.get(def_next_index) {
            let var = transform.var_pool.get(block.name);
            fmtln!(fmt, "pos.insert_block({});", var.name);
        }

        // Delete the original instruction if we didn't have an opportunity to replace it.
        if !replace_inst {
            fmt.line("let removed = pos.remove_inst();");
            fmt.line("debug_assert_eq!(removed, inst);");
        }

        if transform.block_pool.is_empty() {
            if transform.def_pool.get(transform.src).apply.inst.is_branch {
                // A branch might have been legalized into multiple branches, so we need to recompute
                // the cfg.
                fmt.line("cfg.recompute_block(pos.func, pos.current_block().unwrap());");
            }
        } else {
            // Update CFG for the new blocks.
            fmt.line("cfg.recompute_block(pos.func, orig_block);");
            for block in &transform.block_pool {
                let var = transform.var_pool.get(block.name);
                fmtln!(fmt, "cfg.recompute_block(pos.func, {});", var.name);
            }
        }

        fmt.line("return true;");
    };

    // Guard the actual expansion by `predicate`.
    if has_extra_constraints {
        fmt.line("if predicate {");
        fmt.indent(|fmt| {
            do_expand(fmt);
        });
        fmt.line("}");
    } else if let Some(pred) = &inst_predicate {
        fmt.multi_line(&format!("if {} {{", pred));
        fmt.indent(|fmt| {
            do_expand(fmt);
        });
        fmt.line("}");
    } else {
        // Unconditional transform (there was no predicate), just emit it.
        do_expand(fmt);
    }
}

fn gen_transform_group<'a>(
    group: &'a TransformGroup,
    transform_groups: &TransformGroups,
    type_sets: &mut UniqueTable<'a, TypeSet>,
    fmt: &mut Formatter,
) {
    fmt.doc_comment(group.doc);
    fmt.line("#[allow(unused_variables,unused_assignments,unused_imports,non_snake_case)]");

    // Function arguments.
    fmtln!(fmt, "pub fn {}(", group.name);
    fmt.indent(|fmt| {
        fmt.line("inst: crate::ir::Inst,");
        fmt.line("func: &mut crate::ir::Function,");
        fmt.line("cfg: &mut crate::flowgraph::ControlFlowGraph,");
        fmt.line("isa: &dyn crate::isa::TargetIsa,");
    });
    fmtln!(fmt, ") -> bool {");

    // Function body.
    fmt.indent(|fmt| {
        fmt.line("use crate::ir::InstBuilder;");
        fmt.line("use crate::cursor::{Cursor, FuncCursor};");
        fmt.line("let mut pos = FuncCursor::new(func).at_inst(inst);");
        fmt.line("pos.use_srcloc(inst);");

        // Group the transforms by opcode so we can generate a big switch.
        // Preserve ordering.
        let mut inst_to_transforms = HashMap::new();
        for transform in &group.transforms {
            let def_index = transform.src;
            let inst = &transform.def_pool.get(def_index).apply.inst;
            inst_to_transforms
                .entry(inst.camel_name.clone())
                .or_insert_with(Vec::new)
                .push(transform);
        }

        let mut sorted_inst_names = Vec::from_iter(inst_to_transforms.keys());
        sorted_inst_names.sort();

        fmt.line("{");
        fmt.indent(|fmt| {
            fmt.line("match pos.func.dfg[inst].opcode() {");
            fmt.indent(|fmt| {
                for camel_name in sorted_inst_names {
                    fmtln!(fmt, "ir::Opcode::{} => {{", camel_name);
                    fmt.indent(|fmt| {
                        let transforms = inst_to_transforms.get(camel_name).unwrap();

                        // Unwrap the source instruction, create local variables for the input variables.
                        let replace_inst = unwrap_inst(&transforms[0], fmt);
                        fmt.empty_line();

                        for (i, transform) in transforms.iter().enumerate() {
                            if i > 0 {
                                fmt.empty_line();
                            }
                            gen_transform(replace_inst, transform, type_sets, fmt);
                        }
                    });
                    fmtln!(fmt, "}");
                    fmt.empty_line();
                }

                // Emit the custom transforms. The Rust compiler will complain about any overlap with
                // the normal transforms.
                let mut sorted_custom_legalizes = Vec::from_iter(&group.custom_legalizes);
                sorted_custom_legalizes.sort();
                for (inst_camel_name, func_name) in sorted_custom_legalizes {
                    fmtln!(fmt, "ir::Opcode::{} => {{", inst_camel_name);
                    fmt.indent(|fmt| {
                        fmtln!(fmt, "{}(inst, func, cfg, isa);", func_name);
                        fmt.line("return true;");
                    });
                    fmtln!(fmt, "}");
                    fmt.empty_line();
                }

                // We'll assume there are uncovered opcodes.
                fmt.line("_ => {},");
            });
            fmt.line("}");
        });
        fmt.line("}");

        // If we fall through, nothing was expanded; call the chain if any.
        match &group.chain_with {
            Some(group_id) => fmtln!(
                fmt,
                "{}(inst, func, cfg, isa)",
                transform_groups.get(*group_id).rust_name()
            ),
            None => fmt.line("false"),
        };
    });
    fmtln!(fmt, "}");
    fmt.empty_line();
}

/// Generate legalization functions for `isa` and add any shared `TransformGroup`s
/// encountered to `shared_groups`.
///
/// Generate `TYPE_SETS` and `LEGALIZE_ACTIONS` tables.
fn gen_isa(
    isa: &TargetIsa,
    transform_groups: &TransformGroups,
    shared_group_names: &mut HashSet<&'static str>,
    fmt: &mut Formatter,
) {
    let mut type_sets = UniqueTable::new();
    for group_index in isa.transitive_transform_groups(transform_groups) {
        let group = transform_groups.get(group_index);
        match group.isa_name {
            Some(isa_name) => {
                assert!(
                    isa_name == isa.name,
                    "ISA-specific legalizations must be used by the same ISA"
                );
                gen_transform_group(group, transform_groups, &mut type_sets, fmt);
            }
            None => {
                shared_group_names.insert(group.name);
            }
        }
    }

    gen_typesets_table(&type_sets, fmt);

    let direct_groups = isa.direct_transform_groups();
    fmtln!(
        fmt,
        "pub static LEGALIZE_ACTIONS: [isa::Legalize; {}] = [",
        direct_groups.len()
    );
    fmt.indent(|fmt| {
        for &group_index in direct_groups {
            fmtln!(fmt, "{},", transform_groups.get(group_index).rust_name());
        }
    });
    fmtln!(fmt, "];");
}

/// Generate the legalizer files.
pub(crate) fn generate(
    isas: &[TargetIsa],
    transform_groups: &TransformGroups,
    extra_legalization_groups: &[&'static str],
    filename_prefix: &str,
    out_dir: &str,
) -> Result<(), error::Error> {
    let mut shared_group_names = HashSet::new();

    for isa in isas {
        let mut fmt = Formatter::new();
        gen_isa(isa, transform_groups, &mut shared_group_names, &mut fmt);
        fmt.update_file(format!("{}-{}.rs", filename_prefix, isa.name), out_dir)?;
    }

    // Add extra legalization groups that were explicitly requested.
    for group in extra_legalization_groups {
        shared_group_names.insert(group);
    }

    // Generate shared legalize groups.
    let mut fmt = Formatter::new();
    // Generate shared legalize groups.
    let mut type_sets = UniqueTable::new();
    let mut sorted_shared_group_names = Vec::from_iter(shared_group_names);
    sorted_shared_group_names.sort();
    for group_name in &sorted_shared_group_names {
        let group = transform_groups.by_name(group_name);
        gen_transform_group(group, transform_groups, &mut type_sets, &mut fmt);
    }
    gen_typesets_table(&type_sets, &mut fmt);
    fmt.update_file(format!("{}r.rs", filename_prefix), out_dir)?;

    Ok(())
}
