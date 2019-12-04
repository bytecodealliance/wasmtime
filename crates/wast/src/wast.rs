use crate::spectest::instantiate_spectest;
use anyhow::{bail, Context as _, Result};
use std::path::Path;
use std::str;
use wasmtime_jit::{
    ActionError, ActionOutcome, Compiler, Context, Features, InstanceHandle, InstantiationError,
    RuntimeValue, SetupError,
};

/// Translate from a `script::Value` to a `RuntimeValue`.
fn runtime_value(v: &wast::Expression<'_>) -> RuntimeValue {
    use wast::Instruction::*;

    if v.instrs.len() != 1 {
        panic!("too many instructions in {:?}", v);
    }
    match &v.instrs[0] {
        I32Const(x) => RuntimeValue::I32(*x),
        I64Const(x) => RuntimeValue::I64(*x),
        F32Const(x) => RuntimeValue::F32(x.bits),
        F64Const(x) => RuntimeValue::F64(x.bits),
        V128Const(x) => RuntimeValue::V128(x.to_le_bytes()),
        other => panic!("couldn't convert {:?} to a runtime value", other),
    }
}

/// The wast test script language allows modules to be defined and actions
/// to be performed on them.
pub struct WastContext {
    /// Wast files have a concept of a "current" module, which is the most
    /// recently defined.
    current: Option<InstanceHandle>,

    context: Context,
}

impl WastContext {
    /// Construct a new instance of `WastContext`.
    pub fn new(compiler: Box<Compiler>) -> Self {
        Self {
            current: None,
            context: Context::new(compiler),
        }
    }

    /// Construct a new instance with the given features using the current `Context`
    pub fn with_features(self, features: Features) -> Self {
        Self {
            context: self.context.with_features(features),
            ..self
        }
    }

    fn get_instance(&mut self, instance_name: Option<&str>) -> Result<&mut InstanceHandle> {
        let instance = if let Some(instance_name) = instance_name {
            self.context
                .get_instance(instance_name)
                .context("failed to fetch instance")?
        } else {
            self.current
                .as_mut()
                .ok_or_else(|| anyhow::format_err!("no current instance"))?
        };

        Ok(instance)
    }

    /// Register "spectest" which is used by the spec testsuite.
    pub fn register_spectest(&mut self) -> Result<()> {
        let instance = instantiate_spectest()?;
        self.context.name_instance("spectest".to_owned(), instance);
        Ok(())
    }

    /// Perform the action portion of a command.
    fn perform_execute(&mut self, exec: wast::WastExecute<'_>) -> Result<ActionOutcome> {
        match exec {
            wast::WastExecute::Invoke(invoke) => self.perform_invoke(invoke),
            wast::WastExecute::Module(mut module) => {
                let binary = module.encode()?;
                let result = self.context.instantiate_module(None, &binary);
                match result {
                    Ok(_) => Ok(ActionOutcome::Returned { values: Vec::new() }),
                    Err(ActionError::Setup(SetupError::Instantiate(
                        InstantiationError::StartTrap(message),
                    ))) => Ok(ActionOutcome::Trapped { message }),
                    Err(e) => Err(e.into()),
                }
            }
            wast::WastExecute::Get { module, global } => self.get(module.map(|s| s.name()), global),
        }
    }

    fn perform_invoke(&mut self, exec: wast::WastInvoke<'_>) -> Result<ActionOutcome> {
        self.invoke(exec.module.map(|i| i.name()), exec.name, &exec.args)
    }

    /// Define a module and register it.
    fn module(&mut self, instance_name: Option<&str>, module: &[u8]) -> Result<()> {
        let index = self
            .context
            .instantiate_module(instance_name.map(|s| s.to_string()), module)?;
        self.current = Some(index);
        Ok(())
    }

    /// Register an instance to make it available for performing actions.
    fn register(&mut self, name: Option<&str>, as_name: &str) -> Result<()> {
        let instance = self.get_instance(name)?.clone();
        self.context.name_instance(as_name.to_string(), instance);
        Ok(())
    }

    /// Invoke an exported function from an instance.
    fn invoke(
        &mut self,
        instance_name: Option<&str>,
        field: &str,
        args: &[wast::Expression],
    ) -> Result<ActionOutcome> {
        let value_args = args.iter().map(runtime_value).collect::<Vec<_>>();
        let mut instance = self.get_instance(instance_name)?.clone();
        let result = self
            .context
            .invoke(&mut instance, field, &value_args)
            .with_context(|| format!("failed to invoke `{}`", field))?;
        Ok(result)
    }

    /// Get the value of an exported global from an instance.
    fn get(&mut self, instance_name: Option<&str>, field: &str) -> Result<ActionOutcome> {
        let instance = self
            .get_instance(instance_name.as_ref().map(|x| &**x))?
            .clone();
        let result = self
            .context
            .get(&instance, field)
            .with_context(|| format!("failed to get field `{}`", field))?;
        Ok(result)
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
                    ActionOutcome::Returned { values } => {
                        for (v, e) in values.iter().zip(results.iter().map(runtime_value)) {
                            if *v == e {
                                continue;
                            }
                            bail!("{}\nexpected {}, got {}", context(span), e, v)
                        }
                    }
                    ActionOutcome::Trapped { message } => {
                        bail!("{}\nunexpected trap: {}", context(span), message)
                    }
                },
                AssertTrap {
                    span,
                    exec,
                    message,
                } => match self.perform_execute(exec).with_context(|| context(span))? {
                    ActionOutcome::Returned { values } => {
                        bail!("{}\nexpected trap, got {:?}", context(span), values)
                    }
                    ActionOutcome::Trapped {
                        message: trap_message,
                    } => {
                        if trap_message.contains(message) {
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
                            trap_message
                        )
                    }
                },
                AssertExhaustion {
                    span,
                    call,
                    message,
                } => match self.perform_invoke(call).with_context(|| context(span))? {
                    ActionOutcome::Returned { values } => {
                        bail!("{}\nexpected trap, got {:?}", context(span), values)
                    }
                    ActionOutcome::Trapped {
                        message: trap_message,
                    } => {
                        if trap_message.contains(message) {
                            continue;
                        }
                        bail!(
                            "{}\nexpected exhaustion with {}, got {}",
                            context(span),
                            message,
                            trap_message
                        )
                    }
                },
                AssertReturnCanonicalNan { span, invoke } => {
                    match self.perform_invoke(invoke).with_context(|| context(span))? {
                        ActionOutcome::Returned { values } => {
                            for v in values.iter() {
                                match v {
                                    RuntimeValue::I32(_) | RuntimeValue::I64(_) => {
                                        bail!("{}\nunexpected integer in NaN test", context(span))
                                    }
                                    RuntimeValue::V128(_) => {
                                        bail!("{}\nunexpected vector in NaN test", context(span))
                                    }
                                    RuntimeValue::F32(x) => {
                                        if (x & 0x7fffffff) != 0x7fc00000 {
                                            bail!("{}\nexpected canonical NaN", context(span))
                                        }
                                    }
                                    RuntimeValue::F64(x) => {
                                        if (x & 0x7fffffffffffffff) != 0x7ff8000000000000 {
                                            bail!("{}\nexpected canonical NaN", context(span))
                                        }
                                    }
                                };
                            }
                        }
                        ActionOutcome::Trapped { message } => {
                            bail!("{}\nunexpected trap: {}", context(span), message)
                        }
                    }
                }
                AssertReturnArithmeticNan { span, invoke } => {
                    match self.perform_invoke(invoke).with_context(|| context(span))? {
                        ActionOutcome::Returned { values } => {
                            for v in values.iter() {
                                match v {
                                    RuntimeValue::I32(_) | RuntimeValue::I64(_) => {
                                        bail!("{}\nunexpected integer in NaN test", context(span))
                                    }
                                    RuntimeValue::V128(_) => {
                                        bail!("{}\nunexpected vector in NaN test", context(span))
                                    }
                                    RuntimeValue::F32(x) => {
                                        if (x & 0x00400000) != 0x00400000 {
                                            bail!("{}\nexpected arithmetic NaN", context(span))
                                        }
                                    }
                                    RuntimeValue::F64(x) => {
                                        if (x & 0x0008000000000000) != 0x0008000000000000 {
                                            bail!("{}\nexpected arithmetic NaN", context(span))
                                        }
                                    }
                                };
                            }
                        }
                        ActionOutcome::Trapped { message } => {
                            bail!("{}\nunexpected trap: {}", context(span), message)
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
                AssertReturnFunc { .. } => panic!("need to implement assert_return_func"),
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
