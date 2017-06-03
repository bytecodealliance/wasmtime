extern crate filecheck;

use filecheck::{CheckerBuilder, NO_VARIABLES, Error as FcError};

fn e2s(e: FcError) -> String {
    e.to_string()
}

#[test]
fn empty() {
    let c = CheckerBuilder::new().finish();
    assert!(c.is_empty());

    // An empty checker matches anything.
    assert_eq!(c.check("", NO_VARIABLES).map_err(e2s), Ok(true));
    assert_eq!(c.check("hello", NO_VARIABLES).map_err(e2s), Ok(true));
}

#[test]
fn no_directives() {
    let c = CheckerBuilder::new().text("nothing here").unwrap().finish();
    assert!(c.is_empty());

    // An empty checker matches anything.
    assert_eq!(c.check("", NO_VARIABLES).map_err(e2s), Ok(true));
    assert_eq!(c.check("hello", NO_VARIABLES).map_err(e2s), Ok(true));
}

#[test]
fn no_matches() {
    let c = CheckerBuilder::new()
        .text("regex: FOO=bar")
        .unwrap()
        .finish();
    assert!(!c.is_empty());

    // An empty checker matches anything.
    assert_eq!(c.check("", NO_VARIABLES).map_err(e2s), Ok(true));
    assert_eq!(c.check("hello", NO_VARIABLES).map_err(e2s), Ok(true));
}

#[test]
fn simple() {
    let c = CheckerBuilder::new()
        .text("
        check: one
        check: two
        ")
        .unwrap()
        .finish();

    let t = "
        zero
        one
        and a half
        two
        three
        ";
    assert_eq!(c.check(t, NO_VARIABLES).map_err(e2s), Ok(true));

    let t = "
        zero
        and a half
        two
        one
        three
        ";
    assert_eq!(c.check(t, NO_VARIABLES).map_err(e2s), Ok(false));
}

#[test]
fn sameln() {
    let c = CheckerBuilder::new()
        .text("
        check: one
        sameln: two
        ")
        .unwrap()
        .finish();

    let t = "
        zero
        one
        and a half
        two
        three
        ";
    assert_eq!(c.check(t, NO_VARIABLES).map_err(e2s), Ok(false));

    let t = "
        zero
        one
        two
        three
        ";
    assert_eq!(c.check(t, NO_VARIABLES).map_err(e2s), Ok(false));

    let t = "
        zero
        one two
        three
        ";
    assert_eq!(c.check(t, NO_VARIABLES).map_err(e2s), Ok(true));
}

#[test]
fn nextln() {
    let c = CheckerBuilder::new()
        .text("
        check: one
        nextln: two
        ")
        .unwrap()
        .finish();

    let t = "
        zero
        one
        and a half
        two
        three
        ";
    assert_eq!(c.check(t, NO_VARIABLES).map_err(e2s), Ok(false));

    let t = "
        zero
        one
        two
        three
        ";
    assert_eq!(c.check(t, NO_VARIABLES).map_err(e2s), Ok(true));

    let t = "
        zero
        one two
        three
        ";
    assert_eq!(c.check(t, NO_VARIABLES).map_err(e2s), Ok(false));

    let t = "
        zero
        one
        two";
    assert_eq!(c.check(t, NO_VARIABLES).map_err(e2s), Ok(true));
}

#[test]
fn leading_nextln() {
    // A leading nextln directive should match from line 2.
    // This is somewhat arbitrary, but consistent with a preceeding 'check: $()' directive.
    let c = CheckerBuilder::new()
        .text("
        nextln: one
        nextln: two
        ")
        .unwrap()
        .finish();

    let t = "zero
        one
        two
        three
        ";
    assert_eq!(c.check(t, NO_VARIABLES).map_err(e2s), Ok(true));

    let t = "one
        two
        three
        ";
    assert_eq!(c.check(t, NO_VARIABLES).map_err(e2s), Ok(false));
}

#[test]
fn leading_sameln() {
    // A leading sameln directive should match from line 1.
    let c = CheckerBuilder::new()
        .text("
        sameln: one
        sameln: two
        ")
        .unwrap()
        .finish();

    let t = "zero
        one two three
        ";
    assert_eq!(c.check(t, NO_VARIABLES).map_err(e2s), Ok(false));

    let t = "zero one two three";
    assert_eq!(c.check(t, NO_VARIABLES).map_err(e2s), Ok(true));

    let t = "zero one
        two three";
    assert_eq!(c.check(t, NO_VARIABLES).map_err(e2s), Ok(false));
}

#[test]
fn not() {
    let c = CheckerBuilder::new()
        .text("
        check: one$()
        not: $()eat$()
        check: $()two
        ")
        .unwrap()
        .finish();

    let t = "onetwo";
    assert_eq!(c.check(t, NO_VARIABLES).map_err(e2s), Ok(true));

    let t = "one eat two";
    assert_eq!(c.check(t, NO_VARIABLES).map_err(e2s), Ok(false));

    let t = "oneeattwo";
    assert_eq!(c.check(t, NO_VARIABLES).map_err(e2s), Ok(false));

    let t = "oneatwo";
    assert_eq!(c.check(t, NO_VARIABLES).map_err(e2s), Ok(true));
}

#[test]
fn notnot() {
    let c = CheckerBuilder::new()
        .text("
        check: one$()
        not: $()eat$()
        not: half
        check: $()two
        ")
        .unwrap()
        .finish();

    let t = "onetwo";
    assert_eq!(c.check(t, NO_VARIABLES).map_err(e2s), Ok(true));

    let t = "one eat two";
    assert_eq!(c.check(t, NO_VARIABLES).map_err(e2s), Ok(false));

    let t = "one half two";
    assert_eq!(c.check(t, NO_VARIABLES).map_err(e2s), Ok(false));

    let t = "oneeattwo";
    assert_eq!(c.check(t, NO_VARIABLES).map_err(e2s), Ok(false));

    // The `not: half` pattern only matches whole words, but the bracketing matches are considered
    // word boundaries, so it does match in this case.
    let t = "onehalftwo";
    assert_eq!(c.check(t, NO_VARIABLES).map_err(e2s), Ok(false));

    let t = "oneatwo";
    assert_eq!(c.check(t, NO_VARIABLES).map_err(e2s), Ok(true));
}

#[test]
fn unordered() {
    let c = CheckerBuilder::new()
        .text("
        check: one
        unordered: two
        unordered: three
        check: four
        ")
        .unwrap()
        .finish();

    assert_eq!(c.check("one two three four", NO_VARIABLES).map_err(e2s),
               Ok(true));
    assert_eq!(c.check("one three two four", NO_VARIABLES).map_err(e2s),
               Ok(true));

    assert_eq!(c.check("one two four three four", NO_VARIABLES)
                   .map_err(e2s),
               Ok(true));
    assert_eq!(c.check("one three four two four", NO_VARIABLES)
                   .map_err(e2s),
               Ok(true));

    assert_eq!(c.check("one two four three", NO_VARIABLES).map_err(e2s),
               Ok(false));
    assert_eq!(c.check("one three four two", NO_VARIABLES).map_err(e2s),
               Ok(false));
}

#[test]
fn leading_unordered() {
    let c = CheckerBuilder::new()
        .text("
        unordered: two
        unordered: three
        check: four
        ")
        .unwrap()
        .finish();

    assert_eq!(c.check("one two three four", NO_VARIABLES).map_err(e2s),
               Ok(true));
    assert_eq!(c.check("one three two four", NO_VARIABLES).map_err(e2s),
               Ok(true));

    assert_eq!(c.check("one two four three four", NO_VARIABLES)
                   .map_err(e2s),
               Ok(true));
    assert_eq!(c.check("one three four two four", NO_VARIABLES)
                   .map_err(e2s),
               Ok(true));

    assert_eq!(c.check("one two four three", NO_VARIABLES).map_err(e2s),
               Ok(false));
    assert_eq!(c.check("one three four two", NO_VARIABLES).map_err(e2s),
               Ok(false));
}

#[test]
fn trailing_unordered() {
    let c = CheckerBuilder::new()
        .text("
        check: one
        unordered: two
        unordered: three
        ")
        .unwrap()
        .finish();

    assert_eq!(c.check("one two three four", NO_VARIABLES).map_err(e2s),
               Ok(true));
    assert_eq!(c.check("one three two four", NO_VARIABLES).map_err(e2s),
               Ok(true));

    assert_eq!(c.check("one two four three four", NO_VARIABLES)
                   .map_err(e2s),
               Ok(true));
    assert_eq!(c.check("one three four two four", NO_VARIABLES)
                   .map_err(e2s),
               Ok(true));

    assert_eq!(c.check("one two four three", NO_VARIABLES).map_err(e2s),
               Ok(true));
    assert_eq!(c.check("one three four two", NO_VARIABLES).map_err(e2s),
               Ok(true));
}
