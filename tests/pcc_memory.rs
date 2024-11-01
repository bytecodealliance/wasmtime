//! Tests for proof-carrying-code-based validation of memory accesses
//! in Wasmtime/Cranelift-compiled Wasm, with various combinations of
//! memory settings.

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
mod pcc_memory_tests {
    use wasmtime::*;

    const TESTS: &'static [&'static str] = &[
        r#"
  local.get 0
  i32.load8_u
  drop
    "#,
        r#"
  local.get 0
  i32.load8_u offset=0x10000
  drop
    "#,
        r#"
  local.get 0
  i32.load16_u
  drop
    "#,
        r#"
  local.get 0
  i32.load16_u offset=0x10000
  drop
    "#,
        r#"
  local.get 0
  i32.load
  drop
    "#,
        r#"
  local.get 0
  i32.load offset=0x10000
  drop
    "#,
        r#"
  local.get 0
  i64.load
  drop
    "#,
        r#"
  local.get 0
  i64.load offset=0x10000
  drop
    "#,
    ];

    #[test]
    #[cfg_attr(miri, ignore)]
    fn test_build() {
        let _ = env_logger::try_init();
        const KIB: u64 = 1024;
        const MIB: u64 = 1024 * KIB;
        const GIB: u64 = 1024 * MIB;

        let mut bodies = vec![];
        for (mem_min, mem_max) in [(1, 1), (10, 20)] {
            for &snippet in TESTS {
                bodies.push(format!(
                    "(module (memory {mem_min} {mem_max}) (func (param i32) {snippet}))"
                ));
            }
            let all_snippets = TESTS
                .iter()
                .map(|s| s.to_owned())
                .collect::<Vec<_>>()
                .join("\n");
            bodies.push(format!(
                "(module (memory {mem_min} {mem_max}) (func (param i32) {all_snippets}))"
            ));
        }

        for test in &bodies {
            for static_memory_maximum_size in [4 * GIB] {
                for guard_size in [2 * GIB] {
                    for enable_spectre in [true /* not yet supported by PCC: false */] {
                        for _memory_bits in [32 /* not yet supported by PCC: 64 */] {
                            log::trace!("test:\n{}\n", test);
                            log::trace!(
                                "static {:x} guard {:x}",
                                static_memory_maximum_size,
                                guard_size
                            );
                            let mut cfg = Config::new();
                            cfg.static_memory_maximum_size(static_memory_maximum_size);
                            cfg.memory_guard_size(guard_size);
                            cfg.cranelift_pcc(true);
                            unsafe {
                                cfg.cranelift_flag_set(
                                    "enable_heap_access_spectre_mitigation",
                                    &enable_spectre.to_string(),
                                );
                            }
                            // TODO: substitute memory32/memory64 into
                            // test module.

                            let engine = Engine::new(&cfg).unwrap();

                            let _module = Module::new(&engine, test)
                                .expect("compilation with PCC should succeed");
                        }
                    }
                }
            }
        }
    }
}
