mod utils;
use utils::{all_failure_result, all_success_result, just_8_result, lte_64_success_result, custom_result};
use utils::{
    test_from_file, test_from_file_custom_prelude, test_from_file_self_contained,
    test_from_files_with_lhs_termname, 
};
use veri_ir::{VerificationResult, Counterexample};

#[test]
fn test_iadds() {
    test_from_file_custom_prelude(
        "./tests/code/selfcontained/simple_prelude.isle",
        "./tests/code/selfcontained/simple_iadd.isle",
        lte_64_success_result(),
    );

    test_from_file_custom_prelude(
        "./tests/code/selfcontained/simple_prelude.isle",
        "./tests/code/selfcontained/iadd_to_sub.isle",
        lte_64_success_result(),
    );
}

#[test]
fn test_implicit_conversions() {
    test_from_file_custom_prelude(
        "./tests/code/selfcontained/prelude.isle",
        "./tests/code/selfcontained/simple_iadd_implicit_conv.isle",
        lte_64_success_result(),
    );

    test_from_file_custom_prelude(
        "./tests/code/selfcontained/prelude.isle",
        "./tests/code/selfcontained/iadd_to_sub_implicit_conv.isle",
        lte_64_success_result(),
    );
}

#[test]
fn test_iadd_from_file() {
    test_from_file("./examples/iadd.isle", lte_64_success_result())
}

#[test]
fn test_broken_iadd_from_file() {
    test_from_file("./examples/broken_iadd.isle", all_failure_result())
}

// DISABLED for now while ruin chaining is on hold
// #[test]
// fn test_chained_iadd_from_file() {
//     test_from_file(
//         "./examples/iadd-two-rule-chain.isle",
//         lte_64_success_result(),
//     )
// }

#[test]
fn test_ineg() {
    test_from_file("./examples/ineg.isle", lte_64_success_result())
}

#[test]
fn test_uextend() {
    test_from_file("./examples/uextend.isle", all_success_result())
}

#[test]
fn test_sextend() {
    test_from_file("./examples/sextend.isle", all_success_result())
}

#[test]
fn test_broken_uextend() {
    // In the spec for extend, zero_extend and sign_extend are swapped. 
    // However, this should still work in the case where the query with
    // is the same as the register width (64).
    test_from_file("./examples/broken_uextend.isle", custom_result(&|w| {
        (
            w,
            if (w as usize) < 64 {
                VerificationResult::Failure(Counterexample{})
            } else {
                VerificationResult::Success
            },
        )
    }))
}

// #[test]
// fn test_rotl_from_file() {
//     let files = vec![
//         "../../../codegen/src/isa/aarch64/inst.isle",
//         "../../../codegen/src/isa/aarch64/lower.isle",
//     ];
//     // TODO: enable once this works
//     test_from_files_with_lhs_termname(files, "rotl", just_8_result());
// }

#[test]
fn test_let() {
    test_from_file_self_contained("./tests/code/selfcontained/let.isle", just_8_result());
}
