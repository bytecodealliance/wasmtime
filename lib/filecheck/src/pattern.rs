//! Pattern matching for a single directive.

use error::{Error, Result};
use variable::{varname_prefix, VariableMap, Value};
use std::str::FromStr;
use std::fmt::{self, Display, Formatter, Write};
use regex::{Regex, RegexBuilder, quote};

/// A pattern to match as specified in a directive.
///
/// Each pattern is broken into a sequence of parts that must match in order. The kinds of parts
/// are:
///
/// 1. Plain text match.
/// 2. Variable match, `$FOO` or `$(FOO)`. The variable `FOO` may expand to plain text or a regex.
/// 3. Variable definition from literal regex, `$(foo=.*)`. Match the regex and assign matching text
///    to variable `foo`.
/// 4. Variable definition from regex variable, `$(foo=$RX)`. Lookup variable `RX` which should
///    expand to a regex, match the regex, and assign matching text to variable `foo`.
///
pub struct Pattern {
    parts: Vec<Part>,
    // Variables defined by this pattern.
    defs: Vec<String>,
}

/// One atomic part of a pattern.
#[derive(Debug, PartialEq, Eq)]
pub enum Part {
    /// Match a plain string.
    Text(String),
    /// Match a regular expression. The regex has already been wrapped in a non-capturing group if
    /// necessary, so it is safe to concatenate.
    Regex(String),
    /// Match the contents of a variable, which can be plain text or regex.
    Var(String),
    /// Match literal regex, then assign match to variable.
    /// The regex has already been wrapped in a named capture group.
    DefLit { def: usize, regex: String },
    /// Lookup variable `var`, match resulting regex, assign matching text to variable `defs[def]`.
    DefVar { def: usize, var: String },
}

impl Part {
    /// Get the variabled referenced by this part, if any.
    pub fn ref_var(&self) -> Option<&str> {
        match *self {
            Part::Var(ref var) => Some(var),
            Part::DefVar { ref var, .. } => Some(var),
            _ => None,
        }
    }
}

impl Pattern {
    /// Create a new blank pattern. Use the `FromStr` trait to generate Patterns with content.
    fn new() -> Pattern {
        Pattern {
            parts: Vec::new(),
            defs: Vec::new(),
        }
    }

    /// Check if the variable `v` is defined by this pattern.
    pub fn defines_var(&self, v: &str) -> bool {
        self.defs.iter().any(|d| d == v)
    }

    /// Add a definition of a new variable.
    /// Return the allocated def number.
    fn add_def(&mut self, v: &str) -> Result<usize> {
        if self.defines_var(v) {
            Err(Error::DuplicateDef(format!("duplicate definition of ${} in same pattern", v)))
        } else {
            let idx = self.defs.len();
            self.defs.push(v.to_string());
            Ok(idx)
        }
    }

    /// Parse a `Part` from a prefix of `s`.
    /// Return the part and the number of bytes consumed from `s`.
    /// Adds defined variables to `self.defs`.
    fn parse_part(&mut self, s: &str) -> Result<(Part, usize)> {
        let dollar = s.find('$');
        if dollar != Some(0) {
            // String doesn't begin with a dollar sign, so match plain text up to the dollar sign.
            let end = dollar.unwrap_or(s.len());
            return Ok((Part::Text(s[0..end].to_string()), end));
        }

        // String starts with a dollar sign. Look for these possibilities:
        //
        // 1. `$$`.
        // 2. `$var`.
        // 3. `$(var)`.
        // 4. `$(var=regex)`. Where `regex` is a regular expression possibly containing matching
        //    braces.
        // 5. `$(var=$VAR)`.

        // A doubled dollar sign matches a single dollar sign.
        if s.starts_with("$$") {
            return Ok((Part::Text("$".to_string()), 2));
        }

        // Look for `$var`.
        let varname_end = 1 + varname_prefix(&s[1..]);
        if varname_end != 1 {
            return Ok((Part::Var(s[1..varname_end].to_string()), varname_end));
        }

        // All remaining possibilities start with `$(`.
        if s.len() < 2 || !s.starts_with("$(") {
            return Err(Error::Syntax("pattern syntax error, use $$ to match a single $"
                .to_string()));
        }

        // Match the variable name, allowing for an empty varname in `$()`, or `$(=...)`.
        let varname_end = 2 + varname_prefix(&s[2..]);
        let varname = s[2..varname_end].to_string();

        match s[varname_end..].chars().next() {
            None => {
                return Err(Error::Syntax(format!("unterminated $({}...", varname)));
            }
            Some(')') => {
                let part = if varname.is_empty() {
                    // Match `$()`, turn it into an empty text match.
                    Part::Text(varname)
                } else {
                    // Match `$(var)`.
                    Part::Var(varname)
                };
                return Ok((part, varname_end + 1));
            }
            Some('=') => {
                // Variable definition. Fall through.
            }
            Some(ch) => {
                return Err(Error::Syntax(format!("syntax error in $({}... '{}'", varname, ch)));
            }
        }

        // This is a variable definition of the form `$(var=...`.

        // Allocate a definition index.
        let def = if varname.is_empty() {
            None
        } else {
            Some(try!(self.add_def(&varname)))
        };

        // Match `$(var=$PAT)`.
        if s[varname_end + 1..].starts_with('$') {
            let refname_begin = varname_end + 2;
            let refname_end = refname_begin + varname_prefix(&s[refname_begin..]);
            if refname_begin == refname_end {
                return Err(Error::Syntax(format!("expected variable name in $({}=$...", varname)));
            }
            if !s[refname_end..].starts_with(')') {
                return Err(Error::Syntax(format!("expected ')' after $({}=${}...",
                                                 varname,
                                                 &s[refname_begin..refname_end])));
            }
            let refname = s[refname_begin..refname_end].to_string();
            return if let Some(defidx) = def {
                Ok((Part::DefVar {
                        def: defidx,
                        var: refname,
                    },
                    refname_end + 1))
            } else {
                Err(Error::Syntax(format!("expected variable name in $(=${})", refname)))
            };
        }

        // Last case: `$(var=...)` where `...` is a regular expression, possibly containing matched
        // parentheses.
        let rx_begin = varname_end + 1;
        let rx_end = rx_begin + regex_prefix(&s[rx_begin..]);
        if s[rx_end..].starts_with(')') {
            let part = if let Some(defidx) = def {
                // Wrap the regex in a named capture group.
                Part::DefLit {
                    def: defidx,
                    regex: format!("(?P<{}>{})", varname, &s[rx_begin..rx_end]),
                }
            } else {
                // When the varname is empty just match the regex, don't capture any variables.
                // This is `$(=[a-z])`.
                // Wrap the regex in a non-capturing group to make it concatenation-safe.
                Part::Regex(format!("(?:{})", &s[rx_begin..rx_end]))
            };
            Ok((part, rx_end + 1))
        } else {
            Err(Error::Syntax(format!("missing ')' after regex in $({}={}",
                                      varname,
                                      &s[rx_begin..rx_end])))
        }
    }
}

/// Compute the length of a regular expression terminated by `)` or `}`.
/// Handle nested and escaped parentheses in the rx, but don't actualy parse it.
/// Return the position of the terminating brace or the length of the string.
fn regex_prefix(s: &str) -> usize {
    // The prevous char was a backslash.
    let mut escape = false;
    // State around parsing charsets.
    enum State {
        Normal, // Outside any charset.
        Curly, // Inside curly braces.
        CSFirst, // Immediately after opening `[`.
        CSNeg, // Immediately after `[^`.
        CSBody, // Inside `[...`.
    }
    let mut state = State::Normal;

    // Current nesting level of parens.
    let mut nest = 0usize;

    for (idx, ch) in s.char_indices() {
        if escape {
            escape = false;
            continue;
        } else if ch == '\\' {
            escape = true;
            continue;
        }
        match state {
            State::Normal => {
                match ch {
                    '[' => state = State::CSFirst,
                    '{' => state = State::Curly,
                    '(' => nest += 1,
                    ')' if nest > 0 => nest -= 1,
                    ')' | '}' => return idx,
                    _ => {}
                }
            }
            State::Curly => {
                if ch == '}' {
                    state = State::Normal;
                }
            }
            State::CSFirst => {
                state = match ch {
                    '^' => State::CSNeg,
                    _ => State::CSBody,
                }
            }
            State::CSNeg => state = State::CSBody,
            State::CSBody => {
                if ch == ']' {
                    state = State::Normal;
                }
            }
        }
    }
    s.len()
}

impl FromStr for Pattern {
    type Err = Error;

    fn from_str(s: &str) -> Result<Pattern> {
        // Always remove leading and trailing whitespace.
        // Use `$()` to actually include that in a match.
        let s = s.trim();
        let mut pat = Pattern::new();
        let mut pos = 0;
        while pos < s.len() {
            let (part, len) = try!(pat.parse_part(&s[pos..]));
            if let Some(v) = part.ref_var() {
                if pat.defines_var(v) {
                    return Err(Error::Backref(format!("unsupported back-reference to '${}' \
                                                       defined in same pattern",
                                                      v)));
                }
            }
            pat.parts.push(part);
            pos += len;
        }
        Ok(pat)
    }
}

impl Pattern {
    /// Get a list of parts in this pattern.
    pub fn parts(&self) -> &[Part] {
        &self.parts
    }

    /// Get a list of variable names defined when this pattern matches.
    pub fn defs(&self) -> &[String] {
        &self.defs
    }

    /// Resolve all variable references in this pattern, turning it into a regular expression.
    pub fn resolve(&self, vmap: &VariableMap) -> Result<Regex> {
        let mut out = String::new();

        // Add a word boundary check `\b` to the beginning of the regex, but only if the first part
        // is a plain text match that starts with a word character.
        //
        // This behavior can be disabled by starting the pattern with `$()`.
        if let Some(&Part::Text(ref s)) = self.parts.first() {
            if s.starts_with(char::is_alphanumeric) {
                out.push_str(r"\b");
            }
        }

        for part in &self.parts {
            match *part {
                Part::Text(ref s) => {
                    out.push_str(&quote(s));
                }
                Part::Regex(ref rx) => out.push_str(rx),
                Part::Var(ref var) => {
                    // Resolve the variable. We can handle a plain text expansion.
                    match vmap.lookup(var) {
                        None => {
                            return Err(Error::UndefVariable(format!("undefined variable ${}", var)))
                        }
                        Some(Value::Text(s)) => out.push_str(&quote(&s)),
                        // Wrap regex in non-capturing group for safe concatenation.
                        Some(Value::Regex(rx)) => write!(out, "(?:{})", rx).unwrap(),
                    }
                }
                Part::DefLit { ref regex, .. } => out.push_str(regex),
                Part::DefVar { def, ref var } => {
                    // Wrap regex in a named capture group.
                    write!(out, "(?P<{}>", self.defs[def]).unwrap();
                    match vmap.lookup(var) {
                        None => {
                            return Err(Error::UndefVariable(format!("undefined variable ${}", var)))
                        }
                        Some(Value::Text(s)) => write!(out, "{})", quote(&s[..])).unwrap(),
                        Some(Value::Regex(rx)) => write!(out, "{})", rx).unwrap(),
                    }
                }
            }

        }

        // Add a word boundary check `\b` to the end of the regex, but only if the final part
        // is a plain text match that ends with a word character.
        //
        // This behavior can be disabled by ending the pattern with `$()`.
        if let Some(&Part::Text(ref s)) = self.parts.last() {
            if s.ends_with(char::is_alphanumeric) {
                out.push_str(r"\b");
            }
        }

        Ok(try!(RegexBuilder::new(&out).multi_line(true).compile()))
    }
}

impl Display for Pattern {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        for part in &self.parts {
            use self::Part::*;
            try!(match *part {
                Text(ref txt) if txt == "" => write!(f, "$()"),
                Text(ref txt) if txt == "$" => write!(f, "$$"),
                Text(ref txt) => write!(f, "{}", txt),
                Regex(ref rx) => write!(f, "$(={})", rx),
                Var(ref var) => write!(f, "$({})", var),
                DefLit { def, ref regex } => {
                    let defvar = &self.defs[def];
                    // (?P<defvar>...).
                    let litrx = &regex[5 + defvar.len()..regex.len() - 1];
                    write!(f, "$({}={})", defvar, litrx)
                }
                DefVar { def, ref var } => write!(f, "$({}=${})", self.defs[def], var),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn regex() {
        use super::regex_prefix;

        assert_eq!(regex_prefix(""), 0);
        assert_eq!(regex_prefix(")"), 0);
        assert_eq!(regex_prefix(")c"), 0);
        assert_eq!(regex_prefix("x"), 1);
        assert_eq!(regex_prefix("x)x"), 1);

        assert_eq!(regex_prefix("x(c))x"), 4);
        assert_eq!(regex_prefix("()x(c))x"), 6);
        assert_eq!(regex_prefix("()x(c)"), 6);

        assert_eq!(regex_prefix("x([)]))x"), 6);
        assert_eq!(regex_prefix("x[)])x"), 4);
        assert_eq!(regex_prefix("x[^)])x"), 5);
        assert_eq!(regex_prefix("x[^])x"), 6);
    }

    #[test]
    fn part() {
        use super::{Pattern, Part};
        let mut pat = Pattern::new();

        // This is dubious, should we panic instead?
        assert_eq!(pat.parse_part("").unwrap(), (Part::Text("".to_string()), 0));

        assert_eq!(pat.parse_part("x").unwrap(),
                   (Part::Text("x".to_string()), 1));
        assert_eq!(pat.parse_part("x2").unwrap(),
                   (Part::Text("x2".to_string()), 2));
        assert_eq!(pat.parse_part("x$").unwrap(),
                   (Part::Text("x".to_string()), 1));
        assert_eq!(pat.parse_part("x$$").unwrap(),
                   (Part::Text("x".to_string()), 1));

        assert_eq!(pat.parse_part("$").unwrap_err().to_string(),
                   "pattern syntax error, use $$ to match a single $");

        assert_eq!(pat.parse_part("$$").unwrap(),
                   (Part::Text("$".to_string()), 2));
        assert_eq!(pat.parse_part("$$ ").unwrap(),
                   (Part::Text("$".to_string()), 2));

        assert_eq!(pat.parse_part("$0").unwrap(),
                   (Part::Var("0".to_string()), 2));
        assert_eq!(pat.parse_part("$xx=").unwrap(),
                   (Part::Var("xx".to_string()), 3));
        assert_eq!(pat.parse_part("$xx$").unwrap(),
                   (Part::Var("xx".to_string()), 3));

        assert_eq!(pat.parse_part("$(0)").unwrap(),
                   (Part::Var("0".to_string()), 4));
        assert_eq!(pat.parse_part("$()").unwrap(),
                   (Part::Text("".to_string()), 3));

        assert_eq!(pat.parse_part("$(0").unwrap_err().to_string(),
                   ("unterminated $(0..."));
        assert_eq!(pat.parse_part("$(foo:").unwrap_err().to_string(),
                   ("syntax error in $(foo... ':'"));
        assert_eq!(pat.parse_part("$(foo =").unwrap_err().to_string(),
                   ("syntax error in $(foo... ' '"));
        assert_eq!(pat.parse_part("$(eo0=$bar").unwrap_err().to_string(),
                   ("expected ')' after $(eo0=$bar..."));
        assert_eq!(pat.parse_part("$(eo1=$bar}").unwrap_err().to_string(),
                   ("expected ')' after $(eo1=$bar..."));
        assert_eq!(pat.parse_part("$(eo2=$)").unwrap_err().to_string(),
                   ("expected variable name in $(eo2=$..."));
        assert_eq!(pat.parse_part("$(eo3=$-)").unwrap_err().to_string(),
                   ("expected variable name in $(eo3=$..."));
    }

    #[test]
    fn partdefs() {
        use super::{Pattern, Part};
        let mut pat = Pattern::new();

        assert_eq!(pat.parse_part("$(foo=$bar)").unwrap(),
                   (Part::DefVar {
                        def: 0,
                        var: "bar".to_string(),
                    },
                    11));
        assert_eq!(pat.parse_part("$(foo=$bar)").unwrap_err().to_string(),
                   "duplicate definition of $foo in same pattern");

        assert_eq!(pat.parse_part("$(fxo=$bar)x").unwrap(),
                   (Part::DefVar {
                        def: 1,
                        var: "bar".to_string(),
                    },
                    11));

        assert_eq!(pat.parse_part("$(fo2=[a-z])").unwrap(),
                   (Part::DefLit {
                        def: 2,
                        regex: "(?P<fo2>[a-z])".to_string(),
                    },
                    12));
        assert_eq!(pat.parse_part("$(fo3=[a-)])").unwrap(),
                   (Part::DefLit {
                        def: 3,
                        regex: "(?P<fo3>[a-)])".to_string(),
                    },
                    12));
        assert_eq!(pat.parse_part("$(fo4=)").unwrap(),
                   (Part::DefLit {
                        def: 4,
                        regex: "(?P<fo4>)".to_string(),
                    },
                    7));

        assert_eq!(pat.parse_part("$(=.*)").unwrap(),
                   (Part::Regex("(?:.*)".to_string()), 6));

        assert_eq!(pat.parse_part("$(=)").unwrap(),
                   (Part::Regex("(?:)".to_string()), 4));
        assert_eq!(pat.parse_part("$()").unwrap(),
                   (Part::Text("".to_string()), 3));
    }

    #[test]
    fn pattern() {
        use super::Pattern;

        let p: Pattern = "  Hello world!  ".parse().unwrap();
        assert_eq!(format!("{:?}", p.parts), "[Text(\"Hello world!\")]");

        let p: Pattern = "  $foo=$(bar)  ".parse().unwrap();
        assert_eq!(format!("{:?}", p.parts),
                   "[Var(\"foo\"), Text(\"=\"), Var(\"bar\")]");
    }
}
