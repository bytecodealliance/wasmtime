/// Look for a directive in a comment string.
/// The directive is of the form "foo:" and should follow the leading `;` in the comment:
///
/// ; dominates: block3 block4
///
/// Return the comment text following the directive.
pub fn match_directive<'a>(comment: &'a str, directive: &str) -> Option<&'a str> {
    assert!(
        directive.ends_with(':'),
        "Directive must include trailing colon"
    );
    let text = comment.trim_start_matches(';').trim_start();
    if text.starts_with(directive) {
        Some(text[directive.len()..].trim())
    } else {
        None
    }
}

#[test]
fn test_match_directive() {
    assert_eq!(match_directive("; foo: bar  ", "foo:"), Some("bar"));
    assert_eq!(match_directive(" foo:bar", "foo:"), Some("bar"));
    assert_eq!(match_directive("foo:bar", "foo:"), Some("bar"));
    assert_eq!(match_directive(";x foo: bar", "foo:"), None);
    assert_eq!(match_directive(";;; foo: bar", "foo:"), Some("bar"));
}
