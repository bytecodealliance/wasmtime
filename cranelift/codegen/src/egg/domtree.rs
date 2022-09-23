//! Extended domtree with various traversal support.

use crate::dominator_tree::DominatorTree;
use crate::ir::{Block, Function};
use alloc::vec::Vec;
use cranelift_entity::{packed_option::PackedOption, EntityRef};

#[derive(Clone, Debug)]
pub(crate) struct DomTreeWithChildren {
    nodes: Vec<DomTreeNode>,
    root: Block,
}

#[derive(Clone, Copy, Debug)]
struct DomTreeNode {
    children: PackedOption<Block>,
    next: PackedOption<Block>,
}

impl DomTreeWithChildren {
    pub(crate) fn new(func: &Function, domtree: &DominatorTree) -> DomTreeWithChildren {
        let mut nodes = vec![
            DomTreeNode {
                children: None.into(),
                next: None.into(),
            };
            func.dfg.num_blocks()
        ];
        for block in func.layout.blocks() {
            let idom_inst = match domtree.idom(block) {
                Some(idom_inst) => idom_inst,
                None => continue,
            };
            let idom = match func.layout.inst_block(idom_inst) {
                Some(idom) => idom,
                None => continue,
            };

            nodes[block.index()].next = nodes[idom.index()].children;
            nodes[idom.index()].children = block.into();
        }

        let root = func.layout.entry_block().unwrap();

        Self { nodes, root }
    }

    pub(crate) fn root(&self) -> Block {
        self.root
    }

    pub(crate) fn children<'a>(&'a self, block: Block) -> DomTreeChildIter<'a> {
        let block = self.nodes[block.index()].children;
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
        if self.block.is_none() {
            None
        } else {
            let block = self.block.unwrap();
            self.block = self.domtree.nodes[block.index()].next;
            Some(block)
        }
    }
}
