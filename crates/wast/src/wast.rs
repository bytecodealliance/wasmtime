#[cfg(feature = "component-model")]
use crate::component;
use crate::core;
use crate::spectest::*;
use anyhow::{anyhow, bail, Context as _};
use std::collections::HashMap;
use std::path::Path;
use std::str;
use std::thread;
use wasmtime::*;
use wast::lexer::Lexer;
use wast::parser::{self, ParseBuffer};
use wast::{QuoteWat, Wast, WastArg, WastDirective, WastExecute, WastInvoke, WastRet, Wat};

/// The wast test script language allows modules to be defined and actions
/// to be performed on them.
pub struct WastContext<T> {
    /// Wast files have a concept of a "current" module, which is the most
    /// recently defined.
    current: Option<InstanceKind>,
    core_linker: Linker<T>,
    modules: HashMap<String, ModuleKind>,
    #[cfg(feature = "component-model")]
    component_linker: component::Linker<T>,
    store: Store<T>,
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

impl<T> WastContext<T>
where
    T: Clone + Send + 'static,
{
    /// Construct a new instance of `WastContext`.
    pub fn new(store: Store<T>) -> Self {
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
        }
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
        Ok(
            match self.core_linker.instantiate(&mut self.store, &module) {
                Ok(i) => Outcome::Ok(i),
                Err(e) => Outcome::Trap(e),
            },
        )
    }

    #[cfg(feature = "component-model")]
    fn instantiate_component(
        &mut self,
        component: &component::Component,
    ) -> Result<Outcome<(component::Component, component::Instance)>> {
        Ok(
            match self
                .component_linker
                .instantiate(&mut self.store, &component)
            {
                Ok(i) => Outcome::Ok((component.clone(), i)),
                Err(e) => Outcome::Trap(e),
            },
        )
    }

    /// Register "spectest" which is used by the spec testsuite.
    pub fn register_spectest(&mut self, config: &SpectestConfig) -> Result<()> {
        link_spectest(&mut self.core_linker, &mut self.store, config)?;
        #[cfg(feature = "component-model")]
        link_component_spectest(&mut self.component_linker)?;
        Ok(())
    }

    /// Perform the action portion of a command.
    fn perform_execute(&mut self, exec: WastExecute<'_>) -> Result<Outcome> {
        match exec {
            WastExecute::Invoke(invoke) => self.perform_invoke(invoke),
            WastExecute::Wat(module) => {
                Ok(match self.module_definition(QuoteWat::Wat(module))? {
                    (_, ModuleKind::Core(module)) => self
                        .instantiate_module(&module)?
                        .map(|_| Results::Core(Vec::new())),
                    #[cfg(feature = "component-model")]
                    (_, ModuleKind::Component(component)) => self
                        .instantiate_component(&component)?
                        .map(|_| Results::Component(Vec::new())),
                })
            }
            WastExecute::Get { module, global, .. } => self.get(module.map(|s| s.name()), global),
        }
    }

    fn perform_invoke(&mut self, exec: WastInvoke<'_>) -> Result<Outcome> {
        match self.get_export(exec.module.map(|i| i.name()), exec.name)? {
            Export::Core(export) => {
                let func = export
                    .into_func()
                    .ok_or_else(|| anyhow!("no function named `{}`", exec.name))?;
                let values = exec
                    .args
                    .iter()
                    .map(|v| match v {
                        WastArg::Core(v) => core::val(&mut self.store, v),
                        WastArg::Component(_) => bail!("expected component function, found core"),
                    })
                    .collect::<Result<Vec<_>>>()?;

                let mut results = vec![Val::null_func_ref(); func.ty(&self.store).results().len()];
                Ok(match func.call(&mut self.store, &values, &mut results) {
                    Ok(()) => Outcome::Ok(Results::Core(results.into())),
                    Err(e) => Outcome::Trap(e),
                })
            }
            #[cfg(feature = "component-model")]
            Export::Component(func) => {
                let values = exec
                    .args
                    .iter()
                    .map(|v| match v {
                        WastArg::Component(v) => component::val(v),
                        WastArg::Core(_) => bail!("expected core function, found component"),
                    })
                    .collect::<Result<Vec<_>>>()?;

                let mut results =
                    vec![component::Val::Bool(false); func.results(&self.store).len()];
                Ok(match func.call(&mut self.store, &values, &mut results) {
                    Ok(()) => {
                        func.post_return(&mut self.store)?;
                        Outcome::Ok(Results::Component(results.into()))
                    }
                    Err(e) => Outcome::Trap(e),
                })
            }
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
    fn module_definition<'a>(
        &mut self,
        mut wat: QuoteWat<'a>,
    ) -> Result<(Option<&'a str>, ModuleKind)> {
        let (is_module, name) = match &wat {
            QuoteWat::Wat(Wat::Module(m)) => (true, m.id),
            QuoteWat::QuoteModule(..) => (true, None),
            QuoteWat::Wat(Wat::Component(m)) => (false, m.id),
            QuoteWat::QuoteComponent(..) => (false, None),
        };
        let bytes = wat.encode()?;
        if is_module {
            let module = Module::new(self.store.engine(), &bytes)?;
            Ok((name.map(|n| n.name()), ModuleKind::Core(module)))
        } else {
            #[cfg(feature = "component-model")]
            {
                let component = component::Component::new(self.store.engine(), &bytes)?;
                Ok((name.map(|n| n.name()), ModuleKind::Component(component)))
            }
            #[cfg(not(feature = "component-model"))]
            bail!("component-model support not enabled");
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
        Ok(Outcome::Ok(Results::Core(
            vec![global.get(&mut self.store)],
        )))
    }

    fn assert_return(&self, result: Outcome, results: &[WastRet<'_>]) -> Result<()> {
        match result.into_result()? {
            Results::Core(values) => {
                if values.len() != results.len() {
                    bail!("expected {} results found {}", results.len(), values.len());
                }
                for (i, (v, e)) in values.iter().zip(results).enumerate() {
                    let e = match e {
                        WastRet::Core(core) => core,
                        WastRet::Component(_) => {
                            bail!("expected component value found core value")
                        }
                    };
                    core::match_val(&self.store, v, e)
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
                        WastRet::Core(_) => {
                            bail!("expected component value found core value")
                        }
                        WastRet::Component(val) => val,
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
        {
            return Ok(());
        }
        bail!("expected '{}', got '{}'", expected, actual)
    }

    /// Run a wast script from a byte buffer.
    pub fn run_buffer(&mut self, filename: &str, wast: &[u8]) -> Result<()> {
        let wast = str::from_utf8(wast)?;

        let adjust_wast = |mut err: wast::Error| {
            err.set_path(filename.as_ref());
            err.set_text(wast);
            err
        };

        let mut lexer = Lexer::new(wast);
        lexer.allow_confusing_unicode(filename.ends_with("names.wast"));
        let buf = ParseBuffer::new_with_lexer(lexer).map_err(adjust_wast)?;
        let ast = parser::parse::<Wast>(&buf).map_err(adjust_wast)?;

        self.run_directives(ast.directives, filename, wast)
    }

    fn run_directives(
        &mut self,
        directives: Vec<WastDirective<'_>>,
        filename: &str,
        wast: &str,
    ) -> Result<()> {
        let adjust_wast = |mut err: wast::Error| {
            err.set_path(filename.as_ref());
            err.set_text(wast);
            err
        };

        thread::scope(|scope| {
            let mut threads = HashMap::new();
            for directive in directives {
                let sp = directive.span();
                if log::log_enabled!(log::Level::Debug) {
                    let (line, col) = sp.linecol_in(wast);
                    log::debug!("running directive on {}:{}:{}", filename, line + 1, col);
                }
                self.run_directive(directive, filename, wast, &scope, &mut threads)
                    .map_err(|e| match e.downcast() {
                        Ok(err) => adjust_wast(err).into(),
                        Err(e) => e,
                    })
                    .with_context(|| {
                        let (line, col) = sp.linecol_in(wast);
                        format!("failed directive on {}:{}:{}", filename, line + 1, col)
                    })?;
            }
            Ok(())
        })
    }

    fn run_directive<'a>(
        &mut self,
        directive: WastDirective<'a>,
        filename: &'a str,
        wast: &'a str,
        scope: &'a thread::Scope<'a, '_>,
        threads: &mut HashMap<&'a str, thread::ScopedJoinHandle<'a, Result<()>>>,
    ) -> Result<()>
    where
        T: 'a,
    {
        use wast::WastDirective::*;

        match directive {
            Module(module) => {
                let (name, module) = self.module_definition(module)?;
                self.module(name, &module)?;
            }
            ModuleDefinition(module) => {
                let (name, module) = self.module_definition(module)?;
                if let Some(name) = name {
                    self.modules.insert(name.to_string(), module.clone());
                }
            }
            ModuleInstance {
                instance,
                module,
                span: _,
            } => {
                let module = module
                    .and_then(|n| self.modules.get(n.name()))
                    .cloned()
                    .ok_or_else(|| anyhow!("no module named {module:?}"))?;
                self.module(instance.map(|n| n.name()), &module)?;
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
                module,
                message,
            } => {
                let err = match self.module_definition(module) {
                    Ok(_) => bail!("expected module to fail to build"),
                    Err(e) => e,
                };
                let error_message = format!("{err:?}");
                if !is_matching_assert_invalid_error_message(&message, &error_message) {
                    bail!(
                        "assert_invalid: expected \"{}\", got \"{}\"",
                        message,
                        error_message
                    )
                }
            }
            AssertMalformed {
                module,
                span: _,
                message: _,
            } => {
                if let Ok(_) = self.module_definition(module) {
                    bail!("expected malformed module to fail to instantiate");
                }
            }
            AssertUnlinkable {
                span: _,
                module,
                message,
            } => {
                let (name, module) = self.module_definition(QuoteWat::Wat(module))?;
                let err = match self.module(name, &module) {
                    Ok(_) => bail!("expected module to fail to link"),
                    Err(e) => e,
                };
                let error_message = format!("{err:?}");
                if !error_message.contains(&message) {
                    bail!(
                        "assert_unlinkable: expected {}, got {}",
                        message,
                        error_message
                    )
                }
            }
            AssertException { .. } => bail!("unimplemented assert_exception"),

            Thread(thread) => {
                let mut core_linker = Linker::new(self.store.engine());
                if let Some(id) = thread.shared_module {
                    let items = self
                        .core_linker
                        .iter(&mut self.store)
                        .filter(|(module, _, _)| *module == id.name())
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
                };
                let name = thread.name.name();
                let child =
                    scope.spawn(move || child_cx.run_directives(thread.directives, filename, wast));
                threads.insert(name, child);
            }

            Wait { thread, .. } => {
                let name = thread.name();
                threads
                    .remove(name)
                    .ok_or_else(|| anyhow!("no thread named `{name}`"))?
                    .join()
                    .unwrap()?;
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

fn is_matching_assert_invalid_error_message(expected: &str, actual: &str) -> bool {
    actual.contains(expected)
        // slight difference in error messages
        || (expected.contains("unknown elem segment") && actual.contains("unknown element segment"))
        // The same test here is asserted to have one error message in
        // `memory.wast` and a different error message in
        // `memory64/memory.wast`, so we equate these two error messages to get
        // the memory64 tests to pass.
        || (expected.contains("memory size must be at most 65536 pages") && actual.contains("invalid u32 number"))
        // the spec test suite asserts a different error message than we print
        // for this scenario
        || (expected == "unknown global" && actual.contains("global.get of locally defined global"))
        || (expected == "immutable global" && actual.contains("global is immutable: cannot modify it with `global.set`"))
        || (expected == "table size must be at most 2^32-1" && actual.contains("invalid u32 number: constant out of range"))
}
