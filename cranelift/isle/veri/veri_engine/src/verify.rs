use crate::type_inference::type_rules_with_term_and_types;
use crate::Config;
use cranelift_isle::error::Errors;
use cranelift_isle::{self as isle};
use isle::compile::create_envs;
use isle::sema::{Pattern, RuleId, TermEnv, TypeEnv};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::annotations::parse_annotations;
use crate::solver::run_solver;
use crate::type_inference::RuleSemantics;
use crate::{interp::Context, termname::pattern_contains_termname};
use veri_ir::{ConcreteTest, TermSignature, VerificationResult};

pub fn verify_rules(
    inputs: Vec<PathBuf>,
    config: &Config,
    widths: &Option<Vec<String>>,
) -> Result<(), Errors> {
    // Produces environments including terms, rules, and maps from symbols and
    // names to types
    let (typeenv, termenv, defs) = create_envs(inputs).unwrap();

    let annotation_env = parse_annotations(&defs, &termenv, &typeenv);

    // Get the types/widths for this particular term
    let types = annotation_env
        .get_term_signatures_by_name(&termenv, &typeenv)
        .get(&config.term as &str)
        .unwrap_or_else(|| panic!("Missing term type instantiation for {}", config.term))
        .clone();

    let types_filtered = if let Some(widths) = widths {
        let mut width_types = Vec::new();

        for w in widths {
            let width_type = match w.as_str() {
                "I8" => veri_ir::Type::BitVector(Some(8)),
                "I16" => veri_ir::Type::BitVector(Some(16)),
                "I32" => veri_ir::Type::BitVector(Some(32)),
                "I64" => veri_ir::Type::BitVector(Some(64)),
                _ => panic!("Invalid width type: {}", w),
            };
            width_types.push(width_type);
        }

        types
            .into_iter()
            .filter(|t| {
                if let Some(canonical_type) = &t.canonical_type {
                    width_types.contains(canonical_type)
                } else {
                    false
                }
            })
            .collect::<Vec<_>>()
    } else {
        types
    };

    for type_instantiation in types_filtered {
        let type_sols = type_rules_with_term_and_types(
            &termenv,
            &typeenv,
            &annotation_env,
            config,
            &type_instantiation,
            &None,
        );
        verify_rules_for_term(
            &termenv,
            &typeenv,
            &type_sols,
            type_instantiation,
            &None,
            config,
        );
    }
    Ok(())
}

pub fn verify_rules_for_term(
    termenv: &TermEnv,
    typeenv: &TypeEnv,
    typesols: &HashMap<RuleId, RuleSemantics>,
    types: TermSignature,
    concrete: &Option<ConcreteTest>,
    config: &Config,
) -> VerificationResult {
    let mut rules_checked = 0;
    for rule in &termenv.rules {
        // Only type rules with the given term on the LHS
        if !pattern_contains_termname(
            // Hack for now: typeid not used
            &Pattern::Term(
                cranelift_isle::sema::TypeId(0),
                rule.root_term,
                rule.args.clone(),
            ),
            &config.term,
            termenv,
            typeenv,
        ) {
            continue;
        }
        if let Some(names) = &config.names {
            if rule.name.is_none() {
                continue;
            }
            let name = &typeenv.syms[rule.name.unwrap().index()];
            if !names.contains(name) {
                continue;
            } else {
                log::debug!("Verifying rule: {}", name);
            }
        }
        let ctx = Context::new(typesols);
        if ctx.typesols.get(&rule.id).is_none() {
            continue;
        }
        let rule_sem = &ctx.typesols[&rule.id];
        log::debug!("Term: {}", config.term);
        log::debug!("Type instantiation: {}", types);
        let result = run_solver(rule_sem, rule, termenv, typeenv, concrete, config, &types);
        rules_checked += 1;
        if result != VerificationResult::Success {
            return result;
        }
    }
    if rules_checked > 0 {
        VerificationResult::Success
    } else {
        panic!("No rules checked!")
    }
}
