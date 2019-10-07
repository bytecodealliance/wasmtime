use crate::spectest::instantiate_spectest;
use std::io::Read;
use std::path::Path;
use std::{fmt, fs, io, str};
use wabt::script::{Action, Command, CommandKind, ModuleBinary, ScriptParser, Value};
use wabt::Features as WabtFeatures;
use wasmtime_jit::{
    ActionError, ActionOutcome, Compiler, Context, Features, InstanceHandle, InstantiationError,
    RuntimeValue, UnknownInstance,
};

/// Translate from a `script::Value` to a `RuntimeValue`.
fn runtime_value(v: Value) -> RuntimeValue {
    match v {
        Value::I32(x) => RuntimeValue::I32(x),
        Value::I64(x) => RuntimeValue::I64(x),
        Value::F32(x) => RuntimeValue::F32(x.to_bits()),
        Value::F64(x) => RuntimeValue::F64(x.to_bits()),
        Value::V128(x) => RuntimeValue::V128(x.to_le_bytes()),
    }
}

/// Error message used by `WastContext`.
#[derive(Fail, Debug)]
pub enum WastError {
    /// An assert command was not satisfied.
    Assert(String),
    /// An unknown instance name was used.
    Instance(UnknownInstance),
    /// No default instance has been registered yet.
    NoDefaultInstance,
    /// An error occured while performing an action.
    Action(ActionError),
    /// An action trapped.
    Trap(String),
    /// There was a type error in inputs or outputs of an action.
    Type(String),
    /// The was a syntax error while parsing the wast script.
    Syntax(wabt::script::Error),
    /// The was a character encoding error while parsing the wast script.
    Utf8(str::Utf8Error),
    /// The was an I/O error while reading the wast file.
    IO(io::Error),
}

impl fmt::Display for WastError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            WastError::Assert(ref message) => write!(f, "Assert command failed: {}", message),
            WastError::Instance(ref error) => error.fmt(f),
            WastError::NoDefaultInstance => write!(f, "no default instance defined yet"),
            WastError::Action(ref error) => error.fmt(f),
            WastError::Trap(ref message) => write!(f, "trap: {}", message),
            WastError::Type(ref message) => write!(f, "type error: {}", message),
            WastError::Syntax(ref message) => write!(f, "syntax error: {}", message),
            WastError::Utf8(ref message) => write!(f, "UTF-8 decoding error: {}", message),
            WastError::IO(ref error) => write!(f, "I/O error: {}", error),
        }
    }
}

/// Error message with a source file and line number.
#[derive(Fail, Debug)]
#[fail(display = "{}:{}: {}", filename, line, error)]
pub struct WastFileError {
    filename: String,
    line: u64,
    error: WastError,
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

    fn get_instance(
        &mut self,
        instance_name: Option<&str>,
    ) -> Result<&mut InstanceHandle, WastError> {
        let instance = if let Some(instance_name) = instance_name {
            self.context
                .get_instance(instance_name)
                .map_err(WastError::Instance)
        } else {
            self.current
                .as_mut()
                .ok_or_else(|| WastError::NoDefaultInstance)
        }?;

        Ok(instance)
    }

    /// Register "spectest" which is used by the spec testsuite.
    pub fn register_spectest(&mut self) -> Result<(), InstantiationError> {
        let instance = instantiate_spectest()?;
        self.context.name_instance("spectest".to_owned(), instance);
        Ok(())
    }

    /// Perform the action portion of a command.
    fn perform_action(&mut self, action: Action) -> Result<ActionOutcome, WastError> {
        match action {
            Action::Invoke {
                module: instance_name,
                field,
                args,
            } => self.invoke(instance_name, &field, &args),
            Action::Get {
                module: instance_name,
                field,
            } => self.get(instance_name, &field),
        }
    }

    /// Define a module and register it.
    fn module(
        &mut self,
        instance_name: Option<String>,
        module: ModuleBinary,
    ) -> Result<(), ActionError> {
        let index = self
            .context
            .instantiate_module(instance_name, &module.into_vec())?;
        self.current = Some(index);
        Ok(())
    }

    /// Register an instance to make it available for performing actions.
    fn register(&mut self, name: Option<String>, as_name: String) -> Result<(), WastError> {
        let instance = self.get_instance(name.as_ref().map(|x| &**x))?.clone();
        self.context.name_instance(as_name, instance);
        Ok(())
    }

    /// Invoke an exported function from an instance.
    fn invoke(
        &mut self,
        instance_name: Option<String>,
        field: &str,
        args: &[Value],
    ) -> Result<ActionOutcome, WastError> {
        let value_args = args
            .iter()
            .map(|arg| runtime_value(*arg))
            .collect::<Vec<_>>();
        let mut instance = self
            .get_instance(instance_name.as_ref().map(|x| &**x))?
            .clone();
        self.context
            .invoke(&mut instance, field, &value_args)
            .map_err(WastError::Action)
    }

    /// Get the value of an exported global from an instance.
    fn get(
        &mut self,
        instance_name: Option<String>,
        field: &str,
    ) -> Result<ActionOutcome, WastError> {
        let instance = self
            .get_instance(instance_name.as_ref().map(|x| &**x))?
            .clone();
        self.context
            .get(&instance, field)
            .map_err(WastError::Action)
    }

    /// Perform the action of a `PerformAction`.
    fn perform_action_command(&mut self, action: Action) -> Result<(), WastError> {
        match self.perform_action(action)? {
            ActionOutcome::Returned { .. } => Ok(()),
            ActionOutcome::Trapped { message } => Err(WastError::Trap(message)),
        }
    }

    /// Run a wast script from a byte buffer.
    pub fn run_buffer(&mut self, filename: &str, wast: &[u8]) -> Result<(), WastFileError> {
        let features: WabtFeatures = convert_features(self.context.features());

        // Work around https://github.com/pepyakin/wabt-rs/issues/59
        let test_filename = Path::new(filename)
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned();

        let mut parser = ScriptParser::from_source_and_name_with_features(
            str::from_utf8(wast)
                .map_err(|error| WastFileError {
                    filename: filename.to_string(),
                    line: 0,
                    error: WastError::Utf8(error),
                })?
                .as_bytes(),
            &test_filename,
            features,
        )
        .map_err(|error| WastFileError {
            filename: filename.to_string(),
            line: 0,
            error: WastError::Syntax(error),
        })?;

        while let Some(Command { kind, line }) = parser.next().expect("parser") {
            match kind {
                CommandKind::Module {
                    module: instance_name,
                    name,
                } => {
                    self.module(name, instance_name)
                        .map_err(|error| WastFileError {
                            filename: filename.to_string(),
                            line,
                            error: WastError::Action(error),
                        })?;
                }
                CommandKind::Register { name, as_name } => {
                    self.register(name, as_name)
                        .map_err(|error| WastFileError {
                            filename: filename.to_string(),
                            line,
                            error,
                        })?;
                }
                CommandKind::PerformAction(action) => {
                    self.perform_action_command(action)
                        .map_err(|error| WastFileError {
                            filename: filename.to_string(),
                            line,
                            error,
                        })?;
                }
                CommandKind::AssertReturn { action, expected } => {
                    match self.perform_action(action).map_err(|error| WastFileError {
                        filename: filename.to_string(),
                        line,
                        error,
                    })? {
                        ActionOutcome::Returned { values } => {
                            for (v, e) in values
                                .iter()
                                .cloned()
                                .zip(expected.iter().cloned().map(runtime_value))
                            {
                                if v != e {
                                    return Err(WastFileError {
                                        filename: filename.to_string(),
                                        line,
                                        error: WastError::Assert(format!(
                                            "expected {}, got {}",
                                            e, v
                                        )),
                                    });
                                }
                            }
                        }
                        ActionOutcome::Trapped { message } => {
                            return Err(WastFileError {
                                filename: filename.to_string(),
                                line,
                                error: WastError::Assert(format!("unexpected trap: {}", message)),
                            });
                        }
                    }
                }
                CommandKind::AssertTrap { action, message } => {
                    match self.perform_action(action).map_err(|error| WastFileError {
                        filename: filename.to_string(),
                        line,
                        error,
                    })? {
                        ActionOutcome::Returned { values } => {
                            return Err(WastFileError {
                                filename: filename.to_string(),
                                line,
                                error: WastError::Assert(format!(
                                    "expected trap, but invoke returned with {:?}",
                                    values
                                )),
                            });
                        }
                        ActionOutcome::Trapped {
                            message: trap_message,
                        } => {
                            if !trap_message.contains(&message) {
                                #[cfg(feature = "lightbeam")]
                                println!(
                                    "{}:{}: TODO: Check the assert_trap message: {}",
                                    filename, line, message
                                );
                                #[cfg(not(feature = "lightbeam"))]
                                return Err(WastFileError {
                                    filename: filename.to_string(),
                                    line,
                                    error: WastError::Assert(format!(
                                        "expected {}, got {}",
                                        message, trap_message
                                    )),
                                });
                            }
                        }
                    }
                }
                CommandKind::AssertExhaustion { action, message } => {
                    match self.perform_action(action).map_err(|error| WastFileError {
                        filename: filename.to_string(),
                        line,
                        error,
                    })? {
                        ActionOutcome::Returned { values } => {
                            return Err(WastFileError {
                                filename: filename.to_string(),
                                line,
                                error: WastError::Assert(format!(
                                    "expected callstack exhaustion, but invoke returned with {:?}",
                                    values
                                )),
                            });
                        }
                        ActionOutcome::Trapped {
                            message: trap_message,
                        } => {
                            if !trap_message.contains(&message) {
                                return Err(WastFileError {
                                    filename: filename.to_string(),
                                    line,
                                    error: WastError::Assert(format!(
                                        "expected exhaustion with {}, got {}",
                                        message, trap_message
                                    )),
                                });
                            }
                        }
                    }
                }
                CommandKind::AssertReturnCanonicalNan { action } => {
                    match self.perform_action(action).map_err(|error| WastFileError {
                        filename: filename.to_string(),
                        line,
                        error,
                    })? {
                        ActionOutcome::Returned { values } => {
                            for v in values.iter() {
                                match v {
                                    RuntimeValue::I32(_) | RuntimeValue::I64(_) => {
                                        return Err(WastFileError {
                                            filename: filename.to_string(),
                                            line,
                                            error: WastError::Type(format!(
                                                "unexpected integer type in NaN test"
                                            )),
                                        });
                                    }
                                    RuntimeValue::F32(x) => {
                                        if (x & 0x7fffffff) != 0x7fc00000 {
                                            return Err(WastFileError {
                                                filename: filename.to_string(),
                                                line,
                                                error: WastError::Assert(format!(
                                                    "expected canonical NaN"
                                                )),
                                            });
                                        }
                                    }
                                    RuntimeValue::F64(x) => {
                                        if (x & 0x7fffffffffffffff) != 0x7ff8000000000000 {
                                            return Err(WastFileError {
                                                filename: filename.to_string(),
                                                line,
                                                error: WastError::Assert(format!(
                                                    "expected canonical NaN"
                                                )),
                                            });
                                        }
                                    }
                                    RuntimeValue::V128(_) => {
                                        return Err(WastFileError {
                                            filename: filename.to_string(),
                                            line,
                                            error: WastError::Type(format!(
                                                "unexpected vector type in NaN test"
                                            )),
                                        });
                                    }
                                };
                            }
                        }
                        ActionOutcome::Trapped { message } => {
                            return Err(WastFileError {
                                filename: filename.to_string(),
                                line,
                                error: WastError::Assert(format!("unexpected trap: {}", message)),
                            });
                        }
                    }
                }
                CommandKind::AssertReturnArithmeticNan { action } => {
                    match self.perform_action(action).map_err(|error| WastFileError {
                        filename: filename.to_string(),
                        line,
                        error,
                    })? {
                        ActionOutcome::Returned { values } => {
                            for v in values.iter() {
                                match v {
                                    RuntimeValue::I32(_) | RuntimeValue::I64(_) => {
                                        return Err(WastFileError {
                                            filename: filename.to_string(),
                                            line,
                                            error: WastError::Type(format!(
                                                "unexpected integer type in NaN test",
                                            )),
                                        });
                                    }
                                    RuntimeValue::F32(x) => {
                                        if (x & 0x00400000) != 0x00400000 {
                                            return Err(WastFileError {
                                                filename: filename.to_string(),
                                                line,
                                                error: WastError::Assert(format!(
                                                    "expected arithmetic NaN"
                                                )),
                                            });
                                        }
                                    }
                                    RuntimeValue::F64(x) => {
                                        if (x & 0x0008000000000000) != 0x0008000000000000 {
                                            return Err(WastFileError {
                                                filename: filename.to_string(),
                                                line,
                                                error: WastError::Assert(format!(
                                                    "expected arithmetic NaN"
                                                )),
                                            });
                                        }
                                    }
                                    RuntimeValue::V128(_) => {
                                        return Err(WastFileError {
                                            filename: filename.to_string(),
                                            line,
                                            error: WastError::Type(format!(
                                                "unexpected vector type in NaN test",
                                            )),
                                        });
                                    }
                                };
                            }
                        }
                        ActionOutcome::Trapped { message } => {
                            return Err(WastFileError {
                                filename: filename.to_string(),
                                line,
                                error: WastError::Assert(format!("unexpected trap: {}", message)),
                            });
                        }
                    }
                }
                CommandKind::AssertInvalid { module, message } => {
                    self.module(None, module).expect_err(&format!(
                        "{}:{}: invalid module was successfully instantiated",
                        filename, line
                    ));
                    println!(
                        "{}:{}: TODO: Check the assert_invalid message: {}",
                        filename, line, message
                    );
                }
                CommandKind::AssertMalformed { module, message } => {
                    self.module(None, module).expect_err(&format!(
                        "{}:{}: malformed module was successfully instantiated",
                        filename, line
                    ));
                    println!(
                        "{}:{}: TODO: Check the assert_malformed message: {}",
                        filename, line, message
                    );
                }
                CommandKind::AssertUninstantiable { module, message } => {
                    let _err = self.module(None, module).expect_err(&format!(
                        "{}:{}: uninstantiable module was successfully instantiated",
                        filename, line
                    ));
                    println!(
                        "{}:{}: TODO: Check the assert_uninstantiable message: {}",
                        filename, line, message
                    );
                }
                CommandKind::AssertUnlinkable { module, message } => {
                    let _err = self.module(None, module).expect_err(&format!(
                        "{}:{}: unlinkable module was successfully linked",
                        filename, line
                    ));
                    println!(
                        "{}:{}: TODO: Check the assert_unlinkable message: {}",
                        filename, line, message
                    );
                }
            }
        }

        Ok(())
    }

    /// Run a wast script from a file.
    pub fn run_file(&mut self, path: &Path) -> Result<(), WastFileError> {
        let filename = path.display().to_string();
        let buffer = read_to_end(path).map_err(|e| WastFileError {
            filename,
            line: 0,
            error: WastError::IO(e),
        })?;
        self.run_buffer(&path.display().to_string(), &buffer)
    }
}

fn read_to_end(path: &Path) -> Result<Vec<u8>, io::Error> {
    let mut buf: Vec<u8> = Vec::new();
    let mut file = fs::File::open(path)?;
    file.read_to_end(&mut buf)?;
    Ok(buf)
}

/// Helper to convert wasmtime features to WABT features; would be nicer as Into<WabtFeatures> but
/// wasmtime-jit does not have a wabt dependency
fn convert_features(features: &Features) -> WabtFeatures {
    let mut wabt_features = WabtFeatures::new();
    if features.simd {
        wabt_features.enable_simd()
    }
    if features.multi_value {
        wabt_features.enable_multi_value()
    }
    if features.bulk_memory {
        wabt_features.enable_bulk_memory()
    }
    if features.threads {
        wabt_features.enable_threads()
    }
    wabt_features
}
