#![no_main]

use libfuzzer_sys::{
    arbitrary::{self, Arbitrary, Unstructured},
    fuzz_target,
};

use std::collections::HashMap;

use cranelift_codegen::cursor::{Cursor, FuncCursor};
use cranelift_codegen::dominator_tree::{DominatorTree, SimpleDominatorTree};
use cranelift_codegen::flowgraph::ControlFlowGraph;
use cranelift_codegen::ir::{
    types::I32, Block, BlockCall, Function, InstBuilder, JumpTableData, Value,
};

const MAX_BLOCKS: u16 = 1 << 12;

#[derive(Debug)]
struct ArbitraryFunction {
    func: Function,
}

fn build_func(data: &mut Unstructured<'_>) -> arbitrary::Result<Function> {
    let mut func = Function::new();

    let mut num_to_block = Vec::new();

    let mut cfg = HashMap::<Block, Vec<Block>>::new();

    for edge in data.arbitrary_iter::<(u16, u16)>()? {
        let (a, b) = edge?;

        let a = a % MAX_BLOCKS;
        let b = b % MAX_BLOCKS;

        while a >= num_to_block.len() as u16 {
            num_to_block.push(func.dfg.make_block());
        }

        let a = num_to_block[a as usize];

        while b >= num_to_block.len() as u16 {
            num_to_block.push(func.dfg.make_block());
        }

        let b = num_to_block[b as usize];

        cfg.entry(a).or_default().push(b);
    }

    let mut cursor = FuncCursor::new(&mut func);

    let mut v0: Option<Value> = None;

    for block in num_to_block {
        cursor.insert_block(block);

        if v0.is_none() {
            v0 = Some(cursor.ins().iconst(I32, 0));
        }

        if let Some(children) = cfg.get(&block) {
            if children.len() == 1 {
                cursor.ins().jump(children[0], &[]);
            } else {
                let block_calls = children
                    .iter()
                    .map(|&block| BlockCall::new(block, &[], &mut cursor.func.dfg.value_lists))
                    .collect::<Vec<_>>();

                let data = JumpTableData::new(block_calls[0], &block_calls[1..]);
                let jt = cursor.func.create_jump_table(data);
                cursor.ins().br_table(v0.unwrap(), jt);
            }
        } else {
            cursor.ins().return_(&[]);
        }
    }

    Ok(func)
}

impl Arbitrary<'_> for ArbitraryFunction {
    fn arbitrary(data: &mut Unstructured<'_>) -> arbitrary::Result<Self> {
        Ok(Self {
            func: build_func(data)?,
        })
    }
}

fuzz_target!(|func: ArbitraryFunction| {
    let func = &func.func;
    let cfg = ControlFlowGraph::with_function(&func);
    let domtree = DominatorTree::with_function(&func, &cfg);
    let expected_domtree = SimpleDominatorTree::with_function(&func, &cfg);

    for block in func.layout.blocks() {
        let expected = expected_domtree.idom(block);
        let got = domtree.idom(block);
        if expected != got {
            panic!("Expected dominator for {block} is {expected:?}, got {got:?}");
        }
    }
});
