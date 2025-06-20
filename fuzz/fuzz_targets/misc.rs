#![no_main]

use libfuzzer_sys::arbitrary::{Arbitrary, Result, Unstructured};
use libfuzzer_sys::fuzz_target;
use std::sync::OnceLock;

// Helper macro which takes a static list of fuzzers as input which are then
// delegated to internally based on the fuzz target selected.
//
// In general this fuzz target will execute a number of fuzzers all with the
// same input. The `FUZZER` environment variable can be used to forcibly disable
// all but one.
macro_rules! run_fuzzers {
    ($($fuzzer:ident)*) => {
        static ENABLED: OnceLock<u32> = OnceLock::new();

        fuzz_target!(|bytes: &[u8]| {
            // Use the first byte of input as a discriminant of which fuzzer to
            // select.
            let Some((which_fuzzer, bytes)) = bytes.split_first() else {
                return;
            };

            // Lazily initialize this fuzzer in terms of logging as well as
            // enabled fuzzers via the `FUZZER` env var. This creates a bitmask
            // inside of `ENABLED` of enabled fuzzers, returned here as
            // `enabled`.
            let enabled = *ENABLED.get_or_init(|| {
                env_logger::init();
                let configured = std::env::var("FUZZER").ok();
                let configured = configured.as_deref();
                let mut enabled = 0;
                let mut index = 0;

                $(
                    if configured.is_none() || configured == Some(stringify!($fuzzer)) {
                        enabled |= 1 << index;
                    }
                    index += 1;
                )*
                let _ = index;

                enabled
            });

            // Generate a linear check for each fuzzer. Only run each fuzzer if
            // the fuzzer is enabled, and also only if the `which_fuzzer`
            // discriminant matches the fuzzer being run.
            //
            // Note that it's a bit wonky here due to rust macros.
            let mut index = 0;
            $(
                if enabled & (1 << index) != 0 && *which_fuzzer == index {
                    let _: Result<()> = $fuzzer(Unstructured::new(bytes));
                }
                index += 1;
            )*
            let _ = index;
        });
    };
}

run_fuzzers! {
    pulley_roundtrip
    assembler_roundtrip
    memory_accesses
    table_ops
    stacks
    api_calls
    dominator_tree
}

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

fn table_ops(u: Unstructured<'_>) -> Result<()> {
    let (config, ops) = Arbitrary::arbitrary_take_rest(u)?;
    let _ = wasmtime_fuzzing::oracles::table_ops(config, ops);
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
