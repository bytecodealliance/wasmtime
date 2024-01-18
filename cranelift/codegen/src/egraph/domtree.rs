//! Extended domtree with various traversal support.

use crate::dominator_tree::DominatorTree;
use crate::ir::{Block, Function};
use cranelift_entity::{packed_option::PackedOption, SecondaryMap};

#[derive(Clone, Debug)]
/// Like a [`DominatorTree`], but with an explicit list of children,
/// rather than parent pointers.
pub(crate) struct DomTreeWithChildren {
    nodes: SecondaryMap<Block, DomTreeNode>,
    root: Block,
}

#[derive(Clone, Copy, Debug, Default)]
struct DomTreeNode {
    /// Points to the first child of the node, and implicitly to an entire
    /// linked list of the children.
    children: PackedOption<Block>,
    /// Points to the next sibling, if any.
    next: PackedOption<Block>,
}

impl DomTreeWithChildren {
    pub(crate) fn new(func: &Function, domtree: &DominatorTree) -> DomTreeWithChildren {
        let mut nodes: SecondaryMap<Block, DomTreeNode> =
            SecondaryMap::with_capacity(func.dfg.num_blocks());

        for block in func.layout.blocks() {
            let Some(idom_inst) = domtree.idom(block) else {
                continue;
            };
            let idom = func
                .layout
                .inst_block(idom_inst)
                .expect("Dominating instruction should be part of a block");

            // Insert at the front of nodes[idom].children
            nodes[block].next = nodes[idom].children;
            nodes[idom].children = block.into();
        }

        let root = func.layout.entry_block().unwrap();

        Self { nodes, root }
    }

    pub(crate) fn root(&self) -> Block {
        self.root
    }

    pub(crate) fn children<'a>(&'a self, block: Block) -> DomTreeChildIter<'a> {
        let block = self.nodes[block].children;
        DomTreeChildIter {
            domtree: self,
            block,
        }
    }
}

pub(crate) struct DomTreeChildIter<'a> {
    domtree: &'a DomTreeWithChildren,
    block: PackedOption<Block>,
}

impl<'a> Iterator for DomTreeChildIter<'a> {
    type Item = Block;
    fn next(&mut self) -> Option<Block> {
        self.block.expand().map(|block| {
            self.block = self.domtree.nodes[block].next;
            block
        })
    }
}
