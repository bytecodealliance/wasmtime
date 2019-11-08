#[test]
fn tests() {
    let t = trybuild::TestCases::new();
    t.pass("tests/no_args.rs");
    t.pass("tests/val_args.rs");
    t.pass("tests/ref_args.rs");
    t.pass("tests/mut_args.rs");
    t.pass("tests/array_args.rs");
}
