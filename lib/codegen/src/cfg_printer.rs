//! The `CFGPrinter` utility.

use std::fmt::{Display, Formatter, Result, Write};

use flowgraph::{BasicBlock, ControlFlowGraph};
use ir::instructions::BranchInfo;
use ir::Function;

/// A utility for pretty-printing the CFG of a `Function`.
pub struct CFGPrinter<'a> {
    func: &'a Function,
    cfg: ControlFlowGraph,
}

/// A utility for pretty-printing the CFG of a `Function`.
impl<'a> CFGPrinter<'a> {
    /// Create a new CFGPrinter.
    pub fn new(func: &'a Function) -> Self {
        Self {
            func,
            cfg: ControlFlowGraph::with_function(func),
        }
    }

    /// Write the CFG for this function to `w`.
    pub fn write(&self, w: &mut Write) -> Result {
        self.header(w)?;
        self.ebb_nodes(w)?;
        self.cfg_connections(w)?;
        writeln!(w, "}}")
    }

    fn header(&self, w: &mut Write) -> Result {
        writeln!(w, "digraph \"{}\" {{", self.func.name)?;
        if let Some(entry) = self.func.layout.entry_block() {
            writeln!(w, "    {{rank=min; {}}}", entry)?;
        }
        Ok(())
    }

    fn ebb_nodes(&self, w: &mut Write) -> Result {
        for ebb in &self.func.layout {
            write!(w, "    {} [shape=record, label=\"{{{}", ebb, ebb)?;
            // Add all outgoing branch instructions to the label.
            for inst in self.func.layout.ebb_insts(ebb) {
                let idata = &self.func.dfg[inst];
                match idata.analyze_branch(&self.func.dfg.value_lists) {
                    BranchInfo::SingleDest(dest, _) => {
                        write!(w, " | <{}>{} {}", inst, idata.opcode(), dest)?
                    }
                    BranchInfo::Table(table, dest) => {
                        write!(w, " | <{}>{} {}", inst, idata.opcode(), table)?;
                        if let Some(dest) = dest {
                            write!(w, " {}", dest)?
                        }
                    }
                    BranchInfo::NotABranch => {}
                }
            }
            writeln!(w, "}}\"]")?
        }
        Ok(())
    }

    fn cfg_connections(&self, w: &mut Write) -> Result {
        for ebb in &self.func.layout {
            for BasicBlock { ebb: parent, inst } in self.cfg.pred_iter(ebb) {
                writeln!(w, "    {}:{} -> {}", parent, inst, ebb)?;
            }
        }
        Ok(())
    }
}

impl<'a> Display for CFGPrinter<'a> {
    fn fmt(&self, f: &mut Formatter) -> Result {
        self.write(f)
    }
}
