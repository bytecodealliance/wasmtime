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
    current: Option<HostRef<Instance>>,

    instances: HashMap<String, HostRef<Instance>>,
    store: Store,
    spectest: Option<HashMap<&'static str, Extern>>,
}

enum Outcome<T = Vec<Val>> {
    Ok(T),
    Trap(Trap),
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

    fn get_instance(&self, instance_name: Option<&str>) -> Result<HostRef<Instance>> {
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

    fn instantiate(&self, module: &[u8]) -> Result<Outcome<HostRef<Instance>>> {
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
                .borrow()
                .find_export_by_name(import.name())
                .ok_or_else(|| anyhow!("unknown import `{}::{}`", import.name(), import.module()))?
                .clone();
            imports.push(export);
        }
        let instance = match Instance::new(&self.store, &module, &imports) {
            Ok(i) => i,
            Err(e) => {
                let err = e.chain().filter_map(|e| e.downcast_ref::<Trap>()).next();
                if let Some(trap) = err {
                    return Ok(Outcome::Trap(trap.clone()));
                }
                return Err(e);
            }
        };
        Ok(Outcome::Ok(HostRef::new(instance)))
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
        self.invoke(exec.module.map(|i| i.name()), exec.name, &exec.args)
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
        args: &[wast::Expression],
    ) -> Result<Outcome> {
        let values = args.iter().map(runtime_value).collect::<Result<Vec<_>>>()?;
        let instance = self.get_instance(instance_name.as_ref().map(|x| &**x))?;
        let instance = instance.borrow();
        let export = instance
            .find_export_by_name(field)
            .ok_or_else(|| anyhow!("no global named `{}`", field))?;
        let func = match export {
            Extern::Func(f) => f.borrow(),
            _ => bail!("export of `{}` wasn't a global", field),
        };
        Ok(match func.call(&values) {
            Ok(result) => Outcome::Ok(result.into()),
            Err(e) => Outcome::Trap(e),
        })
    }

    /// Get the value of an exported global from an instance.
    fn get(&mut self, instance_name: Option<&str>, field: &str) -> Result<Outcome> {
        let instance = self.get_instance(instance_name.as_ref().map(|x| &**x))?;
        let instance = instance.borrow();
        let export = instance
            .find_export_by_name(field)
            .ok_or_else(|| anyhow!("no global named `{}`", field))?;
        let global = match export {
            Extern::Global(g) => g.borrow(),
            _ => bail!("export of `{}` wasn't a global", field),
        };
        Ok(Outcome::Ok(vec![global.get()]))
    }

    /// Run a wast script from a byte buffer.
    pub fn run_buffer(&mut self, filename: &str, wast: &[u8]) -> Result<()> {
        use wast::WastDirective::*;

        let wast = str::from_utf8(wast)?;

        let adjust_wast = |mut err: wast::Error| {
            err.set_path(filename.as_ref());
            err.set_text(wast);
            err
        };
        let context = |sp: wast::Span| {
            let (line, col) = sp.linecol_in(wast);
            format!("for directive on {}:{}:{}", filename, line + 1, col)
        };

        let buf = wast::parser::ParseBuffer::new(wast).map_err(adjust_wast)?;
        let wast = wast::parser::parse::<wast::Wast>(&buf).map_err(adjust_wast)?;

        for directive in wast.directives {
            match directive {
                Module(mut module) => {
                    let binary = module.encode().map_err(adjust_wast)?;
                    self.module(module.name.map(|s| s.name()), &binary)
                        .with_context(|| context(module.span))?;
                }
                Register { span, name, module } => {
                    self.register(module.map(|s| s.name()), name)
                        .with_context(|| context(span))?;
                }
                Invoke(i) => {
                    let span = i.span;
                    self.perform_invoke(i).with_context(|| context(span))?;
                }
                AssertReturn {
                    span,
                    exec,
                    results,
                } => match self.perform_execute(exec).with_context(|| context(span))? {
                    Outcome::Ok(values) => {
                        for (v, e) in values.iter().zip(results.iter().map(runtime_value)) {
                            let e = e?;
                            if values_equal(v, &e)? {
                                continue;
                            }
                            bail!("{}\nexpected {:?}, got {:?}", context(span), e, v)
                        }
                    }
                    Outcome::Trap(t) => {
                        bail!("{}\nunexpected trap: {}", context(span), t.message())
                    }
                },
                AssertTrap {
                    span,
                    exec,
                    message,
                } => match self.perform_execute(exec).with_context(|| context(span))? {
                    Outcome::Ok(values) => {
                        bail!("{}\nexpected trap, got {:?}", context(span), values)
                    }
                    Outcome::Trap(t) => {
                        if t.message().contains(message) {
                            continue;
                        }
                        if cfg!(feature = "lightbeam") {
                            println!(
                                "{}\nTODO: Check the assert_trap message: {}",
                                context(span),
                                message
                            );
                            continue;
                        }
                        bail!(
                            "{}\nexpected {}, got {}",
                            context(span),
                            message,
                            t.message(),
                        )
                    }
                },
                AssertExhaustion {
                    span,
                    call,
                    message,
                } => match self.perform_invoke(call).with_context(|| context(span))? {
                    Outcome::Ok(values) => {
                        bail!("{}\nexpected trap, got {:?}", context(span), values)
                    }
                    Outcome::Trap(t) => {
                        if t.message().contains(message) {
                            continue;
                        }
                        bail!(
                            "{}\nexpected exhaustion with {}, got {}",
                            context(span),
                            message,
                            t.message(),
                        )
                    }
                },
                AssertReturnCanonicalNan { span, invoke } => {
                    match self.perform_invoke(invoke).with_context(|| context(span))? {
                        Outcome::Ok(values) => {
                            for v in values.iter() {
                                match v {
                                    Val::F32(x) => {
                                        if !is_canonical_f32_nan(*x) {
                                            bail!("{}\nexpected canonical NaN", context(span))
                                        }
                                    }
                                    Val::F64(x) => {
                                        if !is_canonical_f64_nan(*x) {
                                            bail!("{}\nexpected canonical NaN", context(span))
                                        }
                                    }
                                    other => bail!("expected float, got {:?}", other),
                                };
                            }
                        }
                        Outcome::Trap(t) => {
                            bail!("{}\nunexpected trap: {}", context(span), t.message())
                        }
                    }
                }
                AssertReturnCanonicalNanF32x4 { span, invoke } => {
                    match self.perform_invoke(invoke).with_context(|| context(span))? {
                        Outcome::Ok(values) => {
                            for v in values.iter() {
                                let val = match v {
                                    Val::V128(x) => x,
                                    other => bail!("expected v128, got {:?}", other),
                                };
                                for l in 0..4 {
                                    if !is_canonical_f32_nan(extract_lane_as_u32(val, l)?) {
                                        bail!(
                                            "{}\nexpected f32x4 canonical NaN in lane {}",
                                            context(span),
                                            l
                                        )
                                    }
                                }
                            }
                        }
                        Outcome::Trap(t) => {
                            bail!("{}\nunexpected trap: {}", context(span), t.message())
                        }
                    }
                }
                AssertReturnCanonicalNanF64x2 { span, invoke } => {
                    match self.perform_invoke(invoke).with_context(|| context(span))? {
                        Outcome::Ok(values) => {
                            for v in values.iter() {
                                let val = match v {
                                    Val::V128(x) => x,
                                    other => bail!("expected v128, got {:?}", other),
                                };
                                for l in 0..2 {
                                    if !is_canonical_f64_nan(extract_lane_as_u64(val, l)?) {
                                        bail!(
                                            "{}\nexpected f64x2 canonical NaN in lane {}",
                                            context(span),
                                            l
                                        )
                                    }
                                }
                            }
                        }
                        Outcome::Trap(t) => {
                            bail!("{}\nunexpected trap: {}", context(span), t.message())
                        }
                    }
                }
                AssertReturnArithmeticNan { span, invoke } => {
                    match self.perform_invoke(invoke).with_context(|| context(span))? {
                        Outcome::Ok(values) => {
                            for v in values.iter() {
                                match v {
                                    Val::F32(x) => {
                                        if !is_arithmetic_f32_nan(*x) {
                                            bail!("{}\nexpected arithmetic NaN", context(span))
                                        }
                                    }
                                    Val::F64(x) => {
                                        if !is_arithmetic_f64_nan(*x) {
                                            bail!("{}\nexpected arithmetic NaN", context(span))
                                        }
                                    }
                                    other => bail!("expected float, got {:?}", other),
                                };
                            }
                        }
                        Outcome::Trap(t) => {
                            bail!("{}\nunexpected trap: {}", context(span), t.message())
                        }
                    }
                }
                AssertReturnArithmeticNanF32x4 { span, invoke } => {
                    match self.perform_invoke(invoke).with_context(|| context(span))? {
                        Outcome::Ok(values) => {
                            for v in values.iter() {
                                let val = match v {
                                    Val::V128(x) => x,
                                    other => bail!("expected v128, got {:?}", other),
                                };
                                for l in 0..4 {
                                    if !is_arithmetic_f32_nan(extract_lane_as_u32(val, l)?) {
                                        bail!(
                                            "{}\nexpected f32x4 arithmetic NaN in lane {}",
                                            context(span),
                                            l
                                        )
                                    }
                                }
                            }
                        }
                        Outcome::Trap(t) => {
                            bail!("{}\nunexpected trap: {}", context(span), t.message())
                        }
                    }
                }
                AssertReturnArithmeticNanF64x2 { span, invoke } => {
                    match self.perform_invoke(invoke).with_context(|| context(span))? {
                        Outcome::Ok(values) => {
                            for v in values.iter() {
                                let val = match v {
                                    Val::V128(x) => x,
                                    other => bail!("expected v128, got {:?}", other),
                                };
                                for l in 0..2 {
                                    if !is_arithmetic_f64_nan(extract_lane_as_u64(val, l)?) {
                                        bail!(
                                            "{}\nexpected f64x2 arithmetic NaN in lane {}",
                                            context(span),
                                            l
                                        )
                                    }
                                }
                            }
                        }
                        Outcome::Trap(t) => {
                            bail!("{}\nunexpected trap: {}", context(span), t.message())
                        }
                    }
                }
                AssertInvalid {
                    span,
                    mut module,
                    message,
                } => {
                    let bytes = module.encode().map_err(adjust_wast)?;
                    let err = match self.module(None, &bytes) {
                        Ok(()) => bail!("{}\nexpected module to fail to build", context(span)),
                        Err(e) => e,
                    };
                    let error_message = format!("{:?}", err);
                    if !error_message.contains(&message) {
                        // TODO: change to bail!
                        println!(
                            "{}\nassert_invalid: expected {}, got {}",
                            context(span),
                            message,
                            error_message
                        )
                    }
                }
                AssertMalformed {
                    span,
                    module,
                    message,
                } => {
                    let mut module = match module {
                        wast::QuoteModule::Module(m) => m,
                        // this is a `*.wat` parser test which we're not
                        // interested in
                        wast::QuoteModule::Quote(_) => return Ok(()),
                    };
                    let bytes = module.encode().map_err(adjust_wast)?;
                    let err = match self.module(None, &bytes) {
                        Ok(()) => {
                            bail!("{}\nexpected module to fail to instantiate", context(span))
                        }
                        Err(e) => e,
                    };
                    let error_message = format!("{:?}", err);
                    if !error_message.contains(&message) {
                        // TODO: change to bail!
                        println!(
                            "{}\nassert_malformed: expected {}, got {}",
                            context(span),
                            message,
                            error_message
                        )
                    }
                }
                AssertUnlinkable {
                    span,
                    mut module,
                    message,
                } => {
                    let bytes = module.encode().map_err(adjust_wast)?;
                    let err = match self.module(None, &bytes) {
                        Ok(()) => bail!("{}\nexpected module to fail to link", context(span)),
                        Err(e) => e,
                    };
                    let error_message = format!("{:?}", err);
                    if !error_message.contains(&message) {
                        bail!(
                            "{}\nassert_unlinkable: expected {}, got {}",
                            context(span),
                            message,
                            error_message
                        )
                    }
                }
                AssertReturnFunc { .. } => bail!("need to implement assert_return_func"),
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

fn extract_lane_as_u32(bytes: &u128, lane: usize) -> Result<u32> {
    Ok((*bytes >> (lane * 32)) as u32)
}

fn extract_lane_as_u64(bytes: &u128, lane: usize) -> Result<u64> {
    Ok((*bytes >> (lane * 64)) as u64)
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

fn values_equal(v1: &Val, v2: &Val) -> Result<bool> {
    Ok(match (v1, v2) {
        (Val::I32(a), Val::I32(b)) => a == b,
        (Val::I64(a), Val::I64(b)) => a == b,
        // Note that these float comparisons are comparing bits, not float
        // values, so we're testing for bit-for-bit equivalence
        (Val::F32(a), Val::F32(b)) => a == b,
        (Val::F64(a), Val::F64(b)) => a == b,
        (Val::V128(a), Val::V128(b)) => a == b,
        _ => bail!("don't know how to compare {:?} and {:?} yet", v1, v2),
    })
}
