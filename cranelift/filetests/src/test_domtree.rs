//! Test command for verifying dominator trees.
//!
//! The `test domtree` test command looks for annotations on instructions like this:
//!
//! ```clif
//!     jump block3 ; dominates: block3
//! ```
//!
//! This annotation means that the jump instruction is expected to be the immediate dominator of
//! `block3`.
//!
//! We verify that the dominator tree annotations are complete and correct.
//!

use crate::match_directive::match_directive;
use crate::subtest::{run_filecheck, Context, SubTest};
use cranelift_codegen::dominator_tree::{DominatorTree, DominatorTreePreorder};
use cranelift_codegen::flowgraph::ControlFlowGraph;
use cranelift_codegen::ir::entities::AnyEntity;
use cranelift_codegen::ir::Function;
use cranelift_reader::TestCommand;
use std::borrow::{Borrow, Cow};
use std::collections::HashMap;
use std::fmt::{self, Write};

struct TestDomtree;

pub fn subtest(parsed: &TestCommand) -> anyhow::Result<Box<dyn SubTest>> {
    assert_eq!(parsed.command, "domtree");
    if !parsed.options.is_empty() {
        anyhow::bail!("No options allowed on {}", parsed)
    }
    Ok(Box::new(TestDomtree))
}

impl SubTest for TestDomtree {
    fn name(&self) -> &'static str {
        "domtree"
    }

    // Extract our own dominator tree from
    fn run(&self, func: Cow<Function>, context: &Context) -> anyhow::Result<()> {
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
                        anyhow::bail!(
                            "annotation on non-inst {}: {}",
                            comment.entity,
                            comment.text
                        );
                    }
                };
                for src_block in tail.split_whitespace() {
                    let block = match context.details.map.lookup_str(src_block) {
                        Some(AnyEntity::Block(block)) => block,
                        _ => anyhow::bail!("expected defined block, got {}", src_block),
                    };

                    // Annotations say that `inst` is the idom of `block`.
                    if expected.insert(block, inst).is_some() {
                        anyhow::bail!("multiple dominators for {}", src_block);
                    }

                    // Compare to computed domtree.
                    match domtree.idom(block) {
                        Some(got_inst) if got_inst != inst => {
                            anyhow::bail!(
                                "mismatching idoms for {}:\n\
                                 want: {}, got: {}",
                                src_block,
                                inst,
                                got_inst
                            );
                        }
                        None => {
                            anyhow::bail!(
                                "mismatching idoms for {}:\n\
                                 want: {}, got: unreachable",
                                src_block,
                                inst
                            );
                        }
                        _ => {}
                    }
                }
            }
        }

        // Now we know that everything in `expected` is consistent with `domtree`.
        // All other block's should be either unreachable or the entry block.
        for block in func
            .layout
            .blocks()
            .skip(1)
            .filter(|block| !expected.contains_key(block))
        {
            if let Some(got_inst) = domtree.idom(block) {
                anyhow::bail!(
                    "mismatching idoms for renumbered {}:\n\
                     want: unreachable, got: {}",
                    block,
                    got_inst
                );
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
    for &block in domtree.cfg_postorder() {
        write!(s, " {}", block)?;
    }
    writeln!(s)?;

    // Compute and print out a pre-order of the dominator tree.
    writeln!(s, "domtree_preorder {{")?;
    let mut dtpo = DominatorTreePreorder::new();
    dtpo.compute(domtree, &func.layout);
    let mut stack = Vec::new();
    stack.extend(func.layout.entry_block());
    while let Some(block) = stack.pop() {
        write!(s, "    {}:", block)?;
        let i = stack.len();
        for ch in dtpo.children(block) {
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
