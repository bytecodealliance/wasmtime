use crate::cdsl::ast::{Def, DefPool, VarPool};
use crate::cdsl::formats::FormatRegistry;
use crate::cdsl::isa::TargetIsa;
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
fn unwrap_inst(
    transform: &Transform,
    format_registry: &FormatRegistry,
    fmt: &mut Formatter,
) -> bool {
    let var_pool = &transform.var_pool;
    let def_pool = &transform.def_pool;

    let def = def_pool.get(transform.src);
    let apply = &def.apply;
    let inst = &apply.inst;
    let iform = format_registry.get(inst.format);

    fmt.comment(format!(
        "Unwrap {}",
        def.to_comment_string(&transform.var_pool)
    ));

    // Extract the Var arguments.
    let arg_names = apply
        .args
        .iter()
        .map(|arg| match arg.maybe_var() {
            Some(var_index) => var_pool.get(var_index).name,
            None => "_",
        })
        .collect::<Vec<_>>()
        .join(", ");

    fmtln!(
        fmt,
        "let ({}, predicate) = if let crate::ir::InstructionData::{} {{",
        arg_names,
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
        fmt.line("let func = &pos.func;");

        if iform.has_value_list {
            fmt.line("let args = args.as_slice(&func.dfg.value_lists);");
        } else if iform.num_value_operands == 1 {
            fmt.line("let args = [arg];")
        }

        // Generate the values for the tuple.
        fmt.line("(");
        fmt.indent(|fmt| {
            for (op_num, op) in inst.operands_in.iter().enumerate() {
                if op.is_immediate() {
                    let n = inst.imm_opnums.iter().position(|&i| i == op_num).unwrap();
                    fmtln!(fmt, "{},", iform.imm_fields[n].member);
                } else if op.is_value() {
                    let n = inst.value_opnums.iter().position(|&i| i == op_num).unwrap();
                    fmtln!(fmt, "func.dfg.resolve_aliases(args[{}]),", n);
                } else if op.is_varargs() {
                    let n = inst.imm_opnums.iter().chain(inst.value_opnums.iter()).max().map(|n| n + 1).unwrap_or(0);
                    // We need to create a `Vec` here, as using a slice would result in a borrowck
                    // error later on.
                    fmtln!(fmt, "\
                        args.iter().skip({}).map(|&arg| func.dfg.resolve_aliases(arg)).collect::<Vec<_>>(),\
                    ", n);
                }
            }

            // Evaluate the instruction predicate if any.
            fmt.multi_line(
                &apply
                    .inst_predicate_with_ctrl_typevar(format_registry, var_pool)
                    .rust_predicate(),
            );
        });
        fmt.line(")");

        fmt.outdented_line("} else {");
        fmt.line(r#"unreachable!("bad instruction format")"#);
    });
    fmtln!(fmt, "};");

    assert_eq!(inst.operands_in.len(), apply.args.len());
    for (i, op) in inst.operands_in.iter().enumerate() {
        if op.is_varargs() {
            let name = var_pool
                .get(apply.args[i].maybe_var().expect("vararg without name"))
                .name;

            // Above name is set to an `Vec` representing the varargs. However it is expected to be
            // `&[Value]` below, so we borrow it.
            fmtln!(fmt, "let {} = &{};", name, name);
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
    if def.defined_vars.len() > 0 {
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
fn emit_runtime_typecheck<'a, 'b>(
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
            .map(|&var_index| var_pool.get(var_index).name)
            .collect::<Vec<_>>();
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
    transform: &'a Transform,
    format_registry: &FormatRegistry,
    type_sets: &mut UniqueTable<'a, TypeSet>,
    fmt: &mut Formatter,
) {
    // Unwrap the source instruction, create local variables for the input variables.
    let replace_inst = unwrap_inst(&transform, format_registry, fmt);

    // Emit any runtime checks; these will rebind `predicate` emitted by unwrap_inst().
    for constraint in &transform.type_env.constraints {
        emit_runtime_typecheck(constraint, type_sets, fmt);
    }

    // Guard the actual expansion by `predicate`.
    fmt.line("if predicate {");
    fmt.indent(|fmt| {
        // If we're going to delete `inst`, we need to detach its results first so they can be
        // reattached during pattern expansion.
        if !replace_inst {
            fmt.line("pos.func.dfg.clear_results(inst);");
        }

        // Emit the destination pattern.
        for &def_index in &transform.dst {
            emit_dst_inst(
                transform.def_pool.get(def_index),
                &transform.def_pool,
                &transform.var_pool,
                fmt,
            );
        }

        // Delete the original instruction if we didn't have an opportunity to replace it.
        if !replace_inst {
            fmt.line("let removed = pos.remove_inst();");
            fmt.line("debug_assert_eq!(removed, inst);");
        }

        if transform.def_pool.get(transform.src).apply.inst.is_branch {
            // A branch might have been legalized into multiple branches, so we need to recompute
            // the cfg.
            fmt.line("cfg.recompute_ebb(pos.func, pos.current_ebb().unwrap());");
        }

        fmt.line("return true;");
    });
    fmt.line("}");
}

fn gen_transform_group<'a>(
    group: &'a TransformGroup,
    format_registry: &FormatRegistry,
    transform_groups: &TransformGroups,
    type_sets: &mut UniqueTable<'a, TypeSet>,
    fmt: &mut Formatter,
) {
    fmt.doc_comment(group.doc);
    fmt.line("#[allow(unused_variables,unused_assignments,non_snake_case)]");

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
                .or_insert(Vec::new())
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
                        for transform in inst_to_transforms.get(camel_name).unwrap() {
                            gen_transform(transform, format_registry, type_sets, fmt);
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
                        fmtln!(fmt, "{}(inst, pos.func, cfg, isa);", func_name);
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
                "{}(inst, pos.func, cfg, isa)",
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
    format_registry: &FormatRegistry,
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
                gen_transform_group(
                    group,
                    format_registry,
                    transform_groups,
                    &mut type_sets,
                    fmt,
                );
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
pub fn generate(
    isas: &Vec<TargetIsa>,
    format_registry: &FormatRegistry,
    transform_groups: &TransformGroups,
    filename_prefix: &str,
    out_dir: &str,
) -> Result<(), error::Error> {
    let mut shared_group_names = HashSet::new();

    for isa in isas {
        let mut fmt = Formatter::new();
        gen_isa(
            isa,
            format_registry,
            transform_groups,
            &mut shared_group_names,
            &mut fmt,
        );
        fmt.update_file(format!("{}-{}.rs", filename_prefix, isa.name), out_dir)?;
    }

    // Generate shared legalize groups.
    let mut fmt = Formatter::new();
    let mut type_sets = UniqueTable::new();
    let mut sorted_shared_group_names = Vec::from_iter(shared_group_names);
    sorted_shared_group_names.sort();
    for group_name in &sorted_shared_group_names {
        let group = transform_groups.by_name(group_name);
        gen_transform_group(
            group,
            format_registry,
            transform_groups,
            &mut type_sets,
            &mut fmt,
        );
    }
    gen_typesets_table(&type_sets, &mut fmt);
    fmt.update_file(format!("{}r.rs", filename_prefix), out_dir)?;

    Ok(())
}
