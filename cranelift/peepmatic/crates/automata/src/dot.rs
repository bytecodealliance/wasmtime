//! Helpers for generating [GraphViz
//! Dot](https://graphviz.gitlab.io/_pages/pdf/dotguide.pdf) files to visually
//! render automata.
//!
//! **This module only exists when the `"dot"` cargo feature is enabled.**

use crate::{Automaton, Output, State};
use std::fmt::{Debug, Display};
use std::fs;
use std::hash::Hash;
use std::io::{self, Write};
use std::path::Path;

/// Format the user-provided bits of an `Automaton` for Graphviz Dot output.
///
/// There are two provided implementations of `DotFmt`:
///
/// * [`DebugDotFmt`][crate::dot::DebugDotFmt] -- format each type parameter
///   with its `std::fmt::Debug` implementation.
///
/// * [`DisplayDotFmt`][crate::dot::DisplayDotFmt] -- format each type parameter
///   with its `std::fmt::Display` implementation.
///
/// You can also implement this trait yourself if your type parameters don't
/// implement `Debug` or `Display`, or if you want to format them in some other
/// way.
pub trait DotFmt<TAlphabet, TState, TOutput> {
    /// Format a transition edge: `from ---input---> to`.
    ///
    /// This will be inside an [HTML
    /// label](https://www.graphviz.org/doc/info/shapes.html#html), so you may
    /// use balanced HTML tags.
    fn fmt_transition(
        &self,
        w: &mut impl Write,
        from: Option<&TState>,
        input: &TAlphabet,
        to: Option<&TState>,
    ) -> io::Result<()>;

    /// Format the custom data associated with a state.
    ///
    /// This will be inside an [HTML
    /// label](https://www.graphviz.org/doc/info/shapes.html#html), so you may
    /// use balanced HTML tags.
    fn fmt_state(&self, w: &mut impl Write, state: &TState) -> io::Result<()>;

    /// Format a transition's output or the final output of a final state.
    ///
    /// This will be inside an [HTML
    /// label](https://www.graphviz.org/doc/info/shapes.html#html), so you may
    /// use balanced HTML tags.
    fn fmt_output(&self, w: &mut impl Write, output: &TOutput) -> io::Result<()>;
}

impl<TAlphabet, TState, TOutput> Automaton<TAlphabet, TState, TOutput>
where
    TAlphabet: Clone + Eq + Hash + Ord,
    TState: Clone + Eq + Hash,
    TOutput: Output,
{
    /// Write this `Automaton` out as a [GraphViz
    /// Dot](https://graphviz.gitlab.io/_pages/pdf/dotguide.pdf) file at the
    /// given path.
    ///
    /// The `formatter` parameter controls how `TAlphabet`, `TState`, and
    /// `TOutput` are rendered. See the [`DotFmt`][crate::dot::DotFmt] trait for
    /// details.
    ///
    /// **This method only exists when the `"dot"` cargo feature is enabled.**
    pub fn write_dot_file(
        &self,
        formatter: &impl DotFmt<TAlphabet, TState, TOutput>,
        path: impl AsRef<Path>,
    ) -> io::Result<()> {
        let mut file = fs::File::create(path)?;
        self.write_dot(formatter, &mut file)?;
        Ok(())
    }

    /// Write this `Automaton` out to the given write-able as a [GraphViz
    /// Dot](https://graphviz.gitlab.io/_pages/pdf/dotguide.pdf) file.
    ///
    /// The `formatter` parameter controls how `TAlphabet`, `TState`, and
    /// `TOutput` are rendered. See the [`DotFmt`][crate::dot::DotFmt] trait for
    /// details.
    ///
    /// **This method only exists when the `"dot"` cargo feature is enabled.**
    pub fn write_dot(
        &self,
        formatter: &impl DotFmt<TAlphabet, TState, TOutput>,
        w: &mut impl Write,
    ) -> io::Result<()> {
        writeln!(w, "digraph {{")?;
        writeln!(w, "  rankdir = \"LR\";")?;
        writeln!(w, "  nodesep = 2;")?;

        // Fake state for the incoming arrow to the start state.
        writeln!(w, "  \"\" [shape = none];")?;

        // Each state, its associated custom data, and its final output.
        for (i, state_data) in self.state_data.iter().enumerate() {
            write!(
                w,
                r#"  state_{i} [shape = {shape}, label = <<table border="0"><tr><td cellpadding="5">{i}</td></tr><tr><td cellpadding="5">"#,
                i = i,
                shape = if self.final_states.contains_key(&State(i as u32)) {
                    "doublecircle"
                } else {
                    "circle"
                }
            )?;
            if let Some(state_data) = state_data {
                formatter.fmt_state(w, state_data)?;
            } else {
                write!(w, "(no state data)")?;
            }
            write!(w, "</td></tr>")?;
            if let Some(final_output) = self.final_states.get(&State(i as u32)) {
                write!(w, r#"<tr><td cellpadding="5" align="left">"#)?;
                formatter.fmt_output(w, final_output)?;
                write!(w, "</td></tr>")?;
            }
            writeln!(w, "</table>>];")?;
        }

        // Fake transition to the start state.
        writeln!(w, r#"  "" -> state_{};"#, self.start_state.0)?;

        // Transitions between states and their outputs.
        for (from, transitions) in self.transitions.iter().enumerate() {
            for (input, (to, output)) in transitions {
                write!(
                    w,
                    r#"  state_{from} -> state_{to} [label = <<table border="0"><tr><td cellpadding="5" align="left">Input:</td><td cellpadding="5" align="left">"#,
                    from = from,
                    to = to.0,
                )?;
                formatter.fmt_transition(
                    w,
                    self.state_data[from].as_ref(),
                    input,
                    self.state_data[to.0 as usize].as_ref(),
                )?;
                write!(
                    w,
                    r#"</td></tr><tr><td cellpadding="5" align="left">Output:</td><td cellpadding="5" align="left">"#,
                )?;
                formatter.fmt_output(w, output)?;
                writeln!(w, "</td></tr></table>>];")?;
            }
        }

        writeln!(w, "}}")?;
        Ok(())
    }
}

/// Format an `Automaton`'s `TAlphabet`, `TState`, and `TOutput` with their
/// `std::fmt::Debug` implementations.
#[derive(Debug)]
pub struct DebugDotFmt;

impl<TAlphabet, TState, TOutput> DotFmt<TAlphabet, TState, TOutput> for DebugDotFmt
where
    TAlphabet: Debug,
    TState: Debug,
    TOutput: Debug,
{
    fn fmt_transition(
        &self,
        w: &mut impl Write,
        _from: Option<&TState>,
        input: &TAlphabet,
        _to: Option<&TState>,
    ) -> io::Result<()> {
        write!(w, r#"<font face="monospace">{:?}</font>"#, input)
    }

    fn fmt_state(&self, w: &mut impl Write, state: &TState) -> io::Result<()> {
        write!(w, r#"<font face="monospace">{:?}</font>"#, state)
    }

    fn fmt_output(&self, w: &mut impl Write, output: &TOutput) -> io::Result<()> {
        write!(w, r#"<font face="monospace">{:?}</font>"#, output)
    }
}

/// Format an `Automaton`'s `TAlphabet`, `TState`, and `TOutput` with their
/// `std::fmt::Display` implementations.
#[derive(Debug)]
pub struct DisplayDotFmt;

impl<TAlphabet, TState, TOutput> DotFmt<TAlphabet, TState, TOutput> for DisplayDotFmt
where
    TAlphabet: Display,
    TState: Display,
    TOutput: Display,
{
    fn fmt_transition(
        &self,
        w: &mut impl Write,
        _from: Option<&TState>,
        input: &TAlphabet,
        _to: Option<&TState>,
    ) -> io::Result<()> {
        write!(w, "{}", input)
    }

    fn fmt_state(&self, w: &mut impl Write, state: &TState) -> io::Result<()> {
        write!(w, "{}", state)
    }

    fn fmt_output(&self, w: &mut impl Write, output: &TOutput) -> io::Result<()> {
        write!(w, "{}", output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Builder;

    #[test]
    fn test_write_dot() {
        let mut builder = Builder::<char, (), u64>::new();

        // Insert "mon" -> 1
        let mut insertion = builder.insert();
        insertion.next('m', 1).next('o', 0).next('n', 0);
        insertion.finish();

        // Insert "sat" -> 6
        let mut insertion = builder.insert();
        insertion.next('s', 6).next('a', 0).next('t', 0);
        insertion.finish();

        // Insert "sun" -> 0
        let mut insertion = builder.insert();
        insertion.next('s', 0).next('u', 0).next('n', 0);
        insertion.finish();

        let automata = builder.finish();

        let expected = r#"
digraph {
  rankdir = "LR";
  nodesep = 2;
  "" [shape = none];
  state_0 [shape = doublecircle, label = <<table border="0"><tr><td cellpadding="5">0</td></tr><tr><td cellpadding="5">(no state data)</td></tr><tr><td cellpadding="5" align="left"><font face="monospace">0</font></td></tr></table>>];
  state_1 [shape = circle, label = <<table border="0"><tr><td cellpadding="5">1</td></tr><tr><td cellpadding="5">(no state data)</td></tr></table>>];
  state_2 [shape = circle, label = <<table border="0"><tr><td cellpadding="5">2</td></tr><tr><td cellpadding="5">(no state data)</td></tr></table>>];
  state_3 [shape = circle, label = <<table border="0"><tr><td cellpadding="5">3</td></tr><tr><td cellpadding="5">(no state data)</td></tr></table>>];
  state_4 [shape = circle, label = <<table border="0"><tr><td cellpadding="5">4</td></tr><tr><td cellpadding="5">(no state data)</td></tr></table>>];
  state_5 [shape = circle, label = <<table border="0"><tr><td cellpadding="5">5</td></tr><tr><td cellpadding="5">(no state data)</td></tr></table>>];
  "" -> state_5;
  state_1 -> state_0 [label = <<table border="0"><tr><td cellpadding="5" align="left">Input:</td><td cellpadding="5" align="left"><font face="monospace">'n'</font></td></tr><tr><td cellpadding="5" align="left">Output:</td><td cellpadding="5" align="left"><font face="monospace">0</font></td></tr></table>>];
  state_2 -> state_1 [label = <<table border="0"><tr><td cellpadding="5" align="left">Input:</td><td cellpadding="5" align="left"><font face="monospace">'o'</font></td></tr><tr><td cellpadding="5" align="left">Output:</td><td cellpadding="5" align="left"><font face="monospace">0</font></td></tr></table>>];
  state_3 -> state_0 [label = <<table border="0"><tr><td cellpadding="5" align="left">Input:</td><td cellpadding="5" align="left"><font face="monospace">'t'</font></td></tr><tr><td cellpadding="5" align="left">Output:</td><td cellpadding="5" align="left"><font face="monospace">0</font></td></tr></table>>];
  state_4 -> state_3 [label = <<table border="0"><tr><td cellpadding="5" align="left">Input:</td><td cellpadding="5" align="left"><font face="monospace">'a'</font></td></tr><tr><td cellpadding="5" align="left">Output:</td><td cellpadding="5" align="left"><font face="monospace">6</font></td></tr></table>>];
  state_4 -> state_1 [label = <<table border="0"><tr><td cellpadding="5" align="left">Input:</td><td cellpadding="5" align="left"><font face="monospace">'u'</font></td></tr><tr><td cellpadding="5" align="left">Output:</td><td cellpadding="5" align="left"><font face="monospace">0</font></td></tr></table>>];
  state_5 -> state_2 [label = <<table border="0"><tr><td cellpadding="5" align="left">Input:</td><td cellpadding="5" align="left"><font face="monospace">'m'</font></td></tr><tr><td cellpadding="5" align="left">Output:</td><td cellpadding="5" align="left"><font face="monospace">1</font></td></tr></table>>];
  state_5 -> state_4 [label = <<table border="0"><tr><td cellpadding="5" align="left">Input:</td><td cellpadding="5" align="left"><font face="monospace">'s'</font></td></tr><tr><td cellpadding="5" align="left">Output:</td><td cellpadding="5" align="left"><font face="monospace">0</font></td></tr></table>>];
}
"#;

        let mut buf = vec![];
        automata.write_dot(&DebugDotFmt, &mut buf).unwrap();
        let actual = String::from_utf8(buf).unwrap();
        eprintln!("{}", actual);
        assert_eq!(expected.trim(), actual.trim());
    }
}
