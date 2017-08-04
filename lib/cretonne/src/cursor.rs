//! Cursor library.
//!
//! This module defines cursor data types that can be used for inserting instructions.

use ir;
use isa::TargetIsa;

// Re-export these types, anticipating their being moved here.
pub use ir::layout::CursorBase as Cursor;
pub use ir::layout::CursorPosition;
pub use ir::layout::Cursor as LayoutCursor;

/// Function cursor.
///
/// A `FuncCursor` holds a mutable reference to a whole `ir::Function` while keeping a position
/// too. The function can be re-borrowed by accessing the public `cur.func` member.
///
/// This cursor is for use before legalization. The inserted instructions are not given an
/// encoding.
pub struct FuncCursor<'f> {
    pos: CursorPosition,
    pub func: &'f mut ir::Function,
}

impl<'f> FuncCursor<'f> {
    /// Create a new `FuncCursor` pointing nowhere.
    pub fn new(func: &'f mut ir::Function) -> FuncCursor<'f> {
        FuncCursor {
            pos: CursorPosition::Nowhere,
            func,
        }
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
        self.insert_inst(inst);
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
    built_inst: Option<ir::Inst>,
    pub func: &'f mut ir::Function,
    pub isa: &'f TargetIsa,
}

impl<'f> EncCursor<'f> {
    /// Create a new `EncCursor` pointing nowhere.
    pub fn new(func: &'f mut ir::Function, isa: &'f TargetIsa) -> EncCursor<'f> {
        EncCursor {
            pos: CursorPosition::Nowhere,
            built_inst: None,
            func,
            isa,
        }
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

    fn insert_built_inst(self,
                         inst: ir::Inst,
                         ctrl_typevar: ir::Type)
                         -> &'c mut ir::DataFlowGraph {
        // Insert the instruction and remember the reference.
        self.insert_inst(inst);
        self.built_inst = Some(inst);

        // Assign an encoding.
        match self.isa
                  .encode(&self.func.dfg, &self.func.dfg[inst], ctrl_typevar) {
            Ok(e) => *self.func.encodings.ensure(inst) = e,
            Err(_) => panic!("can't encode {}", self.display_inst(inst)),
        }

        &mut self.func.dfg
    }
}
