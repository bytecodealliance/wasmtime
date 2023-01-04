//! Prototype verification tool for Cranelift's ISLE lowering rules.

use clap::{Arg, Command};
use cranelift_isle as isle;
use isle::compile::create_envs;
use std::env;
use std::path::PathBuf;
use veri_annotation::parser_wrapper::parse_annotations;
use veri_engine_lib::rule_tree::verify_rules_with_lhs_root;
use veri_engine_lib::type_inference::type_all_rules;

fn main() {
    let cur_dir = env::current_dir().expect("Can't access current working directory");

    // TODO: clean up path logic
    let clif_isle = cur_dir.join("../../../codegen/src").join("clif_lower.isle");
    let prelude_isle = cur_dir.join("../../../codegen/src").join("prelude.isle");
    let prelude_lower_isle = cur_dir
        .join("../../../codegen/src")
        .join("prelude_lower.isle");

    // Disable for now to not have to consider all rules
    // let aarch64_isle = cur_dir.join("../../../codegen/src/isa/aarch64").join("inst.isle");

    let matches = Command::new("Verification Engine for ISLE")
        .arg(
            Arg::new("INPUT")
                .help("Sets the input file")
                .required(true)
                .index(1),
        )
        .get_matches();
    let input = PathBuf::from(matches.value_of("INPUT").unwrap());

    let inputs = vec![prelude_isle, prelude_lower_isle, clif_isle, input];
    let lexer = isle::lexer::Lexer::from_files(&inputs).unwrap();
    // Parses to an AST, as a list of definitions
    let defs = isle::parser::parse(lexer).expect("should parse");

    // Produces environments including terms, rules, and maps from symbols and
    // names to types
    let (typeenv, termenv) = create_envs(&defs).unwrap();

    let annotation_env = parse_annotations(&inputs);

    let type_sols = type_all_rules(defs, &termenv, &typeenv, &annotation_env);

    // For now, verify rules rooted in `lower`
    verify_rules_with_lhs_root("lower", &termenv, &typeenv, &annotation_env, &type_sols);
}
