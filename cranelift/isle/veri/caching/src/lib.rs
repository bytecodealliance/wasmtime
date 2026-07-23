//! A transparent caching layer over [`easy_smt`].
//!
//! This crate exposes a [`Context`] and [`ContextBuilder`] with the same API
//! as [`easy_smt::Context`] and [`easy_smt::ContextBuilder`], so a client can
//! switch to it by changing imports. The layer adds persistent caching of
//! solver query results:
//!
//! - The context tracks the *tree-path* of commands issued so far: a stack of
//!   frames delimited by `push`/`pop`, each holding the commands (declares,
//!   asserts, ...) issued within it. A `pop` discards its frame, so the path
//!   always reflects exactly the definitions visible to the solver at this
//!   point.
//!
//! - Any command that produces an interesting response — `check-sat`,
//!   `get-value`, `get-model`, and friends — is a cacheable point. Its cache
//!   key is derived from the solver name, the replay script of the current
//!   path, and the command itself; the cached value is the solver's response.
//!
//! - No solver subprocess is spawned until a query actually *misses* the
//!   cache. On a miss, a solver is launched and the recorded path is played
//!   into it to reconstruct the state, then the query is forwarded. The live
//!   solver is kept only while the path grows monotonically (so a `get-value`
//!   immediately after a missed `check-sat` does not re-solve); it is dropped
//!   on `pop` rather than reusing internal solver state across context
//!   frames.
//!
//! The cache itself ([`Cache`]) is a directory of JSON files with an optional
//! read-only source and a read-write destination, and supports a
//! read-only-enforcing mode that fails on any miss without ever invoking a
//! solver. See [`CacheMode`].

mod cache;
mod context;
mod convert;

pub use cache::{Cache, CacheMode};
pub use context::{Context, ContextBuilder};

// Re-export the easy-smt surface that clients use alongside the context, so
// that switching to this crate is a pure import change.
pub use easy_smt::{
    DisplayExpr, IntoBinary, IntoDecimal, IntoNumeral, KnownAtoms, Response, SExpr, SExprData,
};
