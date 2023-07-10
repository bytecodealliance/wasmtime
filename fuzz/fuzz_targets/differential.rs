#![no_main]

use libfuzzer_sys::arbitrary::{Result, Unstructured};
use libfuzzer_sys::fuzz_target;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::SeqCst;
use std::sync::Once;
use wasmtime_fuzzing::generators::CompilerStrategy;
use wasmtime_fuzzing::generators::{Config, DiffValue, DiffValueType, SingleInstModule};
use wasmtime_fuzzing::oracles::diff_wasmtime::WasmtimeInstance;
use wasmtime_fuzzing::oracles::engine::{build_allowed_env_list, parse_env_list};
use wasmtime_fuzzing::oracles::{differential, engine, log_wasm, DiffEqResult};

// Upper limit on the number of invocations for each WebAssembly function
// executed by this fuzz target.
const NUM_INVOCATIONS: usize = 5;

// Only run once when the fuzz target loads.
static SETUP: Once = Once::new();

// Environment-specified configuration for controlling the kinds of engines and
// modules used by this fuzz target. E.g.:
// - ALLOWED_ENGINES=wasmi,spec cargo +nightly fuzz run ...
// - ALLOWED_ENGINES=-v8 cargo +nightly fuzz run ...
// - ALLOWED_MODULES=single-inst cargo +nightly fuzz run ...
// - FUZZ_WINCH=1 cargo +nightly fuzz run ...
static mut ALLOWED_ENGINES: Vec<&str> = vec![];
static mut ALLOWED_MODULES: Vec<&str> = vec![];
static mut FUZZ_WINCH: bool = false;

// Statistics about what's actually getting executed during fuzzing
static STATS: RuntimeStats = RuntimeStats::new();

fuzz_target!(|data: &[u8]| {
    SETUP.call_once(|| {
        // To avoid a uncaught `SIGSEGV` due to signal handlers; see comments on
        // `setup_ocaml_runtime`.
        engine::setup_engine_runtimes();

        // Retrieve the configuration for this fuzz target from `ALLOWED_*`
        // environment variables.
        let allowed_engines = build_allowed_env_list(
            parse_env_list("ALLOWED_ENGINES"),
            &["wasmtime", "wasmi", "spec", "v8"],
        );
        let allowed_modules = build_allowed_env_list(
            parse_env_list("ALLOWED_MODULES"),
            &["wasm-smith", "single-inst"],
        );

        let fuzz_winch = match std::env::var("FUZZ_WINCH").map(|v| v == "1") {
            Ok(v) => v,
            _ => false,
        };

        unsafe {
            ALLOWED_ENGINES = allowed_engines;
            ALLOWED_MODULES = allowed_modules;
            FUZZ_WINCH = fuzz_winch;
        }
    });

    // Errors in `run` have to do with not enough input in `data`, which we
    // ignore here since it doesn't affect how we'd like to fuzz.
    drop(execute_one(&data));
});

fn execute_one(data: &[u8]) -> Result<()> {
    STATS.bump_attempts();

    let mut u = Unstructured::new(data);
    let fuzz_winch = unsafe { FUZZ_WINCH };

    // Generate a Wasmtime and module configuration and update its settings
    // initially to be suitable for differential execution where the generated
    // wasm will behave the same in two different engines. This will get further
    // refined below.
    let mut config: Config = u.arbitrary()?;
    config.set_differential_config();

    // When fuzzing Winch, explicitly override the compiler strategy, which by
    // default its arbitrary implementation unconditionally returns
    // `Cranelift`.
    // We also explicitly disable multi-value support.
    if fuzz_winch {
        config.wasmtime.compiler_strategy = CompilerStrategy::Winch;
        config.module_config.config.multi_value_enabled = false;
    }

    // Choose an engine that Wasmtime will be differentially executed against.
    // The chosen engine is then created, which might update `config`, and
    // returned as a trait object.
    let lhs = u.choose(unsafe { &ALLOWED_ENGINES })?;
    let mut lhs = match engine::build(&mut u, lhs, &mut config)? {
        Some(engine) => engine,
        // The chosen engine does not have support compiled into the fuzzer,
        // discard this test case.
        None => return Ok(()),
    };

    // Using the now-legalized module configuration generate the Wasm module;
    // this is specified by either the ALLOWED_MODULES environment variable or a
    // random selection between wasm-smith and single-inst.
    let build_wasm_smith_module = |u: &mut Unstructured, config: &Config| -> Result<_> {
        STATS.wasm_smith_modules.fetch_add(1, SeqCst);
        let module = config.generate(u, Some(1000))?;
        Ok(module.to_bytes())
    };
    let build_single_inst_module = |u: &mut Unstructured, config: &Config| -> Result<_> {
        STATS.single_instruction_modules.fetch_add(1, SeqCst);
        let module = SingleInstModule::new(u, &config.module_config)?;
        Ok(module.to_bytes())
    };
    if unsafe { ALLOWED_MODULES.is_empty() } {
        panic!("unable to generate a module to fuzz against; check `ALLOWED_MODULES`")
    }
    let wasm = match *u.choose(unsafe { ALLOWED_MODULES.as_slice() })? {
        "wasm-smith" => build_wasm_smith_module(&mut u, &config)?,
        "single-inst" => build_single_inst_module(&mut u, &config)?,
        _ => unreachable!(),
    };

    if fuzz_winch && !winch_supports_module(&wasm) {
        return Ok(());
    }

    log_wasm(&wasm);

    // Instantiate the generated wasm file in the chosen differential engine.
    let lhs_instance = lhs.instantiate(&wasm);
    STATS.bump_engine(lhs.name());

    // Always use Wasmtime as the second engine to instantiate within.
    let rhs_store = config.to_store();
    let rhs_module = wasmtime::Module::new(rhs_store.engine(), &wasm).unwrap();
    let rhs_instance = WasmtimeInstance::new(rhs_store, rhs_module);

    let (mut lhs_instance, mut rhs_instance) =
        match DiffEqResult::new(&*lhs, lhs_instance, rhs_instance) {
            // Both sides successful, continue below to invoking exports.
            DiffEqResult::Success(l, r) => (l, r),

            // Both sides failed, or computation has diverged. In both cases this
            // test case is done.
            DiffEqResult::Poisoned | DiffEqResult::Failed => return Ok(()),
        };

    // Call each exported function with different sets of arguments.
    'outer: for (name, signature) in rhs_instance.exported_functions() {
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
            let ok = differential(
                lhs_instance.as_mut(),
                lhs.as_ref(),
                &mut rhs_instance,
                &name,
                &arguments,
                &result_tys,
            )
            .expect("failed to run differential evaluation");

            invocations += 1;
            STATS.total_invocations.fetch_add(1, SeqCst);

            // If this differential execution has resulted in the two instances
            // diverging in state we can't keep executing so don't execute any
            // more functions.
            if !ok {
                break 'outer;
            }

            // We evaluate the same function with different arguments until we
            // Hit a predetermined limit or we run out of unstructured data--it
            // does not make sense to re-evaluate the same arguments over and
            // over.
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

// Returns true if the module only contains operators supported by
// Winch. Winch's x86_64 target has broader support for Wasm operators
// than the aarch64 target. This list assumes fuzzing on the x86_64
// target.
fn winch_supports_module(module: &[u8]) -> bool {
    use wasmparser::{Operator::*, Parser, Payload};

    let mut supported = true;
    let mut parser = Parser::new(0).parse_all(module);

    'main: while let Some(payload) = parser.next() {
        match payload.unwrap() {
            Payload::CodeSectionEntry(body) => {
                let op_reader = body.get_operators_reader().unwrap();
                for op in op_reader {
                    match op.unwrap() {
                        I32Const { .. }
                        | I64Const { .. }
                        | I32Add { .. }
                        | I64Add { .. }
                        | I32Sub { .. }
                        | I32Mul { .. }
                        | I32DivS { .. }
                        | I32DivU { .. }
                        | I64DivS { .. }
                        | I64DivU { .. }
                        | I64RemU { .. }
                        | I64RemS { .. }
                        | I32RemU { .. }
                        | I32RemS { .. }
                        | I64Mul { .. }
                        | I64Sub { .. }
                        | I32Eq { .. }
                        | I64Eq { .. }
                        | I32Ne { .. }
                        | I64Ne { .. }
                        | I32LtS { .. }
                        | I64LtS { .. }
                        | I32LtU { .. }
                        | I64LtU { .. }
                        | I32LeS { .. }
                        | I64LeS { .. }
                        | I32LeU { .. }
                        | I64LeU { .. }
                        | I32GtS { .. }
                        | I64GtS { .. }
                        | I32GtU { .. }
                        | I64GtU { .. }
                        | I32GeS { .. }
                        | I64GeS { .. }
                        | I32GeU { .. }
                        | I64GeU { .. }
                        | I32Eqz { .. }
                        | I64Eqz { .. }
                        | I32And { .. }
                        | I64And { .. }
                        | I32Or { .. }
                        | I64Or { .. }
                        | I32Xor { .. }
                        | I64Xor { .. }
                        | I32Shl { .. }
                        | I64Shl { .. }
                        | I32ShrS { .. }
                        | I64ShrS { .. }
                        | I32ShrU { .. }
                        | I64ShrU { .. }
                        | I32Rotl { .. }
                        | I64Rotl { .. }
                        | I32Rotr { .. }
                        | I64Rotr { .. }
                        | I32Clz { .. }
                        | I64Clz { .. }
                        | I32Ctz { .. }
                        | I64Ctz { .. }
                        | I32Popcnt { .. }
                        | I64Popcnt { .. }
                        | LocalGet { .. }
                        | LocalSet { .. }
                        | LocalTee { .. }
                        | GlobalGet { .. }
                        | GlobalSet { .. }
                        | Call { .. }
                        | Nop { .. }
                        | End { .. }
                        | If { .. }
                        | Else { .. }
                        | Block { .. }
                        | Loop { .. }
                        | Br { .. }
                        | BrIf { .. }
                        | Unreachable { .. }
                        | Return { .. } => {}
                        _ => {
                            supported = false;
                            break 'main;
                        }
                    }
                }
            }
            _ => {}
        }
    }

    supported
}
