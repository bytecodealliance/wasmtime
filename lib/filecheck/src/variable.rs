use std::borrow::Cow;

/// A variable name is one or more ASCII alphanumerical characters, including underscore.
/// Note that numerical variable names like `$45` are allowed too.
///
/// Try to parse a variable name from the begining of `s`.
/// Return the index of the character following the varname.
/// This returns 0 if `s` doesn't have a prefix that is a variable name.
pub fn varname_prefix(s: &str) -> usize {
    for (idx, ch) in s.char_indices() {
        match ch {
            'a'...'z' | 'A'...'Z' | '0'...'9' | '_' => {}
            _ => return idx,
        }
    }
    s.len()
}

/// A variable can contain either a regular expression or plain text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value<'a> {
    Text(Cow<'a, str>),
    Regex(Cow<'a, str>),
}

/// Resolve variables by name.
pub trait VariableMap {
    /// Get the value of the variable `varname`, or return `None` for an unknown variable name.
    fn lookup(&self, varname: &str) -> Option<Value>;
}

impl VariableMap for () {
    fn lookup(&self, _: &str) -> Option<Value> {
        None
    }
}

/// An empty variable map.
pub const NO_VARIABLES: &'static VariableMap = &();

#[cfg(test)]
mod tests {
    #[test]
    fn varname() {
        use super::varname_prefix;

        assert_eq!(varname_prefix(""), 0);
        assert_eq!(varname_prefix("\0"), 0);
        assert_eq!(varname_prefix("_"), 1);
        assert_eq!(varname_prefix("0"), 1);
        assert_eq!(varname_prefix("01"), 2);
        assert_eq!(varname_prefix("b"), 1);
        assert_eq!(varname_prefix("C"), 1);
        assert_eq!(varname_prefix("."), 0);
        assert_eq!(varname_prefix(".s"), 0);
        assert_eq!(varname_prefix("0."), 1);
        assert_eq!(varname_prefix("01="), 2);
        assert_eq!(varname_prefix("0a)"), 2);
    }
}
