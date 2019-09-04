use std::fmt;

use cranelift_entity::EntityRef;

use crate::cdsl::camel_case;
use crate::cdsl::formats::{FormatRegistry, InstructionFormat};
use crate::cdsl::instructions::{AllInstructions, Instruction};
use crate::cdsl::operands::Operand;
use crate::cdsl::typevar::{TypeSet, TypeVar};

use crate::shared::Definitions as SharedDefinitions;

use crate::constant_hash;
use crate::error;
use crate::srcgen::{Formatter, Match};
use crate::unique_table::{UniqueSeqTable, UniqueTable};

// TypeSet indexes are encoded in 8 bits, with `0xff` reserved.
const TYPESET_LIMIT: usize = 0xff;

/// Generate an instruction format enumeration.
fn gen_formats(registry: &FormatRegistry, fmt: &mut Formatter) {
    fmt.doc_comment(
        r#"
        An instruction format

        Every opcode has a corresponding instruction format
        which is represented by both the `InstructionFormat`
        and the `InstructionData` enums.
    "#,
    );
    fmt.line("#[derive(Copy, Clone, PartialEq, Eq, Debug)]");
    fmt.line("pub enum InstructionFormat {");
    fmt.indent(|fmt| {
        for format in registry.iter() {
            fmt.doc_comment(format.to_string());
            fmtln!(fmt, "{},", format.name);
        }
    });
    fmt.line("}");
    fmt.empty_line();

    // Emit a From<InstructionData> which also serves to verify that
    // InstructionFormat and InstructionData are in sync.
    fmt.line("impl<'a> From<&'a InstructionData> for InstructionFormat {");
    fmt.indent(|fmt| {
        fmt.line("fn from(inst: &'a InstructionData) -> Self {");
        fmt.indent(|fmt| {
            let mut m = Match::new("*inst");
            for format in registry.iter() {
                m.arm(
                    format!("InstructionData::{}", format.name),
                    vec![".."],
                    format!("InstructionFormat::{}", format.name),
                );
            }
            fmt.add_match(m);
        });
        fmt.line("}");
    });
    fmt.line("}");
    fmt.empty_line();
}

/// Generate the InstructionData enum.
///
/// Every variant must contain an `opcode` field. The size of `InstructionData` should be kept at
/// 16 bytes on 64-bit architectures. If more space is needed to represent an instruction, use a
/// `ValueList` to store the additional information out of line.
fn gen_instruction_data(registry: &FormatRegistry, fmt: &mut Formatter) {
    fmt.line("#[derive(Clone, Debug)]");
    fmt.line("#[allow(missing_docs)]");
    fmt.line("pub enum InstructionData {");
    fmt.indent(|fmt| {
        for format in registry.iter() {
            fmtln!(fmt, "{} {{", format.name);
            fmt.indent(|fmt| {
                fmt.line("opcode: Opcode,");
                if format.typevar_operand.is_some() {
                    if format.has_value_list {
                        fmt.line("args: ValueList,");
                    } else if format.num_value_operands == 1 {
                        fmt.line("arg: Value,");
                    } else {
                        fmtln!(fmt, "args: [Value; {}],", format.num_value_operands);
                    }
                }
                for field in &format.imm_fields {
                    fmtln!(fmt, "{}: {},", field.member, field.kind.rust_type);
                }
            });
            fmtln!(fmt, "},");
        }
    });
    fmt.line("}");
}

fn gen_arguments_method(registry: &FormatRegistry, fmt: &mut Formatter, is_mut: bool) {
    let (method, mut_, rslice, as_slice) = if is_mut {
        ("arguments_mut", "mut ", "ref_slice_mut", "as_mut_slice")
    } else {
        ("arguments", "", "ref_slice", "as_slice")
    };

    fmtln!(
        fmt,
        "pub fn {}<'a>(&'a {}self, pool: &'a {}ir::ValueListPool) -> &{}[Value] {{",
        method,
        mut_,
        mut_,
        mut_
    );
    fmt.indent(|fmt| {
        let mut m = Match::new("*self");
        for format in registry.iter() {
            let name = format!("InstructionData::{}", format.name);

            // Formats with a value list put all of their arguments in the list. We don't split
            // them up, just return it all as variable arguments. (I expect the distinction to go
            // away).
            if format.has_value_list {
                m.arm(
                    name,
                    vec![format!("ref {}args", mut_), "..".to_string()],
                    format!("args.{}(pool)", as_slice),
                );
                continue;
            }

            // Fixed args.
            let mut fields = Vec::new();
            let arg = if format.num_value_operands == 0 {
                format!("&{}[]", mut_)
            } else if format.num_value_operands == 1 {
                fields.push(format!("ref {}arg", mut_));
                format!("{}(arg)", rslice)
            } else {
                let arg = format!("args_arity{}", format.num_value_operands);
                fields.push(format!("args: ref {}{}", mut_, arg));
                arg
            };
            fields.push("..".into());

            m.arm(name, fields, arg);
        }
        fmt.add_match(m);
    });
    fmtln!(fmt, "}");
}

/// Generate the boring parts of the InstructionData implementation.
///
/// These methods in `impl InstructionData` can be generated automatically from the instruction
/// formats:
///
/// - `pub fn opcode(&self) -> Opcode`
/// - `pub fn arguments(&self, &pool) -> &[Value]`
/// - `pub fn arguments_mut(&mut self, &pool) -> &mut [Value]`
/// - `pub fn take_value_list(&mut self) -> Option<ir::ValueList>`
/// - `pub fn put_value_list(&mut self, args: ir::ValueList>`
/// - `pub fn eq(&self, &other: Self, &pool) -> bool`
/// - `pub fn hash<H: Hasher>(&self, state: &mut H, &pool)`
fn gen_instruction_data_impl(registry: &FormatRegistry, fmt: &mut Formatter) {
    fmt.line("impl InstructionData {");
    fmt.indent(|fmt| {
        fmt.doc_comment("Get the opcode of this instruction.");
        fmt.line("pub fn opcode(&self) -> Opcode {");
        fmt.indent(|fmt| {
            let mut m = Match::new("*self");
            for format in registry.iter() {
                m.arm(format!("InstructionData::{}", format.name), vec!["opcode", ".."],
                      "opcode".to_string());
            }
            fmt.add_match(m);
        });
        fmt.line("}");
        fmt.empty_line();

        fmt.doc_comment("Get the controlling type variable operand.");
        fmt.line("pub fn typevar_operand(&self, pool: &ir::ValueListPool) -> Option<Value> {");
        fmt.indent(|fmt| {
            let mut m = Match::new("*self");
            for format in registry.iter() {
                let name = format!("InstructionData::{}", format.name);
                if format.typevar_operand.is_none() {
                    m.arm(name, vec![".."], "None".to_string());
                } else if format.has_value_list {
                    // We keep all arguments in a value list.
                    m.arm(name, vec!["ref args", ".."], format!("args.get({}, pool)", format.typevar_operand.unwrap()));
                } else if format.num_value_operands == 1 {
                    m.arm(name, vec!["arg", ".."], "Some(arg)".to_string());
                } else {
                    // We have multiple value operands and an array `args`.
                    // Which `args` index to use?
                    let args = format!("args_arity{}", format.num_value_operands);
                    m.arm(name, vec![format!("args: ref {}", args), "..".to_string()],
                        format!("Some({}[{}])", args, format.typevar_operand.unwrap()));
                }
            }
            fmt.add_match(m);
        });
        fmt.line("}");
        fmt.empty_line();

        fmt.doc_comment("Get the value arguments to this instruction.");
        gen_arguments_method(registry, fmt, false);
        fmt.empty_line();

        fmt.doc_comment(r#"Get mutable references to the value arguments to this
                        instruction."#);
        gen_arguments_method(registry, fmt, true);
        fmt.empty_line();

        fmt.doc_comment(r#"
            Take out the value list with all the value arguments and return
            it.

            This leaves the value list in the instruction empty. Use
            `put_value_list` to put the value list back.
        "#);
        fmt.line("pub fn take_value_list(&mut self) -> Option<ir::ValueList> {");
        fmt.indent(|fmt| {
            let mut m = Match::new("*self");

            for format in registry.iter() {
                if format.has_value_list {
                    m.arm(format!("InstructionData::{}", format.name),
                    vec!["ref mut args", ".."],
                    "Some(args.take())".to_string());
                }
            }

            m.arm_no_fields("_", "None");

            fmt.add_match(m);
        });
        fmt.line("}");
        fmt.empty_line();

        fmt.doc_comment(r#"
            Put back a value list.

            After removing a value list with `take_value_list()`, use this
            method to put it back. It is required that this instruction has
            a format that accepts a value list, and that the existing value
            list is empty. This avoids leaking list pool memory.
        "#);
        fmt.line("pub fn put_value_list(&mut self, vlist: ir::ValueList) {");
        fmt.indent(|fmt| {
            fmt.line("let args = match *self {");
            fmt.indent(|fmt| {
                for format in registry.iter() {
                    if format.has_value_list {
                        fmtln!(fmt, "InstructionData::{} {{ ref mut args, .. }} => args,", format.name);
                    }
                }
                fmt.line("_ => panic!(\"No value list: {:?}\", self),");
            });
            fmt.line("};");
            fmt.line("debug_assert!(args.is_empty(), \"Value list already in use\");");
            fmt.line("*args = vlist;");
        });
        fmt.line("}");
        fmt.empty_line();

        fmt.doc_comment(r#"
            Compare two `InstructionData` for equality.

            This operation requires a reference to a `ValueListPool` to
            determine if the contents of any `ValueLists` are equal.
        "#);
        fmt.line("pub fn eq(&self, other: &Self, pool: &ir::ValueListPool) -> bool {");
        fmt.indent(|fmt| {
            fmt.line("if ::core::mem::discriminant(self) != ::core::mem::discriminant(other) {");
            fmt.indent(|fmt| {
                fmt.line("return false;");
            });
            fmt.line("}");

            fmt.line("match (self, other) {");
            fmt.indent(|fmt| {
                for format in registry.iter() {
                    let name = format!("&InstructionData::{}", format.name);
                    let mut members = vec!["opcode"];

                    let args_eq = if format.typevar_operand.is_none() {
                        None
                    } else if format.has_value_list {
                        members.push("args");
                        Some("args1.as_slice(pool) == args2.as_slice(pool)")
                    } else if format.num_value_operands == 1 {
                        members.push("arg");
                        Some("arg1 == arg2")
                    } else {
                        members.push("args");
                        Some("args1 == args2")
                    };

                    for field in &format.imm_fields {
                        members.push(field.member);
                    }

                    let pat1 = members.iter().map(|x| format!("{}: ref {}1", x, x)).collect::<Vec<_>>().join(", ");
                    let pat2 = members.iter().map(|x| format!("{}: ref {}2", x, x)).collect::<Vec<_>>().join(", ");
                    fmtln!(fmt, "({} {{ {} }}, {} {{ {} }}) => {{", name, pat1, name, pat2);
                    fmt.indent(|fmt| {
                        fmt.line("opcode1 == opcode2");
                        for field in &format.imm_fields {
                            fmtln!(fmt, "&& {}1 == {}2", field.member, field.member);
                        }
                        if let Some(args_eq) = args_eq {
                            fmtln!(fmt, "&& {}", args_eq);
                        }
                    });
                    fmtln!(fmt, "}");
                }
                fmt.line("_ => unreachable!()");
            });
            fmt.line("}");
        });
        fmt.line("}");
        fmt.empty_line();

        fmt.doc_comment(r#"
            Hash an `InstructionData`.

            This operation requires a reference to a `ValueListPool` to
            hash the contents of any `ValueLists`.
        "#);
        fmt.line("pub fn hash<H: ::core::hash::Hasher>(&self, state: &mut H, pool: &ir::ValueListPool) {");
        fmt.indent(|fmt| {
            fmt.line("match *self {");
            fmt.indent(|fmt| {
                for format in registry.iter() {
                    let name = format!("InstructionData::{}", format.name);
                    let mut members = vec!["opcode"];

                    let args = if format.typevar_operand.is_none() {
                        "&()"
                    } else if format.has_value_list {
                        members.push("ref args");
                        "args.as_slice(pool)"
                    } else if format.num_value_operands == 1 {
                        members.push("ref arg");
                        "arg"
                    } else {
                        members.push("ref args");
                        "args"
                    };

                    for field in &format.imm_fields {
                        members.push(field.member);
                    }
                    let members = members.join(", ");

                    fmtln!(fmt, "{}{{{}}} => {{", name, members ); // beware the moustaches
                    fmt.indent(|fmt| {
                        fmt.line("::core::hash::Hash::hash( &::core::mem::discriminant(self), state);");
                        fmt.line("::core::hash::Hash::hash(&opcode, state);");
                        for field in &format.imm_fields {
                            fmtln!(fmt, "::core::hash::Hash::hash(&{}, state);", field.member);
                        }
                        fmtln!(fmt, "::core::hash::Hash::hash({}, state);", args);
                    });
                    fmtln!(fmt, "}");
                }
            });
            fmt.line("}");
        });
        fmt.line("}");
    });
    fmt.line("}");
}

fn gen_bool_accessor<T: Fn(&Instruction) -> bool>(
    all_inst: &AllInstructions,
    get_attr: T,
    name: &'static str,
    doc: &'static str,
    fmt: &mut Formatter,
) {
    fmt.doc_comment(doc);
    fmtln!(fmt, "pub fn {}(self) -> bool {{", name);
    fmt.indent(|fmt| {
        let mut m = Match::new("self");
        for inst in all_inst.values() {
            if get_attr(inst) {
                m.arm_no_fields(format!("Opcode::{}", inst.camel_name), "true");
            }
        }
        m.arm_no_fields("_", "false");
        fmt.add_match(m);
    });
    fmtln!(fmt, "}");
    fmt.empty_line();
}

fn gen_opcodes<'a>(all_inst: &AllInstructions, formats: &FormatRegistry, fmt: &mut Formatter) {
    fmt.doc_comment(
        r#"
        An instruction opcode.

        All instructions from all supported ISAs are present.
    "#,
    );
    fmt.line("#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]");

    // We explicitly set the discriminant of the first variant to 1, which allows us to take
    // advantage of the NonZero optimization, meaning that wrapping enums can use the 0
    // discriminant instead of increasing the size of the whole type, and so the size of
    // Option<Opcode> is the same as Opcode's.
    fmt.line("pub enum Opcode {");
    fmt.indent(|fmt| {
        let mut is_first_opcode = true;
        for inst in all_inst.values() {
            let format = formats.get(inst.format);
            fmt.doc_comment(format!("`{}`. ({})", inst, format.name));

            // Document polymorphism.
            if let Some(poly) = &inst.polymorphic_info {
                if poly.use_typevar_operand {
                    let op_num = inst.value_opnums[format.typevar_operand.unwrap()];
                    fmt.doc_comment(format!(
                        "Type inferred from `{}`.",
                        inst.operands_in[op_num].name
                    ));
                }
            }

            // Enum variant itself.
            if is_first_opcode {
                assert!(inst.opcode_number.index() == 0);
                // TODO the python crate requires opcode numbers to start from one.
                fmtln!(fmt, "{} = 1,", inst.camel_name);
                is_first_opcode = false;
            } else {
                fmtln!(fmt, "{},", inst.camel_name)
            }
        }
    });
    fmt.line("}");
    fmt.empty_line();

    fmt.line("impl Opcode {");
    fmt.indent(|fmt| {
        gen_bool_accessor(
            all_inst,
            |inst| inst.is_terminator,
            "is_terminator",
            "True for instructions that terminate the EBB",
            fmt,
        );
        gen_bool_accessor(
            all_inst,
            |inst| inst.is_branch,
            "is_branch",
            "True for all branch or jump instructions.",
            fmt,
        );
        gen_bool_accessor(
            all_inst,
            |inst| inst.is_indirect_branch,
            "is_indirect_branch",
            "True for all indirect branch or jump instructions.",
            fmt,
        );
        gen_bool_accessor(
            all_inst,
            |inst| inst.is_call,
            "is_call",
            "Is this a call instruction?",
            fmt,
        );
        gen_bool_accessor(
            all_inst,
            |inst| inst.is_return,
            "is_return",
            "Is this a return instruction?",
            fmt,
        );
        gen_bool_accessor(
            all_inst,
            |inst| inst.is_ghost,
            "is_ghost",
            "Is this a ghost instruction?",
            fmt,
        );
        gen_bool_accessor(
            all_inst,
            |inst| inst.can_load,
            "can_load",
            "Can this instruction read from memory?",
            fmt,
        );
        gen_bool_accessor(
            all_inst,
            |inst| inst.can_store,
            "can_store",
            "Can this instruction write to memory?",
            fmt,
        );
        gen_bool_accessor(
            all_inst,
            |inst| inst.can_trap,
            "can_trap",
            "Can this instruction cause a trap?",
            fmt,
        );
        gen_bool_accessor(
            all_inst,
            |inst| inst.other_side_effects,
            "other_side_effects",
            "Does this instruction have other side effects besides can_* flags?",
            fmt,
        );
        gen_bool_accessor(
            all_inst,
            |inst| inst.writes_cpu_flags,
            "writes_cpu_flags",
            "Does this instruction write to CPU flags?",
            fmt,
        );
    });
    fmt.line("}");
    fmt.empty_line();

    // Generate a private opcode_format table.
    fmtln!(
        fmt,
        "const OPCODE_FORMAT: [InstructionFormat; {}] = [",
        all_inst.len()
    );
    fmt.indent(|fmt| {
        for inst in all_inst.values() {
            let format = formats.get(inst.format);
            fmtln!(fmt, "InstructionFormat::{}, // {}", format.name, inst.name);
        }
    });
    fmtln!(fmt, "];");
    fmt.empty_line();

    // Generate a private opcode_name function.
    fmt.line("fn opcode_name(opc: Opcode) -> &\'static str {");
    fmt.indent(|fmt| {
        let mut m = Match::new("opc");
        for inst in all_inst.values() {
            m.arm_no_fields(
                format!("Opcode::{}", inst.camel_name),
                format!("\"{}\"", inst.name),
            );
        }
        fmt.add_match(m);
    });
    fmt.line("}");
    fmt.empty_line();

    // Generate an opcode hash table for looking up opcodes by name.
    let hash_table = constant_hash::generate_table(all_inst.values(), all_inst.len(), |inst| {
        constant_hash::simple_hash(&inst.name)
    });
    fmtln!(
        fmt,
        "const OPCODE_HASH_TABLE: [Option<Opcode>; {}] = [",
        hash_table.len()
    );
    fmt.indent(|fmt| {
        for i in hash_table {
            match i {
                Some(i) => fmtln!(fmt, "Some(Opcode::{}),", i.camel_name),
                None => fmtln!(fmt, "None,"),
            }
        }
    });
    fmtln!(fmt, "];");
    fmt.empty_line();
}

/// Get the value type constraint for an SSA value operand, where
/// `ctrl_typevar` is the controlling type variable.
///
/// Each operand constraint is represented as a string, one of:
/// - `Concrete(vt)`, where `vt` is a value type name.
/// - `Free(idx)` where `idx` is an index into `type_sets`.
/// - `Same`, `Lane`, `AsBool` for controlling typevar-derived constraints.
fn get_constraint<'entries, 'table>(
    operand: &'entries Operand,
    ctrl_typevar: Option<&TypeVar>,
    type_sets: &'table mut UniqueTable<'entries, TypeSet>,
) -> String {
    assert!(operand.is_value());
    let type_var = operand.type_var().unwrap();

    if let Some(typ) = type_var.singleton_type() {
        return format!("Concrete({})", typ.rust_name());
    }

    if let Some(free_typevar) = type_var.free_typevar() {
        if ctrl_typevar.is_some() && free_typevar != *ctrl_typevar.unwrap() {
            assert!(type_var.base.is_none());
            return format!("Free({})", type_sets.add(&type_var.get_raw_typeset()));
        }
    }

    if let Some(base) = &type_var.base {
        assert!(base.type_var == *ctrl_typevar.unwrap());
        return camel_case(base.derived_func.name());
    }

    assert!(type_var == ctrl_typevar.unwrap());
    return "Same".into();
}

fn gen_bitset<'a, T: IntoIterator<Item = &'a u16>>(
    iterable: T,
    name: &'static str,
    field_size: u8,
    fmt: &mut Formatter,
) {
    let bits = iterable.into_iter().fold(0, |acc, x| {
        assert!(x.is_power_of_two());
        assert!(u32::from(*x) < (1 << u32::from(field_size)));
        acc | x
    });
    fmtln!(fmt, "{}: BitSet::<u{}>({}),", name, field_size, bits);
}

fn iterable_to_string<I: fmt::Display, T: IntoIterator<Item = I>>(iterable: T) -> String {
    let elems = iterable
        .into_iter()
        .map(|x| x.to_string())
        .collect::<Vec<_>>()
        .join(", ");
    format!("{{{}}}", elems)
}

fn typeset_to_string(ts: &TypeSet) -> String {
    let mut result = format!("TypeSet(lanes={}", iterable_to_string(&ts.lanes));
    if ts.ints.len() > 0 {
        result += &format!(", ints={}", iterable_to_string(&ts.ints));
    }
    if ts.floats.len() > 0 {
        result += &format!(", floats={}", iterable_to_string(&ts.floats));
    }
    if ts.bools.len() > 0 {
        result += &format!(", bools={}", iterable_to_string(&ts.bools));
    }
    if ts.bitvecs.len() > 0 {
        result += &format!(", bitvecs={}", iterable_to_string(&ts.bitvecs));
    }
    if ts.specials.len() > 0 {
        result += &format!(", specials=[{}]", iterable_to_string(&ts.specials));
    }
    if ts.refs.len() > 0 {
        result += &format!(", refs={}", iterable_to_string(&ts.refs));
    }
    result += ")";
    result
}

/// Generate the table of ValueTypeSets described by type_sets.
pub fn gen_typesets_table(type_sets: &UniqueTable<TypeSet>, fmt: &mut Formatter) {
    if type_sets.len() == 0 {
        return;
    }

    fmt.comment("Table of value type sets.");
    assert!(type_sets.len() <= TYPESET_LIMIT, "Too many type sets!");
    fmtln!(
        fmt,
        "const TYPE_SETS: [ir::instructions::ValueTypeSet; {}] = [",
        type_sets.len()
    );
    fmt.indent(|fmt| {
        for ts in type_sets.iter() {
            fmt.line("ir::instructions::ValueTypeSet {");
            fmt.indent(|fmt| {
                assert!(ts.bitvecs.len() == 0, "Bitvector types are not emittable.");
                fmt.comment(typeset_to_string(ts));
                gen_bitset(&ts.lanes, "lanes", 16, fmt);
                gen_bitset(&ts.ints, "ints", 8, fmt);
                gen_bitset(&ts.floats, "floats", 8, fmt);
                gen_bitset(&ts.bools, "bools", 8, fmt);
                gen_bitset(&ts.refs, "refs", 8, fmt);
            });
            fmt.line("},");
        }
    });
    fmtln!(fmt, "];");
}

/// Generate value type constraints for all instructions.
/// - Emit a compact constant table of ValueTypeSet objects.
/// - Emit a compact constant table of OperandConstraint objects.
/// - Emit an opcode-indexed table of instruction constraints.
fn gen_type_constraints(all_inst: &AllInstructions, fmt: &mut Formatter) {
    // Table of TypeSet instances.
    let mut type_sets = UniqueTable::new();

    // Table of operand constraint sequences (as tuples). Each operand
    // constraint is represented as a string, one of:
    // - `Concrete(vt)`, where `vt` is a value type name.
    // - `Free(idx)` where `idx` is an index into `type_sets`.
    // - `Same`, `Lane`, `AsBool` for controlling typevar-derived constraints.
    let mut operand_seqs = UniqueSeqTable::new();

    // Preload table with constraints for typical binops.
    operand_seqs.add(&vec!["Same".to_string(); 3]);

    fmt.comment("Table of opcode constraints.");
    fmtln!(
        fmt,
        "const OPCODE_CONSTRAINTS: [OpcodeConstraints; {}] = [",
        all_inst.len()
    );
    fmt.indent(|fmt| {
        for inst in all_inst.values() {
            let (ctrl_typevar, ctrl_typeset) = if let Some(poly) = &inst.polymorphic_info {
                let index = type_sets.add(&*poly.ctrl_typevar.get_raw_typeset());
                (Some(&poly.ctrl_typevar), index)
            } else {
                (None, TYPESET_LIMIT)
            };

            // Collect constraints for the value results, not including `variable_args` results
            // which are always special cased.
            let mut constraints = Vec::new();
            for &index in &inst.value_results {
                constraints.push(get_constraint(&inst.operands_out[index], ctrl_typevar, &mut type_sets));
            }
            for &index in &inst.value_opnums {
                constraints.push(get_constraint(&inst.operands_in[index], ctrl_typevar, &mut type_sets));
            }

            let constraint_offset = operand_seqs.add(&constraints);

            let fixed_results = inst.value_results.len();
            let fixed_values = inst.value_opnums.len();

            // Can the controlling type variable be inferred from the designated operand?
            let use_typevar_operand = if let Some(poly) = &inst.polymorphic_info {
                poly.use_typevar_operand
            } else {
                false
            };

            // Can the controlling type variable be inferred from the result?
            let use_result = fixed_results > 0 && inst.operands_out[inst.value_results[0]].type_var() == ctrl_typevar;

            // Are we required to use the designated operand instead of the result?
            let requires_typevar_operand = use_typevar_operand && !use_result;

            fmt.comment(
                format!("{}: fixed_results={}, use_typevar_operand={}, requires_typevar_operand={}, fixed_values={}",
                inst.camel_name,
                fixed_results,
                use_typevar_operand,
                requires_typevar_operand,
                fixed_values)
            );
            fmt.comment(format!("Constraints=[{}]", constraints
                .iter()
                .map(|x| format!("'{}'", x))
                .collect::<Vec<_>>()
                .join(", ")));
            if let Some(poly) = &inst.polymorphic_info {
                fmt.comment(format!("Polymorphic over {}", typeset_to_string(&poly.ctrl_typevar.get_raw_typeset())));
            }

            // Compute the bit field encoding, c.f. instructions.rs.
            assert!(fixed_results < 8 && fixed_values < 8, "Bit field encoding too tight");
            let mut flags = fixed_results; // 3 bits
            if use_typevar_operand {
                flags |= 1<<3; // 4th bit
            }
            if requires_typevar_operand {
                flags |= 1<<4; // 5th bit
            }
            flags |= fixed_values << 5; // 6th bit and more

            fmt.line("OpcodeConstraints {");
            fmt.indent(|fmt| {
                fmtln!(fmt, "flags: {:#04x},", flags);
                fmtln!(fmt, "typeset_offset: {},", ctrl_typeset);
                fmtln!(fmt, "constraint_offset: {},", constraint_offset);
            });
            fmt.line("},");
        }
    });
    fmtln!(fmt, "];");
    fmt.empty_line();

    gen_typesets_table(&type_sets, fmt);
    fmt.empty_line();

    fmt.comment("Table of operand constraint sequences.");
    fmtln!(
        fmt,
        "const OPERAND_CONSTRAINTS: [OperandConstraint; {}] = [",
        operand_seqs.len()
    );
    fmt.indent(|fmt| {
        for constraint in operand_seqs.iter() {
            fmtln!(fmt, "OperandConstraint::{},", constraint);
        }
    });
    fmtln!(fmt, "];");
}

/// Emit member initializers for an instruction format.
fn gen_member_inits(format: &InstructionFormat, fmt: &mut Formatter) {
    // Immediate operands.
    // We have local variables with the same names as the members.
    for f in &format.imm_fields {
        fmtln!(fmt, "{},", f.member);
    }

    // Value operands.
    if format.has_value_list {
        fmt.line("args,");
    } else if format.num_value_operands == 1 {
        fmt.line("arg: arg0,");
    } else if format.num_value_operands > 1 {
        let mut args = Vec::new();
        for i in 0..format.num_value_operands {
            args.push(format!("arg{}", i));
        }
        fmtln!(fmt, "args: [{}],", args.join(", "));
    }
}

/// Emit a method for creating and inserting an instruction format.
///
/// All instruction formats take an `opcode` argument and a `ctrl_typevar` argument for deducing
/// the result types.
fn gen_format_constructor(format: &InstructionFormat, fmt: &mut Formatter) {
    // Construct method arguments.
    let mut args = vec![
        "self".to_string(),
        "opcode: Opcode".into(),
        "ctrl_typevar: Type".into(),
    ];

    // Normal operand arguments. Start with the immediate operands.
    for f in &format.imm_fields {
        args.push(format!("{}: {}", f.member, f.kind.rust_type));
    }

    // Then the value operands.
    if format.has_value_list {
        // Take all value arguments as a finished value list. The value lists
        // are created by the individual instruction constructors.
        args.push("args: ir::ValueList".into());
    } else {
        // Take a fixed number of value operands.
        for i in 0..format.num_value_operands {
            args.push(format!("arg{}: Value", i));
        }
    }

    let proto = format!(
        "{}({}) -> (Inst, &'f mut ir::DataFlowGraph)",
        format.name,
        args.join(", ")
    );

    fmt.doc_comment(format.to_string());
    fmt.line("#[allow(non_snake_case)]");
    fmtln!(fmt, "fn {} {{", proto);
    fmt.indent(|fmt| {
        // Generate the instruction data.
        fmtln!(fmt, "let data = ir::InstructionData::{} {{", format.name);
        fmt.indent(|fmt| {
            fmt.line("opcode,");
            gen_member_inits(format, fmt);
        });
        fmtln!(fmt, "};");
        fmt.line("self.build(data, ctrl_typevar)");
    });
    fmtln!(fmt, "}");
}

/// Emit a method for generating the instruction `inst`.
///
/// The method will create and insert an instruction, then return the result values, or the
/// instruction reference itself for instructions that don't have results.
fn gen_inst_builder(inst: &Instruction, format: &InstructionFormat, fmt: &mut Formatter) {
    // Construct method arguments.
    let mut args = vec![if format.has_value_list {
        "mut self"
    } else {
        "self"
    }
    .to_string()];

    // The controlling type variable will be inferred from the input values if
    // possible. Otherwise, it is the first method argument.
    if let Some(poly) = &inst.polymorphic_info {
        if !poly.use_typevar_operand {
            args.push(format!("{}: crate::ir::Type", poly.ctrl_typevar.name));
        }
    }

    let mut tmpl_types = Vec::new();
    let mut into_args = Vec::new();
    for op in &inst.operands_in {
        let t = if op.is_pure_immediate() {
            let t = format!("T{}{}", tmpl_types.len() + 1, op.kind.name);
            tmpl_types.push(format!("{}: Into<{}>", t, op.kind.rust_type));
            into_args.push(op.name);
            t
        } else {
            op.kind.rust_type.clone()
        };
        args.push(format!("{}: {}", op.name, t));
    }

    let rtype = match inst.value_results.len() {
        0 => "Inst".into(),
        1 => "Value".into(),
        _ => format!("({})", vec!["Value"; inst.value_results.len()].join(", ")),
    };

    let tmpl = if tmpl_types.len() > 0 {
        format!("<{}>", tmpl_types.join(", "))
    } else {
        "".into()
    };

    let proto = format!(
        "{}{}({}) -> {}",
        inst.snake_name(),
        tmpl,
        args.join(", "),
        rtype
    );

    fmt.doc_comment(&inst.doc);
    fmt.line("#[allow(non_snake_case)]");
    fmtln!(fmt, "fn {} {{", proto);
    fmt.indent(|fmt| {
        // Convert all of the `Into<>` arguments.
        for arg in &into_args {
            fmtln!(fmt, "let {} = {}.into();", arg, arg);
        }

        // Arguments for instruction constructor.
        let first_arg = format!("Opcode::{}", inst.camel_name);
        let mut args = vec![first_arg.as_str()];
        if let Some(poly) = &inst.polymorphic_info {
            if poly.use_typevar_operand {
                // Infer the controlling type variable from the input operands.
                let op_num = inst.value_opnums[format.typevar_operand.unwrap()];
                fmtln!(
                    fmt,
                    "let ctrl_typevar = self.data_flow_graph().value_type({});",
                    inst.operands_in[op_num].name
                );

                // The format constructor will resolve the result types from the type var.
                args.push("ctrl_typevar");
            } else {
                // This was an explicit method argument.
                args.push(&poly.ctrl_typevar.name);
            }
        } else {
            // No controlling type variable needed.
            args.push("types::INVALID");
        }

        // Now add all of the immediate operands to the constructor arguments.
        for &op_num in &inst.imm_opnums {
            args.push(inst.operands_in[op_num].name);
        }

        // Finally, the value operands.
        if format.has_value_list {
            // We need to build a value list with all the arguments.
            fmt.line("let mut vlist = ir::ValueList::default();");
            args.push("vlist");
            fmt.line("{");
            fmt.indent(|fmt| {
                fmt.line("let pool = &mut self.data_flow_graph_mut().value_lists;");
                for op in &inst.operands_in {
                    if op.is_value() {
                        fmtln!(fmt, "vlist.push({}, pool);", op.name);
                    } else if op.is_varargs() {
                        fmtln!(fmt, "vlist.extend({}.iter().cloned(), pool);", op.name);
                    }
                }
            });
            fmt.line("}");
        } else {
            // With no value list, we're guaranteed to just have a set of fixed value operands.
            for &op_num in &inst.value_opnums {
                args.push(inst.operands_in[op_num].name);
            }
        }

        // Call to the format constructor,
        let fcall = format!("self.{}({})", format.name, args.join(", "));

        if inst.value_results.len() == 0 {
            fmtln!(fmt, "{}.0", fcall);
            return;
        }

        fmtln!(fmt, "let (inst, dfg) = {};", fcall);
        if inst.value_results.len() == 1 {
            fmt.line("dfg.first_result(inst)");
        } else {
            fmtln!(
                fmt,
                "let results = &dfg.inst_results(inst)[0..{}];",
                inst.value_results.len()
            );
            fmtln!(
                fmt,
                "({})",
                inst.value_results
                    .iter()
                    .enumerate()
                    .map(|(i, _)| format!("results[{}]", i))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
    });
    fmtln!(fmt, "}")
}

/// Generate a Builder trait with methods for all instructions.
fn gen_builder(instructions: &AllInstructions, formats: &FormatRegistry, fmt: &mut Formatter) {
    fmt.doc_comment(
        r#"
        Convenience methods for building instructions.

        The `InstBuilder` trait has one method per instruction opcode for
        conveniently constructing the instruction with minimum arguments.
        Polymorphic instructions infer their result types from the input
        arguments when possible. In some cases, an explicit `ctrl_typevar`
        argument is required.

        The opcode methods return the new instruction's result values, or
        the `Inst` itself for instructions that don't have any results.

        There is also a method per instruction format. These methods all
        return an `Inst`.
    "#,
    );
    fmt.line("pub trait InstBuilder<'f>: InstBuilderBase<'f> {");
    fmt.indent(|fmt| {
        for inst in instructions.values() {
            gen_inst_builder(inst, formats.get(inst.format), fmt);
        }
        for format in formats.iter() {
            gen_format_constructor(format, fmt);
        }
    });
    fmt.line("}");
}

pub(crate) fn generate(
    shared_defs: &SharedDefinitions,
    opcode_filename: &str,
    inst_builder_filename: &str,
    out_dir: &str,
) -> Result<(), error::Error> {
    let format_registry = &shared_defs.format_registry;
    let all_inst = &shared_defs.all_instructions;

    // Opcodes.
    let mut fmt = Formatter::new();
    gen_formats(format_registry, &mut fmt);
    gen_instruction_data(format_registry, &mut fmt);
    fmt.empty_line();
    gen_instruction_data_impl(format_registry, &mut fmt);
    fmt.empty_line();
    gen_opcodes(all_inst, format_registry, &mut fmt);
    gen_type_constraints(all_inst, &mut fmt);
    fmt.update_file(opcode_filename, out_dir)?;

    // Instruction builder.
    let mut fmt = Formatter::new();
    gen_builder(all_inst, format_registry, &mut fmt);
    fmt.update_file(inst_builder_filename, out_dir)?;

    Ok(())
}
