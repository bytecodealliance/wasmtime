#[cfg(feature = "component-model")]
use crate::component;
use crate::core;
use crate::spectest::*;
use anyhow::{Context as _, anyhow, bail};
use json_from_wast::{Action, Command, Const, WasmFile, WasmFileType};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::str;
use std::sync::Arc;
use std::thread;
use wasmtime::*;
use wast::lexer::Lexer;
use wast::parser::{self, ParseBuffer};

/// The wast test script language allows modules to be defined and actions
/// to be performed on them.
pub struct WastContext<T: 'static> {
    /// Wast files have a concept of a "current" module, which is the most
    /// recently defined.
    current: Option<InstanceKind>,
    core_linker: Linker<T>,
    modules: HashMap<String, ModuleKind>,
    #[cfg(feature = "component-model")]
    component_linker: component::Linker<T>,
    pub(crate) store: Store<T>,
    pub(crate) async_runtime: Option<tokio::runtime::Runtime>,
    generate_dwarf: bool,
    precompile_save: Option<PathBuf>,
    precompile_load: Option<PathBuf>,

    modules_by_filename: Arc<HashMap<String, Vec<u8>>>,
}

enum Outcome<T = Results> {
    Ok(T),
    Trap(Error),
}

impl<T> Outcome<T> {
    fn map<U>(self, map: impl FnOnce(T) -> U) -> Outcome<U> {
        match self {
            Outcome::Ok(t) => Outcome::Ok(map(t)),
            Outcome::Trap(t) => Outcome::Trap(t),
        }
    }

    fn into_result(self) -> Result<T> {
        match self {
            Outcome::Ok(t) => Ok(t),
            Outcome::Trap(t) => Err(t),
        }
    }
}

#[derive(Debug)]
enum Results {
    Core(Vec<Val>),
    #[cfg(feature = "component-model")]
    Component(Vec<component::Val>),
}

#[derive(Clone)]
enum ModuleKind {
    Core(Module),
    #[cfg(feature = "component-model")]
    Component(component::Component),
}

enum InstanceKind {
    Core(Instance),
    #[cfg(feature = "component-model")]
    Component(component::Instance),
}

enum Export {
    Core(Extern),
    #[cfg(feature = "component-model")]
    Component(component::Func),
}

/// Whether or not to use async APIs when calling wasm during wast testing.
///
/// Passed to [`WastContext::new`].
#[derive(Copy, Clone, PartialEq)]
#[expect(missing_docs, reason = "self-describing variants")]
pub enum Async {
    Yes,
    No,
}

impl<T> WastContext<T>
where
    T: Clone + Send + 'static,
{
    /// Construct a new instance of `WastContext`.
    ///
    /// Note that the provided `Store<T>` must have `Config::async_support`
    /// enabled as all functions will be run with `call_async`. This is done to
    /// support the component model async features that tests might use.
    pub fn new(store: Store<T>, async_: Async) -> Self {
        // Spec tests will redefine the same module/name sometimes, so we need
        // to allow shadowing in the linker which picks the most recent
        // definition as what to link when linking.
        let mut core_linker = Linker::new(store.engine());
        core_linker.allow_shadowing(true);
        Self {
            current: None,
            core_linker,
            #[cfg(feature = "component-model")]
            component_linker: {
                let mut linker = component::Linker::new(store.engine());
                linker.allow_shadowing(true);
                linker
            },
            store,
            modules: Default::default(),
            async_runtime: if async_ == Async::Yes {
                Some(
                    tokio::runtime::Builder::new_current_thread()
                        .build()
                        .unwrap(),
                )
            } else {
                None
            },
            generate_dwarf: true,
            precompile_save: None,
            precompile_load: None,
            modules_by_filename: Arc::default(),
        }
    }

    /// Saves precompiled modules/components into `path` instead of executing
    /// test directives.
    pub fn precompile_save(&mut self, path: impl AsRef<Path>) -> &mut Self {
        self.precompile_save = Some(path.as_ref().into());
        self
    }

    /// Loads precompiled modules/components from `path` instead of compiling
    /// natively.
    pub fn precompile_load(&mut self, path: impl AsRef<Path>) -> &mut Self {
        self.precompile_load = Some(path.as_ref().into());
        self
    }

    fn get_export(&mut self, module: Option<&str>, name: &str) -> Result<Export> {
        if let Some(module) = module {
            return Ok(Export::Core(
                self.core_linker
                    .get(&mut self.store, module, name)
                    .ok_or_else(|| anyhow!("no item named `{}::{}` found", module, name))?,
            ));
        }

        let cur = self
            .current
            .as_ref()
            .ok_or_else(|| anyhow!("no previous instance found"))?;
        Ok(match cur {
            InstanceKind::Core(i) => Export::Core(
                i.get_export(&mut self.store, name)
                    .ok_or_else(|| anyhow!("no item named `{}` found", name))?,
            ),
            #[cfg(feature = "component-model")]
            InstanceKind::Component(i) => Export::Component(
                i.get_func(&mut self.store, name)
                    .ok_or_else(|| anyhow!("no func named `{}` found", name))?,
            ),
        })
    }

    fn instantiate_module(&mut self, module: &Module) -> Result<Outcome<Instance>> {
        let instance = match &self.async_runtime {
            Some(rt) => rt.block_on(self.core_linker.instantiate_async(&mut self.store, &module)),
            None => self.core_linker.instantiate(&mut self.store, &module),
        };
        Ok(match instance {
            Ok(i) => Outcome::Ok(i),
            Err(e) => Outcome::Trap(e),
        })
    }

    #[cfg(feature = "component-model")]
    fn instantiate_component(
        &mut self,
        component: &component::Component,
    ) -> Result<Outcome<(component::Component, component::Instance)>> {
        let instance = match &self.async_runtime {
            Some(rt) => rt.block_on(
                self.component_linker
                    .instantiate_async(&mut self.store, &component),
            ),
            None => self
                .component_linker
                .instantiate(&mut self.store, &component),
        };
        Ok(match instance {
            Ok(i) => Outcome::Ok((component.clone(), i)),
            Err(e) => Outcome::Trap(e),
        })
    }

    /// Register "spectest" which is used by the spec testsuite.
    pub fn register_spectest(&mut self, config: &SpectestConfig) -> Result<()> {
        link_spectest(&mut self.core_linker, &mut self.store, config)?;
        #[cfg(feature = "component-model")]
        link_component_spectest(&mut self.component_linker)?;
        Ok(())
    }

    /// Perform the action portion of a command.
    fn perform_action(&mut self, action: &Action<'_>) -> Result<Outcome> {
        match action {
            Action::Invoke {
                module,
                field,
                args,
            } => match self.get_export(module.as_deref(), field)? {
                Export::Core(export) => {
                    let func = export
                        .into_func()
                        .ok_or_else(|| anyhow!("no function named `{field}`"))?;
                    let values = args
                        .iter()
                        .map(|v| match v {
                            Const::Core(v) => core::val(self, v),
                            _ => bail!("expected core function, found other other argument {v:?}"),
                        })
                        .collect::<Result<Vec<_>>>()?;

                    let mut results =
                        vec![Val::null_func_ref(); func.ty(&self.store).results().len()];
                    let result = match &self.async_runtime {
                        Some(rt) => {
                            rt.block_on(func.call_async(&mut self.store, &values, &mut results))
                        }
                        None => func.call(&mut self.store, &values, &mut results),
                    };

                    Ok(match result {
                        Ok(()) => Outcome::Ok(Results::Core(results)),
                        Err(e) => Outcome::Trap(e),
                    })
                }
                #[cfg(feature = "component-model")]
                Export::Component(func) => {
                    let values = args
                        .iter()
                        .map(|v| match v {
                            Const::Component(v) => component::val(v),
                            _ => bail!("expected component function, found other argument {v:?}"),
                        })
                        .collect::<Result<Vec<_>>>()?;

                    let mut results =
                        vec![component::Val::Bool(false); func.results(&self.store).len()];
                    let result = match &self.async_runtime {
                        Some(rt) => {
                            rt.block_on(func.call_async(&mut self.store, &values, &mut results))
                        }
                        None => func.call(&mut self.store, &values, &mut results),
                    };
                    Ok(match result {
                        Ok(()) => {
                            match &self.async_runtime {
                                Some(rt) => rt.block_on(func.post_return_async(&mut self.store))?,
                                None => func.post_return(&mut self.store)?,
                            }

                            Outcome::Ok(Results::Component(results))
                        }
                        Err(e) => Outcome::Trap(e),
                    })
                }
            },
            Action::Get { module, field, .. } => self.get(module.as_deref(), field),
        }
    }

    /// Instantiates the `module` provided and registers the instance under the
    /// `name` provided if successful.
    fn module(&mut self, name: Option<&str>, module: &ModuleKind) -> Result<()> {
        match module {
            ModuleKind::Core(module) => {
                let instance = match self.instantiate_module(&module)? {
                    Outcome::Ok(i) => i,
                    Outcome::Trap(e) => return Err(e).context("instantiation failed"),
                };
                if let Some(name) = name {
                    self.core_linker.instance(&mut self.store, name, instance)?;
                }
                self.current = Some(InstanceKind::Core(instance));
            }
            #[cfg(feature = "component-model")]
            ModuleKind::Component(module) => {
                let (component, instance) = match self.instantiate_component(&module)? {
                    Outcome::Ok(i) => i,
                    Outcome::Trap(e) => return Err(e).context("instantiation failed"),
                };
                if let Some(name) = name {
                    let ty = component.component_type();
                    let mut linker = self.component_linker.instance(name)?;
                    let engine = self.store.engine().clone();
                    for (name, item) in ty.exports(&engine) {
                        match item {
                            component::types::ComponentItem::Module(_) => {
                                let module = instance.get_module(&mut self.store, name).unwrap();
                                linker.module(name, &module)?;
                            }
                            component::types::ComponentItem::Resource(_) => {
                                let resource =
                                    instance.get_resource(&mut self.store, name).unwrap();
                                linker.resource(name, resource, |_, _| Ok(()))?;
                            }
                            // TODO: should ideally reflect more than just
                            // modules/resources into the linker's namespace
                            // but that's not easily supported today for host
                            // functions due to the inability to take a
                            // function from one instance and put it into the
                            // linker (must go through the host right now).
                            _ => {}
                        }
                    }
                }
                self.current = Some(InstanceKind::Component(instance));
            }
        }
        Ok(())
    }

    /// Compiles the module `wat` into binary and returns the name found within
    /// it, if any.
    ///
    /// This will not register the name within `self.modules`.
    fn module_definition(&mut self, file: &WasmFile) -> Result<ModuleKind> {
        let name = match file.module_type {
            WasmFileType::Text => file
                .binary_filename
                .as_ref()
                .ok_or_else(|| anyhow!("cannot compile module that isn't a valid binary"))?,
            WasmFileType::Binary => &file.filename,
        };

        match &self.precompile_load {
            Some(path) => {
                let cwasm = path.join(&name[..]).with_extension("cwasm");
                match Engine::detect_precompiled_file(&cwasm)
                    .with_context(|| format!("failed to read {cwasm:?}"))?
                {
                    Some(Precompiled::Module) => {
                        let module =
                            unsafe { Module::deserialize_file(self.store.engine(), &cwasm)? };
                        Ok(ModuleKind::Core(module))
                    }
                    #[cfg(feature = "component-model")]
                    Some(Precompiled::Component) => {
                        let component = unsafe {
                            component::Component::deserialize_file(self.store.engine(), &cwasm)?
                        };
                        Ok(ModuleKind::Component(component))
                    }
                    #[cfg(not(feature = "component-model"))]
                    Some(Precompiled::Component) => {
                        bail!("support for components disabled at compile time")
                    }
                    None => bail!("expected a cwasm file"),
                }
            }
            None => {
                let bytes = &self.modules_by_filename[&name[..]];

                if wasmparser::Parser::is_core_wasm(&bytes) {
                    let module = Module::new(self.store.engine(), &bytes)?;
                    Ok(ModuleKind::Core(module))
                } else {
                    #[cfg(feature = "component-model")]
                    {
                        let component = component::Component::new(self.store.engine(), &bytes)?;
                        Ok(ModuleKind::Component(component))
                    }
                    #[cfg(not(feature = "component-model"))]
                    bail!("component-model support not enabled");
                }
            }
        }
    }

    /// Register an instance to make it available for performing actions.
    fn register(&mut self, name: Option<&str>, as_name: &str) -> Result<()> {
        match name {
            Some(name) => self.core_linker.alias_module(name, as_name),
            None => {
                let current = self
                    .current
                    .as_ref()
                    .ok_or(anyhow!("no previous instance"))?;
                match current {
                    InstanceKind::Core(current) => {
                        self.core_linker
                            .instance(&mut self.store, as_name, *current)?;
                    }
                    #[cfg(feature = "component-model")]
                    InstanceKind::Component(_) => {
                        bail!("register not implemented for components");
                    }
                }
                Ok(())
            }
        }
    }

    /// Get the value of an exported global from an instance.
    fn get(&mut self, instance_name: Option<&str>, field: &str) -> Result<Outcome> {
        let global = match self.get_export(instance_name, field)? {
            Export::Core(e) => e
                .into_global()
                .ok_or_else(|| anyhow!("no global named `{field}`"))?,
            #[cfg(feature = "component-model")]
            Export::Component(_) => bail!("no global named `{field}`"),
        };
        Ok(Outcome::Ok(Results::Core(vec![
            global.get(&mut self.store),
        ])))
    }

    fn assert_return(&mut self, result: Outcome, results: &[Const]) -> Result<()> {
        match result.into_result()? {
            Results::Core(values) => {
                if values.len() != results.len() {
                    bail!("expected {} results found {}", results.len(), values.len());
                }
                for (i, (v, e)) in values.iter().zip(results).enumerate() {
                    let e = match e {
                        Const::Core(core) => core,
                        _ => bail!("expected core value found other value {e:?}"),
                    };
                    core::match_val(&mut self.store, v, e)
                        .with_context(|| format!("result {i} didn't match"))?;
                }
            }
            #[cfg(feature = "component-model")]
            Results::Component(values) => {
                if values.len() != results.len() {
                    bail!("expected {} results found {}", results.len(), values.len());
                }
                for (i, (v, e)) in values.iter().zip(results).enumerate() {
                    let e = match e {
                        Const::Component(val) => val,
                        _ => bail!("expected component value found other value {e:?}"),
                    };
                    component::match_val(e, v)
                        .with_context(|| format!("result {i} didn't match"))?;
                }
            }
        }
        Ok(())
    }

    fn assert_trap(&self, result: Outcome, expected: &str) -> Result<()> {
        let trap = match result {
            Outcome::Ok(values) => bail!("expected trap, got {:?}", values),
            Outcome::Trap(t) => t,
        };
        let actual = format!("{trap:?}");
        if actual.contains(expected)
            // `bulk-memory-operations/bulk.wast` checks for a message that
            // specifies which element is uninitialized, but our traps don't
            // shepherd that information out.
            || (expected.contains("uninitialized element 2") && actual.contains("uninitialized element"))
            // function references call_ref
            || (expected.contains("null function") && (actual.contains("uninitialized element") || actual.contains("null reference")))
            // GC tests say "null $kind reference" but we just say "null reference".
            || (expected.contains("null") && expected.contains("reference") && actual.contains("null reference"))
        {
            return Ok(());
        }
        bail!("expected '{}', got '{}'", expected, actual)
    }

    /// Run a wast script from a byte buffer.
    pub fn run_wast(&mut self, filename: &str, wast: &[u8]) -> Result<()> {
        let wast = str::from_utf8(wast)?;

        let adjust_wast = |mut err: wast::Error| {
            err.set_path(filename.as_ref());
            err.set_text(wast);
            err
        };

        let mut lexer = Lexer::new(wast);
        lexer.allow_confusing_unicode(filename.ends_with("names.wast"));
        let mut buf = ParseBuffer::new_with_lexer(lexer).map_err(adjust_wast)?;
        buf.track_instr_spans(self.generate_dwarf);
        let ast = parser::parse::<wast::Wast>(&buf).map_err(adjust_wast)?;

        let mut ast = json_from_wast::Opts::default()
            .dwarf(self.generate_dwarf)
            .convert(filename, wast, ast)?;
        let modules_by_filename = Arc::get_mut(&mut self.modules_by_filename).unwrap();
        for (name, bytes) in ast.wasms.drain(..) {
            let prev = modules_by_filename.insert(name, bytes);
            assert!(prev.is_none());
        }

        match &self.precompile_save {
            Some(path) => {
                let json_path = path
                    .join(Path::new(filename).file_name().unwrap())
                    .with_extension("json");
                let json = serde_json::to_string(&ast)?;
                std::fs::write(&json_path, json)
                    .with_context(|| format!("failed to write {json_path:?}"))?;
                for (name, bytes) in self.modules_by_filename.iter() {
                    let cwasm_path = path.join(name).with_extension("cwasm");
                    let cwasm = if wasmparser::Parser::is_core_wasm(&bytes) {
                        self.store.engine().precompile_module(bytes)
                    } else {
                        #[cfg(feature = "component-model")]
                        {
                            self.store.engine().precompile_component(bytes)
                        }
                        #[cfg(not(feature = "component-model"))]
                        bail!("component-model support not enabled");
                    };
                    if let Ok(cwasm) = cwasm {
                        std::fs::write(&cwasm_path, cwasm)
                            .with_context(|| format!("failed to write {cwasm_path:?}"))?;
                    }
                }
                Ok(())
            }
            None => self.run_directives(ast.commands, filename),
        }
    }

    fn run_directives(&mut self, directives: Vec<Command<'_>>, filename: &str) -> Result<()> {
        thread::scope(|scope| {
            let mut threads = HashMap::new();
            for directive in directives {
                let line = directive.line();
                log::debug!("running directive on {filename}:{line}");
                self.run_directive(directive, filename, &scope, &mut threads)
                    .with_context(|| format!("failed directive on {filename}:{line}"))?;
            }
            Ok(())
        })
    }

    fn run_directive<'a>(
        &mut self,
        directive: Command<'a>,
        filename: &'a str,
        // wast: &'a str,
        scope: &'a thread::Scope<'a, '_>,
        threads: &mut HashMap<String, thread::ScopedJoinHandle<'a, Result<()>>>,
    ) -> Result<()>
    where
        T: 'a,
    {
        use Command::*;

        match directive {
            Module {
                name,
                file,
                line: _,
            } => {
                let module = self.module_definition(&file)?;
                self.module(name.as_deref(), &module)?;
            }
            ModuleDefinition {
                name,
                file,
                line: _,
            } => {
                let module = self.module_definition(&file)?;
                if let Some(name) = name {
                    self.modules.insert(name.to_string(), module);
                }
            }
            ModuleInstance {
                instance,
                module,
                line: _,
            } => {
                let module = module
                    .as_deref()
                    .and_then(|n| self.modules.get(n))
                    .cloned()
                    .ok_or_else(|| anyhow!("no module named {module:?}"))?;
                self.module(instance.as_deref(), &module)?;
            }
            Register { line: _, name, as_ } => {
                self.register(name.as_deref(), &as_)?;
            }
            Action { action, line: _ } => {
                self.perform_action(&action)?;
            }
            AssertReturn {
                action,
                expected,
                line: _,
            } => {
                let result = self.perform_action(&action)?;
                self.assert_return(result, &expected)?;
            }
            AssertTrap {
                action,
                text,
                line: _,
            } => {
                let result = self.perform_action(&action)?;
                self.assert_trap(result, &text)?;
            }
            AssertUninstantiable {
                file,
                text,
                line: _,
            } => {
                let result = match self.module_definition(&file)? {
                    ModuleKind::Core(module) => self
                        .instantiate_module(&module)?
                        .map(|_| Results::Core(Vec::new())),
                    #[cfg(feature = "component-model")]
                    ModuleKind::Component(component) => self
                        .instantiate_component(&component)?
                        .map(|_| Results::Component(Vec::new())),
                };
                self.assert_trap(result, &text)?;
            }
            AssertExhaustion {
                action,
                text,
                line: _,
            } => {
                let result = self.perform_action(&action)?;
                self.assert_trap(result, &text)?;
            }
            AssertInvalid {
                file,
                text,
                line: _,
            } => {
                let err = match self.module_definition(&file) {
                    Ok(_) => bail!("expected module to fail to build"),
                    Err(e) => e,
                };
                let error_message = format!("{err:?}");
                if !is_matching_assert_invalid_error_message(filename, &text, &error_message) {
                    bail!("assert_invalid: expected \"{text}\", got \"{error_message}\"",)
                }
            }
            AssertMalformed {
                file,
                text: _,
                line: _,
            } => {
                if let Ok(_) = self.module_definition(&file) {
                    bail!("expected malformed module to fail to instantiate");
                }
            }
            AssertUnlinkable {
                file,
                text,
                line: _,
            } => {
                let module = self.module_definition(&file)?;
                let err = match self.module(None, &module) {
                    Ok(_) => bail!("expected module to fail to link"),
                    Err(e) => e,
                };
                let error_message = format!("{err:?}");
                if !error_message.contains(&text[..]) {
                    bail!("assert_unlinkable: expected {text}, got {error_message}",)
                }
            }
            AssertException { .. } => bail!("unimplemented assert_exception"),

            Thread {
                name,
                shared_module,
                commands,
                line: _,
            } => {
                let mut core_linker = Linker::new(self.store.engine());
                if let Some(id) = shared_module {
                    let items = self
                        .core_linker
                        .iter(&mut self.store)
                        .filter(|(module, _, _)| *module == &id[..])
                        .collect::<Vec<_>>();
                    for (module, name, item) in items {
                        core_linker.define(&mut self.store, module, name, item)?;
                    }
                }
                let mut child_cx = WastContext {
                    current: None,
                    core_linker,
                    #[cfg(feature = "component-model")]
                    component_linker: component::Linker::new(self.store.engine()),
                    store: Store::new(self.store.engine(), self.store.data().clone()),
                    modules: self.modules.clone(),
                    async_runtime: self.async_runtime.as_ref().map(|_| {
                        tokio::runtime::Builder::new_current_thread()
                            .build()
                            .unwrap()
                    }),
                    generate_dwarf: self.generate_dwarf,
                    modules_by_filename: self.modules_by_filename.clone(),
                    precompile_load: self.precompile_load.clone(),
                    precompile_save: self.precompile_save.clone(),
                };
                let child = scope.spawn(move || child_cx.run_directives(commands, filename));
                threads.insert(name.to_string(), child);
            }
            Wait { thread, .. } => {
                threads
                    .remove(&thread[..])
                    .ok_or_else(|| anyhow!("no thread named `{thread}`"))?
                    .join()
                    .unwrap()?;
            }

            AssertSuspension { .. } => {
                bail!("unimplemented wast directive");
            }
        }

        Ok(())
    }

    /// Run a wast script from a file.
    pub fn run_file(&mut self, path: &Path) -> Result<()> {
        match &self.precompile_load {
            Some(precompile) => {
                let file = precompile
                    .join(path.file_name().unwrap())
                    .with_extension("json");
                let json = std::fs::read_to_string(&file)
                    .with_context(|| format!("failed to read {file:?}"))?;
                let wast = serde_json::from_str::<json_from_wast::Wast<'_>>(&json)?;
                self.run_directives(wast.commands, &wast.source_filename)
            }
            None => {
                let bytes = std::fs::read(path)
                    .with_context(|| format!("failed to read `{}`", path.display()))?;
                self.run_wast(path.to_str().unwrap(), &bytes)
            }
        }
    }

    /// Whether or not to generate DWARF debugging information in custom
    /// sections in modules being tested.
    pub fn generate_dwarf(&mut self, enable: bool) -> &mut Self {
        self.generate_dwarf = enable;
        self
    }
}

fn is_matching_assert_invalid_error_message(test: &str, expected: &str, actual: &str) -> bool {
    if actual.contains(expected) {
        return true;
    }

    // Historically wasmtime/wasm-tools tried to match the upstream error
    // message. This generally led to a large sequence of matches here which is
    // not easy to maintain and is particularly difficult when test suites and
    // proposals conflict with each other (e.g. one asserts one error message
    // and another asserts a different error message). Overall we didn't benefit
    // a whole lot from trying to match errors so just assume the error is
    // roughly the same and otherwise don't try to match it.
    if test.contains("spec_testsuite") {
        return true;
    }

    // we are in control over all non-spec tests so all the error messages
    // there should exactly match the `assert_invalid` or such
    false
}
