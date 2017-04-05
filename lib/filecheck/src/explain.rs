//! Explaining how *filecheck* matched or failed to match a file.

use MatchRange;
use std::fmt::{self, Display, Formatter};
use std::cmp::min;

/// Record events during matching.
pub trait Recorder {
    /// Set the directive we're talking about now.
    fn directive(&mut self, dct: usize);

    /// Matched a positive check directive (check/sameln/nextln/unordered).
    fn matched_check(&mut self, regex: &str, matched: MatchRange);

    /// Matched a `not:` directive. This means the match will fail.
    fn matched_not(&mut self, regex: &str, matched: MatchRange);

    /// Missed a positive check directive. The range given is the range searched for a match.
    fn missed_check(&mut self, regex: &str, searched: MatchRange);

    /// Missed `not:` directive (as intended).
    fn missed_not(&mut self, regex: &str, searched: MatchRange);

    /// The directive defined a variable.
    fn defined_var(&mut self, varname: &str, value: &str);
}

/// The null recorder just doesn't listen to anything you say.
impl Recorder for () {
    fn directive(&mut self, _: usize) {}
    fn matched_check(&mut self, _: &str, _: MatchRange) {}
    fn matched_not(&mut self, _: &str, _: MatchRange) {}
    fn defined_var(&mut self, _: &str, _: &str) {}
    fn missed_check(&mut self, _: &str, _: MatchRange) {}
    fn missed_not(&mut self, _: &str, _: MatchRange) {}
}

struct Match {
    directive: usize,
    is_match: bool,
    is_not: bool,
    regex: String,
    range: MatchRange,
}

struct VarDef {
    directive: usize,
    varname: String,
    value: String,
}

/// Record an explanation for the matching process, success or failure.
pub struct Explainer<'a> {
    text: &'a str,
    directive: usize,
    matches: Vec<Match>,
    vardefs: Vec<VarDef>,
}

impl<'a> Explainer<'a> {
    pub fn new(text: &'a str) -> Explainer {
        Explainer {
            text: text,
            directive: 0,
            matches: Vec::new(),
            vardefs: Vec::new(),
        }
    }

    /// Finish up after recording all events in a match.
    pub fn finish(&mut self) {
        self.matches.sort_by_key(|m| (m.range, m.directive));
        self.vardefs.sort_by_key(|v| v.directive);
    }
}

impl<'a> Display for Explainer<'a> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        // Offset of beginning of the last line printed.
        let mut curln = 0;
        // Offset of beginning of the next line to be printed.
        let mut nextln = 0;

        for m in &self.matches {
            // Emit lines until m.range.0 is visible.
            while nextln <= m.range.0 && nextln < self.text.len() {
                let newln = self.text[nextln..]
                    .find('\n')
                    .map(|d| nextln + d + 1)
                    .unwrap_or(self.text.len());
                assert!(newln > nextln);
                writeln!(f, "> {}", &self.text[nextln..newln - 1])?;
                curln = nextln;
                nextln = newln;
            }

            // Emit ~~~ under the part of the match in curln.
            if m.is_match {
                write!(f, "  ")?;
                let mend = min(m.range.1, nextln - 1);
                for pos in curln..mend {
                    if pos < m.range.0 {
                        write!(f, " ")
                    } else if pos == m.range.0 {
                        write!(f, "^")
                    } else {
                        write!(f, "~")
                    }?;
                }
                writeln!(f, "")?;
            }

            // Emit the match message itself.
            writeln!(f,
                     "{} #{}{}: {}",
                     if m.is_match { "Matched" } else { "Missed" },
                     m.directive,
                     if m.is_not { " not" } else { "" },
                     m.regex)?;

            // Emit any variable definitions.
            if let Ok(found) = self.vardefs
                   .binary_search_by_key(&m.directive, |v| v.directive) {
                let mut first = found;
                while first > 0 && self.vardefs[first - 1].directive == m.directive {
                    first -= 1;
                }
                for d in &self.vardefs[first..] {
                    if d.directive != m.directive {
                        break;
                    }
                    writeln!(f, "Define {}={}", d.varname, d.value)?;
                }
            }
        }

        // Emit trailing lines.
        for line in self.text[nextln..].lines() {
            writeln!(f, "> {}", line)?;
        }
        Ok(())
    }
}

impl<'a> Recorder for Explainer<'a> {
    fn directive(&mut self, dct: usize) {
        self.directive = dct;
    }

    fn matched_check(&mut self, regex: &str, matched: MatchRange) {
        self.matches
            .push(Match {
                      directive: self.directive,
                      is_match: true,
                      is_not: false,
                      regex: regex.to_owned(),
                      range: matched,
                  });
    }

    fn matched_not(&mut self, regex: &str, matched: MatchRange) {
        self.matches
            .push(Match {
                      directive: self.directive,
                      is_match: true,
                      is_not: true,
                      regex: regex.to_owned(),
                      range: matched,
                  });
    }

    fn missed_check(&mut self, regex: &str, searched: MatchRange) {
        self.matches
            .push(Match {
                      directive: self.directive,
                      is_match: false,
                      is_not: false,
                      regex: regex.to_owned(),
                      range: searched,
                  });
    }

    fn missed_not(&mut self, regex: &str, searched: MatchRange) {
        self.matches
            .push(Match {
                      directive: self.directive,
                      is_match: false,
                      is_not: true,
                      regex: regex.to_owned(),
                      range: searched,
                  });
    }

    fn defined_var(&mut self, varname: &str, value: &str) {
        self.vardefs
            .push(VarDef {
                      directive: self.directive,
                      varname: varname.to_owned(),
                      value: value.to_owned(),
                  });
    }
}
