//! Exception tables: catch handlers on `try_call` instructions.
//!
//! An exception table describes where execution flows after returning
//! from `try_call`. It contains both the "normal" destination -- the
//! block to branch to when the function returns without throwing an
//! exception -- and any "catch" destinations associated with
//! particular exception tags. Each target indicates the arguments to
//! pass to the block that receives control.
//!
//! Like other side-tables (e.g., jump tables), each exception table
//! must be used by only one instruction. Sharing is not permitted
//! because it can complicate transforms (how does one change the
//! table used by only one instruction if others also use it?).
//!
//! In order to allow the `try_call` instruction itself to remain
//! small, the exception table also contains the signature ID of the
//! called function.

use crate::ir::BlockCall;
use crate::ir::entities::{ExceptionTag, SigRef};
use crate::ir::instructions::ValueListPool;
use alloc::vec::Vec;
use core::fmt::{self, Display, Formatter};
use cranelift_entity::packed_option::PackedOption;
#[cfg(feature = "enable-serde")]
use serde_derive::{Deserialize, Serialize};

/// Contents of an exception table.
///
/// The "no exception" target for is stored as the last element of the
/// underlying vector.  It can be accessed through the `normal_return`
/// and `normal_return_mut` functions. Exceptional catch clauses may
/// be iterated using the `catches` and `catches_mut` functions.  All
/// targets may be iterated over using the `all_targets` and
/// `all_targets_mut` functions.
#[derive(Debug, Clone, PartialEq, Hash)]
#[cfg_attr(feature = "enable-serde", derive(Serialize, Deserialize))]
pub struct ExceptionTableData {
    /// All BlockCalls packed together. This is necessary because the
    /// rest of the compiler expects to be able to grab a slice of
    /// branch targets for any branch instruction. The last BlockCall
    /// is the normal-return destination, and the rest correspond to
    /// the tags in `tags` below. Thus, we have the invariant that
    /// `targets.len() == tags.len() + 1`.
    targets: Vec<BlockCall>,

    /// Tags corresponding to targets other than the first one.
    ///
    /// A tag value of `None` indicates a catch-all handler. The
    /// catch-all handler matches only if no other handler matches,
    /// regardless of the order in this vector.
    ///
    /// `tags[i]` corresponds to `targets[i]`. Note that there will be
    /// one more `targets` element than `tags` because the last
    /// element in `targets` is the normal-return path.
    tags: Vec<PackedOption<ExceptionTag>>,

    /// The signature of the function whose invocation is associated
    /// with this handler table.
    sig: SigRef,
}

impl ExceptionTableData {
    /// Create new exception-table data.
    ///
    /// This data represents the destinations upon return from
    /// `try_call` or `try_call_indirect` instruction. There are two
    /// possibilities: "normal return" (no exception thrown), or an
    /// exceptional return corresponding to one of the listed
    /// exception tags.
    ///
    /// The given tags are passed through to the metadata provided
    /// alongside the provided function body, and Cranelift itself
    /// does not implement an unwinder; thus, the meaning of the tags
    /// is ultimately up to the embedder of Cranelift. The tags are
    /// wrapped in `Option` to allow encoding a "catch-all" handler.
    ///
    /// The BlockCalls must have signatures that match the targeted
    /// blocks, as usual. These calls are allowed to use
    /// `BlockArg::TryCallRet` in the normal-return case, with types
    /// corresponding to the signature's return values, and
    /// `BlockArg::TryCallExn` in the exceptional-return cases, with
    /// types corresponding to native machine words and an arity
    /// corresponding to the number of payload values that the calling
    /// convention and platform support. (See [`isa::CallConv`] for
    /// more details.)
    pub fn new(
        sig: SigRef,
        normal_return: BlockCall,
        tags_and_targets: impl IntoIterator<Item = (Option<ExceptionTag>, BlockCall)>,
    ) -> Self {
        let mut targets = vec![];
        let mut tags = vec![];
        for (tag, target) in tags_and_targets {
            tags.push(tag.into());
            targets.push(target);
        }
        targets.push(normal_return);

        ExceptionTableData { targets, tags, sig }
    }

    /// Return a value that can display the contents of this exception
    /// table.
    pub fn display<'a>(&'a self, pool: &'a ValueListPool) -> DisplayExceptionTable<'a> {
        DisplayExceptionTable { table: self, pool }
    }

    /// Get the default target for the non-exceptional return case.
    pub fn normal_return(&self) -> &BlockCall {
        self.targets.last().unwrap()
    }

    /// Get the default target for the non-exceptional return case.
    pub fn normal_return_mut(&mut self) -> &mut BlockCall {
        self.targets.last_mut().unwrap()
    }

    /// Get the targets for exceptional return cases, together with
    /// their tags.
    pub fn catches(&self) -> impl Iterator<Item = (Option<ExceptionTag>, &BlockCall)> + '_ {
        self.tags
            .iter()
            .map(|tag| tag.expand())
            // Skips the last entry of `targets` (the normal return)
            // because `tags` is one element shorter.
            .zip(self.targets.iter())
    }

    /// Get the targets for exceptional return cases, together with
    /// their tags.
    pub fn catches_mut(
        &mut self,
    ) -> impl Iterator<Item = (Option<ExceptionTag>, &mut BlockCall)> + '_ {
        self.tags
            .iter()
            .map(|tag| tag.expand())
            // Skips the last entry of `targets` (the normal return)
            // because `tags` is one element shorter.
            .zip(self.targets.iter_mut())
    }

    /// Get all branch targets.
    pub fn all_branches(&self) -> &[BlockCall] {
        &self.targets[..]
    }

    /// Get all branch targets.
    pub fn all_branches_mut(&mut self) -> &mut [BlockCall] {
        &mut self.targets[..]
    }

    /// Get the signature of the function called with this exception
    /// table.
    pub fn signature(&self) -> SigRef {
        self.sig
    }

    /// Clears all entries in this exception table, but leaves the function signature.
    pub fn clear(&mut self) {
        self.tags.clear();
        self.targets.clear();
    }
}

/// A wrapper for the context required to display a
/// [ExceptionTableData].
pub struct DisplayExceptionTable<'a> {
    table: &'a ExceptionTableData,
    pool: &'a ValueListPool,
}

impl<'a> Display for DisplayExceptionTable<'a> {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> fmt::Result {
        write!(
            fmt,
            "{}, {}, [",
            self.table.sig,
            self.table.normal_return().display(self.pool)
        )?;
        let mut first = true;
        for (tag, block_call) in self.table.catches() {
            if first {
                write!(fmt, " ")?;
                first = false;
            } else {
                write!(fmt, ", ")?;
            }
            if let Some(tag) = tag {
                write!(fmt, "{}: {}", tag, block_call.display(self.pool))?;
            } else {
                write!(fmt, "default: {}", block_call.display(self.pool))?;
            }
        }
        let space = if first { "" } else { " " };
        write!(fmt, "{space}]")?;
        Ok(())
    }
}
