//! Tests for proof-carrying-code-based validation of memory accesses
//! in Wasmtime/Cranelift-compiled Wasm, with various combinations of
//! memory settings.

use wasmtime::*;

const TEST1: &'static str = r#"
(module
 (memory 1 1)
 (func (param i32) (result i32)
  local.get 0
  i32.load
  i32.load offset=0x10000))
"#;

const TEST2: &'static str = r#"
(module
 (memory 10 20)
 (func (param i32) (result i32)
  local.get 0
  i32.load
  i32.load offset=0x10000))
"#;

#[test]
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
fn test_build() {
    let _ = env_logger::try_init();
    const KIB: u64 = 1024;
    const MIB: u64 = 1024 * KIB;
    const GIB: u64 = 1024 * MIB;

    for test in [TEST1, TEST2] {
        for static_memory_maximum_size in [0, 64 * KIB, 1 * MIB, 4 * GIB, 6 * GIB] {
            for guard_size in [0, 64 * KIB, 2 * GIB] {
                log::trace!("test:\n{}\n", test);
                log::trace!(
                    "static {:x} guard {:x}",
                    static_memory_maximum_size,
                    guard_size
                );
                let mut cfg = Config::new();
                cfg.static_memory_maximum_size(static_memory_maximum_size);
                cfg.static_memory_guard_size(guard_size);
                cfg.dynamic_memory_guard_size(guard_size);
                cfg.cranelift_pcc(true);
                let engine = Engine::new(&cfg).unwrap();

                let _module =
                    Module::new(&engine, test).expect("compilation with PCC should succeed");
            }
        }
    }
}
