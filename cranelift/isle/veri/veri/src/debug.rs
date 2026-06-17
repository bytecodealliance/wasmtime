use std::io::{self, Write};

use crate::{
    expand::{Constrain, Expansion},
    program::Program,
    trie::{BindingType, binding_type},
    types::field_name_by_index,
};
use cranelift_isle::{
    sema::{TermId, Type, TypeEnv},
    trie_again::{Binding, BindingId, Constraint, RuleSet},
};

pub fn print_expansion(prog: &Program, expansion: &Expansion) {
    write_expansion(&mut io::stdout(), prog, expansion).expect("write to stdout failed");
}

pub fn write_expansion(
    out: &mut dyn Write,
    prog: &Program,
    expansion: &Expansion,
) -> io::Result<()> {
    writeln!(out, "expansion {{")?;

    // Term.
    writeln!(out, "\tterm = {}", prog.term_name(expansion.term))?;

    // Rules.
    writeln!(out, "\trules = [")?;
    for rule_id in &expansion.rules {
        let rule = &prog.termenv.rules[rule_id.index()];
        writeln!(out, "\t\t{}", rule.identifier(&prog.tyenv, &prog.files))?;
    }
    writeln!(out, "\t]")?;

    // Negated rules.
    writeln!(out, "\tnegated = [")?;
    for rule_id in &expansion.negated {
        let rule = &prog.termenv.rules[rule_id.index()];
        writeln!(out, "\t\t{}", rule.identifier(&prog.tyenv, &prog.files))?;
    }
    writeln!(out, "\t]")?;

    // Bindings.
    let lookup_binding =
        |binding_id: BindingId| expansion.bindings[binding_id.index()].clone().unwrap();
    writeln!(out, "\tbindings = [")?;
    for (i, binding) in expansion.bindings.iter().enumerate() {
        if let Some(binding) = binding {
            let ty = binding_type(binding, expansion.term, prog, lookup_binding);
            writeln!(
                out,
                "\t\t{i}: {}\t{}",
                ty.display(&prog.tyenv),
                binding_string(binding, expansion.term, prog, lookup_binding),
            )?;
        }
    }
    writeln!(out, "\t]")?;

    // Constraints.
    writeln!(out, "\tconstraints = [")?;
    for constrain in &expansion.constraints {
        writeln!(out, "\t\t{}", constrain_string(constrain, &prog.tyenv))?;
    }
    writeln!(out, "\t]")?;

    // Equals.
    if !expansion.equals.is_empty() {
        writeln!(out, "\tequals = [")?;
        for (left, right) in expansion.equalities() {
            writeln!(out, "\t\t{} == {}", left.index(), right.index())?;
        }
        writeln!(out, "\t]")?;
    }

    // Parameters.
    writeln!(out, "\tparameters = [")?;
    for binding_id in &expansion.parameters {
        writeln!(out, "\t\t{}", binding_id.index())?;
    }
    writeln!(out, "\t]")?;

    // Result.
    writeln!(out, "\tresult = {}", expansion.result.index())?;

    // Feasibility.
    writeln!(out, "\tfeasible = {}", expansion.is_feasible())?;

    writeln!(out, "}}")?;
    Ok(())
}

pub fn print_rule_set(prog: &Program, term_id: &TermId, rule_set: &RuleSet) {
    println!("term {{");
    println!("\tname = {}", prog.term_name(*term_id));

    // Bindings.
    let lookup_binding = |binding_id: BindingId| rule_set.bindings[binding_id.index()].clone();
    println!("\tbindings = [");
    for (i, binding) in rule_set.bindings.iter().enumerate() {
        let ty = binding_type(binding, *term_id, prog, lookup_binding);
        println!(
            "\t\t{i}: {}\t{}",
            ty.display(&prog.tyenv),
            binding_string(binding, *term_id, prog, lookup_binding),
        );
    }
    println!("\t]");

    // Rules.
    println!("\trules = [");
    for rule in &rule_set.rules {
        assert_eq!(rule.iterators.len(), 0);
        println!("\t\t{{");
        println!("\t\t\tpos = {}", rule.pos.pretty_print_line(&prog.files));
        println!("\t\t\tconstraints = [");
        for i in 0..rule_set.bindings.len() {
            if let Some(constraint) = rule.get_constraint(i.try_into().unwrap()) {
                println!(
                    "\t\t\t\t{}:\t{}",
                    i,
                    constraint_string(&constraint, &prog.tyenv)
                );
            }
        }
        println!("\t\t\t]");
        if !rule.equals.is_empty() {
            println!("\t\t\tequals = [");
            for i in 0..rule_set.bindings.len() {
                let binding_id = i.try_into().unwrap();
                if let Some(eq) = rule.equals.find(binding_id)
                    && eq != binding_id
                {
                    println!("\t\t\t\t{} == {}", binding_id.index(), eq.index());
                }
            }
            println!("\t\t\t]");
        }
        println!("\t\t\tprio = {}", rule.prio);
        println!("\t\t\tresult = {}", rule.result.index());
        if !rule.impure.is_empty() {
            println!(
                "\t\t\timpure = {impure:?}",
                impure = rule
                    .impure
                    .iter()
                    .copied()
                    .map(BindingId::index)
                    .collect::<Vec<_>>()
            );
        }
        println!("\t\t}}");
    }
    println!("\t]");

    println!("}}");
}

pub fn binding_string(
    binding: &Binding,
    term_id: TermId,
    prog: &Program,
    lookup_binding: impl Fn(BindingId) -> Binding,
) -> String {
    match binding {
        Binding::Argument { index } => format!("argument({})", index.index()),
        Binding::ConstInt { val, ty } => {
            let ty = &prog.tyenv.types[ty.index()];
            format!("const_int({val}, {name})", name = ty.name(&prog.tyenv))
        }
        Binding::ConstBool { val, ty } => {
            let ty = &prog.tyenv.types[ty.index()];
            format!("const_bool({val}, {name})", name = ty.name(&prog.tyenv))
        }
        Binding::ConstPrim { val } => format!("const_prim({})", prog.tyenv.syms[val.index()]),
        Binding::Constructor {
            term,
            parameters,
            instance,
        } => {
            let name = prog.term_name(*term);
            format!(
                "constructor({name}, {parameters:?}, {instance})",
                parameters = parameters
                    .iter()
                    .copied()
                    .map(BindingId::index)
                    .collect::<Vec<_>>()
            )
        }
        Binding::Extractor { term, parameter } => {
            let name = prog.term_name(*term);
            format!(
                "extractor({name}, {parameter})",
                parameter = parameter.index()
            )
        }
        Binding::MatchVariant {
            source,
            variant,
            field,
        } => {
            let source_binding = lookup_binding(*source);
            let source_type = binding_type(&source_binding, term_id, prog, lookup_binding);
            let BindingType::Base(source_type_id) = source_type else {
                unreachable!("source of match variant should be a base type")
            };

            // Lookup variant.
            let enum_ty = &prog.tyenv.types[source_type_id.index()];
            let enum_name = enum_ty.name(&prog.tyenv);
            let variant = match enum_ty {
                Type::Enum { variants, .. } => &variants[variant.index()],
                _ => unreachable!("source match variant should be an enum"),
            };
            let variant_name = &prog.tyenv.syms[variant.name.index()];

            // Field.
            let field_name = field_name_by_index(&variant.fields, field.index(), &prog.tyenv);

            format!(
                "match_variant({source}, {enum_name}::{variant_name}, {field_name})",
                source = source.index(),
            )
        }
        Binding::MakeVariant {
            ty,
            variant,
            fields,
        } => {
            let ty = &prog.tyenv.types[ty.index()];
            let Type::Enum { variants, .. } = ty else {
                unreachable!("source match variant should be an enum")
            };
            let variant = &variants[variant.index()];
            let variant_name = &prog.tyenv.syms[variant.name.index()];
            format!(
                "make_variant({ty}::{variant_name}, {fields:?})",
                ty = ty.name(&prog.tyenv),
                fields = fields
                    .iter()
                    .copied()
                    .map(BindingId::index)
                    .collect::<Vec<_>>()
            )
        }
        Binding::MakeStruct { ty, fields } => {
            let ty = &prog.tyenv.types[ty.index()];
            let Type::Struct { .. } = ty else {
                unreachable!("MakeStruct target should be a struct type")
            };
            format!(
                "make_struct({ty}, {fields:?})",
                ty = ty.name(&prog.tyenv),
                fields = fields
                    .iter()
                    .copied()
                    .map(BindingId::index)
                    .collect::<Vec<_>>()
            )
        }
        Binding::ExtractStruct { source, field } => {
            let source_binding = lookup_binding(*source);
            let source_type = binding_type(&source_binding, term_id, prog, lookup_binding);
            let BindingType::Base(source_type_id) = source_type else {
                unreachable!("source of extract_struct should be a base type")
            };
            let struct_ty = &prog.tyenv.types[source_type_id.index()];
            let struct_name = struct_ty.name(&prog.tyenv);
            let fields = match struct_ty {
                Type::Struct { fields, .. } => fields,
                _ => unreachable!("source of extract_struct should be a struct"),
            };
            let field_name = field_name_by_index(fields, field.index(), &prog.tyenv);
            format!(
                "extract_struct({source}, {struct_name}, {field_name})",
                source = source.index(),
            )
        }
        Binding::MakeSome { inner } => format!("some({inner})", inner = inner.index()),
        Binding::MatchSome { source } => format!("match_some({source})", source = source.index()),
        Binding::MatchTuple { source, field } => format!(
            "match_tuple({source}, {field})",
            source = source.index(),
            field = field.index()
        ),
        Binding::Iterator { .. } => unimplemented!("iterator bindings unsupported"),
    }
}

pub fn constrain_string(constrain: &Constrain, tyenv: &TypeEnv) -> String {
    match constrain {
        Constrain::Match(binding_id, constraint) => format!(
            "{}: {}",
            binding_id.index(),
            constraint_string(constraint, tyenv)
        ),
        Constrain::NotAll(constraints) => {
            format!(
                "not_all({constraints})",
                constraints = constraints
                    .iter()
                    .map(|c| constrain_string(c, tyenv))
                    .collect::<Vec<_>>()
                    .join(", "),
            )
        }
    }
}

pub fn constraint_string(constraint: &Constraint, tyenv: &TypeEnv) -> String {
    match constraint {
        Constraint::Variant { ty, variant, .. } => {
            let ty = &tyenv.types[ty.index()];
            match ty {
                Type::Primitive(_, sym, _) => {
                    format!("variant({})", tyenv.syms[sym.index()].clone())
                }
                Type::Enum { name, variants, .. } => {
                    let name = &tyenv.syms[name.index()];
                    let variant = &variants[variant.index()];
                    let variant_name = &tyenv.syms[variant.name.index()];
                    format!("variant({name}::{variant_name})")
                }
                Type::Builtin(b) => {
                    format!("variant({})", b.name())
                }
                Type::Struct { .. } => {
                    unreachable!("variant constraint should not apply to a struct type")
                }
            }
        }
        Constraint::Struct { ty, .. } => {
            let ty = &tyenv.types[ty.index()];
            format!("struct({})", ty.name(tyenv))
        }
        Constraint::ConstInt { val, .. } => format!("const_int({val})"),
        Constraint::ConstBool { val, .. } => format!("const_bool({val})"),
        Constraint::ConstPrim { val } => format!("const_prim({})", tyenv.syms[val.index()]),
        Constraint::Some => "some".to_string(),
    }
}
