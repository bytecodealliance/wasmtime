//! This crate provides a text pattern matching library with functionality similar to the LLVM
//! project's [FileCheck command](http://llvm.org/docs/CommandGuide/FileCheck.html).
//!
//! A list of directives is typically extracted from a file containing a test case. The test case
//! is then run through the program under test, and its output matched against the directives.
//!
//! See the [CheckerBuilder](struct.CheckerBuilder.html) and [Checker](struct.Checker.html) types
//! for the main library API.
//!
//! # Directives
//!
//! These are the directives recognized by *filecheck*:
//! <pre class="rust">
//! <a href="#the-check-directive">check: <i>&lt;pattern&gt;</i></a>
//! <a href="#the-sameln-directive">sameln: <i>&lt;pattern&gt;</i></a>
//! <a href="#the-nextln-directive">nextln: <i>&lt;pattern&gt;</i></a>
//! <a href="#the-unordered-directive">unordered: <i>&lt;pattern&gt;</i></a>
//! <a href="#the-not-directive">not: <i>&lt;pattern&gt;</i></a>
//! <a href="#the-regex-directive">regex: <i>&lt;variable&gt;</i>=<i>&lt;regex&gt;</i></a>
//! </pre>
//! Each directive is described in more detail below.
//!
//! ## Example
//!
//! The Rust program below prints the primes less than 100. It has *filecheck* directives embedded
//! in comments:
//!
//! ```rust
//! fn is_prime(x: u32) -> bool {
//!     (2..x).all(|d| x % d != 0)
//! }
//!
//! // Check that we get the primes and nothing else:
//! //   regex: NUM=\d+
//! //   not: $NUM
//! //   check: 2
//! //   nextln: 3
//! //   check: 89
//! //   nextln: 97
//! //   not: $NUM
//! fn main() {
//!     for p in (2..10).filter(|&x| is_prime(x)) {
//!         println!("{}", p);
//!     }
//! }
//! ```
//!
//! A test driver compiles and runs the program, then pipes the output through *filecheck*:
//!
//! ```sh
//! $ rustc primes.rs
//! $ ./primes | cton-util filecheck -v
//! #0 regex: NUM=\d+
//! #1 not: $NUM
//! #2 check: 2
//! #3 nextln: 3
//! #4 check: 89
//! #5 nextln: 97
//! #6 not: $NUM
//! no match #1: \d+
//! > 2
//!   ~
//! match #2: \b2\b
//! > 3
//!   ~
//! match #3: \b3\b
//! > 5
//! > 7
//! ...
//! > 79
//! > 83
//! > 89
//!   ~~
//! match #4: \b89\b
//! > 97
//!   ~~
//! match #5: \b97\b
//! no match #6: \d+
//! OK
//! ```
//!
//! ## The `check:` directive
//!
//! Match patterns non-overlapping and in order:
//!
//! ```sh
//! #0 check: one
//! #1 check: two
//! ```
//!
//! These directives will match the string `"one two"`, but not `"two one"`. The second directive
//! must match after the first one, and it can't overlap.
//!
//! ## The `sameln:` directive
//!
//! Match a pattern in the same line as the previous match.
//!
//! ```sh
//! #0 check: one
//! #1 sameln: two
//! ```
//!
//! These directives will match the string `"one two"`, but not `"one\ntwo"`. The second match must
//! be in the same line as the first. Like the `check:` directive, the match must also follow the
//! first match, so `"two one" would not be matched.
//!
//! If there is no previous match, `sameln:` matches on the first line of the input.
//!
//! ## The `nextln:` directive
//!
//! Match a pattern in the next line after the previous match.
//!
//! ```sh
//! #0 check: one
//! #1 nextln: two
//! ```
//!
//! These directives will match the string `"one\ntwo"`, but not `"one two"` or `"one\n\ntwo"`.
//!
//! If there is no previous match, `nextln:` matches on the second line of the input as if there
//! were a previous match on the first line.
//!
//! ## The `unordered:` directive
//!
//! Match patterns in any order, and possibly overlapping each other.
//!
//! ```sh
//! #0 unordered: one
//! #1 unordered: two
//! ```
//!
//! These directives will match the string `"one two"` *and* the string `"two one"`.
//!
//! When a normal ordered match is inserted into a sequence of `unordered:` directives, it acts as
//! a barrier:
//!
//! ```sh
//! #0 unordered: one
//! #1 unordered: two
//! #2 check: three
//! #3 unordered: four
//! #4 unordered: five
//! ```
//!
//! These directives will match `"two one three four five"`, but not `"two three one four five"`.
//! The `unordered:` matches are not allowed to cross the ordered `check:` directive.
//!
//! When `unordered:` matches define and use variables, a topological order is enforced. This means
//! that a match referencing a variable must follow the match where the variable was defined:
//!
//! ```sh
//! #0 regex: V=\bv\d+\b
//! #1 unordered: $(va=$V) = load
//! #2 unordered: $(vb=$V) = iadd $va
//! #3 unordered: $(vc=$V) = load
//! #4 unordered: iadd $va, $vc
//! ```
//!
//! In the above directives, #2 must match after #1, and #4 must match after both #1 and #3, but
//! otherwise they can match in any order.
//!
//! ## The `not:` directive
//!
//! Check that a pattern *does not* appear between matches.
//!
//! ```sh
//! #0 check: one
//! #1 not: two
//! #2 check: three
//! ```
//!
//! The directives above will match `"one five three"`, but not `"one two three"`.
//!
//! The pattern in a `not:` directive can't define any variables. Since it never matches anything,
//! the variables would not get a value.
//!
//! ## The `regex:` directive
//!
//! Define a shorthand name for a regular expression.
//!
//! ```sh
//! #0 regex: ID=\b[_a-zA-Z][_0-9a-zA-Z]*\b
//! #1 check: $ID + $ID
//! ```
//!
//! The `regex:` directive gives a name to a regular expression which can then be used as part of a
//! pattern to match. Patterns are otherwise just plain text strings to match, so this is not
//! simple macro expansion.
//!
//! See [the Rust regex crate](../regex/index.html#syntax) for the regular expression syntax.
//!
//! # Patterns and variables
//!
//! Patterns are plain text strings to be matched in the input file. The dollar sign is used as an
//! escape character to expand variables. The following escape sequences are recognized:
//!
//! <pre>
//! $$                Match single dollar sign.
//! $()               Match the empty string.
//! $(=<i>&lt;regex&gt;</i>)       Match regular expression <i>&lt;regex&gt;</i>.
//! $<i>&lt;var&gt;</i>            Match contents of variable <i>&lt;var&gt;</i>.
//! $(<i>&lt;var&gt;</i>)          Match contents of variable <i>&lt;var&gt;</i>.
//! $(<i>&lt;var&gt;</i>=<i>&lt;regex&gt;</i>)  Match <i>&lt;regex&gt;</i>, then
//!                   define <i>&lt;var&gt;</i> as the matched text.
//! $(<i>&lt;var&gt;</i>=$<i>&lt;rxvar&gt;</i>) Match regex in <i>&lt;rxvar&gt;</i>, then
//!                   define <i>&lt;var&gt;</i> as the matched text.
//! </pre>
//!
//! Variables can contain either plain text or regular expressions. Plain text variables are
//! defined with the `$(var=...)` syntax in a previous directive. They match the same text again.
//! Backreferences within the same pattern are not allowed. When a variable is defined in a
//! pattern, it can't be referenced again in the same pattern.
//!
//! Regular expression variables are defined with the `regex:` directive. They match the regular
//! expression each time they are used, so the matches don't need to be identical.
//!
//! ## Word boundaries
//!
//! If a pattern begins or ends with a (plain text) letter or number, it will only match on a word
//! boundary. Use the `$()` empty string match to prevent this:
//!
//! ```sh
//! check: one$()
//! ```
//!
//! This will match `"one"` and `"onetwo"`, but not `"zeroone"`.
//!
//! The empty match syntax can also be used to require leading or trailing whitespace:
//!
//! ```sh
//! check: one, $()
//! ```
//!
//! This will match `"one, two"` , but not `"one,two"`. Without the `$()`, trailing whitespace
//! would be trimmed from the pattern.

#![deny(missing_docs)]

pub use error::{Error, Result};
pub use variable::{VariableMap, Value, NO_VARIABLES};
pub use checker::{Checker, CheckerBuilder};

extern crate regex;

mod error;
mod variable;
mod pattern;
mod checker;
mod explain;

/// The range of a match in the input text.
pub type MatchRange = (usize, usize);
