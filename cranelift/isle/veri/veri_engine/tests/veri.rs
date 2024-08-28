mod utils;
use utils::{all_failure_result, all_success_result};
use utils::{
    test_aarch64_rule_with_lhs_termname_simple, test_aarch64_with_config_simple,
    test_concrete_aarch64_rule_with_lhs_termname, test_concrete_input_from_file_with_lhs_termname,
    test_from_file_with_config_simple, test_from_file_with_lhs_termname,
    test_from_file_with_lhs_termname_simple, test_x64_rule_with_lhs_termname_simple, Bitwidth,
    TestResult,
};
use veri_engine_lib::Config;
use veri_ir::{ConcreteInput, ConcreteTest, Counterexample, VerificationResult};

#[test]
fn test_named_iadd_base_concrete() {
    test_concrete_aarch64_rule_with_lhs_termname(
        "iadd_base_case",
        "iadd",
        ConcreteTest {
            termname: "iadd".to_string(),
            args: vec![
                ConcreteInput {
                    literal: "#b00000001".to_string(),
                    ty: veri_ir::Type::BitVector(Some(8)),
                },
                ConcreteInput {
                    literal: "#b00000001".to_string(),
                    ty: veri_ir::Type::BitVector(Some(8)),
                },
            ],
            output: ConcreteInput {
                literal: "#b00000010".to_string(),
                ty: veri_ir::Type::BitVector(Some(8)),
            },
        },
    )
}

#[test]
fn test_named_iadd_base() {
    test_aarch64_rule_with_lhs_termname_simple("iadd_base_case", "iadd", all_success_result())
}

#[test]
fn test_named_iadd_imm12_right() {
    test_aarch64_rule_with_lhs_termname_simple("iadd_imm12_right", "iadd", all_success_result())
}

#[test]
fn test_named_iadd_imm12_left() {
    test_aarch64_rule_with_lhs_termname_simple("iadd_imm12_left", "iadd", all_success_result())
}

#[test]
fn test_named_iadd_imm12_neg_left() {
    test_aarch64_rule_with_lhs_termname_simple(
        "iadd_imm12_neg_left",
        "iadd",
        vec![
            (Bitwidth::I8, VerificationResult::Success),
            (Bitwidth::I16, VerificationResult::Success),
            (Bitwidth::I32, VerificationResult::Success),
            (Bitwidth::I64, VerificationResult::Success),
        ],
    )
}

#[test]
fn test_named_iadd_imm12_neg_right() {
    test_aarch64_rule_with_lhs_termname_simple(
        "iadd_imm12_neg_right",
        "iadd",
        vec![
            (Bitwidth::I8, VerificationResult::Success),
            (Bitwidth::I16, VerificationResult::Success),
            (Bitwidth::I32, VerificationResult::Success),
            (Bitwidth::I64, VerificationResult::Success),
        ],
    )
}

// Need a file test because this is a change on top of our latest rebase
#[test]
fn test_named_imm12_from_negated_value() {
    test_aarch64_rule_with_lhs_termname_simple(
        "imm12_from_negated_value",
        "imm12_from_negated_value",
        vec![
            (Bitwidth::I8, VerificationResult::Success),
            (Bitwidth::I16, VerificationResult::Success),
            (Bitwidth::I32, VerificationResult::Success),
            (Bitwidth::I64, VerificationResult::Success),
        ],
    )
}

// Need a file test because this is a change on top of our latest rebase
#[test]
fn test_updated_iadd_imm12neg_right() {
    test_from_file_with_lhs_termname_simple(
        "./examples/iadd/updated_iadd_imm12neg_right.isle",
        "iadd".to_string(),
        all_success_result(),
    )
}

// Need a file test because this is a change on top of our latest rebase
#[test]
fn test_updated_iadd_imm12neg_left() {
    test_from_file_with_lhs_termname_simple(
        "./examples/iadd/updated_iadd_imm12neg_left.isle",
        "iadd".to_string(),
        all_success_result(),
    )
}

#[test]
fn test_named_iadd_extend_right() {
    test_aarch64_rule_with_lhs_termname_simple(
        "iadd_extend_right",
        "iadd",
        vec![
            (Bitwidth::I8, VerificationResult::Success),
            (Bitwidth::I16, VerificationResult::Success),
            (Bitwidth::I32, VerificationResult::Success),
            (Bitwidth::I64, VerificationResult::InapplicableRule),
        ],
    )
}

#[test]
fn test_named_iadd_extend_right_concrete() {
    test_concrete_aarch64_rule_with_lhs_termname(
        "iadd_extend_right",
        "iadd",
        ConcreteTest {
            termname: "iadd".to_string(),
            args: vec![
                ConcreteInput {
                    literal: "#b0000000000000001".to_string(),
                    ty: veri_ir::Type::BitVector(Some(16)),
                },
                ConcreteInput {
                    literal: "#b1111111111111111".to_string(),
                    ty: veri_ir::Type::BitVector(Some(16)),
                },
            ],
            output: ConcreteInput {
                literal: "#b0000000000000000".to_string(),
                ty: veri_ir::Type::BitVector(Some(16)),
            },
        },
    );
    test_concrete_aarch64_rule_with_lhs_termname(
        "iadd_extend_right",
        "iadd",
        ConcreteTest {
            termname: "iadd".to_string(),
            args: vec![
                ConcreteInput {
                    literal: "#b01000000000000000000000000000000".to_string(),
                    ty: veri_ir::Type::BitVector(Some(32)),
                },
                ConcreteInput {
                    literal: "#b00000000000000001111111111111111".to_string(),
                    ty: veri_ir::Type::BitVector(Some(32)),
                },
            ],
            output: ConcreteInput {
                literal: "#b01000000000000001111111111111111".to_string(),
                ty: veri_ir::Type::BitVector(Some(32)),
            },
        },
    )
}

#[test]
fn test_named_iadd_extend_left() {
    test_aarch64_rule_with_lhs_termname_simple(
        "iadd_extend_left",
        "iadd",
        vec![
            (Bitwidth::I8, VerificationResult::Success),
            (Bitwidth::I16, VerificationResult::Success),
            (Bitwidth::I32, VerificationResult::Success),
            (Bitwidth::I64, VerificationResult::InapplicableRule),
        ],
    )
}

#[test]
fn test_broken_iadd_extend() {
    test_from_file_with_lhs_termname_simple(
        "./examples/broken/iadd/broken_add_extend.isle",
        "iadd".to_string(),
        vec![
            // The type of the iadd is the destination type, so for i8 there is no bad extend-to
            (Bitwidth::I8, VerificationResult::Success),
            (
                Bitwidth::I16,
                VerificationResult::Failure(Counterexample {}),
            ),
            (
                Bitwidth::I32,
                VerificationResult::Failure(Counterexample {}),
            ),
            (Bitwidth::I64, VerificationResult::InapplicableRule),
        ],
    )
}

#[test]
fn test_named_iadd_ishl_left() {
    test_aarch64_rule_with_lhs_termname_simple("iadd_ishl_left", "iadd", all_success_result())
}

#[test]
fn test_named_iadd_ishl_right() {
    test_aarch64_rule_with_lhs_termname_simple("iadd_ishl_right", "iadd", all_success_result())
}

#[test]
fn test_named_iadd_imul_right() {
    test_aarch64_rule_with_lhs_termname_simple(
        "iadd_imul_right",
        "iadd",
        vec![
            (Bitwidth::I8, VerificationResult::Success),
            // Too slow right now: https://github.com/avanhatt/wasmtime/issues/36
            // (Bitwidth::I16, VerificationResult::Success),
            // (Bitwidth::I32, VerificationResult::Success),
            // (Bitwidth::I64, VerificationResult::Success),
        ],
    )
}

#[test]
#[ignore]
fn test_named_slow_iadd_imul_right() {
    test_aarch64_rule_with_lhs_termname_simple(
        "iadd_imul_right",
        "iadd",
        vec![
            (Bitwidth::I16, VerificationResult::Unknown),
            (Bitwidth::I32, VerificationResult::Unknown),
            (Bitwidth::I64, VerificationResult::Unknown),
        ],
    )
}

#[test]
fn test_named_iadd_imul_left() {
    test_aarch64_rule_with_lhs_termname_simple(
        "iadd_imul_left",
        "iadd",
        vec![
            (Bitwidth::I8, VerificationResult::Success),
            // Too slow right now: https://github.com/avanhatt/wasmtime/issues/36
            // (Bitwidth::I16, VerificationResult::Success),
            // (Bitwidth::I32, VerificationResult::Success),
            // (Bitwidth::I64, VerificationResult::Success),
        ],
    )
}

#[test]
#[ignore]
fn test_named_slow_iadd_imul_left() {
    test_aarch64_rule_with_lhs_termname_simple(
        "iadd_imul_left",
        "iadd",
        vec![
            (Bitwidth::I16, VerificationResult::Unknown),
            (Bitwidth::I32, VerificationResult::Unknown),
            (Bitwidth::I64, VerificationResult::Unknown),
        ],
    )
}

#[test]
fn test_named_isub_imul() {
    test_aarch64_rule_with_lhs_termname_simple(
        "isub_imul",
        "isub",
        vec![
            (Bitwidth::I8, VerificationResult::Success),
            // Too slow right now: https://github.com/avanhatt/wasmtime/issues/36
            // (Bitwidth::I16, VerificationResult::Success),
            // (Bitwidth::I32, VerificationResult::Success),
            // (Bitwidth::I64, VerificationResult::Success),
        ],
    )
}

#[test]
#[ignore]
fn test_named_slow_isub_imul() {
    test_aarch64_rule_with_lhs_termname_simple(
        "isub_imul",
        "isub",
        vec![
            // Too slow right now: https://github.com/avanhatt/wasmtime/issues/36
            (Bitwidth::I16, VerificationResult::Unknown),
            (Bitwidth::I32, VerificationResult::Unknown),
            (Bitwidth::I64, VerificationResult::Unknown),
        ],
    )
}

#[test]
fn test_broken_iadd_base_case() {
    test_from_file_with_lhs_termname_simple(
        "./examples/broken/iadd/broken_base_case.isle",
        "iadd".to_string(),
        all_failure_result(),
    )
}

#[test]
fn test_broken_iadd_imm12() {
    test_from_file_with_lhs_termname_simple(
        "./examples/broken/iadd/broken_imm12.isle",
        "iadd".to_string(),
        vec![
            (Bitwidth::I8, VerificationResult::Success),
            (
                Bitwidth::I16,
                VerificationResult::Failure(Counterexample {}),
            ),
            (
                Bitwidth::I32,
                VerificationResult::Failure(Counterexample {}),
            ),
            (
                Bitwidth::I64,
                VerificationResult::Failure(Counterexample {}),
            ),
        ],
    )
}

#[test]
fn test_broken_iadd_imm12_2() {
    test_from_file_with_lhs_termname_simple(
        "./examples/broken/iadd/broken_imm12_2.isle",
        "iadd".to_string(),
        vec![
            (Bitwidth::I8, VerificationResult::Failure(Counterexample {})),
            (
                Bitwidth::I16,
                VerificationResult::Failure(Counterexample {}),
            ),
            (
                Bitwidth::I32,
                VerificationResult::Failure(Counterexample {}),
            ),
            (
                Bitwidth::I64,
                VerificationResult::Failure(Counterexample {}),
            ),
        ],
    )
}

#[test]
fn test_broken_iadd_imm12neg_not_distinct() {
    test_from_file_with_lhs_termname_simple(
        "./examples/broken/iadd/broken_imm12neg.isle",
        "iadd".to_string(),
        vec![
            (Bitwidth::I8, VerificationResult::NoDistinctModels),
            (Bitwidth::I16, VerificationResult::NoDistinctModels),
            (Bitwidth::I32, VerificationResult::NoDistinctModels),
            (
                Bitwidth::I64,
                VerificationResult::Failure(Counterexample {}),
            ),
        ],
    )
}

#[test]
fn test_broken_iadd_imm12neg_2_not_distinct() {
    test_from_file_with_lhs_termname_simple(
        "./examples/broken/iadd/broken_imm12neg2.isle",
        "iadd".to_string(),
        vec![
            (Bitwidth::I8, VerificationResult::NoDistinctModels),
            (Bitwidth::I16, VerificationResult::NoDistinctModels),
            (Bitwidth::I32, VerificationResult::NoDistinctModels),
            (
                Bitwidth::I64,
                VerificationResult::Failure(Counterexample {}),
            ),
        ],
    )
}

#[test]
fn test_broken_iadd_imul_right() {
    test_from_file_with_lhs_termname_simple(
        "./examples/broken/iadd/broken_madd.isle",
        "iadd".to_string(),
        all_failure_result(),
    )
}

#[test]
fn test_broken_iadd_imul_left() {
    test_from_file_with_lhs_termname_simple(
        "./examples/broken/iadd/broken_madd2.isle",
        "iadd".to_string(),
        all_failure_result(),
    )
}

#[test]
fn test_broken_iadd_msub() {
    test_from_file_with_lhs_termname_simple(
        "./examples/broken/iadd/broken_msub.isle",
        "isub".to_string(),
        vec![
            (Bitwidth::I8, VerificationResult::Failure(Counterexample {})),
            (
                Bitwidth::I16,
                VerificationResult::Failure(Counterexample {}),
            ),
            (
                Bitwidth::I32,
                VerificationResult::Failure(Counterexample {}),
            ),
            (
                Bitwidth::I64,
                VerificationResult::Failure(Counterexample {}),
            ),
        ],
    )
}

#[test]
fn test_broken_iadd_shift() {
    test_from_file_with_lhs_termname_simple(
        "./examples/broken/iadd/broken_shift.isle",
        "iadd".to_string(),
        all_failure_result(),
    )
}

#[test]
fn test_broken_iadd_shift2() {
    test_from_file_with_lhs_termname_simple(
        "./examples/broken/iadd/broken_shift2.isle",
        "iadd".to_string(),
        all_failure_result(),
    )
}

#[test]
fn test_named_isub_base_case() {
    test_aarch64_rule_with_lhs_termname_simple("isub_base_case", "isub", all_success_result())
}

#[test]
fn test_named_isub_imm12() {
    test_aarch64_rule_with_lhs_termname_simple("isub_imm12", "isub", all_success_result())
}

#[test]
fn test_named_isub_imm12_concrete() {
    test_concrete_aarch64_rule_with_lhs_termname(
        "isub_imm12",
        "isub",
        ConcreteTest {
            termname: "isub".to_string(),
            args: vec![
                ConcreteInput {
                    literal: "#b00000001".to_string(),
                    ty: veri_ir::Type::BitVector(Some(8)),
                },
                ConcreteInput {
                    literal: "#b11111111".to_string(),
                    ty: veri_ir::Type::BitVector(Some(8)),
                },
            ],
            output: ConcreteInput {
                literal: "#b00000010".to_string(),
                ty: veri_ir::Type::BitVector(Some(8)),
            },
        },
    )
}

#[test]
fn test_named_isub_imm12_neg() {
    test_aarch64_rule_with_lhs_termname_simple(
        "isub_imm12_neg",
        "isub",
        vec![
            (Bitwidth::I8, VerificationResult::Success),
            (Bitwidth::I16, VerificationResult::Success),
            (Bitwidth::I32, VerificationResult::Success),
            (Bitwidth::I64, VerificationResult::Success),
        ],
    );
}

// The older version, which did not have distinct models for i8, i16, or i32.
#[test]
fn test_isub_imm12_neg_not_distinct() {
    test_from_file_with_lhs_termname_simple(
        "./examples/broken/isub/broken_imm12neg_not_distinct.isle",
        "isub".to_string(),
        vec![
            (Bitwidth::I8, VerificationResult::NoDistinctModels),
            (Bitwidth::I16, VerificationResult::NoDistinctModels),
            (Bitwidth::I32, VerificationResult::NoDistinctModels),
            (Bitwidth::I64, VerificationResult::Success),
        ],
    );
}

#[test]
fn test_isub_imm12_neg_not_distinct_16_32() {
    test_from_file_with_lhs_termname_simple(
        "./examples/broken/isub/broken_imm12neg_not_distinct.isle",
        "isub".to_string(),
        vec![
            (Bitwidth::I16, VerificationResult::NoDistinctModels),
            (Bitwidth::I32, VerificationResult::NoDistinctModels),
        ],
    );
}

// Need a file test because this is a change on top of our latest rebase
#[test]
fn test_isub_imm12neg_new() {
    test_from_file_with_lhs_termname_simple(
        "./examples/isub/imm12neg_new.isle",
        "isub".to_string(),
        all_success_result(),
    );
}

#[test]
fn test_named_isub_imm12_neg_concrete32() {
    test_concrete_aarch64_rule_with_lhs_termname(
        "isub_imm12_neg",
        "isub",
        ConcreteTest {
            termname: "isub".to_string(),
            args: vec![
                ConcreteInput {
                    literal: "#b0000000000000000000000000000000000000000000000000000000000000001"
                        .to_string(),
                    ty: veri_ir::Type::BitVector(Some(64)),
                },
                ConcreteInput {
                    literal: "#b1111111111111111111111111111111111111111111111111111111111111111"
                        .to_string(),
                    ty: veri_ir::Type::BitVector(Some(64)),
                },
            ],
            output: ConcreteInput {
                literal: "#b0000000000000000000000000000000000000000000000000000000000000010"
                    .to_string(),
                ty: veri_ir::Type::BitVector(Some(64)),
            },
        },
    )
}

#[test]
fn test_named_isub_imm12_neg_concrete64() {
    test_concrete_aarch64_rule_with_lhs_termname(
        "isub_imm12_neg",
        "isub",
        ConcreteTest {
            termname: "isub".to_string(),
            args: vec![
                ConcreteInput {
                    literal: "#b0000000000000000000000000000000000000000000000000000000000000001"
                        .to_string(),
                    ty: veri_ir::Type::BitVector(Some(64)),
                },
                ConcreteInput {
                    literal: "#b1111111111111111111111111111111111111111111111111111111111111111"
                        .to_string(),
                    ty: veri_ir::Type::BitVector(Some(64)),
                },
            ],
            output: ConcreteInput {
                literal: "#b0000000000000000000000000000000000000000000000000000000000000010"
                    .to_string(),
                ty: veri_ir::Type::BitVector(Some(64)),
            },
        },
    )
}

#[test]
fn test_named_isub_extend() {
    test_aarch64_rule_with_lhs_termname_simple(
        "isub_extend",
        "isub",
        vec![
            (Bitwidth::I8, VerificationResult::Success),
            (Bitwidth::I16, VerificationResult::Success),
            (Bitwidth::I32, VerificationResult::Success),
            (Bitwidth::I64, VerificationResult::InapplicableRule),
        ],
    )
}

#[test]
fn test_named_isub_ishl() {
    test_aarch64_rule_with_lhs_termname_simple("isub_ishl", "isub", all_success_result())
}

#[test]
fn test_broken_isub_base_case() {
    test_from_file_with_lhs_termname_simple(
        "./examples/broken/isub/broken_base_case.isle",
        "isub".to_string(),
        vec![
            (Bitwidth::I8, VerificationResult::Failure(Counterexample {})),
            (
                Bitwidth::I16,
                VerificationResult::Failure(Counterexample {}),
            ),
            (
                Bitwidth::I32,
                VerificationResult::Failure(Counterexample {}),
            ),
            (
                Bitwidth::I64,
                VerificationResult::Failure(Counterexample {}),
            ),
        ],
    );
}

#[test]
fn test_broken_isub_imm12() {
    test_from_file_with_lhs_termname_simple(
        "./examples/broken/isub/broken_imm12.isle",
        "isub".to_string(),
        vec![
            (Bitwidth::I8, VerificationResult::Success),
            (
                Bitwidth::I16,
                VerificationResult::Failure(Counterexample {}),
            ),
            (
                Bitwidth::I32,
                VerificationResult::Failure(Counterexample {}),
            ),
            (
                Bitwidth::I64,
                VerificationResult::Failure(Counterexample {}),
            ),
        ],
    );
}

#[test]
fn test_broken_isub_imm12neg_not_distinct() {
    test_from_file_with_lhs_termname_simple(
        "./examples/broken/isub/broken_imm12neg.isle",
        "isub".to_string(),
        vec![
            (Bitwidth::I8, VerificationResult::NoDistinctModels),
            (Bitwidth::I16, VerificationResult::NoDistinctModels),
            (Bitwidth::I32, VerificationResult::NoDistinctModels),
            (
                Bitwidth::I64,
                VerificationResult::Failure(Counterexample {}),
            ),
        ],
    );
}

#[test]
fn test_broken_isub_shift() {
    test_from_file_with_lhs_termname_simple(
        "./examples/broken/isub/broken_shift.isle",
        "isub".to_string(),
        all_failure_result(),
    );
}

#[test]
fn test_named_ineg_base_case() {
    test_aarch64_rule_with_lhs_termname_simple("ineg_base_case", "ineg", all_success_result())
}

#[test]
fn test_named_imul_base_case() {
    test_aarch64_rule_with_lhs_termname_simple(
        "imul_base_case",
        "imul",
        // Too slow right now: https://github.com/avanhatt/wasmtime/issues/36
        vec![
            (Bitwidth::I8, VerificationResult::Success),
            // (Bitwidth::I16, VerificationResult::Success),
            // (Bitwidth::I32, VerificationResult::Success),
            // (Bitwidth::I64, VerificationResult::Success),
        ],
    )
}

#[test]
#[ignore]
fn test_named_slow_imul_base_case() {
    test_aarch64_rule_with_lhs_termname_simple(
        "imul_base_case",
        "imul",
        // Too slow right now: https://github.com/avanhatt/wasmtime/issues/36
        vec![
            // (Bitwidth::I8, VerificationResult::Success),
            (Bitwidth::I16, VerificationResult::Unknown),
            (Bitwidth::I32, VerificationResult::Unknown),
            (Bitwidth::I64, VerificationResult::Unknown),
        ],
    )
}

// TODO traps https://github.com/avanhatt/wasmtime/issues/31
#[test]
fn test_named_udiv() {
    test_aarch64_rule_with_lhs_termname_simple(
        "udiv",
        "udiv",
        // Too slow right now: https://github.com/avanhatt/wasmtime/issues/36
        vec![
            (Bitwidth::I8, VerificationResult::Success),
            // (Bitwidth::I16, VerificationResult::Success),
            // (Bitwidth::I32, VerificationResult::Success),
            // (Bitwidth::I64, VerificationResult::Success),
        ],
    )
}

#[test]
#[ignore]
fn test_named_slow_udiv() {
    test_aarch64_rule_with_lhs_termname_simple(
        "udiv",
        "udiv",
        // Too slow right now: https://github.com/avanhatt/wasmtime/issues/36
        vec![
            // (Bitwidth::I8, VerificationResult::Success),
            (Bitwidth::I16, VerificationResult::Unknown),
            (Bitwidth::I32, VerificationResult::Unknown),
            (Bitwidth::I64, VerificationResult::Unknown),
        ],
    )
}

#[test]
fn test_broken_udiv() {
    test_from_file_with_lhs_termname_simple(
        "./examples/broken/udiv/broken_udiv.isle",
        "udiv".to_string(),
        vec![
            (Bitwidth::I8, VerificationResult::Failure(Counterexample {})),
            (
                Bitwidth::I16,
                VerificationResult::Failure(Counterexample {}),
            ),
            (
                Bitwidth::I32,
                VerificationResult::Failure(Counterexample {}),
            ),
            (Bitwidth::I64, VerificationResult::Success),
        ],
    )
}

#[test]
fn test_named_sdiv_base_case() {
    test_aarch64_rule_with_lhs_termname_simple(
        "sdiv_base_case",
        "sdiv",
        vec![
            (Bitwidth::I8, VerificationResult::Success),
            // Too slow right now: https://github.com/avanhatt/wasmtime/issues/36
            // (Bitwidth::I16, VerificationResult::Success),
            // (Bitwidth::I32, VerificationResult::Success),
            // (Bitwidth::I64, VerificationResult::Success),
        ],
    )
}

#[test]
#[ignore]
fn test_named_slow_sdiv_base_case() {
    test_aarch64_rule_with_lhs_termname_simple(
        "sdiv_base_case",
        "sdiv",
        vec![
            // Too slow right now: https://github.com/avanhatt/wasmtime/issues/36
            (Bitwidth::I16, VerificationResult::Unknown),
            (Bitwidth::I32, VerificationResult::Unknown),
            (Bitwidth::I64, VerificationResult::Unknown),
        ],
    )
}

#[test]
fn test_named_sdiv_safe_divisor() {
    test_aarch64_rule_with_lhs_termname_simple(
        "sdiv_safe_divisor",
        "sdiv",
        vec![
            (Bitwidth::I8, VerificationResult::Success),
            // Too slow right now: https://github.com/avanhatt/wasmtime/issues/36
            // (Bitwidth::I16, VerificationResult::Success),
            // (Bitwidth::I32, VerificationResult::Success),
            // (Bitwidth::I64, VerificationResult::Success),
        ],
    )
}

#[test]
#[ignore]
fn test_named_slow_sdiv_safe_divisor() {
    test_aarch64_rule_with_lhs_termname_simple(
        "sdiv_safe_divisor",
        "sdiv",
        vec![
            // Too slow right now: https://github.com/avanhatt/wasmtime/issues/36
            (Bitwidth::I16, VerificationResult::Unknown),
            (Bitwidth::I32, VerificationResult::Unknown),
            (Bitwidth::I64, VerificationResult::Unknown),
        ],
    )
}

#[test]
fn test_broken_sdiv_safe_const() {
    test_from_file_with_lhs_termname_simple(
        "./examples/broken/sdiv/broken_sdiv_safe_const.isle",
        "sdiv".to_string(),
        vec![
            (Bitwidth::I8, VerificationResult::Failure(Counterexample {})),
            (
                Bitwidth::I16,
                VerificationResult::Failure(Counterexample {}),
            ),
            (
                Bitwidth::I32,
                VerificationResult::Failure(Counterexample {}),
            ),
            (
                Bitwidth::I64,
                VerificationResult::Failure(Counterexample {}),
            ),
        ],
    )
}

#[test]
fn test_broken_sdiv() {
    test_from_file_with_lhs_termname_simple(
        "./examples/broken/sdiv/broken_sdiv.isle",
        "sdiv".to_string(),
        vec![
            (Bitwidth::I8, VerificationResult::Failure(Counterexample {})),
            (
                Bitwidth::I16,
                VerificationResult::Failure(Counterexample {}),
            ),
            (
                Bitwidth::I32,
                VerificationResult::Failure(Counterexample {}),
            ),
            (Bitwidth::I64, VerificationResult::Success),
        ],
    )
}

#[test]
fn test_named_srem() {
    test_aarch64_rule_with_lhs_termname_simple(
        "srem",
        "srem",
        vec![
            (Bitwidth::I8, VerificationResult::Success),
            // Too slow right now: https://github.com/avanhatt/wasmtime/issues/36
            // (Bitwidth::I16, VerificationResult::Success),
            // (Bitwidth::I32, VerificationResult::Success),
            // (Bitwidth::I64, VerificationResult::Success),
        ],
    )
}

#[test]
#[ignore]
fn test_named_slow_srem() {
    test_aarch64_rule_with_lhs_termname_simple(
        "srem",
        "srem",
        vec![
            (Bitwidth::I16, VerificationResult::Unknown),
            (Bitwidth::I32, VerificationResult::Unknown),
            (Bitwidth::I64, VerificationResult::Unknown),
        ],
    )
}

#[test]
fn test_named_urem() {
    test_aarch64_rule_with_lhs_termname_simple(
        "urem",
        "urem",
        vec![
            (Bitwidth::I8, VerificationResult::Success),
            // Too slow right now: https://github.com/avanhatt/wasmtime/issues/36
            // (Bitwidth::I16, VerificationResult::Success),
            // (Bitwidth::I32, VerificationResult::Success),
            // (Bitwidth::I64, VerificationResult::Success),
        ],
    )
}

#[test]
#[ignore]
fn test_named_slow_urem() {
    test_aarch64_rule_with_lhs_termname_simple(
        "urem",
        "urem",
        vec![
            (Bitwidth::I16, VerificationResult::Unknown),
            (Bitwidth::I32, VerificationResult::Unknown),
            (Bitwidth::I64, VerificationResult::Unknown),
        ],
    )
}

#[test]
fn test_named_urem_concrete() {
    test_concrete_aarch64_rule_with_lhs_termname(
        "urem",
        "urem",
        ConcreteTest {
            termname: "urem".to_string(),
            args: vec![
                ConcreteInput {
                    literal: "#b11111110".to_string(),
                    ty: veri_ir::Type::BitVector(Some(8)),
                },
                ConcreteInput {
                    literal: "#b00110001".to_string(),
                    ty: veri_ir::Type::BitVector(Some(8)),
                },
            ],
            output: ConcreteInput {
                literal: "#b00001001".to_string(),
                ty: veri_ir::Type::BitVector(Some(8)),
            },
        },
    )
}

#[test]
fn test_named_uextend() {
    test_aarch64_rule_with_lhs_termname_simple("uextend", "uextend", all_success_result())
}

#[test]
fn test_named_sextend() {
    test_aarch64_rule_with_lhs_termname_simple("sextend", "sextend", all_success_result())
}

#[test]
fn test_broken_uextend() {
    test_from_file_with_lhs_termname(
        "./examples/broken/broken_uextend.isle",
        "uextend".to_string(),
        TestResult::Expect(|sig| {
            // In the spec for extend, zero_extend and sign_extend are swapped.
            // However, this should still succeed if the input and output
            // widths are the same
            if sig.args[0] == sig.ret {
                VerificationResult::Success
            } else {
                VerificationResult::Failure(Counterexample {})
            }
        }),
    );
}

// AVH TODO: this rule requires priorities to be correct for narrow cases
// https://github.com/avanhatt/wasmtime/issues/32
#[test]
fn test_named_clz_32_64() {
    test_aarch64_rule_with_lhs_termname_simple(
        "clz_32_64",
        "clz",
        vec![
            // (Bitwidth::I8, VerificationResult::InapplicableRule),
            // (Bitwidth::I16, VerificationResult::InapplicableRule),
            (Bitwidth::I32, VerificationResult::Success),
            (Bitwidth::I64, VerificationResult::Success),
        ],
    )
}

#[test]
fn test_named_clz_8() {
    test_aarch64_rule_with_lhs_termname_simple(
        "clz_8",
        "clz",
        vec![
            (Bitwidth::I8, VerificationResult::Success),
            (Bitwidth::I16, VerificationResult::InapplicableRule),
            (Bitwidth::I32, VerificationResult::InapplicableRule),
            (Bitwidth::I64, VerificationResult::InapplicableRule),
        ],
    )
}

#[test]
fn test_named_clz_16() {
    test_aarch64_rule_with_lhs_termname_simple(
        "clz_16",
        "clz",
        vec![
            (Bitwidth::I8, VerificationResult::InapplicableRule),
            (Bitwidth::I16, VerificationResult::Success),
            (Bitwidth::I32, VerificationResult::InapplicableRule),
            (Bitwidth::I64, VerificationResult::InapplicableRule),
        ],
    )
}

#[test]
fn test_broken_clz() {
    test_from_file_with_lhs_termname_simple(
        "./examples/broken/clz/broken_clz.isle",
        "clz".to_string(),
        vec![
            (Bitwidth::I8, VerificationResult::Failure(Counterexample {})),
            (
                Bitwidth::I16,
                VerificationResult::Failure(Counterexample {}),
            ),
            (
                Bitwidth::I32,
                VerificationResult::Failure(Counterexample {}),
            ),
            (Bitwidth::I64, VerificationResult::Success),
        ],
    )
}

#[test]
fn test_broken_clz8() {
    test_from_file_with_lhs_termname_simple(
        "./examples/broken/clz/broken_clz8.isle",
        "clz".to_string(),
        vec![
            (Bitwidth::I8, VerificationResult::Failure(Counterexample {})),
            (Bitwidth::I16, VerificationResult::InapplicableRule),
            (Bitwidth::I32, VerificationResult::InapplicableRule),
            (Bitwidth::I64, VerificationResult::InapplicableRule),
        ],
    )
}

#[test]
fn test_broken_clz_n6() {
    test_from_file_with_lhs_termname_simple(
        "./examples/broken/clz/broken_clz16.isle",
        "clz".to_string(),
        vec![
            (Bitwidth::I8, VerificationResult::InapplicableRule),
            (
                Bitwidth::I16,
                VerificationResult::Failure(Counterexample {}),
            ),
            (Bitwidth::I32, VerificationResult::InapplicableRule),
            (Bitwidth::I64, VerificationResult::InapplicableRule),
        ],
    )
}

// AVH TODO: this rule requires priorities to be correct for narrow cases
// https://github.com/avanhatt/wasmtime/issues/32
#[test]
fn test_named_cls_32_64() {
    test_aarch64_rule_with_lhs_termname_simple(
        "cls_32_64",
        "cls",
        vec![
            // (Bitwidth::I8, VerificationResult::InapplicableRule),
            // (Bitwidth::I16, VerificationResult::InapplicableRule),
            (Bitwidth::I32, VerificationResult::Success),
            (Bitwidth::I64, VerificationResult::Success),
        ],
    )
}

#[test]
fn test_named_cls_8() {
    test_aarch64_rule_with_lhs_termname_simple(
        "cls_8",
        "cls",
        vec![
            (Bitwidth::I8, VerificationResult::Success),
            (Bitwidth::I16, VerificationResult::InapplicableRule),
            (Bitwidth::I32, VerificationResult::InapplicableRule),
            (Bitwidth::I64, VerificationResult::InapplicableRule),
        ],
    )
}

#[test]
fn test_named_cls_16() {
    test_aarch64_rule_with_lhs_termname_simple(
        "cls_16",
        "cls",
        vec![
            (Bitwidth::I8, VerificationResult::InapplicableRule),
            (Bitwidth::I16, VerificationResult::Success),
            (Bitwidth::I32, VerificationResult::InapplicableRule),
            (Bitwidth::I64, VerificationResult::InapplicableRule),
        ],
    )
}

#[test]
fn test_broken_cls_32_64() {
    test_from_file_with_lhs_termname_simple(
        "./examples/broken/cls/broken_cls.isle",
        "cls".to_string(),
        vec![
            (Bitwidth::I8, VerificationResult::Failure(Counterexample {})),
            (
                Bitwidth::I16,
                VerificationResult::Failure(Counterexample {}),
            ),
            (Bitwidth::I32, VerificationResult::Success),
            (Bitwidth::I64, VerificationResult::Success),
        ],
    )
}

#[test]
fn test_broken_cls_8() {
    test_from_file_with_lhs_termname_simple(
        "./examples/broken/cls/broken_cls8.isle",
        "cls".to_string(),
        vec![(Bitwidth::I8, VerificationResult::Failure(Counterexample {}))],
    )
}

#[test]
fn test_broken_cls_16() {
    test_from_file_with_lhs_termname_simple(
        "./examples/broken/cls/broken_cls16.isle",
        "cls".to_string(),
        vec![
            (Bitwidth::I8, VerificationResult::InapplicableRule),
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
fn test_named_ctz_32_64() {
    test_aarch64_rule_with_lhs_termname_simple(
        "ctz_32_64",
        "ctz",
        vec![
            // (Bitwidth::I8, VerificationResult::InapplicableRule),
            // (Bitwidth::I16, VerificationResult::InapplicableRule),
            (Bitwidth::I32, VerificationResult::Success),
            (Bitwidth::I64, VerificationResult::Success),
        ],
    )
}

#[test]
fn test_named_ctz_8() {
    test_aarch64_rule_with_lhs_termname_simple(
        "ctz_8",
        "ctz",
        vec![
            (Bitwidth::I8, VerificationResult::Success),
            (Bitwidth::I16, VerificationResult::InapplicableRule),
            (Bitwidth::I32, VerificationResult::InapplicableRule),
            (Bitwidth::I64, VerificationResult::InapplicableRule),
        ],
    )
}

#[test]
fn test_named_ctz_16() {
    test_aarch64_rule_with_lhs_termname_simple(
        "ctz_16",
        "ctz",
        vec![
            (Bitwidth::I8, VerificationResult::InapplicableRule),
            (Bitwidth::I16, VerificationResult::Success),
            (Bitwidth::I32, VerificationResult::InapplicableRule),
            (Bitwidth::I64, VerificationResult::InapplicableRule),
        ],
    )
}

#[test]
fn test_broken_ctz_32_64() {
    test_from_file_with_lhs_termname_simple(
        "./examples/broken/ctz/broken_ctz.isle",
        "clz".to_string(),
        vec![
            (Bitwidth::I8, VerificationResult::Failure(Counterexample {})),
            (
                Bitwidth::I16,
                VerificationResult::Failure(Counterexample {}),
            ),
            (
                Bitwidth::I32,
                VerificationResult::Failure(Counterexample {}),
            ),
            (
                Bitwidth::I64,
                VerificationResult::Failure(Counterexample {}),
            ),
        ],
    )
}

#[test]
fn test_broken_ctz_8() {
    test_from_file_with_lhs_termname_simple(
        "./examples/broken/ctz/broken_ctz8.isle",
        "ctz".to_string(),
        vec![
            (Bitwidth::I8, VerificationResult::Failure(Counterexample {})),
            (Bitwidth::I16, VerificationResult::InapplicableRule),
            (Bitwidth::I32, VerificationResult::InapplicableRule),
            (Bitwidth::I64, VerificationResult::InapplicableRule),
        ],
    )
}

#[test]
fn test_broken_ctz_16() {
    test_from_file_with_lhs_termname_simple(
        "./examples/broken/ctz/broken_ctz16.isle",
        "ctz".to_string(),
        vec![
            (Bitwidth::I8, VerificationResult::InapplicableRule),
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
fn test_named_small_rotr() {
    let config = Config {
        term: "small_rotr".to_string(),
        distinct_check: true,
        custom_assumptions: None,
        custom_verification_condition: Some(Box::new(|smt, args, lhs, rhs| {
            let ty_arg = *args.first().unwrap();
            let lower_8_bits_eq = {
                let mask = smt.atom("#x00000000000000FF");
                smt.eq(smt.bvand(mask, lhs), smt.bvand(mask, rhs))
            };
            let lower_16_bits_eq = {
                let mask = smt.atom("#x000000000000FFFF");
                smt.eq(smt.bvand(mask, lhs), smt.bvand(mask, rhs))
            };
            smt.ite(
                smt.eq(ty_arg, smt.atom("8")),
                lower_8_bits_eq,
                lower_16_bits_eq,
            )
        })),
        names: Some(vec!["small_rotr".to_string()]),
    };
    test_aarch64_with_config_simple(config, vec![(Bitwidth::I64, VerificationResult::Success)]);
}

#[test]
fn test_broken_small_rotr_to_shifts() {
    let config = Config {
        term: "small_rotr".to_string(),
        distinct_check: true,
        custom_assumptions: None,
        custom_verification_condition: Some(Box::new(|smt, args, lhs, rhs| {
            let ty_arg = *args.first().unwrap();
            let lower_8_bits_eq = {
                let mask = smt.atom("#x00000000000000FF");
                smt.eq(smt.bvand(mask, lhs), smt.bvand(mask, rhs))
            };
            let lower_16_bits_eq = {
                let mask = smt.atom("#x000000000000FFFF");
                smt.eq(smt.bvand(mask, lhs), smt.bvand(mask, rhs))
            };
            smt.ite(
                smt.eq(ty_arg, smt.atom("8")),
                lower_8_bits_eq,
                lower_16_bits_eq,
            )
        })),
        names: None,
    };
    test_from_file_with_config_simple(
        "./examples/broken/broken_mask_small_rotr.isle",
        config,
        vec![(
            Bitwidth::I64,
            VerificationResult::Failure(Counterexample {}),
        )],
    );
}

#[test]
fn test_broken_small_rotr_to_shifts_2() {
    let config = Config {
        term: "small_rotr".to_string(),
        distinct_check: true,
        custom_assumptions: None,
        custom_verification_condition: Some(Box::new(|smt, args, lhs, rhs| {
            let ty_arg = *args.first().unwrap();
            let lower_8_bits_eq = {
                let mask = smt.atom("#x00000000000000FF");
                smt.eq(smt.bvand(mask, lhs), smt.bvand(mask, rhs))
            };
            let lower_16_bits_eq = {
                let mask = smt.atom("#x000000000000FFFF");
                smt.eq(smt.bvand(mask, lhs), smt.bvand(mask, rhs))
            };
            smt.ite(
                smt.eq(ty_arg, smt.atom("8")),
                lower_8_bits_eq,
                lower_16_bits_eq,
            )
        })),
        names: None,
    };
    test_from_file_with_config_simple(
        "./examples/broken/broken_rule_or_small_rotr.isle",
        config,
        vec![(
            Bitwidth::I64,
            VerificationResult::Failure(Counterexample {}),
        )],
    );
}

#[test]
fn test_named_small_rotr_imm() {
    let config = Config {
        term: "small_rotr_imm".to_string(),
        distinct_check: true,
        custom_assumptions: None,
        custom_verification_condition: Some(Box::new(|smt, args, lhs, rhs| {
            let ty_arg = *args.first().unwrap();
            let lower_8_bits_eq = {
                let mask = smt.atom("#x00000000000000FF");
                smt.eq(smt.bvand(mask, lhs), smt.bvand(mask, rhs))
            };
            let lower_16_bits_eq = {
                let mask = smt.atom("#x000000000000FFFF");
                smt.eq(smt.bvand(mask, lhs), smt.bvand(mask, rhs))
            };
            smt.ite(
                smt.eq(ty_arg, smt.atom("8")),
                lower_8_bits_eq,
                lower_16_bits_eq,
            )
        })),
        names: Some(vec!["small_rotr_imm".to_string()]),
    };
    test_aarch64_with_config_simple(config, vec![(Bitwidth::I64, VerificationResult::Success)]);
}

#[test]
fn test_named_rotl_fits_in_16() {
    test_aarch64_rule_with_lhs_termname_simple(
        "rotl_fits_in_16",
        "rotl",
        vec![
            (Bitwidth::I8, VerificationResult::Success),
            (Bitwidth::I16, VerificationResult::Success),
            (Bitwidth::I32, VerificationResult::InapplicableRule),
            (Bitwidth::I64, VerificationResult::InapplicableRule),
        ],
    )
}

#[test]
fn test_named_rotl_32_base_case() {
    test_aarch64_rule_with_lhs_termname_simple(
        "rotl_32_base_case",
        "rotl",
        vec![
            (Bitwidth::I8, VerificationResult::InapplicableRule),
            (Bitwidth::I16, VerificationResult::InapplicableRule),
            (Bitwidth::I32, VerificationResult::Success),
            (Bitwidth::I64, VerificationResult::InapplicableRule),
        ],
    )
}

#[test]
fn test_broken_32_general_rotl_to_rotr() {
    test_from_file_with_lhs_termname_simple(
        "./examples/broken/broken_32_general_rotl_to_rotr.isle",
        "rotl".to_string(),
        vec![
            (Bitwidth::I8, VerificationResult::InapplicableRule),
            (Bitwidth::I16, VerificationResult::InapplicableRule),
            (
                Bitwidth::I32,
                VerificationResult::Failure(Counterexample {}),
            ),
            (Bitwidth::I64, VerificationResult::InapplicableRule),
        ],
    )
}

#[test]
fn test_named_rotl_64_base_case() {
    test_aarch64_rule_with_lhs_termname_simple(
        "rotl_64_base_case",
        "rotl",
        vec![
            (Bitwidth::I8, VerificationResult::InapplicableRule),
            (Bitwidth::I16, VerificationResult::InapplicableRule),
            (Bitwidth::I32, VerificationResult::InapplicableRule),
            (Bitwidth::I64, VerificationResult::Success),
        ],
    )
}

#[test]
fn test_broken_fits_in_16_rotl_to_rotr() {
    test_from_file_with_lhs_termname_simple(
        "./examples/broken/broken_fits_in_16_rotl_to_rotr.isle",
        "rotl".to_string(),
        vec![
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
fn test_named_rotl_fits_in_16_imm() {
    test_aarch64_rule_with_lhs_termname_simple(
        "rotl_fits_in_16_imm",
        "rotl",
        vec![
            (Bitwidth::I8, VerificationResult::Success),
            (Bitwidth::I16, VerificationResult::Success),
            (Bitwidth::I32, VerificationResult::InapplicableRule),
            (Bitwidth::I64, VerificationResult::InapplicableRule),
        ],
    )
}

#[test]
fn test_named_rotl_64_imm() {
    test_aarch64_rule_with_lhs_termname_simple(
        "rotl_64_imm",
        "rotl",
        vec![
            (Bitwidth::I8, VerificationResult::InapplicableRule),
            (Bitwidth::I16, VerificationResult::InapplicableRule),
            (Bitwidth::I32, VerificationResult::InapplicableRule),
            (Bitwidth::I64, VerificationResult::Success),
        ],
    )
}

#[test]
fn test_named_rotl_32_imm() {
    test_aarch64_rule_with_lhs_termname_simple(
        "rotl_32_imm",
        "rotl",
        vec![
            (Bitwidth::I8, VerificationResult::InapplicableRule),
            (Bitwidth::I16, VerificationResult::InapplicableRule),
            (Bitwidth::I32, VerificationResult::Success),
            (Bitwidth::I64, VerificationResult::InapplicableRule),
        ],
    )
}

#[test]
fn test_broken_fits_in_16_with_imm_rotl_to_rotr() {
    test_from_file_with_lhs_termname_simple(
        "./examples/broken/broken_fits_in_16_with_imm_rotl_to_rotr.isle",
        "rotl".to_string(),
        vec![
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
fn test_named_rotr_fits_in_16() {
    test_aarch64_rule_with_lhs_termname_simple(
        "rotr_fits_in_16",
        "rotr",
        vec![
            (Bitwidth::I8, VerificationResult::Success),
            (Bitwidth::I16, VerificationResult::Success),
            (Bitwidth::I32, VerificationResult::InapplicableRule),
            (Bitwidth::I64, VerificationResult::InapplicableRule),
        ],
    )
}

#[test]
fn test_named_rotr_fits_in_16_imm() {
    test_aarch64_rule_with_lhs_termname_simple(
        "rotr_fits_in_16_imm",
        "rotr",
        vec![
            (Bitwidth::I8, VerificationResult::Success),
            (Bitwidth::I16, VerificationResult::Success),
            (Bitwidth::I32, VerificationResult::InapplicableRule),
            (Bitwidth::I64, VerificationResult::InapplicableRule),
        ],
    )
}

#[test]
fn test_named_rotr_32_base_case() {
    test_aarch64_rule_with_lhs_termname_simple(
        "rotr_32_base_case",
        "rotr",
        vec![
            (Bitwidth::I8, VerificationResult::InapplicableRule),
            (Bitwidth::I16, VerificationResult::InapplicableRule),
            (Bitwidth::I32, VerificationResult::Success),
            (Bitwidth::I64, VerificationResult::InapplicableRule),
        ],
    )
}

#[test]
fn test_named_rotr_32_imm() {
    test_aarch64_rule_with_lhs_termname_simple(
        "rotr_32_imm",
        "rotr",
        vec![
            (Bitwidth::I8, VerificationResult::InapplicableRule),
            (Bitwidth::I16, VerificationResult::InapplicableRule),
            (Bitwidth::I32, VerificationResult::Success),
            (Bitwidth::I64, VerificationResult::InapplicableRule),
        ],
    )
}

#[test]
fn test_named_rotr_64_base_case() {
    test_aarch64_rule_with_lhs_termname_simple(
        "rotr_64_base_case",
        "rotr",
        vec![
            (Bitwidth::I8, VerificationResult::InapplicableRule),
            (Bitwidth::I16, VerificationResult::InapplicableRule),
            (Bitwidth::I32, VerificationResult::InapplicableRule),
            (Bitwidth::I64, VerificationResult::Success),
        ],
    )
}

#[test]
fn test_named_rotr_64_imm() {
    test_aarch64_rule_with_lhs_termname_simple(
        "rotr_64_imm",
        "rotr",
        vec![
            (Bitwidth::I8, VerificationResult::InapplicableRule),
            (Bitwidth::I16, VerificationResult::InapplicableRule),
            (Bitwidth::I32, VerificationResult::InapplicableRule),
            (Bitwidth::I64, VerificationResult::Success),
        ],
    )
}

#[test]
fn test_named_band_fits_in_64() {
    test_aarch64_rule_with_lhs_termname_simple(
        "band_fits_in_64",
        "band",
        vec![
            (Bitwidth::I8, VerificationResult::Success),
            (Bitwidth::I16, VerificationResult::Success),
            (Bitwidth::I32, VerificationResult::Success),
            (Bitwidth::I64, VerificationResult::Success),
        ],
    )
}

#[test]
fn test_broken_band_fits_in_32() {
    test_from_file_with_lhs_termname_simple(
        "./examples/broken/broken_fits_in_32_band.isle",
        "band".to_string(),
        vec![
            (Bitwidth::I8, VerificationResult::Failure(Counterexample {})),
            (
                Bitwidth::I16,
                VerificationResult::Failure(Counterexample {}),
            ),
            (
                Bitwidth::I32,
                VerificationResult::Failure(Counterexample {}),
            ),
            (Bitwidth::I64, VerificationResult::InapplicableRule),
        ],
    )
}

#[test]
fn test_named_bor_fits_in_64() {
    test_aarch64_rule_with_lhs_termname_simple(
        "bor_fits_in_64",
        "bor",
        vec![
            (Bitwidth::I8, VerificationResult::Success),
            (Bitwidth::I16, VerificationResult::Success),
            (Bitwidth::I32, VerificationResult::Success),
            (Bitwidth::I64, VerificationResult::Success),
        ],
    )
}

#[test]
fn test_broken_bor_fits_in_32() {
    test_from_file_with_lhs_termname_simple(
        "./examples/broken/broken_fits_in_32_bor.isle",
        "bor".to_string(),
        vec![
            (Bitwidth::I8, VerificationResult::Failure(Counterexample {})),
            (
                Bitwidth::I16,
                VerificationResult::Failure(Counterexample {}),
            ),
            (
                Bitwidth::I32,
                VerificationResult::Failure(Counterexample {}),
            ),
            (Bitwidth::I64, VerificationResult::InapplicableRule),
        ],
    )
}

#[test]
fn test_named_bxor_fits_in_64() {
    test_aarch64_rule_with_lhs_termname_simple(
        "bxor_fits_in_64",
        "bxor",
        vec![
            (Bitwidth::I8, VerificationResult::Success),
            (Bitwidth::I16, VerificationResult::Success),
            (Bitwidth::I32, VerificationResult::Success),
            (Bitwidth::I64, VerificationResult::Success),
        ],
    )
}

#[test]
fn test_named_band_not_right() {
    test_aarch64_rule_with_lhs_termname_simple(
        "band_not_right",
        "band",
        vec![
            (Bitwidth::I8, VerificationResult::Success),
            (Bitwidth::I16, VerificationResult::Success),
            (Bitwidth::I32, VerificationResult::Success),
            (Bitwidth::I64, VerificationResult::Success),
        ],
    )
}

#[test]
fn test_named_band_not_left() {
    test_aarch64_rule_with_lhs_termname_simple(
        "band_not_left",
        "band",
        vec![
            (Bitwidth::I8, VerificationResult::Success),
            (Bitwidth::I16, VerificationResult::Success),
            (Bitwidth::I32, VerificationResult::Success),
            (Bitwidth::I64, VerificationResult::Success),
        ],
    )
}

#[test]
fn test_named_bor_not_right() {
    test_aarch64_rule_with_lhs_termname_simple(
        "bor_not_right",
        "bor",
        vec![
            (Bitwidth::I8, VerificationResult::Success),
            (Bitwidth::I16, VerificationResult::Success),
            (Bitwidth::I32, VerificationResult::Success),
            (Bitwidth::I64, VerificationResult::Success),
        ],
    )
}

#[test]
fn test_named_bor_not_left() {
    test_aarch64_rule_with_lhs_termname_simple(
        "bor_not_left",
        "bor",
        vec![
            (Bitwidth::I8, VerificationResult::Success),
            (Bitwidth::I16, VerificationResult::Success),
            (Bitwidth::I32, VerificationResult::Success),
            (Bitwidth::I64, VerificationResult::Success),
        ],
    )
}

#[test]
fn test_named_bxor_not_right() {
    test_aarch64_rule_with_lhs_termname_simple(
        "bxor_not_right",
        "bxor",
        vec![
            (Bitwidth::I8, VerificationResult::Success),
            (Bitwidth::I16, VerificationResult::Success),
            (Bitwidth::I32, VerificationResult::Success),
            (Bitwidth::I64, VerificationResult::Success),
        ],
    )
}

#[test]
fn test_named_bxor_not_left() {
    test_aarch64_rule_with_lhs_termname_simple(
        "bxor_not_left",
        "bxor",
        vec![
            (Bitwidth::I8, VerificationResult::Success),
            (Bitwidth::I16, VerificationResult::Success),
            (Bitwidth::I32, VerificationResult::Success),
            (Bitwidth::I64, VerificationResult::Success),
        ],
    )
}

#[test]
fn test_named_bnot() {
    test_aarch64_rule_with_lhs_termname_simple("bnot_base_case", "bnot", all_success_result())
}

#[test]
fn test_named_bnot_ishl() {
    test_aarch64_rule_with_lhs_termname_simple("bnot_ishl", "bnot", all_success_result())
}

#[test]
fn test_named_ishl_64() {
    test_aarch64_rule_with_lhs_termname_simple(
        "ishl_64",
        "ishl",
        vec![
            (Bitwidth::I8, VerificationResult::InapplicableRule),
            (Bitwidth::I16, VerificationResult::InapplicableRule),
            (Bitwidth::I32, VerificationResult::InapplicableRule),
            (Bitwidth::I64, VerificationResult::Success),
        ],
    )
}

#[test]
fn test_named_ishl_64_concrete() {
    test_concrete_aarch64_rule_with_lhs_termname(
        "ishl_64",
        "ishl",
        ConcreteTest {
            termname: "ishl".to_string(),
            args: vec![
                ConcreteInput {
                    literal: "#b0000000000000000000000000000000000000000000000000000000000000001"
                        .to_string(),
                    ty: veri_ir::Type::BitVector(Some(64)),
                },
                ConcreteInput {
                    literal: "#b0000000000000000000000000000000000000000000000000000000000000010"
                        .to_string(),
                    ty: veri_ir::Type::BitVector(Some(64)),
                },
            ],
            output: ConcreteInput {
                literal: "#b0000000000000000000000000000000000000000000000000000000000000100"
                    .to_string(),
                ty: veri_ir::Type::BitVector(Some(64)),
            },
        },
    )
}

#[test]
fn test_named_ishl_fits_in_32() {
    test_aarch64_rule_with_lhs_termname_simple(
        "ishl_fits_in_32",
        "ishl",
        vec![
            (Bitwidth::I8, VerificationResult::Success),
            (Bitwidth::I16, VerificationResult::Success),
            (Bitwidth::I32, VerificationResult::Success),
            (Bitwidth::I64, VerificationResult::InapplicableRule),
        ],
    )
}

#[test]
fn test_named_ishl_fits_in_32_concrete() {
    test_concrete_aarch64_rule_with_lhs_termname(
        "ishl_fits_in_32",
        "ishl",
        ConcreteTest {
            termname: "ishl".to_string(),
            args: vec![
                ConcreteInput {
                    literal: "#b00000001".to_string(),
                    ty: veri_ir::Type::BitVector(Some(8)),
                },
                ConcreteInput {
                    literal: "#b00000010".to_string(),
                    ty: veri_ir::Type::BitVector(Some(8)),
                },
            ],
            output: ConcreteInput {
                literal: "#b00000100".to_string(),
                ty: veri_ir::Type::BitVector(Some(8)),
            },
        },
    )
}

#[test]
fn test_named_sshr_64() {
    test_aarch64_rule_with_lhs_termname_simple(
        "sshr_64",
        "sshr",
        vec![
            (Bitwidth::I8, VerificationResult::InapplicableRule),
            (Bitwidth::I16, VerificationResult::InapplicableRule),
            (Bitwidth::I32, VerificationResult::InapplicableRule),
            (Bitwidth::I64, VerificationResult::Success),
        ],
    )
}

#[test]
fn test_named_sshr_fits_in_32() {
    test_aarch64_rule_with_lhs_termname_simple(
        "sshr_fits_in_32",
        "sshr",
        vec![
            (Bitwidth::I8, VerificationResult::Success),
            (Bitwidth::I16, VerificationResult::Success),
            (Bitwidth::I32, VerificationResult::Success),
            (Bitwidth::I64, VerificationResult::InapplicableRule),
        ],
    )
}

#[test]
fn test_named_sshr_fits_in_32_concrete() {
    test_concrete_aarch64_rule_with_lhs_termname(
        "sshr_fits_in_32",
        "sshr",
        ConcreteTest {
            termname: "sshr".to_string(),
            args: vec![
                ConcreteInput {
                    literal: "#b10100000".to_string(),
                    ty: veri_ir::Type::BitVector(Some(8)),
                },
                ConcreteInput {
                    literal: "#b00000001".to_string(),
                    ty: veri_ir::Type::BitVector(Some(8)),
                },
            ],
            output: ConcreteInput {
                literal: "#b11010000".to_string(),
                ty: veri_ir::Type::BitVector(Some(8)),
            },
        },
    )
}

#[test]
fn test_named_ushr_64() {
    test_aarch64_rule_with_lhs_termname_simple(
        "ushr_64",
        "ushr",
        vec![
            (Bitwidth::I8, VerificationResult::InapplicableRule),
            (Bitwidth::I16, VerificationResult::InapplicableRule),
            (Bitwidth::I32, VerificationResult::InapplicableRule),
            (Bitwidth::I64, VerificationResult::Success),
        ],
    )
}

#[test]
fn test_named_ushr_fits_in_32() {
    test_aarch64_rule_with_lhs_termname_simple(
        "ushr_fits_in_32",
        "ushr",
        vec![
            (Bitwidth::I8, VerificationResult::Success),
            (Bitwidth::I16, VerificationResult::Success),
            (Bitwidth::I32, VerificationResult::Success),
            (Bitwidth::I64, VerificationResult::InapplicableRule),
        ],
    )
}

#[test]
fn test_named_ushr_fits_in_32_concrete() {
    test_concrete_aarch64_rule_with_lhs_termname(
        "ushr_fits_in_32",
        "ushr",
        ConcreteTest {
            termname: "ushr".to_string(),
            args: vec![
                ConcreteInput {
                    literal: "#b10100000".to_string(),
                    ty: veri_ir::Type::BitVector(Some(8)),
                },
                ConcreteInput {
                    literal: "#b00000001".to_string(),
                    ty: veri_ir::Type::BitVector(Some(8)),
                },
            ],
            output: ConcreteInput {
                literal: "#b01010000".to_string(),
                ty: veri_ir::Type::BitVector(Some(8)),
            },
        },
    )
}

#[test]
fn test_named_do_shift_64_base_case() {
    test_aarch64_rule_with_lhs_termname_simple(
        "do_shift_64_base_case",
        "do_shift",
        vec![
            (Bitwidth::I8, VerificationResult::InapplicableRule),
            (Bitwidth::I16, VerificationResult::InapplicableRule),
            (Bitwidth::I32, VerificationResult::InapplicableRule),
            (Bitwidth::I64, VerificationResult::Success),
        ],
    )
}

#[test]
fn test_named_do_shift_imm() {
    let config = Config {
        term: "do_shift".to_string(),
        distinct_check: true,
        custom_assumptions: None,
        custom_verification_condition: Some(Box::new(|smt, _args, lhs, rhs| {
            let lower_8_bits_eq = {
                let mask = smt.atom("#x00000000000000FF");
                smt.eq(smt.bvand(mask, lhs), smt.bvand(mask, rhs))
            };
            lower_8_bits_eq
        })),
        names: Some(vec!["do_shift_imm".to_string()]),
    };
    test_aarch64_with_config_simple(config, vec![(Bitwidth::I8, VerificationResult::Success)]);
    let config = Config {
        term: "do_shift".to_string(),
        distinct_check: true,
        custom_assumptions: None,
        custom_verification_condition: Some(Box::new(|smt, _args, lhs, rhs| {
            let lower_16_bits_eq = {
                let mask = smt.atom("#x000000000000FFFF");
                smt.eq(smt.bvand(mask, lhs), smt.bvand(mask, rhs))
            };
            lower_16_bits_eq
        })),
        names: Some(vec!["do_shift_imm".to_string()]),
    };
    test_aarch64_with_config_simple(config, vec![(Bitwidth::I16, VerificationResult::Success)]);
    let config = Config {
        term: "do_shift".to_string(),
        distinct_check: true,
        custom_assumptions: None,
        custom_verification_condition: Some(Box::new(|smt, _args, lhs, rhs| {
            let lower_32_bits_eq = {
                let mask = smt.atom("#x00000000FFFFFFFF");
                smt.eq(smt.bvand(mask, lhs), smt.bvand(mask, rhs))
            };
            lower_32_bits_eq
        })),
        names: Some(vec!["do_shift_imm".to_string()]),
    };
    test_aarch64_with_config_simple(config, vec![(Bitwidth::I32, VerificationResult::Success)]);
    test_aarch64_rule_with_lhs_termname_simple(
        "do_shift_imm",
        "do_shift",
        vec![(Bitwidth::I64, VerificationResult::Success)],
    )
}

#[test]
fn test_named_do_shift_fits_in_16() {
    let config = Config {
        term: "do_shift".to_string(),
        distinct_check: true,
        custom_assumptions: None,
        custom_verification_condition: Some(Box::new(|smt, args, lhs, rhs| {
            let ty_arg = args[1];
            let lower_8_bits_eq = {
                let mask = smt.atom("#x00000000000000FF");
                smt.eq(smt.bvand(mask, lhs), smt.bvand(mask, rhs))
            };
            let lower_16_bits_eq = {
                let mask = smt.atom("#x000000000000FFFF");
                smt.eq(smt.bvand(mask, lhs), smt.bvand(mask, rhs))
            };
            smt.ite(
                smt.eq(ty_arg, smt.atom("8")),
                lower_8_bits_eq,
                lower_16_bits_eq,
            )
        })),
        names: Some(vec!["do_shift_fits_in_16".to_string()]),
    };
    test_aarch64_with_config_simple(
        config,
        vec![
            (Bitwidth::I8, VerificationResult::Success),
            (Bitwidth::I16, VerificationResult::Success),
        ],
    );

    test_aarch64_rule_with_lhs_termname_simple(
        "do_shift_fits_in_16",
        "do_shift",
        vec![
            (Bitwidth::I32, VerificationResult::InapplicableRule),
            (Bitwidth::I64, VerificationResult::InapplicableRule),
        ],
    )
}

#[test]
fn test_named_do_shift_fits_in_16_concrete() {
    // (decl do_shift (ALUOp Type Reg Value) Reg)

    test_concrete_aarch64_rule_with_lhs_termname(
        "do_shift_fits_in_16",
        "do_shift",
        ConcreteTest {
            termname: "do_shift".to_string(),
            args: vec![
                ConcreteInput {
                    literal: "#x0e".to_string(),
                    ty: veri_ir::Type::BitVector(Some(8)),
                },
                ConcreteInput {
                    literal: "16".to_string(),
                    ty: veri_ir::Type::Int,
                },
                ConcreteInput {
                    literal: "#b0000000000000000000000000000000000000000000000000000000000000001"
                        .to_string(),
                    ty: veri_ir::Type::BitVector(Some(64)),
                },
                ConcreteInput {
                    literal: "#b0000000000000001".to_string(),
                    ty: veri_ir::Type::BitVector(Some(16)),
                },
            ],
            output: ConcreteInput {
                literal: "#b0000000000000000000000000000000000000000000000000000000000000010"
                    .to_string(),
                ty: veri_ir::Type::BitVector(Some(64)),
            },
        },
    )
}

#[test]
fn test_named_do_shift_32_base_case() {
    test_aarch64_rule_with_lhs_termname_simple(
        "do_shift_32_base_case",
        "do_shift",
        vec![
            (Bitwidth::I8, VerificationResult::InapplicableRule),
            (Bitwidth::I16, VerificationResult::InapplicableRule),
            (Bitwidth::I64, VerificationResult::InapplicableRule),
        ],
    );
    let config = Config {
        term: "do_shift".to_string(),
        distinct_check: true,
        custom_assumptions: None,
        custom_verification_condition: Some(Box::new(|smt, _args, lhs, rhs| {
            let lower_32_bits_eq = {
                let mask = smt.atom("#x00000000FFFFFFFF");
                smt.eq(smt.bvand(mask, lhs), smt.bvand(mask, rhs))
            };
            lower_32_bits_eq
        })),
        names: Some(vec!["do_shift_32_base_case".to_string()]),
    };
    test_aarch64_with_config_simple(config, vec![(Bitwidth::I32, VerificationResult::Success)]);
}

#[test]
fn test_broken_do_shift_32() {
    test_from_file_with_lhs_termname_simple(
        "./examples/broken/shifts/broken_do_shift_32.isle",
        "do_shift".to_string(),
        vec![
            (Bitwidth::I8, VerificationResult::InapplicableRule),
            (Bitwidth::I16, VerificationResult::InapplicableRule),
            (Bitwidth::I64, VerificationResult::InapplicableRule),
        ],
    );
    let config = Config {
        term: "do_shift".to_string(),
        distinct_check: true,
        custom_assumptions: None,
        custom_verification_condition: Some(Box::new(|smt, _args, lhs, rhs| {
            let lower_32_bits_eq = {
                let mask = smt.atom("#x00000000FFFFFFFF");
                smt.eq(smt.bvand(mask, lhs), smt.bvand(mask, rhs))
            };
            lower_32_bits_eq
        })),
        names: None,
    };
    test_from_file_with_config_simple(
        "./examples/broken/shifts/broken_do_shift_32.isle",
        config,
        vec![(
            Bitwidth::I32,
            VerificationResult::Failure(Counterexample {}),
        )],
    );
}

#[test]
fn test_broken_ishl_to_do_shift_64() {
    test_from_file_with_lhs_termname_simple(
        "./examples/broken/shifts/broken_ishl_to_do_shift_64.isle",
        "ishl".to_string(),
        vec![
            (Bitwidth::I8, VerificationResult::InapplicableRule),
            (Bitwidth::I16, VerificationResult::InapplicableRule),
            (Bitwidth::I32, VerificationResult::InapplicableRule),
            (
                Bitwidth::I64,
                VerificationResult::Failure(Counterexample {}),
            ),
        ],
    )
}

#[test]
fn test_broken_sshr_to_do_shift_fits_in_32() {
    test_from_file_with_lhs_termname_simple(
        "./examples/broken/shifts/broken_sshr_to_do_shift_fits_in_32.isle",
        "sshr".to_string(),
        vec![
            (Bitwidth::I8, VerificationResult::Failure(Counterexample {})),
            (
                Bitwidth::I16,
                VerificationResult::Failure(Counterexample {}),
            ),
            (
                Bitwidth::I32,
                VerificationResult::Failure(Counterexample {}),
            ),
            (Bitwidth::I64, VerificationResult::InapplicableRule),
        ],
    )
}

#[test]
fn test_broken_sshr_to_do_shift_fits_in_32_concrete() {
    test_concrete_input_from_file_with_lhs_termname(
        "./examples/broken/shifts/broken_sshr_to_do_shift_fits_in_32.isle",
        "sshr".to_string(),
        ConcreteTest {
            termname: "sshr".to_string(),
            args: vec![
                ConcreteInput {
                    literal: "#b10100000".to_string(),
                    ty: veri_ir::Type::BitVector(Some(8)),
                },
                ConcreteInput {
                    literal: "#b00000001".to_string(),
                    ty: veri_ir::Type::BitVector(Some(8)),
                },
            ],
            // Wrong output:
            output: ConcreteInput {
                literal: "#b01010000".to_string(),
                ty: veri_ir::Type::BitVector(Some(8)),
            },
        },
    )
}

#[test]
fn test_broken_ushr_to_do_shift_fits_in_32() {
    test_from_file_with_lhs_termname_simple(
        "./examples/broken/shifts/broken_ushr_to_do_shift_fits_in_32.isle",
        "ushr".to_string(),
        vec![
            (Bitwidth::I8, VerificationResult::Failure(Counterexample {})),
            (
                Bitwidth::I16,
                VerificationResult::Failure(Counterexample {}),
            ),
            (
                Bitwidth::I32,
                VerificationResult::Failure(Counterexample {}),
            ),
            (Bitwidth::I64, VerificationResult::InapplicableRule),
        ],
    )
}

#[test]
fn test_if_let() {
    test_from_file_with_lhs_termname_simple(
        "./examples/constructs/if-let.isle",
        "iadd".to_string(),
        all_success_result(),
    );
}

#[test]
fn test_named_icmp_8_16_32_64() {
    test_aarch64_rule_with_lhs_termname_simple(
        "icmp_8_16_32_64",
        "icmp",
        vec![
            (Bitwidth::I8, VerificationResult::Success),
            (Bitwidth::I16, VerificationResult::Success),
            (Bitwidth::I32, VerificationResult::Success),
            (Bitwidth::I64, VerificationResult::Success),
        ],
    )
}

#[test]
fn test_named_lower_icmp_into_reg_8_16_32_64() {
    test_aarch64_rule_with_lhs_termname_simple(
        "lower_icmp_into_reg_8_16_32_64",
        "lower_icmp_into_reg",
        vec![
            (Bitwidth::I8, VerificationResult::Success),
            (Bitwidth::I16, VerificationResult::Success),
            (Bitwidth::I32, VerificationResult::Success),
            (Bitwidth::I64, VerificationResult::Success),
        ],
    )
}

#[test]
fn test_named_lower_icmp_into_reg_8_16_32_64_concrete_1() {
    test_concrete_aarch64_rule_with_lhs_termname(
        "lower_icmp_into_reg_8_16_32_64",
        "lower_icmp_into_reg",
        ConcreteTest {
            termname: "lower_icmp_into_reg".to_string(),
            args: vec![
                ConcreteInput {
                    literal: "#b00000000".to_string(),
                    ty: veri_ir::Type::BitVector(Some(8)),
                },
                ConcreteInput {
                    literal: "#b00000000".to_string(),
                    ty: veri_ir::Type::BitVector(Some(8)),
                },
                ConcreteInput {
                    literal: "#b00000001".to_string(),
                    ty: veri_ir::Type::BitVector(Some(8)),
                },
                ConcreteInput {
                    literal: "8".to_string(),
                    ty: veri_ir::Type::Int,
                },
                ConcreteInput {
                    literal: "8".to_string(),
                    ty: veri_ir::Type::Int,
                },
            ],
            output: ConcreteInput {
                literal: "#b00000000".to_string(),
                ty: veri_ir::Type::BitVector(Some(8)),
            },
        },
    )
}

#[test]
fn test_named_lower_icmp_into_reg_8_16_32_64_concrete_2() {
    test_concrete_aarch64_rule_with_lhs_termname(
        "lower_icmp_into_reg_8_16_32_64",
        "lower_icmp_into_reg",
        ConcreteTest {
            termname: "lower_icmp_into_reg".to_string(),
            args: vec![
                ConcreteInput {
                    literal: "#b00000000".to_string(),
                    ty: veri_ir::Type::BitVector(Some(8)),
                },
                ConcreteInput {
                    literal: "#b00000000".to_string(),
                    ty: veri_ir::Type::BitVector(Some(8)),
                },
                ConcreteInput {
                    literal: "#b00000000".to_string(),
                    ty: veri_ir::Type::BitVector(Some(8)),
                },
                ConcreteInput {
                    literal: "8".to_string(),
                    ty: veri_ir::Type::Int,
                },
                ConcreteInput {
                    literal: "8".to_string(),
                    ty: veri_ir::Type::Int,
                },
            ],
            output: ConcreteInput {
                literal: "#b00000001".to_string(),
                ty: veri_ir::Type::BitVector(Some(8)),
            },
        },
    )
}

// Narrow types fail because of rule priorities
// https://github.com/avanhatt/wasmtime/issues/32
#[test]
fn test_named_lower_icmp_32_64() {
    test_aarch64_rule_with_lhs_termname_simple(
        "lower_icmp_32_64",
        "lower_icmp",
        vec![
            // (Bitwidth::I8, VerificationResult::Failure(Counterexample {})),
            // (
            //     Bitwidth::I16,
            //     VerificationResult::Failure(Counterexample {}),
            // ),
            (Bitwidth::I32, VerificationResult::Success),
            (Bitwidth::I64, VerificationResult::Success),
        ],
    )
}

#[test]
fn test_named_lower_icmp_8_16_signed() {
    test_aarch64_rule_with_lhs_termname_simple(
        "lower_icmp_8_16_signed",
        "lower_icmp",
        vec![
            (Bitwidth::I8, VerificationResult::Success),
            (Bitwidth::I16, VerificationResult::Success),
            (Bitwidth::I32, VerificationResult::InapplicableRule),
            (Bitwidth::I64, VerificationResult::InapplicableRule),
        ],
    )
}

// TODO AVH: Currently fails because needs priorities to show this
// only applies to unsigned cond codes
// https://github.com/avanhatt/wasmtime/issues/32
#[test]
fn test_named_lower_icmp_8_16_unsigned_imm() {
    test_aarch64_rule_with_lhs_termname_simple(
        "lower_icmp_8_16_unsigned_imm",
        "lower_icmp",
        vec![
            // (Bitwidth::I8, VerificationResult::Success),
            // (Bitwidth::I16, VerificationResult::Success),
            (Bitwidth::I32, VerificationResult::InapplicableRule),
            (Bitwidth::I64, VerificationResult::InapplicableRule),
        ],
    )
}

// TODO AVH: Currently fails because needs priorities to show this
// only applies to unsigned cond codes
// https://github.com/avanhatt/wasmtime/issues/32
#[test]
fn test_named_lower_icmp_8_16_unsigned() {
    test_aarch64_rule_with_lhs_termname_simple(
        "lower_icmp_8_16_unsigned",
        "lower_icmp",
        vec![
            // (Bitwidth::I8, VerificationResult::Success),
            // (Bitwidth::I16, VerificationResult::Success),
            (Bitwidth::I32, VerificationResult::InapplicableRule),
            (Bitwidth::I64, VerificationResult::InapplicableRule),
        ],
    )
}

// AVH TODO: this rule requires priorities to be correct for narrow cases
// https://github.com/avanhatt/wasmtime/issues/32
#[test]
fn test_named_lower_icmp_32_64_const() {
    test_aarch64_rule_with_lhs_termname_simple(
        "lower_icmp_32_64_const",
        "lower_icmp",
        vec![
            // (Bitwidth::I8, VerificationResult::InapplicableRule),
            // (Bitwidth::I16, VerificationResult::InapplicableRule),
            (Bitwidth::I32, VerificationResult::Success),
            (Bitwidth::I64, VerificationResult::Success),
        ],
    )
}

#[test]
fn test_named_lower_icmp_const_32_64_imm() {
    test_aarch64_rule_with_lhs_termname_simple(
        "lower_icmp_const_32_64_imm",
        "lower_icmp_const",
        vec![
            (Bitwidth::I8, VerificationResult::InapplicableRule),
            (Bitwidth::I16, VerificationResult::InapplicableRule),
            (Bitwidth::I32, VerificationResult::Success),
            (Bitwidth::I64, VerificationResult::Success),
        ],
    )
}

// AVH TODO: this rule requires priorities and a custom verification condition
// https://github.com/avanhatt/wasmtime/issues/32
#[test]
fn test_named_lower_icmp_const_32_64_sgte() {
    // Note: only one distinct condition code is matched on, so need to disable
    // distinctness check

    let config = Config {
        term: "lower_icmp_const".to_string(),
        distinct_check: false,
        custom_verification_condition: None,
        custom_assumptions: None,
        names: Some(vec!["lower_icmp_const_32_64_sgte".to_string()]),
    };
    test_aarch64_with_config_simple(
        config,
        vec![
            (Bitwidth::I8, VerificationResult::InapplicableRule),
            (Bitwidth::I16, VerificationResult::InapplicableRule),
            // Currently fails! The rewrite is not semantics-preserving
            (
                Bitwidth::I32,
                VerificationResult::Failure(Counterexample {}),
            ),
            (
                Bitwidth::I64,
                VerificationResult::Failure(Counterexample {}),
            ),
        ],
    )
}

// AVH TODO: this rule requires priorities and a custom verification condition
// https://github.com/avanhatt/wasmtime/issues/32
#[test]
fn test_named_lower_icmp_const_32_64_ugte() {
    // Note: only one distinct condition code is matched on, so need to disable
    // distinctness check

    let config = Config {
        term: "lower_icmp_const".to_string(),
        distinct_check: false,
        custom_verification_condition: None,
        custom_assumptions: None,
        names: Some(vec!["lower_icmp_const_32_64_ugte".to_string()]),
    };
    test_aarch64_with_config_simple(
        config,
        vec![
            (Bitwidth::I8, VerificationResult::InapplicableRule),
            (Bitwidth::I16, VerificationResult::InapplicableRule),
            // Currently fails! The rewrite is not semantics-preserving
            (
                Bitwidth::I32,
                VerificationResult::Failure(Counterexample {}),
            ),
            (
                Bitwidth::I64,
                VerificationResult::Failure(Counterexample {}),
            ),
        ],
    )
}

#[test]
fn test_named_lower_icmp_const_32_64() {
    test_aarch64_rule_with_lhs_termname_simple(
        "lower_icmp_const_32_64",
        "lower_icmp_const",
        vec![
            (Bitwidth::I8, VerificationResult::InapplicableRule),
            (Bitwidth::I16, VerificationResult::InapplicableRule),
            (Bitwidth::I32, VerificationResult::Success),
            (Bitwidth::I64, VerificationResult::Success),
        ],
    )
}

#[test]
fn test_named_umax() {
    test_aarch64_rule_with_lhs_termname_simple(
        "umax",
        "umax",
        vec![
            (Bitwidth::I8, VerificationResult::Success),
            (Bitwidth::I16, VerificationResult::Success),
            (Bitwidth::I32, VerificationResult::Success),
            (Bitwidth::I64, VerificationResult::Success),
        ],
    )
}

#[test]
fn test_named_smax() {
    test_aarch64_rule_with_lhs_termname_simple(
        "smax",
        "smax",
        vec![
            (Bitwidth::I8, VerificationResult::Success),
            (Bitwidth::I16, VerificationResult::Success),
            (Bitwidth::I32, VerificationResult::Success),
            (Bitwidth::I64, VerificationResult::Success),
        ],
    )
}

#[test]
fn test_named_umin() {
    test_aarch64_rule_with_lhs_termname_simple(
        "umin",
        "umin",
        vec![
            (Bitwidth::I8, VerificationResult::Success),
            (Bitwidth::I16, VerificationResult::Success),
            (Bitwidth::I32, VerificationResult::Success),
            (Bitwidth::I64, VerificationResult::Success),
        ],
    )
}

#[test]
fn test_named_smin() {
    test_aarch64_rule_with_lhs_termname_simple(
        "smin",
        "smin",
        vec![
            (Bitwidth::I8, VerificationResult::Success),
            (Bitwidth::I16, VerificationResult::Success),
            (Bitwidth::I32, VerificationResult::Success),
            (Bitwidth::I64, VerificationResult::Success),
        ],
    )
}

#[test]
fn test_named_iabs_64() {
    test_aarch64_rule_with_lhs_termname_simple(
        "iabs_64",
        "iabs",
        vec![
            (Bitwidth::I8, VerificationResult::InapplicableRule),
            (Bitwidth::I16, VerificationResult::InapplicableRule),
            (Bitwidth::I32, VerificationResult::InapplicableRule),
            (Bitwidth::I64, VerificationResult::Success),
        ],
    )
}

#[test]
fn test_named_iabs_8_16_32() {
    test_aarch64_rule_with_lhs_termname_simple(
        "iabs_8_16_32",
        "iabs",
        vec![
            (Bitwidth::I8, VerificationResult::Success),
            (Bitwidth::I16, VerificationResult::Success),
            (Bitwidth::I32, VerificationResult::Success),
            (Bitwidth::I64, VerificationResult::InapplicableRule),
        ],
    )
}

#[test]
fn test_named_bitselect() {
    test_aarch64_rule_with_lhs_termname_simple("bitselect", "bitselect", all_success_result())
}

#[test]
fn test_named_iconst() {
    test_aarch64_rule_with_lhs_termname_simple("iconst", "iconst", all_success_result())
}

#[test]
fn test_named_null() {
    test_aarch64_rule_with_lhs_termname_simple("null", "null", all_success_result())
}

// Can't currently verify because ConsumesFlags requires a non-functional
// interpretation
// #[test]
// fn test_named_cmp_and_choose_8_16() {
//
//         let config = Config {
//             dyn_width: false,
//             term: "cmp_and_choose".to_string(),
//             distinct_check: true,
//             custom_verification_condition: Some(Box::new(|smt, args, lhs, rhs| {
//                 let ty_arg = *args.first().unwrap();
//                 let lower_8_bits_eq = {
//                     let mask = smt.atom("#x00000000000000FF");
//                     smt.eq(smt.bvand(mask, lhs), smt.bvand(mask, rhs))
//                 };
//                 let lower_16_bits_eq = {
//                     let mask = smt.atom("#x000000000000FFFF");
//                     smt.eq(smt.bvand(mask, lhs), smt.bvand(mask, rhs))
//                 };
//                 smt.ite(
//                     smt.eq(ty_arg, smt.atom("8")),
//                     lower_8_bits_eq,
//                     lower_16_bits_eq,
//                 )
//             })),
//             names: Some(vec!["cmp_and_choose_8_16".to_string()]),
//         };
//         test_aarch64_with_config_simple(
//             config,
//             vec![
//                 (Bitwidth::I8, VerificationResult::Failure(Counterexample {  })),
//                 (Bitwidth::I16, VerificationResult::Failure(Counterexample {  })),
//                 (Bitwidth::I32, VerificationResult::InapplicableRule),
//                 (Bitwidth::I64, VerificationResult::InapplicableRule),
//             ],
//         );
//     })
// }

#[test]
fn test_named_popcnt_8() {
    test_aarch64_rule_with_lhs_termname_simple(
        "popcnt_8",
        "popcnt",
        vec![
            (Bitwidth::I8, VerificationResult::Success),
            (Bitwidth::I16, VerificationResult::InapplicableRule),
            (Bitwidth::I32, VerificationResult::InapplicableRule),
            (Bitwidth::I64, VerificationResult::InapplicableRule),
        ],
    )
}

#[test]
fn test_named_popcnt_16() {
    test_aarch64_rule_with_lhs_termname_simple(
        "popcnt_16",
        "popcnt",
        vec![
            (Bitwidth::I8, VerificationResult::InapplicableRule),
            (Bitwidth::I16, VerificationResult::Success),
            (Bitwidth::I32, VerificationResult::InapplicableRule),
            (Bitwidth::I64, VerificationResult::InapplicableRule),
        ],
    )
}

#[test]
fn test_named_popcnt_32() {
    test_aarch64_rule_with_lhs_termname_simple(
        "popcnt_32",
        "popcnt",
        vec![
            (Bitwidth::I8, VerificationResult::InapplicableRule),
            (Bitwidth::I16, VerificationResult::InapplicableRule),
            (Bitwidth::I32, VerificationResult::Success),
            (Bitwidth::I64, VerificationResult::InapplicableRule),
        ],
    )
}

// Currently too slow
// https://github.com/avanhatt/wasmtime/issues/36
#[test]
fn test_named_popcnt_64() {
    test_aarch64_rule_with_lhs_termname_simple(
        "popcnt_64",
        "popcnt",
        vec![
            (Bitwidth::I8, VerificationResult::InapplicableRule),
            (Bitwidth::I16, VerificationResult::InapplicableRule),
            (Bitwidth::I32, VerificationResult::InapplicableRule),
            // (Bitwidth::I64, VerificationResult::Success),
        ],
    )
}

// Currently too slow
// https://github.com/avanhatt/wasmtime/issues/36
#[test]
#[ignore]
fn test_named_slow_popcnt_64() {
    test_aarch64_rule_with_lhs_termname_simple(
        "popcnt_64",
        "popcnt",
        vec![(Bitwidth::I64, VerificationResult::Unknown)],
    )
}

#[test]
fn test_named_operand_size_32() {
    // Since there are no bitvectors in the signature, need a custom assumption
    // hook to pass through the value of the type argument

    static EXPECTED: [(Bitwidth, VerificationResult); 4] = [
        (Bitwidth::I8, VerificationResult::Success),
        (Bitwidth::I16, VerificationResult::Success),
        (Bitwidth::I32, VerificationResult::Success),
        (Bitwidth::I64, VerificationResult::InapplicableRule),
    ];
    for (ty, result) in &EXPECTED {
        let config = Config {
            term: "operand_size".to_string(),
            distinct_check: true,
            custom_verification_condition: None,
            custom_assumptions: Some(Box::new(|smt, args| {
                let ty_arg = *args.first().unwrap();
                smt.eq(ty_arg, smt.numeral(*ty as usize))
            })),
            names: Some(vec!["operand_size_32".to_string()]),
        };
        test_aarch64_with_config_simple(config, vec![(ty.clone(), result.clone())]);
    }
}

#[test]
fn test_named_operand_size_64() {
    // Since there are no bitvectors in the signature, need a custom assumption
    // hook to pass through the value of the type argument

    // Lower types precluded by priorities
    static EXPECTED: [(Bitwidth, VerificationResult); 1] = [
        // (Bitwidth::I8, VerificationResult::Success),
        // (Bitwidth::I16, VerificationResult::Success),
        // (Bitwidth::I32, VerificationResult::Success),
        (Bitwidth::I64, VerificationResult::Success),
    ];
    for (ty, result) in &EXPECTED {
        let config = Config {
            term: "operand_size".to_string(),
            distinct_check: true,
            custom_verification_condition: None,
            custom_assumptions: Some(Box::new(|smt, args| {
                let ty_arg = *args.first().unwrap();
                smt.eq(ty_arg, smt.numeral(*ty as usize))
            })),
            names: Some(vec!["operand_size_64".to_string()]),
        };
        test_aarch64_with_config_simple(config, vec![(ty.clone(), result.clone())]);
    }
}

#[test]
fn test_named_output_reg() {
    test_aarch64_rule_with_lhs_termname_simple(
        "output_reg",
        "output_reg",
        vec![
            (Bitwidth::I8, VerificationResult::Success),
            (Bitwidth::I16, VerificationResult::Success),
            (Bitwidth::I32, VerificationResult::Success),
            (Bitwidth::I64, VerificationResult::Success),
        ],
    )
}

#[test]
fn test_broken_imm_udiv_cve_underlying() {
    // Since there are no bitvectors in the signature, need a custom assumption
    // hook to pass through the value of the type argument

    static EXPECTED: [(Bitwidth, VerificationResult); 4] = [
        (Bitwidth::I8, VerificationResult::Failure(Counterexample {})),
        (
            Bitwidth::I16,
            VerificationResult::Failure(Counterexample {}),
        ),
        (
            Bitwidth::I32,
            VerificationResult::Failure(Counterexample {}),
        ),
        (Bitwidth::I64, VerificationResult::Success),
    ];
    for (ty, result) in &EXPECTED {
        let config = Config {
            term: "imm".to_string(),
            distinct_check: true,
            custom_verification_condition: None,
            custom_assumptions: Some(Box::new(|smt, args| {
                let ty_arg = *args.first().unwrap();
                smt.eq(ty_arg, smt.numeral(*ty as usize))
            })),
            names: None,
        };
        test_from_file_with_config_simple(
            "./examples/broken/udiv/udiv_cve_underlying.isle",
            config,
            vec![(ty.clone(), result.clone())],
        );
    }
}

#[test]
fn test_broken_imm_udiv_cve_underlying_32() {
    // Since there are no bitvectors in the signature, need a custom assumption
    // hook to pass through the value of the type argument

    static EXPECTED: [(Bitwidth, VerificationResult); 1] = [(
        Bitwidth::I32,
        VerificationResult::Failure(Counterexample {}),
    )];
    for (ty, result) in &EXPECTED {
        let config = Config {
            term: "imm".to_string(),
            distinct_check: true,
            custom_verification_condition: None,
            custom_assumptions: Some(Box::new(|smt, args| {
                let ty_arg = *args.first().unwrap();
                smt.eq(ty_arg, smt.numeral(*ty as usize))
            })),
            names: None,
        };
        test_from_file_with_config_simple(
            "./examples/broken/udiv/udiv_cve_underlying.isle",
            config,
            vec![(ty.clone(), result.clone())],
        );
    }
}

// x64

#[test]
fn test_named_x64_iadd_base_case_32_or_64_lea() {
    test_x64_rule_with_lhs_termname_simple(
        "iadd_base_case_32_or_64_lea",
        "iadd",
        vec![
            (Bitwidth::I8, VerificationResult::InapplicableRule),
            (Bitwidth::I16, VerificationResult::InapplicableRule),
            (Bitwidth::I32, VerificationResult::Success),
            (Bitwidth::I64, VerificationResult::Success),
        ],
    )
}

#[test]
fn test_named_x64_to_amode_add_base_case() {
    test_x64_rule_with_lhs_termname_simple(
        "to_amode_add_base_case",
        "to_amode_add",
        vec![(Bitwidth::I64, VerificationResult::Success)],
    )
}

#[test]
fn test_named_x64_to_amode_add_const_rhs() {
    test_x64_rule_with_lhs_termname_simple(
        "to_amode_add_const_rhs",
        "to_amode_add",
        vec![
            // TODO: make this work for I32
            // (Bitwidth::I32, VerificationResult::Success),
            (Bitwidth::I64, VerificationResult::Success),
        ],
    )
}

#[test]
fn test_named_x64_to_amode_add_const_lhs() {
    test_x64_rule_with_lhs_termname_simple(
        "to_amode_add_const_lhs",
        "to_amode_add",
        vec![(Bitwidth::I64, VerificationResult::Success)],
    )
}

#[test]
fn test_named_x64_to_amode_add_const_fold_iadd_lhs_rhs() {
    test_x64_rule_with_lhs_termname_simple(
        "to_amode_add_const_fold_iadd_lhs_rhs",
        "to_amode_add",
        vec![(Bitwidth::I64, VerificationResult::Success)],
    )
}

#[test]
fn test_named_x64_to_amode_add_const_fold_iadd_lhs_lhs() {
    test_x64_rule_with_lhs_termname_simple(
        "to_amode_add_const_fold_iadd_lhs_lhs",
        "to_amode_add",
        vec![(Bitwidth::I64, VerificationResult::Success)],
    )
}

#[test]
fn test_named_x64_to_amode_add_const_fold_iadd_rhs_rhs() {
    test_x64_rule_with_lhs_termname_simple(
        "to_amode_add_const_fold_iadd_rhs_rhs",
        "to_amode_add",
        vec![(Bitwidth::I64, VerificationResult::Success)],
    )
}

#[test]
fn test_named_x64_to_amode_add_const_fold_iadd_rhs_lhs() {
    test_x64_rule_with_lhs_termname_simple(
        "to_amode_add_const_fold_iadd_rhs_lhs",
        "to_amode_add",
        vec![(Bitwidth::I64, VerificationResult::Success)],
    )
}

#[test]
fn test_named_x64_amode_imm_reg_base() {
    test_x64_rule_with_lhs_termname_simple(
        "amode_imm_reg_base",
        "amode_imm_reg",
        vec![(Bitwidth::I64, VerificationResult::Success)],
    )
}

#[test]
fn test_named_x64_amode_imm_reg_iadd() {
    test_x64_rule_with_lhs_termname_simple(
        "amode_imm_reg_iadd",
        "amode_imm_reg",
        vec![(Bitwidth::I64, VerificationResult::Success)],
    )
}

#[test]
fn test_named_x64_amode_imm_reg_reg_shift_no_shift() {
    test_x64_rule_with_lhs_termname_simple(
        "amode_imm_reg_reg_shift_no_shift",
        "amode_imm_reg_reg_shift",
        vec![(Bitwidth::I64, VerificationResult::Success)],
    )
}

#[test]
fn test_named_x64_amode_imm_reg_reg_shift_shl_rhs() {
    test_x64_rule_with_lhs_termname_simple(
        "amode_imm_reg_reg_shift_shl_rhs",
        "amode_imm_reg_reg_shift",
        vec![(Bitwidth::I64, VerificationResult::Success)],
    )
}

#[test]
fn test_named_x64_amode_imm_reg_reg_shift_shl_lhs() {
    test_x64_rule_with_lhs_termname_simple(
        "amode_imm_reg_reg_shift_shl_lhs",
        "amode_imm_reg_reg_shift",
        vec![(Bitwidth::I64, VerificationResult::Success)],
    )
}

#[test]
fn test_named_load_i8_aarch64_uload8() {
    test_aarch64_rule_with_lhs_termname_simple(
        "load_i8_aarch64_uload8",
        "load",
        vec![
            (Bitwidth::I8, VerificationResult::Success),
            (Bitwidth::I16, VerificationResult::InapplicableRule),
            (Bitwidth::I32, VerificationResult::InapplicableRule),
            (Bitwidth::I64, VerificationResult::InapplicableRule),
        ],
    )
}

#[test]
fn test_named_load_i16_aarch64_uload16() {
    test_aarch64_rule_with_lhs_termname_simple(
        "load_i16_aarch64_uload16",
        "load",
        vec![
            (Bitwidth::I8, VerificationResult::InapplicableRule),
            (Bitwidth::I16, VerificationResult::Success),
            (Bitwidth::I32, VerificationResult::InapplicableRule),
            (Bitwidth::I64, VerificationResult::InapplicableRule),
        ],
    )
}

#[test]
fn test_named_load_i32_aarch64_uload32() {
    test_aarch64_rule_with_lhs_termname_simple(
        "load_i32_aarch64_uload32",
        "load",
        vec![
            (Bitwidth::I8, VerificationResult::InapplicableRule),
            (Bitwidth::I16, VerificationResult::InapplicableRule),
            (Bitwidth::I32, VerificationResult::Success),
            (Bitwidth::I64, VerificationResult::InapplicableRule),
        ],
    )
}

#[test]
fn test_named_load_i64_aarch64_uload64() {
    test_aarch64_rule_with_lhs_termname_simple(
        "load_i64_aarch64_uload64",
        "load",
        vec![
            (Bitwidth::I8, VerificationResult::InapplicableRule),
            (Bitwidth::I16, VerificationResult::InapplicableRule),
            (Bitwidth::I32, VerificationResult::InapplicableRule),
            (Bitwidth::I64, VerificationResult::Success),
        ],
    )
}

#[test]
fn test_named_store_i8_aarch64_store8() {
    test_aarch64_rule_with_lhs_termname_simple(
        "store_i8_aarch64_store8",
        "store",
        vec![
            (Bitwidth::I8, VerificationResult::Success),
            (Bitwidth::I16, VerificationResult::InapplicableRule),
            (Bitwidth::I32, VerificationResult::InapplicableRule),
            (Bitwidth::I64, VerificationResult::InapplicableRule),
        ],
    )
}

#[test]
fn test_named_store_i16_aarch64_store16() {
    test_aarch64_rule_with_lhs_termname_simple(
        "store_i16_aarch64_store16",
        "store",
        vec![
            (Bitwidth::I8, VerificationResult::InapplicableRule),
            (Bitwidth::I16, VerificationResult::Success),
            (Bitwidth::I32, VerificationResult::InapplicableRule),
            (Bitwidth::I64, VerificationResult::InapplicableRule),
        ],
    )
}

#[test]
fn test_named_store_i32_aarch64_store32() {
    test_aarch64_rule_with_lhs_termname_simple(
        "store_i32_aarch64_store32",
        "store",
        vec![
            (Bitwidth::I8, VerificationResult::InapplicableRule),
            (Bitwidth::I16, VerificationResult::InapplicableRule),
            (Bitwidth::I32, VerificationResult::Success),
            (Bitwidth::I64, VerificationResult::InapplicableRule),
        ],
    )
}

#[test]
fn test_named_store_i64_aarch64_store64() {
    test_aarch64_rule_with_lhs_termname_simple(
        "store_i64_aarch64_store64",
        "store",
        vec![
            (Bitwidth::I8, VerificationResult::InapplicableRule),
            (Bitwidth::I16, VerificationResult::InapplicableRule),
            (Bitwidth::I32, VerificationResult::InapplicableRule),
            (Bitwidth::I64, VerificationResult::Success),
        ],
    )
}

#[test]
fn test_named_load_sub64_x64_movzx() {
    test_x64_rule_with_lhs_termname_simple(
        "load_sub64_x64_movzx",
        "load",
        vec![
            (Bitwidth::I8, VerificationResult::Success),
            (Bitwidth::I16, VerificationResult::Success),
            (Bitwidth::I32, VerificationResult::Success),
            (Bitwidth::I64, VerificationResult::InapplicableRule),
        ],
    )
}

#[test]
fn test_named_store_x64_add_mem() {
    test_x64_rule_with_lhs_termname_simple(
        "store_x64_add_mem",
        "store",
        vec![
            (Bitwidth::I8, VerificationResult::InapplicableRule),
            (Bitwidth::I16, VerificationResult::InapplicableRule),
            (Bitwidth::I32, VerificationResult::Success),
            (Bitwidth::I64, VerificationResult::Success),
        ],
    )
}
#[test]
fn test_named_store_x64_movrm() {
    test_x64_rule_with_lhs_termname_simple(
        "store_x64_movrm",
        "store",
        vec![
            (Bitwidth::I8, VerificationResult::Success),
            (Bitwidth::I16, VerificationResult::Success),
            (Bitwidth::I32, VerificationResult::Success),
            (Bitwidth::I64, VerificationResult::Success),
        ],
    )
}

#[test]
fn test_load_conditional() {
    test_from_file_with_lhs_termname_simple(
        "./examples/load/load_conditional.isle",
        "lhs".to_string(),
        vec![
            (Bitwidth::I8, VerificationResult::Success),
            (Bitwidth::I16, VerificationResult::Success),
            (Bitwidth::I32, VerificationResult::Success),
            (Bitwidth::I64, VerificationResult::Success),
        ],
    );
}

#[test]
fn test_store_switch() {
    test_from_file_with_lhs_termname_simple(
        "./examples/store/store_switch.isle",
        "lhs".to_string(),
        vec![
            (Bitwidth::I8, VerificationResult::Success),
            (Bitwidth::I16, VerificationResult::Success),
            (Bitwidth::I32, VerificationResult::Success),
            (Bitwidth::I64, VerificationResult::Success),
        ],
    );
}

#[test]
#[should_panic]
fn test_load_add_panic() {
    test_from_file_with_lhs_termname_simple(
        "./examples/load/load_add_panic.isle",
        "lhs".to_string(),
        all_failure_result(),
    );
}

#[test]
fn test_broken_isub_store_with_load() {
    test_from_file_with_lhs_termname_simple(
        "./examples/store/broken_isub_store_with_load.isle",
        "store".to_string(),
        vec![
            (Bitwidth::I8, VerificationResult::InapplicableRule),
            (Bitwidth::I16, VerificationResult::InapplicableRule),
            (
                Bitwidth::I32,
                VerificationResult::Failure(Counterexample {}),
            ),
            (
                Bitwidth::I64,
                VerificationResult::Failure(Counterexample {}),
            ),
        ],
    );
}

#[test]
fn test_broken_bvsub_store_with_load() {
    test_from_file_with_lhs_termname_simple(
        "./examples/store/broken_bvsub_store_with_load.isle",
        "store".to_string(),
        vec![
            (Bitwidth::I8, VerificationResult::InapplicableRule),
            (Bitwidth::I16, VerificationResult::InapplicableRule),
            (
                Bitwidth::I32,
                VerificationResult::Failure(Counterexample {}),
            ),
            (
                Bitwidth::I64,
                VerificationResult::Failure(Counterexample {}),
            ),
        ],
    );
}
