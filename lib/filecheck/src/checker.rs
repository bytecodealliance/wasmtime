use error::{Error, Result};
use variable::{VariableMap, Value, varname_prefix};
use pattern::Pattern;
use regex::{Regex, Captures};
use std::borrow::Cow;
use std::collections::HashMap;
use std::cmp::max;
use std::fmt::{self, Display, Formatter};
use MatchRange;
use explain::{Recorder, Explainer};

// The different kinds of directives we support.
enum Directive {
    Check(Pattern),
    SameLn(Pattern),
    NextLn(Pattern),
    Unordered(Pattern),
    Not(Pattern),
    Regex(String, String),
}

// Regular expression matching a directive.
// The match groups are:
//
// 1. Keyword.
// 2. Rest of line / pattern.
//
const DIRECTIVE_RX: &'static str = r"\b(check|sameln|nextln|unordered|not|regex):\s+(.*)";

impl Directive {
    /// Create a new directive from a `DIRECTIVE_RX` match.
    fn new(caps: Captures) -> Result<Directive> {
        let cmd = caps.at(1).expect("group 1 must match");
        let rest = caps.at(2).expect("group 2 must match");

        if cmd == "regex" {
            return Directive::regex(rest);
        }

        // All other commands are followed by a pattern.
        let pat = rest.parse()?;

        match cmd {
            "check" => Ok(Directive::Check(pat)),
            "sameln" => Ok(Directive::SameLn(pat)),
            "nextln" => Ok(Directive::NextLn(pat)),
            "unordered" => Ok(Directive::Unordered(pat)),
            "not" => {
                if !pat.defs().is_empty() {
                    let msg = format!("can't define variables '$({}=...' in not: {}",
                                      pat.defs()[0],
                                      rest);
                    Err(Error::DuplicateDef(msg))
                } else {
                    Ok(Directive::Not(pat))
                }
            }
            _ => panic!("unexpected command {} in regex match", cmd),
        }
    }

    /// Create a `regex:` directive from a `VAR=...` string.
    fn regex(rest: &str) -> Result<Directive> {
        let varlen = varname_prefix(rest);
        if varlen == 0 {
            return Err(Error::Syntax(format!("invalid variable name in regex: {}", rest)));
        }
        let var = rest[0..varlen].to_string();
        if !rest[varlen..].starts_with("=") {
            return Err(Error::Syntax(format!("expected '=' after variable '{}' in regex: {}",
                                             var,
                                             rest)));
        }
        // Ignore trailing white space in the regex, including CR.
        Ok(Directive::Regex(var, rest[varlen + 1..].trim_right().to_string()))
    }
}


/// Builder for constructing a `Checker` instance.
pub struct CheckerBuilder {
    directives: Vec<Directive>,
    linerx: Regex,
}

impl CheckerBuilder {
    /// Create a new, blank `CheckerBuilder`.
    pub fn new() -> CheckerBuilder {
        CheckerBuilder {
            directives: Vec::new(),
            linerx: Regex::new(DIRECTIVE_RX).unwrap(),
        }
    }

    /// Add a potential directive line.
    ///
    /// Returns true if this is a a directive with one of the known prefixes.
    /// Returns false if no known directive was found.
    /// Returns an error if there is a problem with the directive.
    pub fn directive(&mut self, l: &str) -> Result<bool> {
        match self.linerx.captures(l) {
            Some(caps) => {
                self.directives.push(Directive::new(caps)?);
                Ok(true)
            }
            None => Ok(false),
        }
    }

    /// Add multiple directives.
    ///
    /// The text is split into lines that are added individually as potential directives.
    /// This method can be used to parse a whole test file containing multiple directives.
    pub fn text(&mut self, t: &str) -> Result<&mut Self> {
        for caps in self.linerx.captures_iter(t) {
            self.directives.push(Directive::new(caps)?);
        }
        Ok(self)
    }

    /// Get the finished `Checker`.
    pub fn finish(&mut self) -> Checker {
        // Move directives into the new checker, leaving `self.directives` empty and ready for
        // building a new checker.
        Checker::new(self.directives.split_off(0))
    }
}

/// Verify a list of directives against a test input.
///
/// Use a `CheckerBuilder` to construct a `Checker`. Then use the `test` method to verify the list
/// of directives against a test input.
pub struct Checker {
    directives: Vec<Directive>,
}

impl Checker {
    fn new(directives: Vec<Directive>) -> Checker {
        Checker { directives: directives }
    }

    /// An empty checker contains no directives, and will match any input string.
    pub fn is_empty(&self) -> bool {
        self.directives.is_empty()
    }

    /// Verify directives against the input text.
    ///
    /// This returns `true` if the text matches all the directives, `false` if it doesn't.
    /// An error is only returned if there is a problem with the directives.
    pub fn check(&self, text: &str, vars: &VariableMap) -> Result<bool> {
        self.run(text, vars, &mut ())
    }

    /// Explain how directives are matched against the input text.
    pub fn explain(&self, text: &str, vars: &VariableMap) -> Result<(bool, String)> {
        let mut expl = Explainer::new(text);
        let success = self.run(text, vars, &mut expl)?;
        expl.finish();
        Ok((success, expl.to_string()))
    }

    fn run(&self, text: &str, vars: &VariableMap, recorder: &mut Recorder) -> Result<bool> {
        let mut state = State::new(text, vars, recorder);

        // For each pending `not:` check, store (begin-offset, regex).
        let mut nots = Vec::new();

        for (dct_idx, dct) in self.directives.iter().enumerate() {
            let (pat, range) = match *dct {
                Directive::Check(ref pat) => (pat, state.check()),
                Directive::SameLn(ref pat) => (pat, state.sameln()),
                Directive::NextLn(ref pat) => (pat, state.nextln()),
                Directive::Unordered(ref pat) => (pat, state.unordered(pat)),
                Directive::Not(ref pat) => {
                    // Resolve `not:` directives immediately to get the right variable values, but
                    // don't match it until we know the end of the range.
                    //
                    // The `not:` directives test the same range as `unordered:` directives. In
                    // particular, if they refer to defined variables, their range is restricted to
                    // the text following the match that defined the variable.
                    nots.push((dct_idx, state.unordered_begin(pat), pat.resolve(&state)?));
                    continue;
                }
                Directive::Regex(ref var, ref rx) => {
                    state.vars.insert(var.clone(),
                                      VarDef {
                                          value: Value::Regex(Cow::Borrowed(rx)),
                                          offset: 0,
                                      });
                    continue;
                }
            };
            // Check if `pat` matches in `range`.
            state.recorder.directive(dct_idx);
            if let Some((match_begin, match_end)) = state.match_positive(pat, range)? {
                if let &Directive::Unordered(_) = dct {
                    // This was an unordered unordered match.
                    // Keep track of the largest matched position, but leave `last_ordered` alone.
                    state.max_match = max(state.max_match, match_end);
                } else {
                    // Ordered match.
                    state.last_ordered = match_end;
                    state.max_match = match_end;

                    // Verify any pending `not:` directives now that we know their range.
                    for (not_idx, not_begin, rx) in nots.drain(..) {
                        state.recorder.directive(not_idx);
                        if let Some((s, e)) = rx.find(&text[not_begin..match_begin]) {
                            // Matched `not:` pattern.
                            state.recorder.matched_not(rx.as_str(), (not_begin + s, not_begin + e));
                            return Ok(false);
                        } else {
                            state.recorder.missed_not(rx.as_str(), (not_begin, match_begin));
                        }
                    }
                }
            } else {
                // No match!
                return Ok(false);
            }
        }

        // Verify any pending `not:` directives after the last ordered directive.
        for (not_idx, not_begin, rx) in nots.drain(..) {
            state.recorder.directive(not_idx);
            if let Some(_) = rx.find(&text[not_begin..]) {
                // Matched `not:` pattern.
                // TODO: Use matched range for an error message.
                return Ok(false);
            }
        }

        Ok(true)
    }
}

/// A local definition of a variable.
pub struct VarDef<'a> {
    /// The value given to the variable.
    value: Value<'a>,
    /// Offset in input text from where the variable is available.
    offset: usize,
}

struct State<'a> {
    text: &'a str,
    env_vars: &'a VariableMap,
    recorder: &'a mut Recorder,

    vars: HashMap<String, VarDef<'a>>,
    // Offset after the last ordered match. This does not include recent unordered matches.
    last_ordered: usize,
    // Largest offset following a positive match, including unordered matches.
    max_match: usize,
}

impl<'a> State<'a> {
    fn new(text: &'a str, env_vars: &'a VariableMap, recorder: &'a mut Recorder) -> State<'a> {
        State {
            text: text,
            env_vars: env_vars,
            recorder: recorder,
            vars: HashMap::new(),
            last_ordered: 0,
            max_match: 0,
        }
    }

    // Get the offset following the match that defined `var`, or 0 if var is an environment
    // variable or unknown.
    fn def_offset(&self, var: &str) -> usize {
        self.vars
            .get(var)
            .map(|&VarDef { offset, .. }| offset)
            .unwrap_or(0)
    }

    // Get the offset of the beginning of the next line after `pos`.
    fn bol(&self, pos: usize) -> usize {
        if let Some(offset) = self.text[pos..].find('\n') {
            pos + offset + 1
        } else {
            self.text.len()
        }
    }

    // Get the range in text to be matched by a `check:`.
    fn check(&self) -> MatchRange {
        (self.max_match, self.text.len())
    }

    // Get the range in text to be matched by a `sameln:`.
    fn sameln(&self) -> MatchRange {
        let b = self.max_match;
        let e = self.bol(b);
        (b, e)
    }

    // Get the range in text to be matched by a `nextln:`.
    fn nextln(&self) -> MatchRange {
        let b = self.bol(self.max_match);
        let e = self.bol(b);
        (b, e)
    }

    // Get the beginning of the range in text to be matched by a `unordered:` or `not:` directive.
    // The unordered directive must match after the directives that define the variables used.
    fn unordered_begin(&self, pat: &Pattern) -> usize {
        pat.parts()
            .iter()
            .filter_map(|part| part.ref_var())
            .map(|var| self.def_offset(var))
            .fold(self.last_ordered, max)
    }

    // Get the range in text to be matched by a `unordered:` directive.
    fn unordered(&self, pat: &Pattern) -> MatchRange {
        (self.unordered_begin(pat), self.text.len())
    }

    // Search for `pat` in `range`, return the range matched.
    // After a positive match, update variable definitions, if any.
    fn match_positive(&mut self, pat: &Pattern, range: MatchRange) -> Result<Option<MatchRange>> {
        let rx = pat.resolve(self)?;
        let txt = &self.text[range.0..range.1];
        let defs = pat.defs();
        let matched_range = if defs.is_empty() {
            // Pattern defines no variables. Fastest search is `find`.
            rx.find(txt)
        } else {
            // We need the captures to define variables.
            rx.captures(txt).map(|caps| {
                let matched_range = caps.pos(0).expect("whole expression must match");
                for var in defs {
                    let txtval = caps.name(var).unwrap_or("");
                    self.recorder.defined_var(var, txtval);
                    let vardef = VarDef {
                        value: Value::Text(Cow::Borrowed(txtval)),
                        // This offset is the end of the whole matched pattern, not just the text
                        // defining the variable.
                        offset: range.0 + matched_range.1,
                    };
                    self.vars.insert(var.clone(), vardef);
                }
                matched_range
            })
        };
        Ok(if let Some((b, e)) = matched_range {
               let r = (range.0 + b, range.0 + e);
               self.recorder.matched_check(rx.as_str(), r);
               Some(r)
           } else {
               self.recorder.missed_check(rx.as_str(), range);
               None
           })
    }
}

impl<'a> VariableMap for State<'a> {
    fn lookup(&self, varname: &str) -> Option<Value> {
        // First look for a local define.
        if let Some(&VarDef { ref value, .. }) = self.vars.get(varname) {
            Some(value.clone())
        } else {
            // No local, maybe an environment variable?
            self.env_vars.lookup(varname)
        }
    }
}

impl Display for Directive {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        use self::Directive::*;
        match *self {
            Check(ref pat) => writeln!(f, "check: {}", pat),
            SameLn(ref pat) => writeln!(f, "sameln: {}", pat),
            NextLn(ref pat) => writeln!(f, "nextln: {}", pat),
            Unordered(ref pat) => writeln!(f, "unordered: {}", pat),
            Not(ref pat) => writeln!(f, "not: {}", pat),
            Regex(ref var, ref rx) => writeln!(f, "regex: {}={}", var, rx),
        }
    }
}

impl Display for Checker {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        for (idx, dir) in self.directives.iter().enumerate() {
            write!(f, "#{} {}", idx, dir)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::CheckerBuilder;
    use error::Error;

    fn e2s(e: Error) -> String {
        e.to_string()
    }

    #[test]
    fn directive() {
        let mut b = CheckerBuilder::new();

        assert_eq!(b.directive("not here: more text").map_err(e2s), Ok(false));
        assert_eq!(b.directive("not here: regex: X=more text").map_err(e2s),
                   Ok(true));
        assert_eq!(b.directive("regex: X = tommy").map_err(e2s),
                   Err("expected '=' after variable 'X' in regex: X = tommy".to_string()));
        assert_eq!(b.directive("[arm]not:    patt $x $(y) here").map_err(e2s),
                   Ok(true));
        assert_eq!(b.directive("[x86]sameln: $x $(y=[^]]*) there").map_err(e2s),
                   Ok(true));
        // Windows line ending sneaking in.
        assert_eq!(b.directive("regex: Y=foo\r").map_err(e2s), Ok(true));

        let c = b.finish();
        assert_eq!(c.to_string(),
                   "#0 regex: X=more text\n#1 not: patt $(x) $(y) here\n#2 sameln: $(x) \
                    $(y=[^]]*) there\n#3 regex: Y=foo\n");
    }
}
