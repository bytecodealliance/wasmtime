use cranelift_codegen::isa;
use cranelift_entity::PrimaryMap;
use spectest::SpecTest;
use std::collections::HashMap;
use std::io::Read;
use std::path::Path;
use std::{fmt, fs, io, str};
use wabt::script::{Action, Command, CommandKind, ModuleBinary, ScriptParser, Value};
use wasmtime_execute::{ActionError, ActionOutcome, Code, InstanceWorld, RuntimeValue};

/// Translate from a script::Value to a RuntimeValue.
fn runtime_value(v: Value) -> RuntimeValue {
    match v {
        Value::I32(x) => RuntimeValue::I32(x),
        Value::I64(x) => RuntimeValue::I64(x),
        Value::F32(x) => RuntimeValue::F32(x.to_bits()),
        Value::F64(x) => RuntimeValue::F64(x.to_bits()),
    }
}

/// Indicates an unknown module was specified.
#[derive(Fail, Debug)]
pub struct UnknownModule {
    module: Option<String>,
}

impl fmt::Display for UnknownModule {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.module {
            None => write!(f, "no default module present"),
            Some(ref name) => write!(f, "no module {} present", name),
        }
    }
}

/// Error message used by `WastContext`.
#[derive(Fail, Debug)]
pub enum WastError {
    /// An assert command was not satisfied.
    Assert(String),
    /// An unknown module name was used.
    Module(UnknownModule),
    /// An error occured while performing an action.
    Action(ActionError),
    /// An action trapped.
    Trap(String),
    /// There was a type error in inputs or outputs of an action.
    Type(String),
    /// The was an I/O error while reading the wast file.
    IO(io::Error),
}

impl fmt::Display for WastError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            WastError::Assert(ref message) => write!(f, "Assert command failed: {}", message),
            WastError::Module(ref error) => error.fmt(f),
            WastError::Action(ref error) => error.fmt(f),
            WastError::Trap(ref message) => write!(f, "trap: {}", message),
            WastError::Type(ref message) => write!(f, "type error: {}", message),
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

/// An opaque reference to an `InstanceWorld`.
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WorldIndex(u32);
entity_impl!(WorldIndex, "world");

/// The wast test script language allows modules to be defined and actions
/// to be performed on them.
pub struct WastContext {
    /// A namespace of wasm modules, keyed by an optional name.
    worlds: PrimaryMap<WorldIndex, InstanceWorld>,
    current: Option<WorldIndex>,
    namespace: HashMap<String, WorldIndex>,
    code: Code,
    spectest: SpecTest,
}

impl WastContext {
    /// Construct a new instance of `WastContext`.
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            worlds: PrimaryMap::new(),
            current: None,
            namespace: HashMap::new(),
            code: Code::new(),
            spectest: SpecTest::new()?,
        })
    }

    fn instantiate(
        &mut self,
        isa: &isa::TargetIsa,
        module: ModuleBinary,
    ) -> Result<InstanceWorld, ActionError> {
        InstanceWorld::new(&mut self.code, isa, &module.into_vec(), &mut self.spectest)
    }

    fn get_world(&mut self, module: &Option<String>) -> Result<WorldIndex, WastError> {
        let index = *if let Some(name) = module {
            self.namespace.get_mut(name).ok_or_else(|| {
                WastError::Module(UnknownModule {
                    module: Some(name.to_owned()),
                })
            })
        } else {
            self.current
                .as_mut()
                .ok_or_else(|| WastError::Module(UnknownModule { module: None }))
        }?;

        Ok(index)
    }

    /// Define a module and register it.
    pub fn module(
        &mut self,
        isa: &isa::TargetIsa,
        name: Option<String>,
        module: ModuleBinary,
    ) -> Result<(), ActionError> {
        let world = self.instantiate(isa, module)?;
        let index = if let Some(name) = name {
            self.register(name, world)
        } else {
            self.worlds.push(world)
        };
        self.current = Some(index);
        Ok(())
    }

    /// Register a module to make it available for performing actions.
    pub fn register(&mut self, name: String, world: InstanceWorld) -> WorldIndex {
        let index = self.worlds.push(world);
        self.namespace.insert(name, index);
        index
    }

    /// Invoke an exported function from a defined module.
    pub fn invoke(
        &mut self,
        isa: &isa::TargetIsa,
        module: Option<String>,
        field: &str,
        args: &[Value],
    ) -> Result<ActionOutcome, WastError> {
        let mut value_args = Vec::with_capacity(args.len());
        for arg in args {
            value_args.push(runtime_value(*arg));
        }
        let index = self.get_world(&module)?;
        self.worlds[index]
            .invoke(&mut self.code, isa, &field, &value_args)
            .map_err(WastError::Action)
    }

    /// Get the value of an exported global from a defined module.
    pub fn get(&mut self, module: Option<String>, field: &str) -> Result<RuntimeValue, WastError> {
        let index = self.get_world(&module)?;
        self.worlds[index].get(&field).map_err(WastError::Action)
    }

    fn perform_action(
        &mut self,
        isa: &isa::TargetIsa,
        action: Action,
    ) -> Result<ActionOutcome, WastError> {
        match action {
            Action::Invoke {
                module,
                field,
                args,
            } => self.invoke(isa, module, &field, &args),
            Action::Get { module, field } => {
                let value = self.get(module, &field)?;
                Ok(ActionOutcome::Returned {
                    values: vec![value],
                })
            }
        }
    }

    /// Run a wast script from a byte buffer.
    pub fn run_buffer(
        &mut self,
        isa: &isa::TargetIsa,
        filename: &str,
        wast: &[u8],
    ) -> Result<(), WastFileError> {
        let mut parser = ScriptParser::from_str(str::from_utf8(wast).unwrap()).unwrap();

        while let Some(Command { kind, line }) = parser.next().unwrap() {
            match kind {
                CommandKind::Module { module, name } => {
                    self.module(isa, name, module)
                        .map_err(|error| WastFileError {
                            filename: filename.to_string(),
                            line,
                            error: WastError::Action(error),
                        })?;
                }
                CommandKind::Register {
                    name: _name,
                    as_name: _as_name,
                } => {
                    println!("{}:{}: TODO: Implement register", filename, line);
                }
                CommandKind::PerformAction(action) => match self
                    .perform_action(isa, action)
                    .map_err(|error| WastFileError {
                        filename: filename.to_string(),
                        line,
                        error,
                    })? {
                    ActionOutcome::Returned { .. } => {}
                    ActionOutcome::Trapped { message } => {
                        return Err(WastFileError {
                            filename: filename.to_string(),
                            line,
                            error: WastError::Trap(message),
                        });
                    }
                },
                CommandKind::AssertReturn { action, expected } => {
                    match self
                        .perform_action(isa, action)
                        .map_err(|error| WastFileError {
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
                    match self
                        .perform_action(isa, action)
                        .map_err(|error| WastFileError {
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
                    match self
                        .perform_action(isa, action)
                        .map_err(|error| WastFileError {
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
                    match self
                        .perform_action(isa, action)
                        .map_err(|error| WastFileError {
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
                    match self
                        .perform_action(isa, action)
                        .map_err(|error| WastFileError {
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
                CommandKind::AssertInvalid {
                    module: _module,
                    message: _message,
                } => {
                    println!("{}:{}: TODO: Implement assert_invalid", filename, line);
                }
                CommandKind::AssertMalformed {
                    module: _module,
                    message: _message,
                } => {
                    println!("{}:{}: TODO: Implement assert_malformed", filename, line);
                }
                CommandKind::AssertUninstantiable { module, message } => {
                    let _err = self.module(isa, None, module).expect_err(&format!(
                        "{}:{}: uninstantiable module was successfully instantiated",
                        filename, line
                    ));
                    println!(
                        "{}:{}: TODO: Check the assert_uninstantiable message: {}",
                        filename, line, message
                    );
                }
                CommandKind::AssertUnlinkable { module, message } => {
                    let _err = self.module(isa, None, module).expect_err(&format!(
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
    pub fn run_file(&mut self, isa: &isa::TargetIsa, path: &Path) -> Result<(), WastFileError> {
        let filename = path.display().to_string();
        let buffer = read_to_end(path).map_err(|e| WastFileError {
            filename,
            line: 0,
            error: WastError::IO(e),
        })?;
        self.run_buffer(isa, &path.display().to_string(), &buffer)
    }
}

fn read_to_end(path: &Path) -> Result<Vec<u8>, io::Error> {
    let mut buf: Vec<u8> = Vec::new();
    let mut file = fs::File::open(path)?;
    file.read_to_end(&mut buf)?;
    Ok(buf)
}
