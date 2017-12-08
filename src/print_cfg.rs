//! The `print-cfg` sub-command.
//!
//! Read a series of Cretonne IL files and print their control flow graphs
//! in graphviz format.

use std::borrow::Cow;
use std::fmt::{Result, Write, Display, Formatter};

use CommandResult;
use cretonne::flowgraph::ControlFlowGraph;
use cretonne::ir::Function;
use cretonne::ir::instructions::BranchInfo;
use cton_reader::{parse_functions, TestCommand};
use filetest::subtest::{self, SubTest, Context, Result as STResult};
use utils::read_to_string;

pub fn run(files: Vec<String>) -> CommandResult {
    for (i, f) in files.into_iter().enumerate() {
        if i != 0 {
            println!("");
        }
        print_cfg(f)?
    }
    Ok(())
}

struct CFGPrinter<'a> {
    func: &'a Function,
    cfg: ControlFlowGraph,
}

impl<'a> CFGPrinter<'a> {
    pub fn new(func: &'a Function) -> CFGPrinter<'a> {
        CFGPrinter {
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
                    BranchInfo::Table(table) => {
                        write!(w, " | <{}>{} {}", inst, idata.opcode(), table)?
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
            for (parent, inst) in self.cfg.pred_iter(ebb) {
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

fn print_cfg(filename: String) -> CommandResult {
    let buffer = read_to_string(&filename).map_err(
        |e| format!("{}: {}", filename, e),
    )?;
    let items = parse_functions(&buffer).map_err(
        |e| format!("{}: {}", filename, e),
    )?;

    for (idx, func) in items.into_iter().enumerate() {
        if idx != 0 {
            println!("");
        }
        print!("{}", CFGPrinter::new(&func));
    }

    Ok(())
}

/// Object implementing the `test print-cfg` sub-test.
struct TestPrintCfg;

pub fn subtest(parsed: &TestCommand) -> STResult<Box<SubTest>> {
    assert_eq!(parsed.command, "print-cfg");
    if !parsed.options.is_empty() {
        Err(format!("No options allowed on {}", parsed))
    } else {
        Ok(Box::new(TestPrintCfg))
    }
}

impl SubTest for TestPrintCfg {
    fn name(&self) -> Cow<str> {
        Cow::from("print-cfg")
    }

    fn needs_verifier(&self) -> bool {
        false
    }

    fn run(&self, func: Cow<Function>, context: &Context) -> STResult<()> {
        subtest::run_filecheck(&CFGPrinter::new(&func).to_string(), context)
    }
}
