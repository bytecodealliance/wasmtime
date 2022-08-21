#![no_main]

use libfuzzer_sys::arbitrary::{Result, Unstructured};
use libfuzzer_sys::fuzz_target;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::SeqCst;
use std::sync::Once;
use wasmtime::Trap;
use wasmtime_fuzzing::generators::{Config, DiffValue, DiffValueType, SingleInstModule};
use wasmtime_fuzzing::oracles::diff_wasmtime::WasmtimeInstance;
use wasmtime_fuzzing::oracles::{differential, engine, log_wasm};

// Upper limit on the number of invocations for each WebAssembly function
// executed by this fuzz target.
const NUM_INVOCATIONS: usize = 5;

// Statistics about what's actually getting executed during fuzzing
static STATS: RuntimeStats = RuntimeStats::new();

// The spec interpreter requires special one-time setup.
static SETUP: Once = Once::new();

fuzz_target!(|data: &[u8]| {
    // To avoid a uncaught `SIGSEGV` due to signal handlers; see comments on
    // `setup_ocaml_runtime`.
    SETUP.call_once(|| engine::setup_engine_runtimes());

    // Errors in `run` have to do with not enough input in `data`, which we
    // ignore here since it doesn't affect how we'd like to fuzz.
    drop(run(&data));
});

fn run(data: &[u8]) -> Result<()> {
    STATS.bump_attempts();

    let mut u = Unstructured::new(data);
    let mut config: Config = u.arbitrary()?;
    config.set_differential_config();

    // Generate the Wasm module.
    let wasm = if u.arbitrary()? {
        STATS.wasm_smith_modules.fetch_add(1, SeqCst);
        let module = config.generate(&mut u, Some(1000))?;
        module.to_bytes()
    } else {
        STATS.single_instruction_modules.fetch_add(1, SeqCst);
        let module = SingleInstModule::new(&mut u, &mut config.module_config)?;
        module.to_bytes()
    };
    log_wasm(&wasm);

    // Choose a left-hand side Wasm engine.
    let mut lhs = engine::choose(&mut u, &config)?;
    let lhs_instance = lhs.instantiate(&wasm);
    STATS.bump_engine(lhs.name());

    // Choose a right-hand side Wasm engine--this will always be Wasmtime.
    let rhs_store = config.to_store();
    let rhs_module = wasmtime::Module::new(rhs_store.engine(), &wasm).unwrap();
    let rhs_instance = WasmtimeInstance::new(rhs_store, rhs_module);

    // If we fail to instantiate, check that both sides do.
    let (mut lhs_instance, mut rhs_instance) = match (lhs_instance, rhs_instance) {
        (Ok(l), Ok(r)) => (l, r),
        (Err(l), Err(r)) => {
            let err = r.downcast::<Trap>().expect("not a trap");
            lhs.assert_error_match(&err, l);
            return Ok(());
        }
        (l, r) => panic!(
            "failed to instantiate only one side: {:?} != {:?}",
            l.err(),
            r.err()
        ),
    };

    // Call each exported function with different sets of arguments.
    for (name, signature) in rhs_instance.exported_functions() {
        let mut invocations = 0;
        loop {
            let arguments = signature
                .params()
                .map(|t| DiffValue::arbitrary_of_type(&mut u, t.try_into().unwrap()))
                .collect::<Result<Vec<_>>>()?;
            let result_tys = signature
                .results()
                .map(|t| DiffValueType::try_from(t).unwrap())
                .collect::<Vec<_>>();
            differential(
                lhs_instance.as_mut(),
                &mut rhs_instance,
                &name,
                &arguments,
                &result_tys,
            )
            .expect("failed to run differential evaluation");

            // We evaluate the same function with different arguments until we
            // hit a predetermined limit or we run out of unstructured data--it
            // does not make sense to re-evaluate the same arguments over and
            // over.
            invocations += 1;
            STATS.total_invocations.fetch_add(1, SeqCst);
            if invocations > NUM_INVOCATIONS || u.is_empty() {
                break;
            }
        }
    }

    STATS.successes.fetch_add(1, SeqCst);
    Ok(())
}

#[derive(Default)]
struct RuntimeStats {
    /// Total number of fuzz inputs processed
    attempts: AtomicUsize,

    /// Number of times we've invoked engines
    total_invocations: AtomicUsize,

    /// Number of times a fuzz input finished all the way to the end without any
    /// sort of error (including `Arbitrary` errors)
    successes: AtomicUsize,

    // Counters for which engine was chosen
    wasmi: AtomicUsize,
    v8: AtomicUsize,
    spec: AtomicUsize,
    wasmtime: AtomicUsize,

    // Counters for which style of module is chosen
    wasm_smith_modules: AtomicUsize,
    single_instruction_modules: AtomicUsize,
}

impl RuntimeStats {
    const fn new() -> RuntimeStats {
        RuntimeStats {
            attempts: AtomicUsize::new(0),
            total_invocations: AtomicUsize::new(0),
            successes: AtomicUsize::new(0),
            wasmi: AtomicUsize::new(0),
            v8: AtomicUsize::new(0),
            spec: AtomicUsize::new(0),
            wasmtime: AtomicUsize::new(0),
            wasm_smith_modules: AtomicUsize::new(0),
            single_instruction_modules: AtomicUsize::new(0),
        }
    }

    fn bump_attempts(&self) {
        let attempts = self.attempts.fetch_add(1, SeqCst);
        if attempts == 0 || attempts % 1_000 != 0 {
            return;
        }
        let successes = self.successes.load(SeqCst);
        println!(
            "=== Execution rate ({} successes / {} attempted modules): {:.02}% ===",
            successes,
            attempts,
            successes as f64 / attempts as f64 * 100f64,
        );

        let v8 = self.v8.load(SeqCst);
        let spec = self.spec.load(SeqCst);
        let wasmi = self.wasmi.load(SeqCst);
        let wasmtime = self.wasmtime.load(SeqCst);
        let total = v8 + spec + wasmi + wasmtime;
        println!(
            "\twasmi: {:.02}%, spec: {:.02}%, wasmtime: {:.02}%, v8: {:.02}%",
            wasmi as f64 / total as f64 * 100f64,
            spec as f64 / total as f64 * 100f64,
            wasmtime as f64 / total as f64 * 100f64,
            v8 as f64 / total as f64 * 100f64,
        );

        let wasm_smith = self.wasm_smith_modules.load(SeqCst);
        let single_inst = self.single_instruction_modules.load(SeqCst);
        let total = wasm_smith + single_inst;
        println!(
            "\twasm-smith: {:.02}%, single-inst: {:.02}%",
            wasm_smith as f64 / total as f64 * 100f64,
            single_inst as f64 / total as f64 * 100f64,
        );
    }

    fn bump_engine(&self, name: &str) {
        match name {
            "wasmi" => self.wasmi.fetch_add(1, SeqCst),
            "wasmtime" => self.wasmtime.fetch_add(1, SeqCst),
            "spec" => self.spec.fetch_add(1, SeqCst),
            "v8" => self.v8.fetch_add(1, SeqCst),
            _ => return,
        };
    }
}
