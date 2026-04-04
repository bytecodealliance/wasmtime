#![no_main]

use libfuzzer_sys::arbitrary::{Arbitrary, Result, Unstructured};
use libfuzzer_sys::fuzz_target;
use std::sync::OnceLock;

// The first byte of fuzz input selects which fuzzer to run (via modular
// arithmetic), and the remaining bytes are passed as input to that fuzzer.
// Set the `FUZZER` environment variable to a function name (e.g.
// `FUZZER=stacks`) to run only that fuzzer.
const FUZZERS: &[(&str, fn(Unstructured<'_>) -> Result<()>)] = &[
    ("pulley_roundtrip", pulley_roundtrip),
    ("assembler_roundtrip", assembler_roundtrip),
    ("memory_accesses", memory_accesses),
    ("stacks", stacks),
    ("api_calls", api_calls),
    ("dominator_tree", dominator_tree),
];

static ENABLED: OnceLock<Vec<fn(Unstructured<'_>) -> Result<()>>> = OnceLock::new();

fuzz_target!(
    init: wasmtime_fuzzing::init_fuzzing(),
    |bytes: &[u8]| {
        let Some((&which, bytes)) = bytes.split_first() else {
            return;
        };
        let enabled = ENABLED.get_or_init(|| {
            let filter = std::env::var("FUZZER").ok();
            FUZZERS
                .iter()
                .filter(|(name, _)| filter.as_deref().is_none_or(|f| f == *name))
                .map(|(_, f)| *f)
                .collect()
        });
        if enabled.is_empty() {
            return;
        }
        let fuzzer = enabled[which as usize % enabled.len()];
        let _ = fuzzer(Unstructured::new(bytes));
    }
);

fn pulley_roundtrip(u: Unstructured<'_>) -> Result<()> {
    pulley_interpreter_fuzz::roundtrip(Arbitrary::arbitrary_take_rest(u)?);
    Ok(())
}

fn assembler_roundtrip(u: Unstructured<'_>) -> Result<()> {
    use cranelift_assembler_x64::{Inst, fuzz};
    let inst: Inst<fuzz::FuzzRegs> = Arbitrary::arbitrary_take_rest(u)?;
    fuzz::roundtrip(&inst);
    Ok(())
}

fn memory_accesses(u: Unstructured<'_>) -> Result<()> {
    wasmtime_fuzzing::oracles::memory::check_memory_accesses(Arbitrary::arbitrary_take_rest(u)?);
    Ok(())
}

fn stacks(u: Unstructured<'_>) -> Result<()> {
    wasmtime_fuzzing::oracles::check_stacks(Arbitrary::arbitrary_take_rest(u)?);
    Ok(())
}

fn api_calls(u: Unstructured<'_>) -> Result<()> {
    wasmtime_fuzzing::oracles::make_api_calls(Arbitrary::arbitrary_take_rest(u)?);
    Ok(())
}

fn dominator_tree(mut data: Unstructured<'_>) -> Result<()> {
    use cranelift_codegen::cursor::{Cursor, FuncCursor};
    use cranelift_codegen::dominator_tree::{DominatorTree, SimpleDominatorTree};
    use cranelift_codegen::flowgraph::ControlFlowGraph;
    use cranelift_codegen::ir::{
        Block, BlockCall, Function, InstBuilder, JumpTableData, Value, types::I32,
    };
    use std::collections::HashMap;

    const MAX_BLOCKS: u16 = 1 << 12;

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
                    .map(|&block| {
                        BlockCall::new(block, core::iter::empty(), &mut cursor.func.dfg.value_lists)
                    })
                    .collect::<Vec<_>>();

                let data = JumpTableData::new(block_calls[0], &block_calls[1..]);
                let jt = cursor.func.create_jump_table(data);
                cursor.ins().br_table(v0.unwrap(), jt);
            }
        } else {
            cursor.ins().return_(&[]);
        }
    }

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

    Ok(())
}
