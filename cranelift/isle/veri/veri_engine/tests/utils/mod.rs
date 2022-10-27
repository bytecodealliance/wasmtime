use cranelift_isle::compile::create_envs;
use cranelift_isle::lexer::Lexer;
use cranelift_isle::sema::{Rule, TermEnv, TypeEnv};
use std::env;
use std::path::PathBuf;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;
use veri_annotation::parser_wrapper::parse_annotations;
use veri_engine_lib::rule_tree::verify_rules_for_type_wih_rule_filter;
use veri_engine_lib::termname::pattern_contains_termname;
use veri_engine_lib::type_inference::type_all_rules;
use veri_engine_lib::{isle_files_to_terms, rule_tree::verify_rules_for_type_with_lhs_contains};
use veri_ir::{Counterexample, Type, VerificationResult};

// TODO FB: once the opcode situation is resolved, return and:
// - add nice output
// - create a standard prelude and figure out if its more intuitive to send
//   in rule strings or files
// - intermediate tests?

#[derive(Debug, EnumIter, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub enum Bitwidth {
    I1 = 1,
    I8 = 8,
    I16 = 16,
    I32 = 32,
    I64 = 64,
}

type Result = (Bitwidth, VerificationResult);
type TestResult = Vec<Result>;
type TestResultBuilder = dyn Fn(Bitwidth) -> (Bitwidth, VerificationResult);

// Some examples of functions we might need

pub fn just_8_result() -> TestResult {
    vec![(Bitwidth::I8, VerificationResult::Success)]
}

#[allow(dead_code)]
pub fn just_16_result() -> TestResult {
    vec![(Bitwidth::I16, VerificationResult::Success)]
}

#[allow(dead_code)]
pub fn just_32_result() -> TestResult {
    vec![(Bitwidth::I32, VerificationResult::Success)]
}

#[allow(dead_code)]
pub fn just_64_result() -> TestResult {
    vec![(Bitwidth::I64, VerificationResult::Success)]
}

/// All bitwidths verify
#[allow(dead_code)]
pub fn all_success_result() -> TestResult {
    custom_result(&|w| (w, VerificationResult::Success))
}

/// All bitwidths fail
#[allow(dead_code)]
pub fn all_failure_result() -> TestResult {
    custom_result(&|w| (w, VerificationResult::Failure(Counterexample {})))
}

/// Only bitwidths under and including 64 should verify, rest inapplicable
pub fn lte_64_success_result() -> TestResult {
    custom_result(&|w| {
        (
            w,
            if w as usize <= 64 {
                VerificationResult::Success
            } else {
                VerificationResult::InapplicableRule
            },
        )
    })
}

/// Specify a custom set expected result (helpful if you want to test all the bitwidths and expect
/// a range of different success, failure, and inapplicable outcomes)
pub fn custom_result(f: &TestResultBuilder) -> TestResult {
    Bitwidth::iter().map(|w| f(w)).collect()
}

fn test(inputs: Vec<PathBuf>, tr: TestResult) -> () {
    test_with_filter(inputs, None, tr)
}
// TODO: waiting on output thoughts. re do previous?
fn test_with_filter(inputs: Vec<PathBuf>, name_filter: Option<String>, tr: TestResult) -> () {
    let lexer = cranelift_isle::lexer::Lexer::from_files(&inputs).unwrap();
    let defs = cranelift_isle::parser::parse(lexer).expect("should parse");
    let (typeenv, termenv) = create_envs(&defs).unwrap();
    let annotation_env = parse_annotations(&inputs);
    let type_sols = type_all_rules(defs, &termenv, &typeenv, &annotation_env);
    let annotation_env = parse_annotations(&inputs);
    let filter = name_filter.unwrap_or("lower".to_string());

    // For now, verify rules rooted in `lower`
    for (bw, expected_result) in tr {
        let result = verify_rules_for_type_with_lhs_contains(
            &filter,
            &termenv,
            &typeenv,
            &annotation_env,
            &type_sols,
            bw as usize,
        );
        assert_eq!(result, expected_result, "bitwidth: {:?}", bw);
    }
}

fn test_with_rule_filter(
    inputs: Vec<PathBuf>,
    tr: TestResult,
    filter: impl Fn(&Rule, &TermEnv, &TypeEnv) -> bool,
) -> () {
    let lexer = cranelift_isle::lexer::Lexer::from_files(&inputs).unwrap();
    let defs = cranelift_isle::parser::parse(lexer).expect("should parse");
    let (typeenv, termenv) = create_envs(&defs).unwrap();
    let annotation_env = parse_annotations(&inputs);
    let type_sols = type_all_rules(defs, &termenv, &typeenv, &annotation_env);
    let annotation_env = parse_annotations(&inputs);
    for (bw, expected_result) in tr {
        let result = verify_rules_for_type_wih_rule_filter(
            &termenv,
            &typeenv,
            &annotation_env,
            &type_sols,
            bw as usize,
            &filter,
        );
        assert_eq!(result, expected_result);
    }
}

pub fn test_from_file_with_filter(s: &str, filter: String, tr: TestResult) -> () {
    // TODO: clean up path logic
    let cur_dir = env::current_dir().expect("Can't access current working directory");
    let clif_isle = cur_dir.join("../../../codegen/src").join("clif.isle");
    let prelude_isle = cur_dir.join("../../../codegen/src").join("prelude.isle");
    let input = PathBuf::from(s);
    test_with_filter(vec![clif_isle, prelude_isle, input], Some(filter), tr);
}

pub fn test_from_file(s: &str, tr: TestResult) -> () {
    // TODO: clean up path logic
    let cur_dir = env::current_dir().expect("Can't access current working directory");
    let clif_isle = cur_dir.join("../../../codegen/src").join("clif.isle");
    let prelude_isle = cur_dir.join("../../../codegen/src").join("prelude.isle");
    let input = PathBuf::from(s);
    test(vec![clif_isle, prelude_isle, input], tr);
}

pub fn test_from_files_with_lhs_termname(files: Vec<&str>, termname: &str, tr: TestResult) -> () {
    // TODO: clean up path logic
    let cur_dir = env::current_dir().expect("Can't access current working directory");
    let clif_isle = cur_dir.join("../../../codegen/src").join("clif.isle");
    let prelude_isle = cur_dir.join("../../../codegen/src").join("prelude.isle");
    let mut inputs = vec![clif_isle, prelude_isle];
    for f in files {
        inputs.push(PathBuf::from(f));
    }
    test_with_rule_filter(inputs, tr, |rule, termenv, typeenv| {
        pattern_contains_termname(&rule.lhs, termname, termenv, typeenv)
    });
}

pub fn test_from_file_self_contained(s: &str, tr: TestResult) -> () {
    let input = PathBuf::from(s);
    test(vec![input], tr);
}

pub fn test_from_file_custom_prelude(p: &str, s: &str, tr: TestResult) -> () {
    let prelude = PathBuf::from(p);
    let input = PathBuf::from(s);
    test(vec![prelude, input], tr);
}
