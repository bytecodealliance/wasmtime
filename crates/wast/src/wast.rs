use crate::spectest::instantiate_spectest;
use anyhow::{anyhow, bail, Context as _, Result};
use std::collections::HashMap;
use std::path::Path;
use std::str;
use wasmtime::*;

/// Translate from a `script::Value` to a `RuntimeValue`.
fn runtime_value(v: &wast::Expression<'_>) -> Result<Val> {
    use wast::Instruction::*;

    if v.instrs.len() != 1 {
        bail!("too many instructions in {:?}", v);
    }
    Ok(match &v.instrs[0] {
        I32Const(x) => Val::I32(*x),
        I64Const(x) => Val::I64(*x),
        F32Const(x) => Val::F32(x.bits),
        F64Const(x) => Val::F64(x.bits),
        V128Const(x) => Val::V128(u128::from_le_bytes(x.to_le_bytes())),
        other => bail!("couldn't convert {:?} to a runtime value", other),
    })
}

/// The wast test script language allows modules to be defined and actions
/// to be performed on them.
pub struct WastContext {
    /// Wast files have a concept of a "current" module, which is the most
    /// recently defined.
    current: Option<Instance>,

    instances: HashMap<String, Instance>,
    store: Store,
    spectest: Option<HashMap<&'static str, Extern>>,
}

enum Outcome<T = Vec<Val>> {
    Ok(T),
    Trap(Trap),
}

impl<T> Outcome<T> {
    fn into_result(self) -> Result<T, Trap> {
        match self {
            Outcome::Ok(t) => Ok(t),
            Outcome::Trap(t) => Err(t),
        }
    }
}

impl WastContext {
    /// Construct a new instance of `WastContext`.
    pub fn new(store: Store) -> Self {
        Self {
            current: None,
            store,
            spectest: None,
            instances: HashMap::new(),
        }
    }

    fn get_instance(&self, instance_name: Option<&str>) -> Result<Instance> {
        match instance_name {
            Some(name) => self
                .instances
                .get(name)
                .cloned()
                .ok_or_else(|| anyhow!("failed to find instance named `{}`", name)),
            None => self
                .current
                .clone()
                .ok_or_else(|| anyhow!("no previous instance found")),
        }
    }

    fn instantiate(&self, module: &[u8]) -> Result<Outcome<Instance>> {
        let module = Module::new(&self.store, module)?;
        let mut imports = Vec::new();
        for import in module.imports() {
            if import.module() == "spectest" {
                let spectest = self
                    .spectest
                    .as_ref()
                    .ok_or_else(|| anyhow!("spectest module isn't instantiated"))?;
                let export = spectest
                    .get(import.name())
                    .ok_or_else(|| anyhow!("unknown import `spectest::{}`", import.name()))?;
                imports.push(export.clone());
                continue;
            }

            let instance = self
                .instances
                .get(import.module())
                .ok_or_else(|| anyhow!("no module named `{}`", import.module()))?;
            let export = instance
                .get_export(import.name())
                .ok_or_else(|| anyhow!("unknown import `{}::{}`", import.name(), import.module()))?
                .clone();
            imports.push(export);
        }
        let instance = match Instance::new(&module, &imports) {
            Ok(i) => i,
            Err(e) => return e.downcast::<Trap>().map(Outcome::Trap),
        };
        Ok(Outcome::Ok(instance))
    }

    /// Register "spectest" which is used by the spec testsuite.
    pub fn register_spectest(&mut self) -> Result<()> {
        self.spectest = Some(instantiate_spectest(&self.store));
        Ok(())
    }

    /// Perform the action portion of a command.
    fn perform_execute(&mut self, exec: wast::WastExecute<'_>) -> Result<Outcome> {
        match exec {
            wast::WastExecute::Invoke(invoke) => self.perform_invoke(invoke),
            wast::WastExecute::Module(mut module) => {
                let binary = module.encode()?;
                let result = self.instantiate(&binary)?;
                Ok(match result {
                    Outcome::Ok(_) => Outcome::Ok(Vec::new()),
                    Outcome::Trap(e) => Outcome::Trap(e),
                })
            }
            wast::WastExecute::Get { module, global } => self.get(module.map(|s| s.name()), global),
        }
    }

    fn perform_invoke(&mut self, exec: wast::WastInvoke<'_>) -> Result<Outcome> {
        let values = exec
            .args
            .iter()
            .map(runtime_value)
            .collect::<Result<Vec<_>>>()?;
        self.invoke(exec.module.map(|i| i.name()), exec.name, &values)
    }

    /// Define a module and register it.
    fn module(&mut self, instance_name: Option<&str>, module: &[u8]) -> Result<()> {
        let instance = match self.instantiate(module)? {
            Outcome::Ok(i) => i,
            Outcome::Trap(e) => bail!("instantiation failed with: {}", e.message()),
        };
        if let Some(name) = instance_name {
            self.instances.insert(name.to_string(), instance.clone());
        }
        self.current = Some(instance);
        Ok(())
    }

    /// Register an instance to make it available for performing actions.
    fn register(&mut self, name: Option<&str>, as_name: &str) -> Result<()> {
        let instance = self.get_instance(name)?.clone();
        self.instances.insert(as_name.to_string(), instance);
        Ok(())
    }

    /// Invoke an exported function from an instance.
    fn invoke(
        &mut self,
        instance_name: Option<&str>,
        field: &str,
        args: &[Val],
    ) -> Result<Outcome> {
        let instance = self.get_instance(instance_name.as_ref().map(|x| &**x))?;
        let func = instance
            .get_export(field)
            .and_then(|e| e.func())
            .ok_or_else(|| anyhow!("no function named `{}`", field))?;
        Ok(match func.call(args) {
            Ok(result) => Outcome::Ok(result.into()),
            Err(e) => Outcome::Trap(e),
        })
    }

    /// Get the value of an exported global from an instance.
    fn get(&mut self, instance_name: Option<&str>, field: &str) -> Result<Outcome> {
        let instance = self.get_instance(instance_name.as_ref().map(|x| &**x))?;
        let global = instance
            .get_export(field)
            .and_then(|e| e.global())
            .ok_or_else(|| anyhow!("no global named `{}`", field))?;
        Ok(Outcome::Ok(vec![global.get()]))
    }

    fn assert_return(&self, result: Outcome, results: &[wast::AssertExpression]) -> Result<()> {
        let values = result.into_result()?;
        for (v, e) in values.iter().zip(results) {
            if val_matches(v, e)? {
                continue;
            }
            bail!("expected {:?}, got {:?}", e, v)
        }
        Ok(())
    }

    fn assert_trap(&self, result: Outcome, message: &str) -> Result<()> {
        let trap = match result {
            Outcome::Ok(values) => bail!("expected trap, got {:?}", values),
            Outcome::Trap(t) => t,
        };
        if trap.message().contains(message) {
            return Ok(());
        }
        if cfg!(feature = "lightbeam") {
            println!("TODO: Check the assert_trap message: {}", message);
            return Ok(());
        }
        bail!("expected {}, got {}", message, trap.message())
    }

    /// Run a wast script from a byte buffer.
    pub fn run_buffer(&mut self, filename: &str, wast: &[u8]) -> Result<()> {
        let wast = str::from_utf8(wast)?;

        let adjust_wast = |mut err: wast::Error| {
            err.set_path(filename.as_ref());
            err.set_text(wast);
            err
        };

        let buf = wast::parser::ParseBuffer::new(wast).map_err(adjust_wast)?;
        let ast = wast::parser::parse::<wast::Wast>(&buf).map_err(adjust_wast)?;

        for directive in ast.directives {
            let sp = directive.span();
            self.run_directive(directive).with_context(|| {
                let (line, col) = sp.linecol_in(wast);
                format!("failed directive on {}:{}:{}", filename, line + 1, col)
            })?;
        }
        Ok(())
    }

    fn run_directive(&mut self, directive: wast::WastDirective) -> Result<()> {
        use wast::WastDirective::*;

        match directive {
            Module(mut module) => {
                let binary = module.encode()?;
                self.module(module.name.map(|s| s.name()), &binary)?;
            }
            Register {
                span: _,
                name,
                module,
            } => {
                self.register(module.map(|s| s.name()), name)?;
            }
            Invoke(i) => {
                self.perform_invoke(i)?;
            }
            AssertReturn {
                span: _,
                exec,
                results,
            } => {
                let result = self.perform_execute(exec)?;
                self.assert_return(result, &results)?;
            }
            AssertTrap {
                span: _,
                exec,
                message,
            } => {
                let result = self.perform_execute(exec)?;
                self.assert_trap(result, message)?;
            }
            AssertExhaustion {
                span: _,
                call,
                message,
            } => {
                let result = self.perform_invoke(call)?;
                self.assert_trap(result, message)?;
            }
            AssertInvalid {
                span: _,
                mut module,
                message,
            } => {
                let bytes = module.encode()?;
                let err = match self.module(None, &bytes) {
                    Ok(()) => bail!("expected module to fail to build"),
                    Err(e) => e,
                };
                let error_message = format!("{:?}", err);
                if !error_message.contains(&message) {
                    // TODO: change to bail!
                    println!(
                        "assert_invalid: expected {}, got {}",
                        message, error_message
                    )
                }
            }
            AssertMalformed {
                span: _,
                module,
                message,
            } => {
                let mut module = match module {
                    wast::QuoteModule::Module(m) => m,
                    // this is a `*.wat` parser test which we're not
                    // interested in
                    wast::QuoteModule::Quote(_) => return Ok(()),
                };
                let bytes = module.encode()?;
                let err = match self.module(None, &bytes) {
                    Ok(()) => bail!("expected module to fail to instantiate"),
                    Err(e) => e,
                };
                let error_message = format!("{:?}", err);
                if !error_message.contains(&message) {
                    // TODO: change to bail!
                    println!(
                        "assert_malformed: expected {}, got {}",
                        message, error_message
                    )
                }
            }
            AssertUnlinkable {
                span: _,
                mut module,
                message,
            } => {
                let bytes = module.encode()?;
                let err = match self.module(None, &bytes) {
                    Ok(()) => bail!("expected module to fail to link"),
                    Err(e) => e,
                };
                let error_message = format!("{:?}", err);
                if !error_message.contains(&message) {
                    bail!(
                        "assert_unlinkable: expected {}, got {}",
                        message,
                        error_message
                    )
                }
            }
        }

        Ok(())
    }

    /// Run a wast script from a file.
    pub fn run_file(&mut self, path: &Path) -> Result<()> {
        let bytes =
            std::fs::read(path).with_context(|| format!("failed to read `{}`", path.display()))?;
        self.run_buffer(path.to_str().unwrap(), &bytes)
    }
}

fn extract_lane_as_i8(bytes: u128, lane: usize) -> i8 {
    (bytes >> (lane * 8)) as i8
}

fn extract_lane_as_i16(bytes: u128, lane: usize) -> i16 {
    (bytes >> (lane * 16)) as i16
}

fn extract_lane_as_i32(bytes: u128, lane: usize) -> i32 {
    (bytes >> (lane * 32)) as i32
}

fn extract_lane_as_i64(bytes: u128, lane: usize) -> i64 {
    (bytes >> (lane * 64)) as i64
}

fn is_canonical_f32_nan(bits: u32) -> bool {
    (bits & 0x7fff_ffff) == 0x7fc0_0000
}

fn is_canonical_f64_nan(bits: u64) -> bool {
    (bits & 0x7fff_ffff_ffff_ffff) == 0x7ff8_0000_0000_0000
}

fn is_arithmetic_f32_nan(bits: u32) -> bool {
    const AF32_NAN: u32 = 0x0040_0000;
    (bits & AF32_NAN) == AF32_NAN
}

fn is_arithmetic_f64_nan(bits: u64) -> bool {
    const AF64_NAN: u64 = 0x0008_0000_0000_0000;
    (bits & AF64_NAN) == AF64_NAN
}

fn val_matches(actual: &Val, expected: &wast::AssertExpression) -> Result<bool> {
    Ok(match (actual, expected) {
        (Val::I32(a), wast::AssertExpression::I32(b)) => a == b,
        (Val::I64(a), wast::AssertExpression::I64(b)) => a == b,
        // Note that these float comparisons are comparing bits, not float
        // values, so we're testing for bit-for-bit equivalence
        (Val::F32(a), wast::AssertExpression::F32(b)) => f32_matches(*a, b),
        (Val::F64(a), wast::AssertExpression::F64(b)) => f64_matches(*a, b),
        (Val::V128(a), wast::AssertExpression::V128(b)) => v128_matches(*a, b),
        _ => bail!(
            "don't know how to compare {:?} and {:?} yet",
            actual,
            expected
        ),
    })
}

fn f32_matches(actual: u32, expected: &wast::NanPattern<wast::Float32>) -> bool {
    match expected {
        wast::NanPattern::CanonicalNan => is_canonical_f32_nan(actual),
        wast::NanPattern::ArithmeticNan => is_arithmetic_f32_nan(actual),
        wast::NanPattern::Value(expected_value) => actual == expected_value.bits,
    }
}

fn f64_matches(actual: u64, expected: &wast::NanPattern<wast::Float64>) -> bool {
    match expected {
        wast::NanPattern::CanonicalNan => is_canonical_f64_nan(actual),
        wast::NanPattern::ArithmeticNan => is_arithmetic_f64_nan(actual),
        wast::NanPattern::Value(expected_value) => actual == expected_value.bits,
    }
}

fn v128_matches(actual: u128, expected: &wast::V128Pattern) -> bool {
    match expected {
        wast::V128Pattern::I8x16(b) => b
            .iter()
            .enumerate()
            .all(|(i, b)| *b == extract_lane_as_i8(actual, i)),
        wast::V128Pattern::I16x8(b) => b
            .iter()
            .enumerate()
            .all(|(i, b)| *b == extract_lane_as_i16(actual, i)),
        wast::V128Pattern::I32x4(b) => b
            .iter()
            .enumerate()
            .all(|(i, b)| *b == extract_lane_as_i32(actual, i)),
        wast::V128Pattern::I64x2(b) => b
            .iter()
            .enumerate()
            .all(|(i, b)| *b == extract_lane_as_i64(actual, i)),
        wast::V128Pattern::F32x4(b) => b.iter().enumerate().all(|(i, b)| {
            let a = extract_lane_as_i32(actual, i) as u32;
            f32_matches(a, b)
        }),
        wast::V128Pattern::F64x2(b) => b.iter().enumerate().all(|(i, b)| {
            let a = extract_lane_as_i64(actual, i) as u64;
            f64_matches(a, b)
        }),
    }
}
