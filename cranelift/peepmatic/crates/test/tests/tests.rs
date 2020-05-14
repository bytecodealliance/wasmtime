use peepmatic_runtime::{
    cc::ConditionCode,
    operator::Operator,
    part::Constant,
    r#type::{BitWidth, Type},
};
use peepmatic_test::*;

const TEST_ISA: TestIsa = TestIsa {
    native_word_size_in_bits: 32,
};

macro_rules! optimizer {
    ($opts:ident, $source:expr) => {{
        let _ = env_logger::try_init();
        $opts = peepmatic::compile_str($source, std::path::Path::new("peepmatic-test")).unwrap();
        $opts.optimizer(TEST_ISA)
    }};
}

#[test]
fn opcode() {
    let opts;
    let mut optimizer = optimizer!(opts, "(=> (iadd $x 0) $x)");

    let mut program = Program::default();
    let five = program.r#const(Constant::Int(5, BitWidth::ThirtyTwo), BitWidth::ThirtyTwo);
    let zero = program.r#const(Constant::Int(0, BitWidth::ThirtyTwo), BitWidth::ThirtyTwo);
    let add = program.new_instruction(Operator::Iadd, Type::i32(), vec![], vec![five, zero]);

    let new = optimizer.apply_one(&mut program, add);
    let new = new.expect("optimization should have applied");
    assert!(program.structurally_eq(new, five));

    let add = program.new_instruction(Operator::Iadd, Type::i32(), vec![], vec![five, five]);
    let replacement = optimizer.apply_one(&mut program, add);
    assert!(replacement.is_none());
}

#[test]
fn constant() {
    let opts;
    let mut optimizer = optimizer!(opts, "(=> (iadd $C $x) (iadd_imm $C $x))");

    let mut program = Program::default();
    let five = program.r#const(Constant::Int(5, BitWidth::ThirtyTwo), BitWidth::ThirtyTwo);
    let zero = program.r#const(Constant::Int(0, BitWidth::ThirtyTwo), BitWidth::ThirtyTwo);
    let add = program.new_instruction(Operator::Iadd, Type::i32(), vec![], vec![five, zero]);

    let expected = program.new_instruction(
        Operator::IaddImm,
        Type::i32(),
        vec![Constant::Int(5, BitWidth::ThirtyTwo).into()],
        vec![zero],
    );

    let new = optimizer.apply_one(&mut program, add);
    let new = new.expect("optimization should have applied");
    assert!(program.structurally_eq(new, expected));

    let mul = program.new_instruction(Operator::Imul, Type::i32(), vec![], vec![five, zero]);
    let add = program.new_instruction(Operator::Imul, Type::i32(), vec![], vec![mul, five]);
    let replacement = optimizer.apply_one(&mut program, add);
    assert!(replacement.is_none());
}

#[test]
fn boolean() {
    let opts;
    let mut optimizer = optimizer!(opts, "(=> (bint true) 1)");

    let mut program = Program::default();
    let t = program.r#const(Constant::Bool(true, BitWidth::One), BitWidth::One);
    let bint = program.new_instruction(Operator::Bint, Type::i1(), vec![], vec![t]);
    let one = program.r#const(Constant::Int(1, BitWidth::One), BitWidth::ThirtyTwo);

    let new = optimizer.apply_one(&mut program, bint);
    let new = new.expect("optimization should have applied");
    assert!(program.structurally_eq(new, one));

    let f = program.r#const(Constant::Bool(false, BitWidth::One), BitWidth::One);
    let bint = program.new_instruction(Operator::Bint, Type::i1(), vec![], vec![f]);
    let replacement = optimizer.apply_one(&mut program, bint);
    assert!(replacement.is_none());
}

#[test]
fn condition_codes() {
    let opts;
    let mut optimizer = optimizer!(opts, "(=> (icmp eq $x $x) true)");

    let mut program = Program::default();
    let five = program.r#const(Constant::Int(5, BitWidth::ThirtyTwo), BitWidth::One);
    let icmp_eq = program.new_instruction(
        Operator::Icmp,
        Type::b1(),
        vec![ConditionCode::Eq.into()],
        vec![five, five],
    );
    let t = program.r#const(Constant::Bool(true, BitWidth::One), BitWidth::One);

    let new = optimizer.apply_one(&mut program, icmp_eq);
    let new = new.expect("optimization should have applied");
    assert!(program.structurally_eq(new, t));

    let icmp_ne = program.new_instruction(
        Operator::Icmp,
        Type::b1(),
        vec![ConditionCode::Ne.into()],
        vec![five, five],
    );
    let replacement = optimizer.apply_one(&mut program, icmp_ne);
    assert!(replacement.is_none());
}

#[test]
fn is_power_of_two() {
    let opts;
    let mut optimizer = optimizer!(
        opts,
        "
(=> (when (imul $x $C)
          (is-power-of-two $C))
    (ishl $x $(log2 $C)))
"
    );

    let mut program = Program::default();
    let five = program.r#const(Constant::Int(5, BitWidth::ThirtyTwo), BitWidth::ThirtyTwo);
    let two = program.r#const(Constant::Int(2, BitWidth::ThirtyTwo), BitWidth::ThirtyTwo);
    let imul = program.new_instruction(Operator::Imul, Type::i32(), vec![], vec![five, two]);

    let one = program.r#const(Constant::Int(1, BitWidth::ThirtyTwo), BitWidth::ThirtyTwo);
    let ishl = program.new_instruction(Operator::Ishl, Type::i32(), vec![], vec![five, one]);

    let new = optimizer.apply_one(&mut program, imul);
    let new = new.expect("optimization should have applied");
    assert!(program.structurally_eq(new, ishl));

    let three = program.r#const(Constant::Int(3, BitWidth::ThirtyTwo), BitWidth::ThirtyTwo);
    let imul = program.new_instruction(Operator::Imul, Type::i32(), vec![], vec![five, three]);

    let replacement = optimizer.apply_one(&mut program, imul);
    assert!(replacement.is_none());
}

#[test]
fn bit_width() {
    let opts;
    let mut optimizer = optimizer!(
        opts,
        "
(=> (when (imul $C $x)
          (bit-width $C 32))
    (imul_imm $C $x))
"
    );

    let mut program = Program::default();
    let five = program.r#const(Constant::Int(5, BitWidth::ThirtyTwo), BitWidth::ThirtyTwo);
    let two = program.r#const(Constant::Int(2, BitWidth::ThirtyTwo), BitWidth::ThirtyTwo);
    let imul = program.new_instruction(Operator::Imul, Type::i32(), vec![], vec![five, two]);

    let imul_imm = program.new_instruction(
        Operator::ImulImm,
        Type::i32(),
        vec![Constant::Int(5, BitWidth::ThirtyTwo).into()],
        vec![two],
    );

    let new = optimizer.apply_one(&mut program, imul);
    let new = new.expect("optimization should have applied");
    assert!(program.structurally_eq(new, imul_imm));

    let five = program.r#const(Constant::Int(5, BitWidth::SixtyFour), BitWidth::SixtyFour);
    let two = program.r#const(Constant::Int(2, BitWidth::SixtyFour), BitWidth::SixtyFour);
    let imul = program.new_instruction(Operator::Imul, Type::i32(), vec![], vec![five, two]);

    let replacement = optimizer.apply_one(&mut program, imul);
    assert!(replacement.is_none());
}

#[test]
fn fits_in_native_word() {
    let opts;
    let mut optimizer = optimizer!(
        opts,
        "
(=> (when (imul $C $x)
          (fits-in-native-word $C))
    (imul_imm $C $x))
"
    );

    let mut program = Program::default();
    let five = program.r#const(Constant::Int(5, BitWidth::ThirtyTwo), BitWidth::ThirtyTwo);
    let two = program.r#const(Constant::Int(2, BitWidth::ThirtyTwo), BitWidth::ThirtyTwo);
    let imul = program.new_instruction(Operator::Imul, Type::i32(), vec![], vec![five, two]);

    let imul_imm = program.new_instruction(
        Operator::ImulImm,
        Type::i32(),
        vec![Constant::Int(5, BitWidth::ThirtyTwo).into()],
        vec![two],
    );

    let new = optimizer.apply_one(&mut program, imul);
    let new = new.expect("optimization should have applied");
    assert!(program.structurally_eq(new, imul_imm));

    let five = program.r#const(Constant::Int(5, BitWidth::SixtyFour), BitWidth::SixtyFour);
    let two = program.r#const(Constant::Int(2, BitWidth::SixtyFour), BitWidth::SixtyFour);
    let imul = program.new_instruction(Operator::Imul, Type::i64(), vec![], vec![five, two]);

    let replacement = optimizer.apply_one(&mut program, imul);
    assert!(replacement.is_none());
}

#[test]
fn unquote_neg() {
    let opts;
    let mut optimizer = optimizer!(
        opts,
        "
(=> (isub $x $C)
    (iadd_imm $(neg $C) $x))
"
    );

    let mut program = Program::default();
    let five = program.r#const(Constant::Int(5, BitWidth::SixtyFour), BitWidth::SixtyFour);
    let two = program.r#const(Constant::Int(2, BitWidth::SixtyFour), BitWidth::SixtyFour);
    let isub = program.new_instruction(Operator::Isub, Type::i64(), vec![], vec![five, two]);

    let iadd_imm = program.new_instruction(
        Operator::IaddImm,
        Type::i64(),
        vec![Constant::Int(-2 as _, BitWidth::SixtyFour).into()],
        vec![five],
    );

    let new = optimizer.apply_one(&mut program, isub);
    let new = new.expect("optimization should have applied");
    assert!(program.structurally_eq(new, iadd_imm));
}

#[test]
fn subsumption() {
    let opts;
    let mut optimizer = optimizer!(
        opts,
        "
;; NB: the following optimizations are ordered from least to most general, so
;; the first applicable optimization should be the one that is applied.

(=> (iadd (iadd (iadd $w $x) $y) $z)
    (iadd (iadd $w $x) (iadd $y $z)))

(=> (iadd $C1 $C2)
    $(iadd $C1 $C2))

(=> (iadd $C $x)
    (iadd_imm $C $x))

(=> (iadd $x $x)
    (ishl_imm 1 $x))
"
    );

    let mut program = Program::default();

    let w = program.r#const(Constant::Int(11, BitWidth::SixtyFour), BitWidth::SixtyFour);
    let x = program.r#const(Constant::Int(22, BitWidth::SixtyFour), BitWidth::SixtyFour);
    let y = program.r#const(Constant::Int(33, BitWidth::SixtyFour), BitWidth::SixtyFour);
    let z = program.r#const(Constant::Int(44, BitWidth::SixtyFour), BitWidth::SixtyFour);

    log::debug!("(iadd (iadd (iadd w x) y) z) => (iadd (iadd w x) (iadd y z))");

    let iadd = program.new_instruction(Operator::Iadd, Type::i64(), vec![], vec![w, x]);
    let iadd = program.new_instruction(Operator::Iadd, Type::i64(), vec![], vec![iadd, y]);
    let iadd = program.new_instruction(Operator::Iadd, Type::i64(), vec![], vec![iadd, z]);
    let expected_lhs = program.new_instruction(Operator::Iadd, Type::i64(), vec![], vec![w, x]);
    let expected_rhs = program.new_instruction(Operator::Iadd, Type::i64(), vec![], vec![y, z]);
    let expected = program.new_instruction(
        Operator::Iadd,
        Type::i64(),
        vec![],
        vec![expected_lhs, expected_rhs],
    );

    let new = optimizer.apply_one(&mut program, iadd);
    let new = new.expect("optimization should have applied");
    assert!(program.structurally_eq(new, expected));

    log::debug!("(iadd w x) => y");

    let iadd = program.new_instruction(Operator::Iadd, Type::i64(), vec![], vec![w, x]);
    let new = optimizer.apply_one(&mut program, iadd);
    let new = new.expect("optimization should have applied");
    assert!(program.structurally_eq(new, y));

    log::debug!("(iadd x (iadd y z)) => (iadd_imm x (iadd y z))");

    let iadd_y_z = program.new_instruction(Operator::Iadd, Type::i64(), vec![], vec![y, z]);
    let iadd = program.new_instruction(Operator::Iadd, Type::i64(), vec![], vec![x, iadd_y_z]);
    let iadd_imm = program.new_instruction(
        Operator::IaddImm,
        Type::i64(),
        vec![Constant::Int(22, BitWidth::SixtyFour).into()],
        vec![iadd_y_z],
    );
    let new = optimizer.apply_one(&mut program, iadd);
    let new = new.expect("optimization should have applied");
    assert!(program.structurally_eq(new, iadd_imm));

    log::debug!("(iadd (imul_imm x 1) (imul_imm x 1)) => (ishl_imm 1 (imul_imm x 1))");

    let imul_imm = program.new_instruction(
        Operator::ImulImm,
        Type::i64(),
        vec![Constant::Int(1, BitWidth::SixtyFour).into()],
        vec![x],
    );
    let iadd = program.new_instruction(
        Operator::Iadd,
        Type::i64(),
        vec![],
        vec![imul_imm, imul_imm],
    );
    let ishl_imm = program.new_instruction(
        Operator::IshlImm,
        Type::i64(),
        vec![Constant::Int(1, BitWidth::SixtyFour).into()],
        vec![imul_imm],
    );
    let new = optimizer.apply_one(&mut program, iadd);
    let new = new.expect("optimization should have applied");
    assert!(program.structurally_eq(new, ishl_imm));

    log::debug!("(iadd (imul w x) (imul y z)) does not match any optimization.");

    let imul_w_x = program.new_instruction(Operator::Imul, Type::i64(), vec![], vec![w, x]);
    let imul_y_z = program.new_instruction(Operator::Imul, Type::i64(), vec![], vec![y, z]);
    let iadd = program.new_instruction(
        Operator::Iadd,
        Type::i64(),
        vec![],
        vec![imul_w_x, imul_y_z],
    );

    let replacement = optimizer.apply_one(&mut program, iadd);
    assert!(replacement.is_none());
}

#[test]
fn polymorphic_bit_widths() {
    let opts;
    let mut optimizer = optimizer!(opts, "(=> (iadd $C $x) (iadd_imm $C $x))");

    let mut program = Program::default();

    // Applies to 32 bit adds.

    let x = program.r#const(Constant::Int(42, BitWidth::ThirtyTwo), BitWidth::ThirtyTwo);
    let y = program.r#const(Constant::Int(420, BitWidth::ThirtyTwo), BitWidth::ThirtyTwo);
    let iadd = program.new_instruction(Operator::Iadd, Type::i32(), vec![], vec![x, y]);
    let iadd_imm = program.new_instruction(
        Operator::IaddImm,
        Type::i32(),
        vec![Constant::Int(42, BitWidth::ThirtyTwo).into()],
        vec![y],
    );

    let new = optimizer.apply_one(&mut program, iadd);
    let new = new.expect("optimization should have applied");
    assert!(program.structurally_eq(new, iadd_imm));

    // Applies to 16 bit adds.

    let x = program.r#const(Constant::Int(42, BitWidth::Sixteen), BitWidth::Sixteen);
    let y = program.r#const(Constant::Int(420, BitWidth::Sixteen), BitWidth::Sixteen);
    let iadd = program.new_instruction(Operator::Iadd, Type::i16(), vec![], vec![x, y]);
    let iadd_imm = program.new_instruction(
        Operator::IaddImm,
        Type::i16(),
        vec![Constant::Int(42, BitWidth::Sixteen).into()],
        vec![y],
    );

    let new = optimizer.apply_one(&mut program, iadd);
    let new = new.expect("optimization should have applied");
    assert!(program.structurally_eq(new, iadd_imm));
}
