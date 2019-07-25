//! Cursor library.
//!
//! This module defines cursor data types that can be used for inserting instructions.

use crate::ir;
use crate::isa::TargetIsa;

/// The possible positions of a cursor.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CursorPosition {
    /// Cursor is not pointing anywhere. No instructions can be inserted.
    Nowhere,
    /// Cursor is pointing at an existing instruction.
    /// New instructions will be inserted *before* the current instruction.
    At(ir::Inst),
    /// Cursor is before the beginning of an EBB. No instructions can be inserted. Calling
    /// `next_inst()` will move to the first instruction in the EBB.
    Before(ir::Ebb),
    /// Cursor is pointing after the end of an EBB.
    /// New instructions will be appended to the EBB.
    After(ir::Ebb),
}

/// All cursor types implement the `Cursor` which provides common navigation operations.
pub trait Cursor {
    /// Get the current cursor position.
    fn position(&self) -> CursorPosition;

    /// Set the current position.
    fn set_position(&mut self, pos: CursorPosition);

    /// Get the source location that should be assigned to new instructions.
    fn srcloc(&self) -> ir::SourceLoc;

    /// Set the source location that should be assigned to new instructions.
    fn set_srcloc(&mut self, srcloc: ir::SourceLoc);

    /// Borrow a reference to the function layout that this cursor is navigating.
    fn layout(&self) -> &ir::Layout;

    /// Borrow a mutable reference to the function layout that this cursor is navigating.
    fn layout_mut(&mut self) -> &mut ir::Layout;

    /// Exchange this cursor for one with a set source location.
    ///
    /// This is intended to be used as a builder method:
    ///
    /// ```
    /// # use cranelift_codegen::ir::{Function, Ebb, SourceLoc};
    /// # use cranelift_codegen::cursor::{Cursor, FuncCursor};
    /// fn edit_func(func: &mut Function, srcloc: SourceLoc) {
    ///     let mut pos = FuncCursor::new(func).with_srcloc(srcloc);
    ///
    ///     // Use `pos`...
    /// }
    /// ```
    fn with_srcloc(mut self, srcloc: ir::SourceLoc) -> Self
    where
        Self: Sized,
    {
        self.set_srcloc(srcloc);
        self
    }

    /// Rebuild this cursor positioned at `pos`.
    fn at_position(mut self, pos: CursorPosition) -> Self
    where
        Self: Sized,
    {
        self.set_position(pos);
        self
    }

    /// Rebuild this cursor positioned at `inst`.
    ///
    /// This is intended to be used as a builder method:
    ///
    /// ```
    /// # use cranelift_codegen::ir::{Function, Ebb, Inst};
    /// # use cranelift_codegen::cursor::{Cursor, FuncCursor};
    /// fn edit_func(func: &mut Function, inst: Inst) {
    ///     let mut pos = FuncCursor::new(func).at_inst(inst);
    ///
    ///     // Use `pos`...
    /// }
    /// ```
    fn at_inst(mut self, inst: ir::Inst) -> Self
    where
        Self: Sized,
    {
        self.goto_inst(inst);
        self
    }

    /// Rebuild this cursor positioned at the first insertion point for `ebb`.
    /// This differs from `at_first_inst` in that it doesn't assume that any
    /// instructions have been inserted into `ebb` yet.
    ///
    /// This is intended to be used as a builder method:
    ///
    /// ```
    /// # use cranelift_codegen::ir::{Function, Ebb, Inst};
    /// # use cranelift_codegen::cursor::{Cursor, FuncCursor};
    /// fn edit_func(func: &mut Function, ebb: Ebb) {
    ///     let mut pos = FuncCursor::new(func).at_first_insertion_point(ebb);
    ///
    ///     // Use `pos`...
    /// }
    /// ```
    fn at_first_insertion_point(mut self, ebb: ir::Ebb) -> Self
    where
        Self: Sized,
    {
        self.goto_first_insertion_point(ebb);
        self
    }

    /// Rebuild this cursor positioned at the first instruction in `ebb`.
    ///
    /// This is intended to be used as a builder method:
    ///
    /// ```
    /// # use cranelift_codegen::ir::{Function, Ebb, Inst};
    /// # use cranelift_codegen::cursor::{Cursor, FuncCursor};
    /// fn edit_func(func: &mut Function, ebb: Ebb) {
    ///     let mut pos = FuncCursor::new(func).at_first_inst(ebb);
    ///
    ///     // Use `pos`...
    /// }
    /// ```
    fn at_first_inst(mut self, ebb: ir::Ebb) -> Self
    where
        Self: Sized,
    {
        self.goto_first_inst(ebb);
        self
    }

    /// Rebuild this cursor positioned at the last instruction in `ebb`.
    ///
    /// This is intended to be used as a builder method:
    ///
    /// ```
    /// # use cranelift_codegen::ir::{Function, Ebb, Inst};
    /// # use cranelift_codegen::cursor::{Cursor, FuncCursor};
    /// fn edit_func(func: &mut Function, ebb: Ebb) {
    ///     let mut pos = FuncCursor::new(func).at_last_inst(ebb);
    ///
    ///     // Use `pos`...
    /// }
    /// ```
    fn at_last_inst(mut self, ebb: ir::Ebb) -> Self
    where
        Self: Sized,
    {
        self.goto_last_inst(ebb);
        self
    }

    /// Rebuild this cursor positioned after `inst`.
    ///
    /// This is intended to be used as a builder method:
    ///
    /// ```
    /// # use cranelift_codegen::ir::{Function, Ebb, Inst};
    /// # use cranelift_codegen::cursor::{Cursor, FuncCursor};
    /// fn edit_func(func: &mut Function, inst: Inst) {
    ///     let mut pos = FuncCursor::new(func).after_inst(inst);
    ///
    ///     // Use `pos`...
    /// }
    /// ```
    fn after_inst(mut self, inst: ir::Inst) -> Self
    where
        Self: Sized,
    {
        self.goto_after_inst(inst);
        self
    }

    /// Rebuild this cursor positioned at the top of `ebb`.
    ///
    /// This is intended to be used as a builder method:
    ///
    /// ```
    /// # use cranelift_codegen::ir::{Function, Ebb, Inst};
    /// # use cranelift_codegen::cursor::{Cursor, FuncCursor};
    /// fn edit_func(func: &mut Function, ebb: Ebb) {
    ///     let mut pos = FuncCursor::new(func).at_top(ebb);
    ///
    ///     // Use `pos`...
    /// }
    /// ```
    fn at_top(mut self, ebb: ir::Ebb) -> Self
    where
        Self: Sized,
    {
        self.goto_top(ebb);
        self
    }

    /// Rebuild this cursor positioned at the bottom of `ebb`.
    ///
    /// This is intended to be used as a builder method:
    ///
    /// ```
    /// # use cranelift_codegen::ir::{Function, Ebb, Inst};
    /// # use cranelift_codegen::cursor::{Cursor, FuncCursor};
    /// fn edit_func(func: &mut Function, ebb: Ebb) {
    ///     let mut pos = FuncCursor::new(func).at_bottom(ebb);
    ///
    ///     // Use `pos`...
    /// }
    /// ```
    fn at_bottom(mut self, ebb: ir::Ebb) -> Self
    where
        Self: Sized,
    {
        self.goto_bottom(ebb);
        self
    }

    /// Get the EBB corresponding to the current position.
    fn current_ebb(&self) -> Option<ir::Ebb> {
        use self::CursorPosition::*;
        match self.position() {
            Nowhere => None,
            At(inst) => self.layout().inst_ebb(inst),
            Before(ebb) | After(ebb) => Some(ebb),
        }
    }

    /// Get the instruction corresponding to the current position, if any.
    fn current_inst(&self) -> Option<ir::Inst> {
        use self::CursorPosition::*;
        match self.position() {
            At(inst) => Some(inst),
            _ => None,
        }
    }

    /// Go to the position after a specific instruction, which must be inserted
    /// in the layout. New instructions will be inserted after `inst`.
    fn goto_after_inst(&mut self, inst: ir::Inst) {
        debug_assert!(self.layout().inst_ebb(inst).is_some());
        let new_pos = if let Some(next) = self.layout().next_inst(inst) {
            CursorPosition::At(next)
        } else {
            CursorPosition::After(
                self.layout()
                    .inst_ebb(inst)
                    .expect("current instruction removed?"),
            )
        };
        self.set_position(new_pos);
    }

    /// Go to a specific instruction which must be inserted in the layout.
    /// New instructions will be inserted before `inst`.
    fn goto_inst(&mut self, inst: ir::Inst) {
        debug_assert!(self.layout().inst_ebb(inst).is_some());
        self.set_position(CursorPosition::At(inst));
    }

    /// Go to the position for inserting instructions at the beginning of `ebb`,
    /// which unlike `goto_first_inst` doesn't assume that any instructions have
    /// been inserted into `ebb` yet.
    fn goto_first_insertion_point(&mut self, ebb: ir::Ebb) {
        if let Some(inst) = self.layout().first_inst(ebb) {
            self.goto_inst(inst);
        } else {
            self.goto_bottom(ebb);
        }
    }

    /// Go to the first instruction in `ebb`.
    fn goto_first_inst(&mut self, ebb: ir::Ebb) {
        let inst = self.layout().first_inst(ebb).expect("Empty EBB");
        self.goto_inst(inst);
    }

    /// Go to the last instruction in `ebb`.
    fn goto_last_inst(&mut self, ebb: ir::Ebb) {
        let inst = self.layout().last_inst(ebb).expect("Empty EBB");
        self.goto_inst(inst);
    }

    /// Go to the top of `ebb` which must be inserted into the layout.
    /// At this position, instructions cannot be inserted, but `next_inst()` will move to the first
    /// instruction in `ebb`.
    fn goto_top(&mut self, ebb: ir::Ebb) {
        debug_assert!(self.layout().is_ebb_inserted(ebb));
        self.set_position(CursorPosition::Before(ebb));
    }

    /// Go to the bottom of `ebb` which must be inserted into the layout.
    /// At this position, inserted instructions will be appended to `ebb`.
    fn goto_bottom(&mut self, ebb: ir::Ebb) {
        debug_assert!(self.layout().is_ebb_inserted(ebb));
        self.set_position(CursorPosition::After(ebb));
    }

    /// Go to the top of the next EBB in layout order and return it.
    ///
    /// - If the cursor wasn't pointing at anything, go to the top of the first EBB in the
    ///   function.
    /// - If there are no more EBBs, leave the cursor pointing at nothing and return `None`.
    ///
    /// # Examples
    ///
    /// The `next_ebb()` method is intended for iterating over the EBBs in layout order:
    ///
    /// ```
    /// # use cranelift_codegen::ir::{Function, Ebb};
    /// # use cranelift_codegen::cursor::{Cursor, FuncCursor};
    /// fn edit_func(func: &mut Function) {
    ///     let mut cursor = FuncCursor::new(func);
    ///     while let Some(ebb) = cursor.next_ebb() {
    ///         // Edit ebb.
    ///     }
    /// }
    /// ```
    fn next_ebb(&mut self) -> Option<ir::Ebb> {
        let next = if let Some(ebb) = self.current_ebb() {
            self.layout().next_ebb(ebb)
        } else {
            self.layout().entry_block()
        };
        self.set_position(match next {
            Some(ebb) => CursorPosition::Before(ebb),
            None => CursorPosition::Nowhere,
        });
        next
    }

    /// Go to the bottom of the previous EBB in layout order and return it.
    ///
    /// - If the cursor wasn't pointing at anything, go to the bottom of the last EBB in the
    ///   function.
    /// - If there are no more EBBs, leave the cursor pointing at nothing and return `None`.
    ///
    /// # Examples
    ///
    /// The `prev_ebb()` method is intended for iterating over the EBBs in backwards layout order:
    ///
    /// ```
    /// # use cranelift_codegen::ir::{Function, Ebb};
    /// # use cranelift_codegen::cursor::{Cursor, FuncCursor};
    /// fn edit_func(func: &mut Function) {
    ///     let mut cursor = FuncCursor::new(func);
    ///     while let Some(ebb) = cursor.prev_ebb() {
    ///         // Edit ebb.
    ///     }
    /// }
    /// ```
    fn prev_ebb(&mut self) -> Option<ir::Ebb> {
        let prev = if let Some(ebb) = self.current_ebb() {
            self.layout().prev_ebb(ebb)
        } else {
            self.layout().last_ebb()
        };
        self.set_position(match prev {
            Some(ebb) => CursorPosition::After(ebb),
            None => CursorPosition::Nowhere,
        });
        prev
    }

    /// Move to the next instruction in the same EBB and return it.
    ///
    /// - If the cursor was positioned before an EBB, go to the first instruction in that EBB.
    /// - If there are no more instructions in the EBB, go to the `After(ebb)` position and return
    ///   `None`.
    /// - If the cursor wasn't pointing anywhere, keep doing that.
    ///
    /// This method will never move the cursor to a different EBB.
    ///
    /// # Examples
    ///
    /// The `next_inst()` method is intended for iterating over the instructions in an EBB like
    /// this:
    ///
    /// ```
    /// # use cranelift_codegen::ir::{Function, Ebb};
    /// # use cranelift_codegen::cursor::{Cursor, FuncCursor};
    /// fn edit_ebb(func: &mut Function, ebb: Ebb) {
    ///     let mut cursor = FuncCursor::new(func).at_top(ebb);
    ///     while let Some(inst) = cursor.next_inst() {
    ///         // Edit instructions...
    ///     }
    /// }
    /// ```
    /// The loop body can insert and remove instructions via the cursor.
    ///
    /// Iterating over all the instructions in a function looks like this:
    ///
    /// ```
    /// # use cranelift_codegen::ir::{Function, Ebb};
    /// # use cranelift_codegen::cursor::{Cursor, FuncCursor};
    /// fn edit_func(func: &mut Function) {
    ///     let mut cursor = FuncCursor::new(func);
    ///     while let Some(ebb) = cursor.next_ebb() {
    ///         while let Some(inst) = cursor.next_inst() {
    ///             // Edit instructions...
    ///         }
    ///     }
    /// }
    /// ```
    fn next_inst(&mut self) -> Option<ir::Inst> {
        use self::CursorPosition::*;
        match self.position() {
            Nowhere | After(..) => None,
            At(inst) => {
                if let Some(next) = self.layout().next_inst(inst) {
                    self.set_position(At(next));
                    Some(next)
                } else {
                    let pos = After(
                        self.layout()
                            .inst_ebb(inst)
                            .expect("current instruction removed?"),
                    );
                    self.set_position(pos);
                    None
                }
            }
            Before(ebb) => {
                if let Some(next) = self.layout().first_inst(ebb) {
                    self.set_position(At(next));
                    Some(next)
                } else {
                    self.set_position(After(ebb));
                    None
                }
            }
        }
    }

    /// Move to the previous instruction in the same EBB and return it.
    ///
    /// - If the cursor was positioned after an EBB, go to the last instruction in that EBB.
    /// - If there are no more instructions in the EBB, go to the `Before(ebb)` position and return
    ///   `None`.
    /// - If the cursor wasn't pointing anywhere, keep doing that.
    ///
    /// This method will never move the cursor to a different EBB.
    ///
    /// # Examples
    ///
    /// The `prev_inst()` method is intended for iterating backwards over the instructions in an
    /// EBB like this:
    ///
    /// ```
    /// # use cranelift_codegen::ir::{Function, Ebb};
    /// # use cranelift_codegen::cursor::{Cursor, FuncCursor};
    /// fn edit_ebb(func: &mut Function, ebb: Ebb) {
    ///     let mut cursor = FuncCursor::new(func).at_bottom(ebb);
    ///     while let Some(inst) = cursor.prev_inst() {
    ///         // Edit instructions...
    ///     }
    /// }
    /// ```
    fn prev_inst(&mut self) -> Option<ir::Inst> {
        use self::CursorPosition::*;
        match self.position() {
            Nowhere | Before(..) => None,
            At(inst) => {
                if let Some(prev) = self.layout().prev_inst(inst) {
                    self.set_position(At(prev));
                    Some(prev)
                } else {
                    let pos = Before(
                        self.layout()
                            .inst_ebb(inst)
                            .expect("current instruction removed?"),
                    );
                    self.set_position(pos);
                    None
                }
            }
            After(ebb) => {
                if let Some(prev) = self.layout().last_inst(ebb) {
                    self.set_position(At(prev));
                    Some(prev)
                } else {
                    self.set_position(Before(ebb));
                    None
                }
            }
        }
    }

    /// Insert an instruction at the current position.
    ///
    /// - If pointing at an instruction, the new instruction is inserted before the current
    ///   instruction.
    /// - If pointing at the bottom of an EBB, the new instruction is appended to the EBB.
    /// - Otherwise panic.
    ///
    /// In either case, the cursor is not moved, such that repeated calls to `insert_inst()` causes
    /// instructions to appear in insertion order in the EBB.
    fn insert_inst(&mut self, inst: ir::Inst) {
        use self::CursorPosition::*;
        match self.position() {
            Nowhere | Before(..) => panic!("Invalid insert_inst position"),
            At(cur) => self.layout_mut().insert_inst(inst, cur),
            After(ebb) => self.layout_mut().append_inst(inst, ebb),
        }
    }

    /// Remove the instruction under the cursor.
    ///
    /// The cursor is left pointing at the position following the current instruction.
    ///
    /// Return the instruction that was removed.
    fn remove_inst(&mut self) -> ir::Inst {
        let inst = self.current_inst().expect("No instruction to remove");
        self.next_inst();
        self.layout_mut().remove_inst(inst);
        inst
    }

    /// Remove the instruction under the cursor.
    ///
    /// The cursor is left pointing at the position preceding the current instruction.
    ///
    /// Return the instruction that was removed.
    fn remove_inst_and_step_back(&mut self) -> ir::Inst {
        let inst = self.current_inst().expect("No instruction to remove");
        self.prev_inst();
        self.layout_mut().remove_inst(inst);
        inst
    }

    /// Insert an EBB at the current position and switch to it.
    ///
    /// As far as possible, this method behaves as if the EBB header were an instruction inserted
    /// at the current position.
    ///
    /// - If the cursor is pointing at an existing instruction, *the current EBB is split in two*
    ///   and the current instruction becomes the first instruction in the inserted EBB.
    /// - If the cursor points at the bottom of an EBB, the new EBB is inserted after the current
    ///   one, and moved to the bottom of the new EBB where instructions can be appended.
    /// - If the cursor points to the top of an EBB, the new EBB is inserted above the current one.
    /// - If the cursor is not pointing at anything, the new EBB is placed last in the layout.
    ///
    /// This means that it is always valid to call this method, and it always leaves the cursor in
    /// a state that will insert instructions into the new EBB.
    fn insert_ebb(&mut self, new_ebb: ir::Ebb) {
        use self::CursorPosition::*;
        match self.position() {
            At(inst) => {
                self.layout_mut().split_ebb(new_ebb, inst);
                // All other cases move to `After(ebb)`, but in this case we'll stay `At(inst)`.
                return;
            }
            Nowhere => self.layout_mut().append_ebb(new_ebb),
            Before(ebb) => self.layout_mut().insert_ebb(new_ebb, ebb),
            After(ebb) => self.layout_mut().insert_ebb_after(new_ebb, ebb),
        }
        // For everything but `At(inst)` we end up appending to the new EBB.
        self.set_position(After(new_ebb));
    }
}

/// Function cursor.
///
/// A `FuncCursor` holds a mutable reference to a whole `ir::Function` while keeping a position
/// too. The function can be re-borrowed by accessing the public `cur.func` member.
///
/// This cursor is for use before legalization. The inserted instructions are not given an
/// encoding.
pub struct FuncCursor<'f> {
    pos: CursorPosition,
    srcloc: ir::SourceLoc,

    /// The referenced function.
    pub func: &'f mut ir::Function,
}

impl<'f> FuncCursor<'f> {
    /// Create a new `FuncCursor` pointing nowhere.
    pub fn new(func: &'f mut ir::Function) -> Self {
        Self {
            pos: CursorPosition::Nowhere,
            srcloc: Default::default(),
            func,
        }
    }

    /// Use the source location of `inst` for future instructions.
    pub fn use_srcloc(&mut self, inst: ir::Inst) {
        self.srcloc = self.func.srclocs[inst];
    }

    /// Create an instruction builder that inserts an instruction at the current position.
    pub fn ins(&mut self) -> ir::InsertBuilder<&mut FuncCursor<'f>> {
        ir::InsertBuilder::new(self)
    }
}

impl<'f> Cursor for FuncCursor<'f> {
    fn position(&self) -> CursorPosition {
        self.pos
    }

    fn set_position(&mut self, pos: CursorPosition) {
        self.pos = pos
    }

    fn srcloc(&self) -> ir::SourceLoc {
        self.srcloc
    }

    fn set_srcloc(&mut self, srcloc: ir::SourceLoc) {
        self.srcloc = srcloc;
    }

    fn layout(&self) -> &ir::Layout {
        &self.func.layout
    }

    fn layout_mut(&mut self) -> &mut ir::Layout {
        &mut self.func.layout
    }
}

impl<'c, 'f> ir::InstInserterBase<'c> for &'c mut FuncCursor<'f> {
    fn data_flow_graph(&self) -> &ir::DataFlowGraph {
        &self.func.dfg
    }

    fn data_flow_graph_mut(&mut self) -> &mut ir::DataFlowGraph {
        &mut self.func.dfg
    }

    fn insert_built_inst(self, inst: ir::Inst, _: ir::Type) -> &'c mut ir::DataFlowGraph {
        // TODO: Remove this assertion once #796 is fixed.
        #[cfg(feature = "basic-blocks")]
        #[cfg(debug_assertions)]
        {
            if let CursorPosition::At(_) = self.position() {
                if let Some(curr) = self.current_inst() {
                    if let Some(prev) = self.layout().prev_inst(curr) {
                        let prev_op = self.data_flow_graph()[prev].opcode();
                        let inst_op = self.data_flow_graph()[inst].opcode();
                        let curr_op = self.data_flow_graph()[curr].opcode();
                        if prev_op.is_branch() && !prev_op.is_terminator() {
                            if !inst_op.is_terminator() {
                                panic!(
                                    "Inserting instruction {} after {}, and before {}",
                                    inst_op, prev_op, curr_op
                                )
                            };
                        }
                    };
                };
            };
        }
        self.insert_inst(inst);
        if !self.srcloc.is_default() {
            self.func.srclocs[inst] = self.srcloc;
        }
        &mut self.func.dfg
    }
}

/// Encoding cursor.
///
/// An `EncCursor` can be used to insert instructions that are immediately assigned an encoding.
/// The cursor holds a mutable reference to the whole function which can be re-borrowed from the
/// public `pos.func` member.
pub struct EncCursor<'f> {
    pos: CursorPosition,
    srcloc: ir::SourceLoc,
    built_inst: Option<ir::Inst>,

    /// The referenced function.
    pub func: &'f mut ir::Function,

    /// The target ISA that will be used to encode instructions.
    pub isa: &'f dyn TargetIsa,
}

impl<'f> EncCursor<'f> {
    /// Create a new `EncCursor` pointing nowhere.
    pub fn new(func: &'f mut ir::Function, isa: &'f dyn TargetIsa) -> Self {
        Self {
            pos: CursorPosition::Nowhere,
            srcloc: Default::default(),
            built_inst: None,
            func,
            isa,
        }
    }

    /// Use the source location of `inst` for future instructions.
    pub fn use_srcloc(&mut self, inst: ir::Inst) {
        self.srcloc = self.func.srclocs[inst];
    }

    /// Create an instruction builder that will insert an encoded instruction at the current
    /// position.
    ///
    /// The builder will panic if it is used to insert an instruction that can't be encoded for
    /// `self.isa`.
    pub fn ins(&mut self) -> ir::InsertBuilder<&mut EncCursor<'f>> {
        ir::InsertBuilder::new(self)
    }

    /// Get the last built instruction.
    ///
    /// This returns the last instruction that was built using the `ins()` method on this cursor.
    /// Panics if no instruction was built.
    pub fn built_inst(&self) -> ir::Inst {
        self.built_inst.expect("No instruction was inserted")
    }

    /// Return an object that can display `inst`.
    ///
    /// This is a convenience wrapper for the DFG equivalent.
    pub fn display_inst(&self, inst: ir::Inst) -> ir::dfg::DisplayInst {
        self.func.dfg.display_inst(inst, self.isa)
    }
}

impl<'f> Cursor for EncCursor<'f> {
    fn position(&self) -> CursorPosition {
        self.pos
    }

    fn set_position(&mut self, pos: CursorPosition) {
        self.pos = pos
    }

    fn srcloc(&self) -> ir::SourceLoc {
        self.srcloc
    }

    fn set_srcloc(&mut self, srcloc: ir::SourceLoc) {
        self.srcloc = srcloc;
    }

    fn layout(&self) -> &ir::Layout {
        &self.func.layout
    }

    fn layout_mut(&mut self) -> &mut ir::Layout {
        &mut self.func.layout
    }
}

impl<'c, 'f> ir::InstInserterBase<'c> for &'c mut EncCursor<'f> {
    fn data_flow_graph(&self) -> &ir::DataFlowGraph {
        &self.func.dfg
    }

    fn data_flow_graph_mut(&mut self) -> &mut ir::DataFlowGraph {
        &mut self.func.dfg
    }

    fn insert_built_inst(
        self,
        inst: ir::Inst,
        ctrl_typevar: ir::Type,
    ) -> &'c mut ir::DataFlowGraph {
        // TODO: Remove this assertion once #796 is fixed.
        #[cfg(feature = "basic-blocks")]
        #[cfg(debug_assertions)]
        {
            if let CursorPosition::At(_) = self.position() {
                if let Some(curr) = self.current_inst() {
                    if let Some(prev) = self.layout().prev_inst(curr) {
                        let prev_op = self.data_flow_graph()[prev].opcode();
                        let inst_op = self.data_flow_graph()[inst].opcode();
                        if prev_op.is_branch() && !prev_op.is_terminator() {
                            if !inst_op.is_terminator() {
                                panic!(
                                    "Inserting instruction {} after {} and before {}",
                                    self.display_inst(inst),
                                    self.display_inst(prev),
                                    self.display_inst(curr)
                                )
                            };
                        }
                    };
                };
            };
        }
        // Insert the instruction and remember the reference.
        self.insert_inst(inst);
        self.built_inst = Some(inst);

        if !self.srcloc.is_default() {
            self.func.srclocs[inst] = self.srcloc;
        }
        // Assign an encoding.
        // XXX Is there a way to describe this error to the user?
        #[cfg_attr(feature = "cargo-clippy", allow(clippy::match_wild_err_arm))]
        match self
            .isa
            .encode(&self.func, &self.func.dfg[inst], ctrl_typevar)
        {
            Ok(e) => self.func.encodings[inst] = e,
            Err(_) => panic!("can't encode {}", self.display_inst(inst)),
        }

        &mut self.func.dfg
    }
}
