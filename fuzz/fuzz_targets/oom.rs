#![no_main]

use libfuzzer_sys::arbitrary::{Arbitrary, Result, Unstructured};
use wasmtime::{Engine, Module, Store, Trap, Val, error::OutOfMemory};
use wasmtime_core::alloc::TryVec;
use wasmtime_fuzzing::generators::Config;
use wasmtime_fuzzing::oom::{OomTest, OomTestAllocator};
use wasmtime_fuzzing::oracles::dummy;
use wasmtime_fuzzing::single_module_fuzzer::KnownValid;

const OOM_TEST_ITERS: u32 = 10;
const OOM_TEST_FUEL: u64 = 1000;

#[global_allocator]
static GLOBAL_ALLOCATOR: OomTestAllocator = OomTestAllocator::new();

wasmtime_fuzzing::single_module_fuzzer!(execute gen_module);

#[derive(Debug)]
struct OomInput {
    config: Config,
    seed: u64,
}

impl<'a> Arbitrary<'a> for OomInput {
    fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self> {
        let mut config: Config = u.arbitrary()?;
        config.module_config.config.exceptions_enabled = false;
        config.module_config.config.gc_enabled = false;
        config.module_config.config.reference_types_enabled = false;
        config.module_config.function_references_enabled = false;
        config.module_config.config.export_everything = true;
        config.wasmtime.strategy =
            wasmtime_fuzzing::generators::InstanceAllocationStrategy::OnDemand;
        let seed = u.arbitrary()?;
        Ok(OomInput { config, seed })
    }
}

fn compile(config: &Config, wasm: &[u8]) -> wasmtime::Result<Vec<u8>> {
    let mut wasmtime_config = config.to_wasmtime();
    wasmtime_config.concurrency_support(false);
    wasmtime_config.consume_fuel(true);
    let engine = Engine::new(&wasmtime_config)?;
    let module = Module::new(&engine, wasm)?;
    module.serialize()
}

fn execute(
    module: &[u8],
    _known_valid: KnownValid,
    input: OomInput,
    _u: &mut Unstructured<'_>,
) -> Result<()> {
    if cfg!(not(arc_try_new)) {
        panic!(
            "The OOM fuzzer is disabled because `cfg(arc_try_new)` was not enabled. Build with \
             `RUSTFLAGS=--cfg=arc_try_new` to enable."
        );
    }

    let module_bytes = match compile(&input.config, module) {
        Ok(bytes) => bytes,
        Err(_) => return Ok(()),
    };

    let mut oom_config = input.config.to_wasmtime();
    oom_config.enable_compiler(false);
    oom_config.concurrency_support(false);
    oom_config.consume_fuel(true);

    // Prevent real process-level OOM: fuzzer-generated configs can set
    // `memory_reservation(0)`, forcing `mmap` to commit real pages that bypass
    // `OomTestAllocator`. Use large virtual reservations instead.
    oom_config.memory_reservation(1 << 32);
    oom_config.memory_guard_size(1 << 31);

    let oom_engine = match Engine::new(&oom_config) {
        Ok(e) => e,
        Err(_) => return Ok(()),
    };

    let _ = OomTest::new()
        .seed(input.seed)
        .max_iters(OOM_TEST_ITERS)
        .allow_alloc_after_oom(true)
        .alloc_succeeds_after_oom(true)
        .allow_missed_oom_errors(true)
        .fuzz(|| {
            let module = unsafe { Module::deserialize(&oom_engine, &module_bytes)? };

            let mut store = Store::try_new(&oom_engine, ())?;
            store.set_fuel(OOM_TEST_FUEL).unwrap();

            let linker = dummy::dummy_linker(&mut store, &module)?;
            let instance = linker.instantiate(&mut store, &module)?;

            'export_loop: for export in module.exports() {
                let extern_ty = export.ty();
                let Some(func_ty) = extern_ty.func() else {
                    continue;
                };
                let func = instance.get_func(&mut store, export.name()).unwrap();

                // Build default params; skip if any param type has no default.
                let mut params: TryVec<Val> = TryVec::with_capacity(func_ty.params().len())?;
                for p in func_ty.params() {
                    match p.default_value() {
                        Some(v) => params.push(v)?,
                        None => {
                            continue 'export_loop;
                        }
                    }
                }

                let mut results: TryVec<Val> = TryVec::with_capacity(func_ty.results().len())?;
                for _ in 0..func_ty.results().len() {
                    results.push(Val::I32(0))?;
                }

                match func.call(&mut store, &params, &mut results) {
                    // OOM; return from this OOM test iteration.
                    Err(e) if e.is::<OutOfMemory>() => return Err(e),

                    // Out of fuel; stop calling exports.
                    Err(e)
                        if e.downcast_ref::<Trap>()
                            .is_some_and(|trap| *trap == Trap::OutOfFuel) =>
                    {
                        break;
                    }

                    Err(_) | Ok(_) => {}
                }
            }

            Ok(())
        });

    Ok(())
}

fn gen_module(input: &mut OomInput, u: &mut Unstructured<'_>) -> Result<(Vec<u8>, KnownValid)> {
    let module = input.config.generate(u, Some(1000))?;
    Ok((module.to_bytes(), KnownValid::Yes))
}
