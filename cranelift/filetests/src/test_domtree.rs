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
use crate::subtest::{Context, SubTest, run_filecheck};
use cranelift_codegen::dominator_tree::DominatorTree;
use cranelift_codegen::flowgraph::ControlFlowGraph;
use cranelift_codegen::ir::Function;
use cranelift_codegen::ir::entities::AnyEntity;
use cranelift_reader::TestCommand;
use std::borrow::{Borrow, Cow};
use std::collections::HashMap;
use std::fmt::{self, Write};

struct TestDomtree;

pub fn subtest(parsed: &TestCommand) -> anyhow::Result<Box<dyn SubTest>> {
    assert_eq!(parsed.command, "domtree");
    if !parsed.options.is_empty() {
        anyhow::bail!("No options allowed on {parsed}")
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

                let expected_block = match func.layout.inst_block(inst) {
                    Some(expected_block) => expected_block,
                    _ => anyhow::bail!("instruction {inst} is not in layout"),
                };
                for src_block in tail.split_whitespace() {
                    let block = match context.details.map.lookup_str(src_block) {
                        Some(AnyEntity::Block(block)) => block,
                        _ => anyhow::bail!("expected defined block, got {src_block}"),
                    };

                    // Annotations say that `expected_block` is the idom of `block`.
                    if expected.insert(block, expected_block).is_some() {
                        anyhow::bail!("multiple dominators for {src_block}");
                    }

                    // Compare to computed domtree.
                    match domtree.idom(block) {
                        Some(got_block) if got_block != expected_block => {
                            anyhow::bail!(
                                "mismatching idoms for {src_block}:\n\
                                 want: {inst}, got: {got_block}"
                            );
                        }
                        None => {
                            anyhow::bail!(
                                "mismatching idoms for {src_block}:\n\
                                 want: {inst}, got: unreachable"
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
            if let Some(got_block) = domtree.idom(block) {
                anyhow::bail!(
                    "mismatching idoms for renumbered {block}:\n\
                     want: unreachable, got: {got_block}"
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
        write!(s, " {block}")?;
    }
    writeln!(s)?;

    // Compute and print out a pre-order of the dominator tree.
    writeln!(s, "domtree_preorder {{")?;
    let mut stack = Vec::new();
    stack.extend(func.layout.entry_block());
    while let Some(block) = stack.pop() {
        write!(s, "    {block}:")?;
        let i = stack.len();
        for ch in domtree.children(block) {
            write!(s, " {ch}")?;
            stack.push(ch);
        }
        writeln!(s)?;
        // Reverse the children we just pushed so we'll pop them in order.
        stack[i..].reverse();
    }
    writeln!(s, "}}")?;

    Ok(s)
}
