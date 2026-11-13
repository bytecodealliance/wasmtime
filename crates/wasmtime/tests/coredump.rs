#![cfg(all(feature = "coredump", feature = "cranelift", feature = "wat"))]

use wasmtime::*;

#[test]
#[cfg_attr(miri, ignore)]
fn coredump_custom_page_sizes() -> Result<()> {
    let mut config = Config::new();
    config.coredump_on_trap(true);
    config.wasm_custom_page_sizes(true);
    config.wasm_memory64(true);
    let engine = Engine::new(&config)?;

    for memory64 in [false, true] {
        // Cover empty memories, partial chunks, exact chunks, and a partial
        // chunk after a full one. Include zero-filled memory as well.
        for size in [5001, 100, 4096, 8192, 0] {
            for nonzero in [true, false] {
                let index_type = if memory64 { "i64" } else { "i32" };
                let module = Module::new(
                    &engine,
                    format!(
                        r#"(module
                            (memory (export "memory") {index_type} {size} (pagesize 1))
                            (func (export "trap") unreachable))"#
                    ),
                )?;
                let mut store = Store::new(&engine, ());
                let instance = Instance::new(&mut store, &module, &[])?;
                let memory = instance.get_memory(&mut store, "memory").unwrap();
                let mut expected = vec![0; size];
                if nonzero {
                    for offset in [0, 10, 4090, 4990] {
                        if offset < size {
                            let end = (offset + 20).min(size);
                            expected[offset..end].fill(42);
                        }
                    }
                    if let Some(last) = expected.last_mut() {
                        *last = 127;
                    }
                }
                memory.write(&mut store, 0, &expected)?;
                let trap = instance.get_typed_func::<(), ()>(&mut store, "trap")?;
                let err = trap.call(&mut store, ()).unwrap_err();
                let dump = err.downcast_ref::<WasmCoreDump>().unwrap();
                let bytes = dump.serialize(&mut store, "custom-page-sizes");
                wasmparser::Validator::new_with_features(wasmparser::WasmFeatures::all())
                    .validate_all(&bytes)?;

                let mut memories = Vec::new();
                let mut actual = vec![0; size];
                for payload in wasmparser::Parser::new(0).parse_all(&bytes) {
                    match payload? {
                        wasmparser::Payload::MemorySection(section) => {
                            for ty in section {
                                memories.push(ty?);
                            }
                        }
                        wasmparser::Payload::DataSection(section) => {
                            for segment in section {
                                let segment = segment?;
                                let wasmparser::DataKind::Active {
                                    memory_index,
                                    offset_expr,
                                } = segment.kind
                                else {
                                    panic!("core dump data must be active");
                                };
                                assert_eq!(memory_index, 0);
                                let offset = match offset_expr.get_operators_reader().read()? {
                                    wasmparser::Operator::I32Const { value } => {
                                        assert!(!memory64);
                                        value as usize
                                    }
                                    wasmparser::Operator::I64Const { value } => {
                                        assert!(memory64);
                                        value as usize
                                    }
                                    other => panic!("unexpected offset: {other:?}"),
                                };
                                actual[offset..offset + segment.data.len()]
                                    .copy_from_slice(segment.data);
                            }
                        }
                        _ => {}
                    }
                }
                assert!(
                    actual == expected,
                    "core dump memory bytes differ: size={size}, memory64={memory64}, nonzero={nonzero}"
                );
                assert_eq!(memories.len(), 1);
                assert_eq!(memories[0].initial, size as u64);
                assert_eq!(memories[0].page_size_log2, Some(0));
                assert_eq!(memories[0].memory64, memory64);
            }
        }
    }
    Ok(())
}
