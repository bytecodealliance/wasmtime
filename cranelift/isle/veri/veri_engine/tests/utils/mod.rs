use cranelift_codegen_meta::{
    generate_isle,
    isle::{get_isle_compilations, shared_isle_lower_paths},
};
use cranelift_isle::compile::create_envs;
use std::env;
use std::path::PathBuf;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;
use veri_engine_lib::annotations::parse_annotations;
use veri_engine_lib::type_inference::type_rules_with_term_and_types;
use veri_engine_lib::verify::verify_rules_for_term;
use veri_engine_lib::Config;
use veri_ir::{ConcreteTest, Counterexample, TermSignature, VerificationResult};

#[derive(Debug, EnumIter, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
#[repr(usize)]
pub enum Bitwidth {
    I8 = 8,
    I16 = 16,
    I32 = 32,
    I64 = 64,
}

pub enum TestResult {
    Simple(Vec<(Bitwidth, VerificationResult)>),
    Expect(fn(&TermSignature) -> VerificationResult),
}

type TestResultBuilder = dyn Fn(Bitwidth) -> (Bitwidth, VerificationResult);

use std::sync::Once;

static INIT: Once = Once::new();

pub fn get_isle_files(name: &str) -> Vec<std::path::PathBuf> {
    let cur_dir = env::current_dir().expect("Can't access current working directory");
    let gen_dir = cur_dir.join("test_output");
    INIT.call_once(|| {
        // Logger
        env_logger::init();
        // Test directory
        if !gen_dir.is_dir() {
            std::fs::create_dir(gen_dir.as_path()).unwrap();
        }
        // Generate ISLE files.
        generate_isle(gen_dir.as_path()).expect("Can't generate ISLE");
    });

    let codegen_crate_dir = cur_dir.join("../../../codegen");
    let inst_specs_isle = codegen_crate_dir.join("src").join("inst_specs.isle");

    match name {
        "shared_lower" => {
            let mut shared = shared_isle_lower_paths(codegen_crate_dir.as_path());
            shared.push(gen_dir.join("clif_lower.isle"));
            shared
        }
        _ => {
            // Lookup ISLE shared .
            let compilations =
                get_isle_compilations(codegen_crate_dir.as_path(), gen_dir.as_path());

            // Return inputs from the matching compilation, if any.
            let mut inputs = compilations.lookup(name).unwrap().inputs();
            inputs.push(inst_specs_isle);
            inputs
        }
    }
}

// Some examples of functions we might need
#[allow(dead_code)]
pub fn just_8_result() -> TestResult {
    TestResult::Simple(vec![(Bitwidth::I8, VerificationResult::Success)])
}

#[allow(dead_code)]
pub fn just_16_result() -> TestResult {
    TestResult::Simple(vec![(Bitwidth::I16, VerificationResult::Success)])
}

#[allow(dead_code)]
pub fn just_32_result() -> TestResult {
    TestResult::Simple(vec![(Bitwidth::I32, VerificationResult::Success)])
}

#[allow(dead_code)]
pub fn just_64_result() -> TestResult {
    TestResult::Simple(vec![(Bitwidth::I64, VerificationResult::Success)])
}

/// All bitwidths verify
#[allow(dead_code)]
pub fn all_success_result() -> Vec<(Bitwidth, VerificationResult)> {
    custom_result(&|w| (w, VerificationResult::Success))
}

/// All bitwidths fail
#[allow(dead_code)]
pub fn all_failure_result() -> Vec<(Bitwidth, VerificationResult)> {
    custom_result(&|w| (w, VerificationResult::Failure(Counterexample {})))
}

/// Specify a custom set expected result (helpful if you want to test all the bitwidths and expect
/// a range of different success, failure, and inapplicable outcomes)
pub fn custom_result(f: &TestResultBuilder) -> Vec<(Bitwidth, VerificationResult)> {
    Bitwidth::iter().map(f).collect()
}

fn test_rules_with_term(inputs: Vec<PathBuf>, tr: TestResult, config: Config) {
    let (typeenv, termenv, defs) = create_envs(inputs).unwrap();
    let annotation_env = parse_annotations(&defs, &termenv, &typeenv);

    let term_signatures = annotation_env
        .get_term_signatures_by_name(&termenv, &typeenv)
        .get(config.term.as_str())
        .unwrap_or_else(|| panic!("Missing term type instantiation for {}", config.term))
        .clone();
    let instantiations = match tr {
        TestResult::Simple(s) => {
            let mut res = vec![];
            for (width, result) in s {
                let ty = match width {
                    Bitwidth::I8 => veri_ir::Type::BitVector(Some(8)),
                    Bitwidth::I16 => veri_ir::Type::BitVector(Some(16)),
                    Bitwidth::I32 => veri_ir::Type::BitVector(Some(32)),
                    Bitwidth::I64 => veri_ir::Type::BitVector(Some(64)),
                };
                // Find the type instantiations with this as the canonical type
                let all_instantiations: Vec<&TermSignature> = term_signatures
                    .iter()
                    .filter(|sig| sig.canonical_type.unwrap() == ty)
                    .collect();
                if all_instantiations.is_empty() {
                    panic!("Missing type instantiation for width {:?}", width);
                }
                for i in all_instantiations {
                    res.push((i.clone(), result.clone()));
                }
            }
            res
        }
        TestResult::Expect(expect) => term_signatures
            .iter()
            .map(|sig| (sig.clone(), expect(sig)))
            .collect(),
    };

    for (type_instantiation, expected_result) in instantiations {
        log::debug!("Expected result: {:?}", expected_result);
        let type_sols = type_rules_with_term_and_types(
            &termenv,
            &typeenv,
            &annotation_env,
            &config,
            &type_instantiation,
            &None,
        );
        let result = verify_rules_for_term(
            &termenv,
            &typeenv,
            &type_sols,
            type_instantiation,
            &None,
            &config,
        );
        assert_eq!(result, expected_result);
    }
}

pub fn test_from_file_with_lhs_termname_simple(
    file: &str,
    termname: String,
    tr: Vec<(Bitwidth, VerificationResult)>,
) {
    test_from_file_with_lhs_termname(file, termname, TestResult::Simple(tr))
}

pub fn test_from_file_with_lhs_termname(file: &str, termname: String, tr: TestResult) {
    println!("Verifying {} rules in file: {}", termname, file);
    let mut inputs = get_isle_files("shared_lower");
    inputs.push(PathBuf::from(file));
    let config = Config {
        term: termname,
        distinct_check: true,
        custom_verification_condition: None,
        custom_assumptions: None,
        names: None,
    };
    test_rules_with_term(inputs, tr, config);
}

pub fn test_aarch64_rule_with_lhs_termname_simple(
    rulename: &str,
    termname: &str,
    tr: Vec<(Bitwidth, VerificationResult)>,
) {
    test_aarch64_rule_with_lhs_termname(rulename, termname, TestResult::Simple(tr))
}

pub fn test_aarch64_rule_with_lhs_termname(rulename: &str, termname: &str, tr: TestResult) {
    println!("Verifying rule `{}` with termname {} ", rulename, termname);
    let inputs = get_isle_files("aarch64");
    let config = Config {
        term: termname.to_string(),
        distinct_check: true,
        custom_verification_condition: None,
        custom_assumptions: None,
        names: Some(vec![rulename.to_string()]),
    };
    test_rules_with_term(inputs, tr, config);
}

pub fn test_x64_rule_with_lhs_termname_simple(
    rulename: &str,
    termname: &str,
    tr: Vec<(Bitwidth, VerificationResult)>,
) {
    test_x64_rule_with_lhs_termname(rulename, termname, TestResult::Simple(tr))
}

pub fn test_x64_rule_with_lhs_termname(rulename: &str, termname: &str, tr: TestResult) {
    println!("Verifying rule `{}` with termname {} ", rulename, termname);
    let inputs = get_isle_files("x64");
    let config = Config {
        term: termname.to_string(),
        distinct_check: true,
        custom_verification_condition: None,
        custom_assumptions: None,
        names: Some(vec![rulename.to_string()]),
    };
    test_rules_with_term(inputs, tr, config);
}

pub fn test_from_file_with_config_simple(
    file: &str,
    config: Config,
    tr: Vec<(Bitwidth, VerificationResult)>,
) {
    test_from_file_with_config(file, config, TestResult::Simple(tr))
}
pub fn test_from_file_with_config(file: &str, config: Config, tr: TestResult) {
    println!("Verifying {} rules in file: {}", config.term, file);
    let mut inputs = get_isle_files("shared_lower");
    inputs.push(PathBuf::from(file));
    test_rules_with_term(inputs, tr, config);
}

pub fn test_aarch64_with_config_simple(
    config: Config,
    tr: Vec<(Bitwidth, VerificationResult)>,
) {
    test_aarch64_with_config(config, TestResult::Simple(tr))
}

pub fn test_aarch64_with_config(config: Config, tr: TestResult) {
    println!(
        "Verifying rules {:?} with termname {}",
        config.names, config.term
    );
    let inputs = get_isle_files("aarch64");
    test_rules_with_term(inputs, tr, config);
}

pub fn test_concrete_aarch64_rule_with_lhs_termname(
    rulename: &str,
    termname: &str,
    concrete: ConcreteTest,
) {
    println!(
        "Verifying concrete input rule `{}` with termname {} ",
        rulename, termname
    );
    let inputs = get_isle_files("aarch64");
    let (typeenv, termenv, defs) = create_envs(inputs).unwrap();
    let annotation_env = parse_annotations(&defs, &termenv, &typeenv);

    let config = Config {
        term: termname.to_string(),
        distinct_check: false,
        custom_verification_condition: None,
        custom_assumptions: None,
        names: Some(vec![rulename.to_string()]),
    };

    // Get the types/widths for this particular term
    let args = concrete.args.iter().map(|i| i.ty).collect();
    let ret = concrete.output.ty;
    let t = TermSignature {
        args,
        ret,
        canonical_type: None,
    };

    let type_sols = type_rules_with_term_and_types(
        &termenv,
        &typeenv,
        &annotation_env,
        &config,
        &t,
        &Some(concrete.clone()),
    );
    let result = verify_rules_for_term(&termenv, &typeenv, &type_sols, t, &Some(concrete), &config);
    assert_eq!(result, VerificationResult::Success);
}

pub fn test_concrete_input_from_file_with_lhs_termname(
    file: &str,
    termname: String,
    concrete: ConcreteTest,
) {
    println!(
        "Verifying concrete input {} rule in file: {}",
        termname, file
    );
    let mut inputs = get_isle_files("shared_lower");
    inputs.push(PathBuf::from(file));

    let (typeenv, termenv, defs) = create_envs(inputs).unwrap();
    let annotation_env = parse_annotations(&defs, &termenv, &typeenv);

    let config = Config {
        term: termname.clone(),
        distinct_check: false,
        custom_verification_condition: None,
        custom_assumptions: None,
        names: None,
    };

    // Get the types/widths for this particular term
    let args = concrete.args.iter().map(|i| i.ty).collect();
    let ret = concrete.output.ty;
    let t = TermSignature {
        args,
        ret,
        canonical_type: None,
    };

    let type_sols = type_rules_with_term_and_types(
        &termenv,
        &typeenv,
        &annotation_env,
        &config,
        &t,
        &Some(concrete.clone()),
    );
    let result = verify_rules_for_term(&termenv, &typeenv, &type_sols, t, &Some(concrete), &config);
    assert_eq!(result, VerificationResult::Success);
}
