//! Test command for verifying dominator trees.
//!
//! The `test domtree` test command looks for annotations on instructions like this:
//!
//! ```clif
//!     jump ebb3 ; dominates: ebb3
//! ```
//!
//! This annotation means that the jump instruction is expected to be the immediate dominator of
//! `ebb3`.
//!
//! We verify that the dominator tree annotations are complete and correct.
//!

use crate::match_directive::match_directive;
use crate::subtest::{run_filecheck, Context, SubTest, SubtestResult};
use cranelift_codegen::dominator_tree::{DominatorTree, DominatorTreePreorder};
use cranelift_codegen::flowgraph::ControlFlowGraph;
use cranelift_codegen::ir::entities::AnyEntity;
use cranelift_codegen::ir::Function;
use cranelift_reader::TestCommand;
use std::borrow::{Borrow, Cow};
use std::collections::HashMap;
use std::fmt::{self, Write};

struct TestDomtree;

pub fn subtest(parsed: &TestCommand) -> SubtestResult<Box<dyn SubTest>> {
    assert_eq!(parsed.command, "domtree");
    if !parsed.options.is_empty() {
        Err(format!("No options allowed on {}", parsed))
    } else {
        Ok(Box::new(TestDomtree))
    }
}

impl SubTest for TestDomtree {
    fn name(&self) -> &'static str {
        "domtree"
    }

    // Extract our own dominator tree from
    fn run(&self, func: Cow<Function>, context: &Context) -> SubtestResult<()> {
        let func = func.borrow();
        let cfg = ControlFlowGraph::with_function(func);
        let domtree = DominatorTree::with_function(func, &cfg);

        // Build an expected domtree from the source annotations.
        let mut expected = HashMap::new();
        for comment in &context.details.comments {
            if let Some(tail) = match_directive(comment.text, "dominates:") {
                let inst = match comment.entity {
                    AnyEntity::Inst(inst) => inst,
                    _ => {
                        return Err(format!(
                            "annotation on non-inst {}: {}",
                            comment.entity, comment.text
                        ));
                    }
                };
                for src_ebb in tail.split_whitespace() {
                    let ebb = match context.details.map.lookup_str(src_ebb) {
                        Some(AnyEntity::Ebb(ebb)) => ebb,
                        _ => return Err(format!("expected defined EBB, got {}", src_ebb)),
                    };

                    // Annotations say that `inst` is the idom of `ebb`.
                    if expected.insert(ebb, inst).is_some() {
                        return Err(format!("multiple dominators for {}", src_ebb));
                    }

                    // Compare to computed domtree.
                    match domtree.idom(ebb) {
                        Some(got_inst) if got_inst != inst => {
                            return Err(format!(
                                "mismatching idoms for {}:\n\
                                 want: {}, got: {}",
                                src_ebb, inst, got_inst
                            ));
                        }
                        None => {
                            return Err(format!(
                                "mismatching idoms for {}:\n\
                                 want: {}, got: unreachable",
                                src_ebb, inst
                            ));
                        }
                        _ => {}
                    }
                }
            }
        }

        // Now we know that everything in `expected` is consistent with `domtree`.
        // All other EBB's should be either unreachable or the entry block.
        for ebb in func
            .layout
            .ebbs()
            .skip(1)
            .filter(|ebb| !expected.contains_key(ebb))
        {
            if let Some(got_inst) = domtree.idom(ebb) {
                return Err(format!(
                    "mismatching idoms for renumbered {}:\n\
                     want: unrechable, got: {}",
                    ebb, got_inst
                ));
            }
        }

        let text = filecheck_text(func, &domtree).expect("formatting error");
        run_filecheck(&text, context)
    }
}

// Generate some output for filecheck testing
fn filecheck_text(func: &Function, domtree: &DominatorTree) -> Result<String, fmt::Error> {
    let mut s = String::new();

    write!(s, "cfg_postorder:")?;
    for &ebb in domtree.cfg_postorder() {
        write!(s, " {}", ebb)?;
    }
    writeln!(s)?;

    // Compute and print out a pre-order of the dominator tree.
    writeln!(s, "domtree_preorder {{")?;
    let mut dtpo = DominatorTreePreorder::new();
    dtpo.compute(domtree, &func.layout);
    let mut stack = Vec::new();
    stack.extend(func.layout.entry_block());
    while let Some(ebb) = stack.pop() {
        write!(s, "    {}:", ebb)?;
        let i = stack.len();
        for ch in dtpo.children(ebb) {
            write!(s, " {}", ch)?;
            stack.push(ch);
        }
        writeln!(s)?;
        // Reverse the children we just pushed so we'll pop them in order.
        stack[i..].reverse();
    }
    writeln!(s, "}}")?;

    Ok(s)
}
