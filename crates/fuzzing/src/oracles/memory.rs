//! Oracles related to memory.

use crate::generators::{HeapImage, MemoryAccesses};
use wasmtime::*;

/// Oracle to perform the described memory accesses and check that they are all
/// in- or out-of-bounds as expected
pub fn check_memory_accesses(input: MemoryAccesses) {
    crate::init_fuzzing();
    log::info!("Testing memory accesses: {input:#x?}");

    let offset = input.offset;
    let growth = input.growth;
    let wasm = build_wasm(&input.image, offset);
    crate::oracles::log_wasm(&wasm);
    let offset = u64::from(offset);

    let mut config = input.config.to_wasmtime();

    // Force-enable proposals if the heap image needs them.
    if input.image.memory64 {
        config.wasm_memory64(true);
    }
    if input.image.page_size_log2.is_some() {
        config.wasm_custom_page_sizes(true);
    }

    let engine = Engine::new(&config).unwrap();
    let module = match Module::new(&engine, &wasm) {
        Ok(m) => m,
        Err(e) => {
            let e = format!("{e:?}");
            log::info!("Failed to create `Module`: {e}");
            if cfg!(feature = "fuzz-pcc") && e.contains("Compilation error: Proof-carrying-code") {
                return;
            }
            assert!(
                e.contains("bytes which exceeds the configured maximum of")
                    || e.contains("exceeds the limit of"),
                "bad module compilation error: {e:?}",
            );
            return;
        }
    };

    let limits = super::StoreLimits::new();
    let mut store = Store::new(&engine, limits);
    input.config.configure_store(&mut store);

    // If we are using fuel, make sure we add enough that we won't ever run out.
    if input.config.wasmtime.consume_fuel {
        store.set_fuel(u64::MAX).unwrap();
    }

    let instance = match Instance::new(&mut store, &module, &[]) {
        Ok(x) => x,
        Err(e) => {
            log::info!("Failed to instantiate: {e:?}");
            assert!(format!("{e:?}").contains("Cannot allocate memory"));
            return;
        }
    };

    let memory = instance.get_memory(&mut store, "memory").unwrap();
    let load8 = instance
        .get_typed_func::<u64, u32>(&mut store, "load8")
        .unwrap();
    let load16 = instance
        .get_typed_func::<u64, u32>(&mut store, "load16")
        .unwrap();
    let load32 = instance
        .get_typed_func::<u64, u32>(&mut store, "load32")
        .unwrap();
    let load64 = instance
        .get_typed_func::<u64, u64>(&mut store, "load64")
        .unwrap();

    let do_accesses = |store: &mut Store<_>, msg: &str| {
        let len = memory.data_size(&mut *store);
        let len = u64::try_from(len).unwrap();

        if let Some(n) = len.checked_sub(8).and_then(|n| n.checked_sub(offset)) {
            // Test various in-bounds accesses near the bound.
            for i in 0..=7 {
                let addr = n + i;
                assert!(addr + offset + 1 <= len);
                let result = load8.call(&mut *store, addr);
                assert!(
                    result.is_ok(),
                    "{msg}: len={len:#x}, offset={offset:#x}, load8({n:#x} + {i:#x} = {addr:#x}) \
                     should be in bounds, got {result:?}"
                );
            }
            for i in 0..=6 {
                let addr = n + offset + i;
                assert!(addr + 2 <= len);
                let result = load16.call(&mut *store, n + i);
                assert!(
                    result.is_ok(),
                    "{msg}: len={len:#x}, offset={offset:#x}, load16({n:#x} + {i:#x} = {addr:#x}) \
                     should be in bounds, got {result:?}"
                );
            }
            for i in 0..=4 {
                let addr = n + offset + i;
                assert!(addr + 4 <= len);
                let result = load32.call(&mut *store, n + i);
                assert!(
                    result.is_ok(),
                    "{msg}: len={len:#x}, offset={offset:#x}, load32({n:#x} + {i:#x} = {addr:#x}) \
                     should be in bounds, got {result:?}"
                );
            }
            assert!(n + offset + 8 <= len);
            let result = load64.call(&mut *store, n);
            assert!(
                result.is_ok(),
                "{msg}: len={len:#x}, offset={offset:#x}, load64({n:#x}) should be in bounds, \
                 got {result:?}"
            );

            // Test various out-of-bounds accesses overlapping the memory bound.
            for i in 1..2 {
                let addr = len - i;
                assert!(addr + offset + 2 > len);
                let result = load16.call(&mut *store, addr);
                assert!(
                    result.is_err(),
                    "{msg}: len={len:#x}, offset={offset:#x}, load16({len:#x} - {i:#x} = {addr:#x}) \
                     should trap, got {result:?}"
                );
            }
            for i in 1..4 {
                let addr = len - i;
                assert!(addr + offset + 4 > len);
                let result = load32.call(&mut *store, addr);
                assert!(
                    result.is_err(),
                    "{msg}: len={len:#x}, offset={offset:#x}, load32({len:#x} - {i:#x} = {addr:#x}) \
                     should trap, got {result:?}"
                );
            }
            for i in 1..8 {
                let addr = len - i;
                assert!(addr + offset + 8 > len);
                let result = load64.call(&mut *store, addr);
                assert!(
                    result.is_err(),
                    "{msg}: len={len:#x}, offset={offset:#x}, load64({len:#x} - {i:#x} = {addr:#x}) \
                     should trap, got {result:?}"
                );
            }
        }

        // Test that out-of-bounds accesses just after the memory bound trap.
        if let Some(n) = len.checked_sub(offset) {
            for i in 0..=1 {
                let addr = n + i;
                assert!(addr + offset + 1 > len);
                let result = load8.call(&mut *store, addr);
                assert!(
                    result.is_err(),
                    "{msg}: len={len:#x}, offset={offset:#x}, load8({n:#x} + {i:#x} = {addr:#x}) \
                     should trap, got {result:?}"
                );
                assert!(addr + offset + 2 > len);
                let result = load16.call(&mut *store, addr);
                assert!(
                    result.is_err(),
                    "{msg}: len={len:#x}, offset={offset:#x}, load16({n:#x} + {i:#x} = {addr:#x}) \
                     should trap, got {result:?}"
                );
                assert!(addr + offset + 4 > len);
                let result = load32.call(&mut *store, addr);
                assert!(
                    result.is_err(),
                    "{msg}: len={len:#x}, offset={offset:#x}, load32({n:#x} + {i:#x} = {addr:#x}) \
                     should trap, got {result:?}"
                );
                assert!(addr + offset + 8 > len);
                let result = load64.call(&mut *store, addr);
                assert!(
                    result.is_err(),
                    "{msg}: len={len:#x}, offset={offset:#x}, load64({n:#x} + {i:#x} = {addr:#x}) \
                     should trap, got {result:?}"
                );
            }
        }

        // Test out-of-bounds accesses near the end of the index type's range to
        // double check our overflow handling inside the bounds checks.
        let len_is_4gib = len == u64::from(u32::MAX) + 1;
        let end_delta = (input.image.memory64 && len_is_4gib) as u64;
        let max = if input.image.memory64 {
            u64::MAX
        } else {
            u64::from(u32::MAX)
        };
        for i in 0..(1 - end_delta) {
            let addr = max - i;
            let result = load8.call(&mut *store, addr);
            assert!(
                result.is_err(),
                "{msg}: len={len:#x}, offset={offset:#x}, load8({max:#x} - {i:#x} = {addr:#x}) \
                 should trap, got {result:?}"
            );
        }
        for i in 0..(2 - end_delta) {
            let addr = max - i;
            let result = load16.call(&mut *store, addr);
            assert!(
                result.is_err(),
                "{msg}: len={len:#x}, offset={offset:#x}, load16({max:#x} - {i:#x} = {addr:#x}) \
                 should trap, got {result:?}"
            );
        }
        for i in 0..(4 - end_delta) {
            let addr = max - i;
            let result = load32.call(&mut *store, addr);
            assert!(
                result.is_err(),
                "{msg}: len={len:#x}, offset={offset:#x}, load32({max:#x} - {i:#x} = {addr:#x}) \
                 should trap, got {result:?}"
            );
        }
        for i in 0..(8 - end_delta) {
            let addr = max - i;
            let result = load64.call(&mut *store, addr);
            assert!(
                result.is_err(),
                "{msg}: len={len:#x}, offset={offset:#x}, load64({max:#x} - {i:#x} = {addr:#x}) \
                 should trap, got {result:?}"
            );
        }
    };

    do_accesses(&mut store, "initial size");
    let _ = memory.grow(&mut store, u64::from(growth));
    do_accesses(&mut store, "after growing");
}

/// Build a Wasm module with a single memory in the shape of the given heap
/// image, exports that memory, and also exports four functions:
/// `load{8,16,32,64}`. Each of these functions takes an `i64` address,
/// truncates it to `i32` if the memory is not 64-bit, and loads its associated
/// number of bits from memory at `address + offset`.
///
/// ```wat
/// (module
///   (memory (export "memory") ...)
///   (func (export "load8") (param i64) (result i32)
///     (i32.load8_u offset=${offset} (local.get 0))
///   )
///   ...
/// )
/// ```
fn build_wasm(image: &HeapImage, offset: u32) -> Vec<u8> {
    let mut module = wasm_encoder::Module::new();

    {
        let mut types = wasm_encoder::TypeSection::new();
        types.function([wasm_encoder::ValType::I64], [wasm_encoder::ValType::I32]);
        types.function([wasm_encoder::ValType::I64], [wasm_encoder::ValType::I64]);
        module.section(&types);
    }

    {
        let mut funcs = wasm_encoder::FunctionSection::new();
        funcs.function(0);
        funcs.function(0);
        funcs.function(0);
        funcs.function(1);
        module.section(&funcs);
    }

    {
        let mut memories = wasm_encoder::MemorySection::new();
        memories.memory(wasm_encoder::MemoryType {
            minimum: u64::from(image.minimum),
            maximum: image.maximum.map(Into::into),
            memory64: image.memory64,
            shared: false,
            page_size_log2: image.page_size_log2,
        });
        module.section(&memories);
    }

    {
        let mut exports = wasm_encoder::ExportSection::new();
        exports.export("memory", wasm_encoder::ExportKind::Memory, 0);
        exports.export("load8", wasm_encoder::ExportKind::Func, 0);
        exports.export("load16", wasm_encoder::ExportKind::Func, 1);
        exports.export("load32", wasm_encoder::ExportKind::Func, 2);
        exports.export("load64", wasm_encoder::ExportKind::Func, 3);
        module.section(&exports);
    }

    {
        let mut code = wasm_encoder::CodeSection::new();
        {
            let mut func = wasm_encoder::Function::new([]);
            func.instruction(&wasm_encoder::Instruction::LocalGet(0));
            if !image.memory64 {
                func.instruction(&wasm_encoder::Instruction::I32WrapI64);
            }
            func.instruction(&wasm_encoder::Instruction::I32Load8U(
                wasm_encoder::MemArg {
                    offset: u64::from(offset),
                    align: 0,
                    memory_index: 0,
                },
            ));
            func.instruction(&wasm_encoder::Instruction::End);
            code.function(&func);
        }
        {
            let mut func = wasm_encoder::Function::new([]);
            func.instruction(&wasm_encoder::Instruction::LocalGet(0));
            if !image.memory64 {
                func.instruction(&wasm_encoder::Instruction::I32WrapI64);
            }
            func.instruction(&wasm_encoder::Instruction::I32Load16U(
                wasm_encoder::MemArg {
                    offset: u64::from(offset),
                    align: 0,
                    memory_index: 0,
                },
            ));
            func.instruction(&wasm_encoder::Instruction::End);
            code.function(&func);
        }
        {
            let mut func = wasm_encoder::Function::new([]);
            func.instruction(&wasm_encoder::Instruction::LocalGet(0));
            if !image.memory64 {
                func.instruction(&wasm_encoder::Instruction::I32WrapI64);
            }
            func.instruction(&wasm_encoder::Instruction::I32Load(wasm_encoder::MemArg {
                offset: u64::from(offset),
                align: 0,
                memory_index: 0,
            }));
            func.instruction(&wasm_encoder::Instruction::End);
            code.function(&func);
        }
        {
            let mut func = wasm_encoder::Function::new([]);
            func.instruction(&wasm_encoder::Instruction::LocalGet(0));
            if !image.memory64 {
                func.instruction(&wasm_encoder::Instruction::I32WrapI64);
            }
            func.instruction(&wasm_encoder::Instruction::I64Load(wasm_encoder::MemArg {
                offset: u64::from(offset),
                align: 0,
                memory_index: 0,
            }));
            func.instruction(&wasm_encoder::Instruction::End);
            code.function(&func);
        }
        module.section(&code);
    }

    {
        let mut datas = wasm_encoder::DataSection::new();
        for (offset, data) in image.segments.iter() {
            datas.segment(wasm_encoder::DataSegment {
                mode: wasm_encoder::DataSegmentMode::Active {
                    memory_index: 0,
                    offset: &if image.memory64 {
                        wasm_encoder::ConstExpr::i64_const(*offset as i64)
                    } else {
                        wasm_encoder::ConstExpr::i32_const(*offset as i32)
                    },
                },
                data: data.iter().copied(),
            });
        }
        module.section(&datas);
    }

    module.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use arbitrary::{Arbitrary, Unstructured};
    use rand::prelude::*;

    #[test]
    fn smoke_test_memory_access() {
        let mut rng = SmallRng::seed_from_u64(0);
        let mut buf = vec![0; 1024];

        for _ in 0..1024 {
            rng.fill_bytes(&mut buf);
            let u = Unstructured::new(&buf);
            if let Ok(input) = MemoryAccesses::arbitrary_take_rest(u) {
                check_memory_accesses(input);
            }
        }
    }
}
