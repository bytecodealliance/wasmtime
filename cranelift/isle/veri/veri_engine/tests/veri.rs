mod utils;
use utils::{
    all_failure_result, all_success_result, custom_result, just_8_result, lte_64_success_result,
};
use utils::{
    test_from_file, test_from_file_custom_prelude, test_from_file_self_contained,
    test_from_file_with_filter, test_from_files_with_lhs_termname, Bitwidth,
};
use veri_ir::{Counterexample, VerificationResult};

#[test]
fn test_iadds() {
    /*test_from_file_custom_prelude(
        "./tests/code/selfcontained/simple_prelude.isle",
        "./tests/code/selfcontained/simple_iadd.isle",
        lte_64_success_result(),
    );

    test_from_file_custom_prelude(
        "./tests/code/selfcontained/simple_prelude.isle",
        "./tests/code/selfcontained/iadd_to_sub.isle",
        lte_64_success_result(),
    );*/

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
    test_from_file("./examples/iadd/base_case.isle", lte_64_success_result());
    test_from_file("./examples/iadd/madd.isle", lte_64_success_result());
    test_from_file("./examples/iadd/madd2.isle", lte_64_success_result());
    test_from_file("./examples/iadd/msub.isle", lte_64_success_result());
/*
    test_from_file("./examples/iadd/", lte_64_success_result());
    test_from_file("./examples/iadd/", lte_64_success_result());
    test_from_file("./examples/iadd/", lte_64_success_result());
    test_from_file("./examples/iadd/", lte_64_success_result());

    test_from_file("./examples/iadd/", lte_64_success_result());
    test_from_file("./examples/iadd/", lte_64_success_result());
    test_from_file("./examples/iadd/", lte_64_success_result());
    test_from_file("./examples/iadd/", lte_64_success_result());
    */
}

#[test]
fn test_broken_iadd_from_file() {
    test_from_file("./examples/broken/iadd/broken_base_case.isle", all_failure_result());
    test_from_file("./examples/broken/iadd/broken_madd.isle", all_failure_result());
    test_from_file("./examples/broken/iadd/broken_madd2.isle", all_failure_result());
    test_from_file("./examples/broken/iadd/broken_msub.isle",
        vec![
            (Bitwidth::I1, VerificationResult::Success),
            (Bitwidth::I8, VerificationResult::Failure(Counterexample {})),
            (
                Bitwidth::I16,
                VerificationResult::Failure(Counterexample {}),
            ),
            (Bitwidth::I32, VerificationResult::Failure(Counterexample{})),
            (Bitwidth::I64, VerificationResult::Failure(Counterexample{})),
        ],
    );
}

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
    test_from_file(
        "./examples/broken/broken_uextend.isle",
        custom_result(&|w| {
            (
                w,
                if (w as usize) < 64 {
                    VerificationResult::Failure(Counterexample {})
                } else {
                    VerificationResult::Success
                },
            )
        }),
    )
}

#[test]
fn test_small_rotr_to_shifts() {
    test_from_file_with_filter(
        "./examples/small_rotr_to_shifts.isle",
        "small_rotr".to_string(),
        vec![
            (Bitwidth::I1, VerificationResult::Success),
            (Bitwidth::I8, VerificationResult::Success),
            (Bitwidth::I16, VerificationResult::Success),
            (Bitwidth::I32, VerificationResult::InapplicableRule),
            (Bitwidth::I64, VerificationResult::InapplicableRule),
        ],
    )
}

#[test]
fn test_small_rotr_to_shifts_broken() {
    test_from_file_with_filter(
        "./examples/broken/broken_mask_small_rotr.isle",
        "small_rotr".to_string(),
        vec![
            (Bitwidth::I1, VerificationResult::Success),
            (Bitwidth::I8, VerificationResult::Failure(Counterexample {})),
            (
                Bitwidth::I16,
                VerificationResult::Failure(Counterexample {}),
            ),
            (Bitwidth::I32, VerificationResult::InapplicableRule),
            (Bitwidth::I64, VerificationResult::InapplicableRule),
        ],
    );
    test_from_file_with_filter(
        "./examples/broken/broken_rule_or_small_rotr.isle",
        "small_rotr".to_string(),
        vec![
            (Bitwidth::I1, VerificationResult::Failure(Counterexample {})),
            (Bitwidth::I8, VerificationResult::Failure(Counterexample {})),
            (
                Bitwidth::I16,
                VerificationResult::Failure(Counterexample {}),
            ),
            (Bitwidth::I32, VerificationResult::InapplicableRule),
            (Bitwidth::I64, VerificationResult::InapplicableRule),
        ],
    )
}

#[test]
fn test_small_rotr_imm_to_shifts() {
    test_from_file_with_filter(
        "./examples/small_rotr_imm_to_shifts.isle",
        "small_rotr_imm".to_string(),
        vec![
            (Bitwidth::I1, VerificationResult::Success),
            (Bitwidth::I8, VerificationResult::Success),
            (Bitwidth::I16, VerificationResult::Success),
            (Bitwidth::I32, VerificationResult::InapplicableRule),
            (Bitwidth::I64, VerificationResult::InapplicableRule),
        ],
    )
}

#[test]
fn test_fits_in_16_rotl_to_rotr() {
    test_from_file_with_filter(
        "./examples/fits_in_16_rotl_to_rotr.isle",
        "rotl".to_string(),
        vec![
            (Bitwidth::I1, VerificationResult::Success),
            (Bitwidth::I8, VerificationResult::Success),
            (Bitwidth::I16, VerificationResult::Success),
            (Bitwidth::I32, VerificationResult::InapplicableRule),
            (Bitwidth::I64, VerificationResult::InapplicableRule),
        ],
    )
}

#[test]
fn test_32_general_rotl_to_rotr() {
    test_from_file_with_filter(
        "./examples/32_general_rotl_to_rotr.isle",
        "rotl".to_string(),
        vec![
            (Bitwidth::I1, VerificationResult::InapplicableRule),
            (Bitwidth::I8, VerificationResult::InapplicableRule),
            (Bitwidth::I16, VerificationResult::InapplicableRule),
            (Bitwidth::I32, VerificationResult::Success),
            (Bitwidth::I64, VerificationResult::InapplicableRule),
        ],
    )
}

#[test]
fn test_broken_32_general_rotl_to_rotr() {
    test_from_file_with_filter(
        "./examples/broken/broken_32_general_rotl_to_rotr.isle",
        "rotl".to_string(),
        vec![
            (Bitwidth::I1, VerificationResult::InapplicableRule),
            (Bitwidth::I8, VerificationResult::InapplicableRule),
            (Bitwidth::I16, VerificationResult::InapplicableRule),
            (Bitwidth::I32, VerificationResult::Failure(Counterexample{})),
            (Bitwidth::I64, VerificationResult::InapplicableRule),
        ],
    )
}


#[test]
fn test_64_general_rotl_to_rotr() {
    test_from_file_with_filter(
        "./examples/64_general_rotl_to_rotr.isle",
        "rotl".to_string(),
        vec![
            (Bitwidth::I1, VerificationResult::InapplicableRule),
            (Bitwidth::I8, VerificationResult::InapplicableRule),
            (Bitwidth::I16, VerificationResult::InapplicableRule),
            (Bitwidth::I32, VerificationResult::InapplicableRule),
            (Bitwidth::I64, VerificationResult::Success),
        ],
    )
}

#[test]
fn test_broken_fits_in_16_rotl_to_rotr() {
    test_from_file_with_filter(
        "./examples/broken/broken_fits_in_16_rotl_to_rotr.isle",
        "rotl".to_string(),
        vec![
            (Bitwidth::I1, VerificationResult::Success),
            (Bitwidth::I8, VerificationResult::Failure(Counterexample {})),
            (
                Bitwidth::I16,
                VerificationResult::Failure(Counterexample {}),
            ),
            (Bitwidth::I32, VerificationResult::InapplicableRule),
            (Bitwidth::I64, VerificationResult::InapplicableRule),
        ],
    )
}

#[test]
fn test_fits_in_16_with_imm_rotl_to_rotr() {
    test_from_file_with_filter(
        "./examples/fits_in_16_with_imm_rotl_to_rotr.isle",
        "rotl".to_string(),
        vec![
            (Bitwidth::I1, VerificationResult::Success),
            (Bitwidth::I8, VerificationResult::Success),
            (Bitwidth::I16, VerificationResult::Success),
            (Bitwidth::I32, VerificationResult::InapplicableRule),
            (Bitwidth::I64, VerificationResult::InapplicableRule),
        ],
    )
}

#[test]
fn test_64_with_imm_rotl_to_rotr() {
    test_from_file_with_filter(
        "./examples/64_with_imm_rotl_to_rotr.isle",
        "rotl".to_string(),
        vec![
            (Bitwidth::I1, VerificationResult::InapplicableRule),
            (Bitwidth::I8, VerificationResult::InapplicableRule),
            (Bitwidth::I16, VerificationResult::InapplicableRule),
            (Bitwidth::I32, VerificationResult::InapplicableRule),
            (Bitwidth::I64, VerificationResult::Success),
        ],
    )
}

#[test]
fn test_32_with_imm_rotl_to_rotr() {
    test_from_file_with_filter(
        "./examples/32_with_imm_rotl_to_rotr.isle",
        "rotl".to_string(),
        vec![
            (Bitwidth::I1, VerificationResult::InapplicableRule),
            (Bitwidth::I8, VerificationResult::InapplicableRule),
            (Bitwidth::I16, VerificationResult::InapplicableRule),
            (Bitwidth::I32, VerificationResult::Success),
            (Bitwidth::I64, VerificationResult::InapplicableRule),
        ],
    )
}

#[test]
fn test_broken_fits_in_16_with_imm_rotl_to_rotr() {
    test_from_file_with_filter(
        "./examples/broken/broken_fits_in_16_with_imm_rotl_to_rotr.isle",
        "rotl".to_string(),
        vec![
            (Bitwidth::I1, VerificationResult::Success),
            (Bitwidth::I8, VerificationResult::Failure(Counterexample {})),
            (Bitwidth::I16, VerificationResult::Failure(Counterexample {})),
            (Bitwidth::I32, VerificationResult::InapplicableRule),
            (Bitwidth::I64, VerificationResult::InapplicableRule),
        ],
    )
}

#[test]
fn test_fits_in_16_rotr() {
    test_from_file_with_filter(
        "./examples/fits_in_16_rotr.isle",
        "rotr".to_string(),
        vec![
            (Bitwidth::I1, VerificationResult::Success),
            (Bitwidth::I8, VerificationResult::Success),
            (Bitwidth::I16, VerificationResult::Success),
            (Bitwidth::I32, VerificationResult::InapplicableRule),
            (Bitwidth::I64, VerificationResult::InapplicableRule),
        ],
    )
}

#[test]
fn test_fits_in_16_with_imm_rotr() {
    test_from_file_with_filter(
        "./examples/fits_in_16_rotr.isle",
        "rotr".to_string(),
        vec![
            (Bitwidth::I1, VerificationResult::Success),
            (Bitwidth::I8, VerificationResult::Success),
            (Bitwidth::I16, VerificationResult::Success),
            (Bitwidth::I32, VerificationResult::InapplicableRule),
            (Bitwidth::I64, VerificationResult::InapplicableRule),
        ],
    )
}

#[test]
fn test_32_rotr() {
    test_from_file_with_filter(
        "./examples/32_rotr.isle",
        "rotr".to_string(),
        vec![
            (Bitwidth::I1, VerificationResult::InapplicableRule),
            (Bitwidth::I8, VerificationResult::InapplicableRule),
            (Bitwidth::I16, VerificationResult::InapplicableRule),
            (Bitwidth::I32, VerificationResult::Success),
            (Bitwidth::I64, VerificationResult::InapplicableRule),
        ],
    )
}

#[test]
fn test_32_with_imm_rotr() {
    test_from_file_with_filter(
        "./examples/32_with_imm_rotr.isle",
        "rotr".to_string(),
        vec![
            (Bitwidth::I1, VerificationResult::InapplicableRule),
            (Bitwidth::I8, VerificationResult::InapplicableRule),
            (Bitwidth::I16, VerificationResult::InapplicableRule),
            (Bitwidth::I32, VerificationResult::Success),
            (Bitwidth::I64, VerificationResult::InapplicableRule),
        ],
    )
}

#[test]
fn test_64_rotr() {
    test_from_file_with_filter(
        "./examples/64_rotr.isle",
        "rotr".to_string(),
        vec![
            (Bitwidth::I1, VerificationResult::InapplicableRule),
            (Bitwidth::I8, VerificationResult::InapplicableRule),
            (Bitwidth::I16, VerificationResult::InapplicableRule),
            (Bitwidth::I32, VerificationResult::InapplicableRule),
            (Bitwidth::I64, VerificationResult::Success),
        ],
    )
}

#[test]
fn test_64_with_imm_rotr() {
    test_from_file_with_filter(
        "./examples/64_with_imm_rotr.isle",
        "rotr".to_string(),
        vec![
            (Bitwidth::I1, VerificationResult::InapplicableRule),
            (Bitwidth::I8, VerificationResult::InapplicableRule),
            (Bitwidth::I16, VerificationResult::InapplicableRule),
            (Bitwidth::I32, VerificationResult::InapplicableRule),
            (Bitwidth::I64, VerificationResult::Success),
        ],
    )
}

#[test]
fn test_if_let() {
    test_from_file("./examples/constructs/if-let.isle", all_success_result());
}


#[test]
fn test_let() {
    test_from_file_self_contained("./tests/code/selfcontained/let.isle", just_8_result());
}
