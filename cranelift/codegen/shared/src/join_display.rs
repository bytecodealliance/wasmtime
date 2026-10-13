//! This module contains utils for `strings.join(" ")` using [`core::fmt::Display`] trait.

use alloc::vec::Vec;
use core::cell::RefCell;
use core::fmt::Display;
use core::fmt::Formatter;
use core::fmt::Result;

/// Optimized `.join(" ")` for iterators. Taken from `rustdoc/display.rs`
pub trait Joined {
    /// Takes an iterator over elements that implement [`Display`], and format them into `f`, separated by `sep`.
    ///
    /// This is similar to [`Itertools::format`], but instead of returning an implementation of `Display`,
    /// it formats directly into a [`Formatter`].
    ///
    /// The performance of `joined` is slightly better than `format`, since it doesn't need to use a `Cell` to keep track of whether [`fmt`](Display::fmt)
    /// was already called (`joined`'s API doesn't allow it be called more than once).
    ///
    /// [`Itertools::format`]: https://docs.rs/itertools/latest/itertools/trait.Itertools.html#method.format
    fn joined(&mut self, sep: impl Display, f: &mut Formatter<'_>) -> Result;
}

impl<I, T> Joined for I
where
    I: Iterator<Item = T>,
    T: Display,
{
    fn joined(&mut self, sep: impl Display, f: &mut Formatter<'_>) -> Result {
        let Some(first) = self.next() else {
            return Ok(());
        };
        first.fmt(f)?;
        for item in self {
            sep.fmt(f)?;
            item.fmt(f)?;
        }
        Ok(())
    }
}

/// Joins strings, just like `.join(" ")` but with [`Display`]
pub struct JoinDisplay<'s, 'a, S: Display>(pub &'s str, pub &'a [S]);

impl<'s, 'a, S: Display> Display for JoinDisplay<'s, 'a, S> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let mut sep = false;
        for s in self.1 {
            if sep {
                f.write_str(self.0)?;
            }
            sep = true;
            s.fmt(f)?;
        }
        Ok(())
    }
}

/// Join extension trait for `&[&str]`
pub trait JoinDisplayExt<'s, 'a, S: Display> {
    /// Joins strings, just like `.join(" ")` but with [`Display`]
    fn join_display(&'a self, sep: &'s str) -> JoinDisplay<'s, 'a, S>;
}

impl<'s, 'a, S: Display> JoinDisplayExt<'s, 'a, S> for &[S] {
    fn join_display(&'a self, sep: &'s str) -> JoinDisplay<'s, 'a, S> {
        JoinDisplay(sep, self)
    }
}

/// Joins strings, just like `.join(" ")` but with [`Display`]
pub struct JoinDisplayVec<'s, S: Display>(pub &'s str, pub Vec<S>);

impl<'s, S: Display> Display for JoinDisplayVec<'s, S> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        JoinDisplay(self.0, &self.1).fmt(f)
    }
}

/// Join extension trait for [`Vec<&str>`]
pub trait JoinDisplayVecExt<'s, S: Display> {
    /// Joins strings, just like `.join(" ")` but with [`Display`]
    fn join_display(self, sep: &'s str) -> JoinDisplayVec<'s, S>;
}

impl<'s, S: Display> JoinDisplayVecExt<'s, S> for Vec<S> {
    fn join_display(self, sep: &'s str) -> JoinDisplayVec<'s, S> {
        JoinDisplayVec(sep, self)
    }
}

/// A [`Display`] adapter that calls the given closure only once.
pub struct DisplayOnceFn<F> {
    f: RefCell<Option<F>>,
}

impl<F> Display for DisplayOnceFn<F>
where
    F: FnOnce(&mut Formatter<'_>) -> Result,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        // Move the FnOnce out of the Option. Only the first caller gets `Some`.
        match self.f.borrow_mut().take() {
            Some(func) => func(f),
            None => f.write_str("called DisplayOnceFn more than once"),
        }
    }
}

/// Returns a [`Display`] adapter that calls the given closure only once.
///
/// Otherwise prints: `called DisplayOnceFn more than once`
pub fn display_once<F>(f: F) -> DisplayOnceFn<F>
where
    F: FnOnce(&mut Formatter<'_>) -> Result,
{
    DisplayOnceFn {
        f: RefCell::new(Some(f)),
    }
}
