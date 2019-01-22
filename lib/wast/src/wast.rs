use crate::spectest::instantiate_spectest;
use std::cell::RefCell;
use std::collections::HashMap;
use std::io::Read;
use std::path::Path;
use std::rc::Rc;
use std::{fmt, fs, io, str};
use wabt::script::{Action, Command, CommandKind, ModuleBinary, ScriptParser, Value};
use wasmparser::{validate, OperatorValidatorConfig, ValidatingParserConfig};
use wasmtime_jit::{
    instantiate, ActionError, ActionOutcome, Compiler, Instance, InstanceIndex, InstantiationError,
    Namespace, RuntimeValue, SetupError,
};

/// Translate from a `script::Value` to a `RuntimeValue`.
fn runtime_value(v: Value) -> RuntimeValue {
    match v {
        Value::I32(x) => RuntimeValue::I32(x),
        Value::I64(x) => RuntimeValue::I64(x),
        Value::F32(x) => RuntimeValue::F32(x.to_bits()),
        Value::F64(x) => RuntimeValue::F64(x.to_bits()),
    }
}

/// Indicates an unknown instance was specified.
#[derive(Fail, Debug)]
pub struct UnknownInstance {
    instance: Option<String>,
}

impl fmt::Display for UnknownInstance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.instance {
            None => write!(f, "no default instance present"),
            Some(ref name) => write!(f, "no instance {} present", name),
        }
    }
}

/// Error message used by `WastContext`.
#[derive(Fail, Debug)]
pub enum WastError {
    /// An assert command was not satisfied.
    Assert(String),
    /// An unknown instance name was used.
    Instance(UnknownInstance),
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
    /// A namespace of wasm modules, keyed by an optional name.
    current: Option<InstanceIndex>,
    namespace: Namespace,
    compiler: Box<Compiler>,
}

impl WastContext {
    /// Construct a new instance of `WastContext`.
    pub fn new(compiler: Box<Compiler>) -> Self {
        Self {
            current: None,
            namespace: Namespace::new(),
            compiler,
        }
    }

    fn validate(&mut self, data: &[u8]) -> Result<(), String> {
        let config = ValidatingParserConfig {
            operator_config: OperatorValidatorConfig {
                enable_threads: false,
                enable_reference_types: false,
            },
            mutable_global_imports: true,
        };

        // TODO: Fix Cranelift to be able to perform validation itself, rather
        // than calling into wasmparser ourselves here.
        if validate(data, Some(config)) {
            Ok(())
        } else {
            // TODO: Work with wasmparser to get better error messages.
            Err("module did not validate".to_owned())
        }
    }

    fn instantiate(&mut self, module: ModuleBinary) -> Result<Instance, SetupError> {
        let data = module.into_vec();

        self.validate(&data).map_err(SetupError::Validate)?;

        instantiate(
            &mut *self.compiler,
            &data,
            &mut self.namespace,
            Rc::new(RefCell::new(HashMap::new())),
        )
    }

    fn get_index(&mut self, instance_name: &Option<String>) -> Result<InstanceIndex, WastError> {
        let index = *if let Some(instance_name) = instance_name {
            self.namespace
                .get_instance_index(instance_name)
                .ok_or_else(|| {
                    WastError::Instance(UnknownInstance {
                        instance: Some(instance_name.to_string()),
                    })
                })
        } else {
            self.current
                .as_mut()
                .ok_or_else(|| WastError::Instance(UnknownInstance { instance: None }))
        }?;

        Ok(index)
    }

    /// Register "spectest" which is used by the spec testsuite.
    pub fn register_spectest(&mut self) -> Result<(), InstantiationError> {
        let instance = instantiate_spectest()?;
        self.namespace.instance(Some("spectest"), instance);
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
        let instance = self.instantiate(module).map_err(ActionError::Setup)?;
        let index = self
            .namespace
            .instance(instance_name.as_ref().map(String::as_str), instance);
        self.current = Some(index);
        Ok(())
    }

    /// Register an instance to make it available for performing actions.
    fn register(&mut self, name: Option<String>, as_name: String) -> Result<(), WastError> {
        let index = self.get_index(&name)?;
        self.namespace.register(as_name, index);
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
        let index = self.get_index(&instance_name)?;
        self.namespace
            .invoke(&mut *self.compiler, index, field, &value_args)
            .map_err(WastError::Action)
    }

    /// Get the value of an exported global from an instance.
    fn get(
        &mut self,
        instance_name: Option<String>,
        field: &str,
    ) -> Result<ActionOutcome, WastError> {
        let index = self.get_index(&instance_name)?;
        let value = self
            .namespace
            .get(index, field)
            .map_err(WastError::Action)?;
        Ok(ActionOutcome::Returned {
            values: vec![value],
        })
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
        let mut parser =
            ScriptParser::from_str(str::from_utf8(wast).map_err(|error| WastFileError {
                filename: filename.to_string(),
                line: 0,
                error: WastError::Utf8(error),
            })?)
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
                            println!(
                                "{}:{}: TODO: Check the assert_trap message: expected {}, got {}",
                                filename, line, message, trap_message
                            );
                        }
                    }
                }
                CommandKind::AssertExhaustion { action } => {
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
                        ActionOutcome::Trapped { message } => {
                            println!(
                                "{}:{}: TODO: Check the assert_exhaustion message: {}",
                                filename, line, message
                            );
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
