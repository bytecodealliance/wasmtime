extern crate cranelift_codegen;
extern crate wabt;
extern crate wasmtime_environ;
extern crate wasmtime_execute;

use cranelift_codegen::settings::Configurable;
use cranelift_codegen::{isa, settings};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::io::Read;
use std::path::Path;
use std::str;
use wabt::script::{self, Action, Command, CommandKind, ScriptParser};
use wasmtime_environ::{Compilation, Module, ModuleEnvironment, Tunables};
use wasmtime_execute::{
    compile_and_link_module, finish_instantiation, invoke, Code, Instance, InvokeOutcome, Value,
};

struct InstanceWorld {
    module: Module,
    context: Vec<*mut u8>,
    // FIXME
    #[allow(dead_code)]
    instance: Instance,
    compilation: Compilation,
}

impl InstanceWorld {
    fn new(code: &mut Code, isa: &isa::TargetIsa, data: &[u8]) -> Result<Self, String> {
        let mut module = Module::new();
        let tunables = Tunables::default();
        let (context, instance, compilation) = {
            let translation = {
                let environ = ModuleEnvironment::new(isa, &mut module, tunables);

                environ.translate(&data).map_err(|e| e.to_string())?
            };

            let imports_resolver = |_env: &str, _function: &str| None;

            let compilation = compile_and_link_module(isa, &translation, &imports_resolver)?;
            let mut instance = Instance::new(
                translation.module,
                &compilation,
                &translation.lazy.data_initializers,
            )?;

            (
                finish_instantiation(code, isa, &translation.module, &compilation, &mut instance)?,
                instance,
                compilation,
            )
        };

        Ok(Self {
            module,
            context,
            instance,
            compilation,
        })
    }

    fn invoke(
        &mut self,
        code: &mut Code,
        isa: &isa::TargetIsa,
        f: &str,
        args: &[Value],
    ) -> Result<InvokeOutcome, String> {
        invoke(
            code,
            isa,
            &self.module,
            &self.compilation,
            &mut self.context,
            &f,
            args,
        ).map_err(|e| e.to_string())
    }
}

fn translate(code: &mut Code, isa: &isa::TargetIsa, data: &[u8]) -> Result<InstanceWorld, String> {
    InstanceWorld::new(code, isa, data)
}

struct Instances {
    current: Option<InstanceWorld>,
    namespace: HashMap<String, InstanceWorld>,
}

impl Instances {
    fn new() -> Self {
        Self {
            current: None,
            namespace: HashMap::new(),
        }
    }

    fn unnamed(&mut self, instance: InstanceWorld) {
        self.current = Some(instance);
    }

    fn named(&mut self, name: String, instance: InstanceWorld) {
        self.namespace.insert(name, instance);
    }

    fn perform_action(
        &mut self,
        code: &mut Code,
        isa: &isa::TargetIsa,
        action: Action,
    ) -> InvokeOutcome {
        match action {
            Action::Invoke {
                module,
                field,
                args,
            } => {
                let mut value_args = Vec::new();
                for a in args {
                    value_args.push(match a {
                        script::Value::I32(i) => Value::I32(i),
                        script::Value::I64(i) => Value::I64(i),
                        script::Value::F32(i) => Value::F32(i.to_bits()),
                        script::Value::F64(i) => Value::F64(i.to_bits()),
                    });
                }
                match module {
                    None => match self.current {
                        None => panic!("invoke performed with no module present"),
                        Some(ref mut instance_world) => instance_world
                            .invoke(code, isa, &field, &value_args)
                            .expect(&format!("error invoking {} in current module", field)),
                    },
                    Some(name) => self
                        .namespace
                        .get_mut(&name)
                        .expect(&format!("module {} not declared", name))
                        .invoke(code, isa, &field, &value_args)
                        .expect(&format!("error invoking {} in module {}", field, name)),
                }
            }
            _ => panic!("unsupported action {:?}", action),
        }
    }
}

#[test]
fn spec_core() {
    let mut flag_builder = settings::builder();
    flag_builder.enable("enable_verifier").unwrap();

    let isa_builder = cranelift_native::builder().unwrap_or_else(|_| {
        panic!("host machine is not a supported target");
    });
    let isa = isa_builder.finish(settings::Flags::new(flag_builder));

    let mut paths: Vec<_> = fs::read_dir("tests/wast")
        .unwrap()
        .map(|r| r.unwrap())
        .filter(|p| {
            // Ignore files starting with `.`, which could be editor temporary files
            if let Some(stem) = p.path().file_stem() {
                if let Some(stemstr) = stem.to_str() {
                    return !stemstr.starts_with('.');
                }
            }
            false
        }).collect();
    paths.sort_by_key(|dir| dir.path());
    for path in paths {
        let path = path.path();
        let source = read_to_end(&path).unwrap();
        test_wast(&path, &*isa, &source);
    }
}

#[cfg(test)]
fn read_to_end(path: &Path) -> Result<Vec<u8>, io::Error> {
    let mut buf: Vec<u8> = Vec::new();
    let mut file = fs::File::open(path)?;
    file.read_to_end(&mut buf)?;
    Ok(buf)
}

#[cfg(test)]
fn test_wast(path: &Path, isa: &isa::TargetIsa, wast: &[u8]) {
    println!("Testing {}", path.display());

    let mut parser = ScriptParser::from_str(str::from_utf8(wast).unwrap()).unwrap();
    let mut instances = Instances::new();
    let mut code = Code::new();

    while let Some(Command { kind, line }) = parser.next().unwrap() {
        match kind {
            CommandKind::Module { module, name } => {
                if let Some(name) = name {
                    instances.named(
                        name,
                        translate(&mut code, &*isa, &module.clone().into_vec()).unwrap(),
                    );
                }

                instances.unnamed(translate(&mut code, &*isa, &module.clone().into_vec()).unwrap());
            }
            CommandKind::PerformAction(action) => {
                match instances.perform_action(&mut code, &*isa, action) {
                    InvokeOutcome::Returned { .. } => {}
                    InvokeOutcome::Trapped { message } => {
                        panic!("{}:{}: a trap occurred: {}", path.display(), line, message);
                    }
                }
            }
            CommandKind::AssertReturn { action, expected } => {
                match instances.perform_action(&mut code, &*isa, action) {
                    InvokeOutcome::Returned { values } => {
                        for (v, e) in values.iter().zip(expected.iter()) {
                            match *e {
                                script::Value::I32(x) => {
                                    assert_eq!(x, v.unwrap_i32(), "at {}:{}", path.display(), line)
                                }
                                script::Value::I64(x) => {
                                    assert_eq!(x, v.unwrap_i64(), "at {}:{}", path.display(), line)
                                }
                                script::Value::F32(x) => assert_eq!(
                                    x.to_bits(),
                                    v.unwrap_f32(),
                                    "at {}:{}",
                                    path.display(),
                                    line
                                ),
                                script::Value::F64(x) => assert_eq!(
                                    x.to_bits(),
                                    v.unwrap_f64(),
                                    "at {}:{}",
                                    path.display(),
                                    line
                                ),
                            };
                        }
                    }
                    InvokeOutcome::Trapped { message } => {
                        panic!(
                            "{}:{}: expected normal return, but a trap occurred: {}",
                            path.display(),
                            line,
                            message
                        );
                    }
                }
            }
            CommandKind::AssertTrap { action, message } => {
                match instances.perform_action(&mut code, &*isa, action) {
                    InvokeOutcome::Returned { values } => panic!(
                        "{}:{}: expected trap, but invoke returned with {:?}",
                        path.display(),
                        line,
                        values
                    ),
                    InvokeOutcome::Trapped {
                        message: trap_message,
                    } => {
                        println!(
                            "{}:{}: TODO: Check the trap message: expected {}, got {}",
                            path.display(),
                            line,
                            message,
                            trap_message
                        );
                    }
                }
            }
            CommandKind::AssertExhaustion { action } => {
                match instances.perform_action(&mut code, &*isa, action) {
                    InvokeOutcome::Returned { values } => panic!(
                        "{}:{}: expected exhaustion, but invoke returned with {:?}",
                        path.display(),
                        line,
                        values
                    ),
                    InvokeOutcome::Trapped { message } => {
                        println!(
                            "{}:{}: TODO: Check the exhaustion message: {}",
                            path.display(),
                            line,
                            message
                        );
                    }
                }
            }
            command => {
                println!("{}:{}: TODO: implement {:?}", path.display(), line, command);
            }
        }
    }
}
