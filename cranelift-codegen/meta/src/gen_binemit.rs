//! Generate binary emission code for each ISA.

use cranelift_entity::EntityRef;

use crate::error;
use crate::srcgen::Formatter;

use crate::cdsl::formats::FormatRegistry;
use crate::cdsl::recipes::{EncodingRecipe, OperandConstraint, Recipes};

/// Generate code to handle a single recipe.
///
/// - Unpack the instruction data, knowing the format.
/// - Determine register locations for operands with register constraints.
/// - Determine stack slot locations for operands with stack constraints.
/// - Call hand-written code for the actual emission.
fn gen_recipe(formats: &FormatRegistry, recipe: &EncodingRecipe, fmt: &mut Formatter) {
    let inst_format = formats.get(recipe.format);
    let num_value_ops = inst_format.num_value_operands;

    let want_args = recipe.operands_in.iter().any(|c| match c {
        OperandConstraint::RegClass(_) | OperandConstraint::Stack(_) => true,
        OperandConstraint::FixedReg(_) | OperandConstraint::TiedInput(_) => false,
    });
    assert!(!want_args || num_value_ops > 0 || inst_format.has_value_list);

    let want_outs = recipe.operands_out.iter().any(|c| match c {
        OperandConstraint::RegClass(_) | OperandConstraint::Stack(_) => true,
        OperandConstraint::FixedReg(_) | OperandConstraint::TiedInput(_) => false,
    });

    let is_regmove = ["RegMove", "RegSpill", "RegFill"].contains(&inst_format.name);

    // Unpack the instruction data.
    fmtln!(fmt, "if let &InstructionData::{} {{", inst_format.name);
    fmt.indent(|fmt| {
        fmt.line("opcode,");
        for f in &inst_format.imm_fields {
            fmtln!(fmt, "{},", f.member);
        }
        if want_args {
            if inst_format.has_value_list || num_value_ops > 1 {
                fmt.line("ref args,");
            } else {
                fmt.line("arg,");
            }
        }
        fmt.line("..");

        fmt.outdented_line("} = inst_data {");

        // Pass recipe arguments in this order: inputs, imm_fields, outputs.
        let mut args = String::new();

        if want_args && !is_regmove {
            if inst_format.has_value_list {
                fmt.line("let args = args.as_slice(&func.dfg.value_lists);");
            } else if num_value_ops == 1 {
                fmt.line("let args = [arg];");
            }
            args += &unwrap_values(&recipe.operands_in, "in", "args", fmt);
        }

        for f in &inst_format.imm_fields {
            args += &format!(", {}", f.member);
        }

        // Unwrap interesting output arguments.
        if want_outs {
            if recipe.operands_out.len() == 1 {
                fmt.line("let results = [func.dfg.first_result(inst)];")
            } else {
                fmt.line("let results = func.dfg.inst_results(inst);");
            }
            args += &unwrap_values(&recipe.operands_out, "out", "results", fmt);
        }

        // Optimization: Only update the register diversion tracker for regmove instructions.
        if is_regmove {
            fmt.line("divert.apply(inst_data);")
        }

        match &recipe.emit {
            Some(emit) => {
                fmt.multi_line(emit);
                fmt.line("return;");
            }
            None => {
                fmtln!(
                    fmt,
                    "return recipe_{}(func, inst, sink, bits{});",
                    recipe.name.to_lowercase(),
                    args
                );
            }
        }
    });
    fmt.line("}");
}

/// Emit code that unwraps values living in registers or stack slots.
///
/// :param args: Input or output constraints.
/// :param prefix: Prefix to be used for the generated local variables.
/// :param values: Name of slice containing the values to be unwrapped.
/// :returns: Comma separated list of the generated variables
fn unwrap_values(
    args: &[OperandConstraint],
    prefix: &str,
    values_slice: &str,
    fmt: &mut Formatter,
) -> String {
    let mut varlist = String::new();
    for (i, cst) in args.iter().enumerate() {
        match cst {
            OperandConstraint::RegClass(_reg_class) => {
                let v = format!("{}_reg{}", prefix, i);
                varlist += &format!(", {}", v);
                fmtln!(
                    fmt,
                    "let {} = divert.reg({}[{}], &func.locations);",
                    v,
                    values_slice,
                    i
                );
            }
            OperandConstraint::Stack(stack) => {
                let v = format!("{}_stk{}", prefix, i);
                varlist += &format!(", {}", v);
                fmtln!(fmt, "let {} = StackRef::masked(", v);
                fmt.indent(|fmt| {
                    fmtln!(
                        fmt,
                        "divert.stack({}[{}], &func.locations),",
                        values_slice,
                        i
                    );
                    fmt.line(format!("{}, ", stack.stack_base_mask()));
                    fmt.line("&func.stack_slots,");
                });
                fmt.line(").unwrap();");
            }
            _ => {}
        }
    }
    varlist
}

fn gen_isa(formats: &FormatRegistry, isa_name: &str, recipes: &Recipes, fmt: &mut Formatter) {
    fmt.doc_comment(format!(
        "Emit binary machine code for `inst` for the {} ISA.",
        isa_name
    ));

    if recipes.is_empty() {
        fmt.line("pub fn emit_inst<CS: CodeSink + ?Sized>(");
        fmt.indent(|fmt| {
            fmt.line("func: &Function,");
            fmt.line("inst: Inst,");
            fmt.line("_divert: &mut RegDiversions,");
            fmt.line("_sink: &mut CS,");
        });
        fmt.line(") {");
        fmt.indent(|fmt| {
            // No encoding recipes: Emit a stub.
            fmt.line("bad_encoding(func, inst)");
        });
        fmt.line("}");
        return;
    }

    fmt.line("#[allow(unused_variables, unreachable_code)]");
    fmt.line("pub fn emit_inst<CS: CodeSink + ?Sized>(");
    fmt.indent(|fmt| {
        fmt.line("func: &Function,");
        fmt.line("inst: Inst,");
        fmt.line("divert: &mut RegDiversions,");
        fmt.line("sink: &mut CS,");
    });

    fmt.line(") {");
    fmt.indent(|fmt| {
        fmt.line("let encoding = func.encodings[inst];");
        fmt.line("let bits = encoding.bits();");
        fmt.line("let inst_data = &func.dfg[inst];");
        fmt.line("match encoding.recipe() {");
        fmt.indent(|fmt| {
            for (i, recipe) in recipes.iter() {
                fmt.comment(format!("Recipe {}", recipe.name));
                fmtln!(fmt, "{} => {{", i.index());
                fmt.indent(|fmt| {
                    gen_recipe(formats, recipe, fmt);
                });
                fmt.line("}");
            }
            fmt.line("_ => {},");
        });
        fmt.line("}");

        // Allow for unencoded ghost instructions. The verifier will check details.
        fmt.line("if encoding.is_legal() {");
        fmt.indent(|fmt| {
            fmt.line("bad_encoding(func, inst);");
        });
        fmt.line("}");
    });
    fmt.line("}");
}

pub fn generate(
    formats: &FormatRegistry,
    isa_name: &str,
    recipes: &Recipes,
    binemit_filename: &str,
    out_dir: &str,
) -> Result<(), error::Error> {
    let mut fmt = Formatter::new();
    gen_isa(formats, isa_name, recipes, &mut fmt);
    fmt.update_file(binemit_filename, out_dir)?;
    Ok(())
}
