use crate::externals::Extern;
use crate::module::Module;
use crate::runtime::{Config, Store};
use crate::trap::Trap;
use anyhow::{bail, Error, Result};
use wasmtime_jit::{CompiledModule, Resolver};
use wasmtime_runtime::{Export, InstanceHandle, InstantiationError, SignatureRegistry};

struct SimpleResolver<'a> {
    imports: &'a [Extern],
}

impl Resolver for SimpleResolver<'_> {
    fn resolve(&mut self, idx: u32, _name: &str, _field: &str) -> Option<Export> {
        self.imports
            .get(idx as usize)
            .map(|i| i.get_wasmtime_export())
    }
}

fn instantiate(
    config: &Config,
    compiled_module: &CompiledModule,
    imports: &[Extern],
    sig_registry: &SignatureRegistry,
) -> Result<InstanceHandle, Error> {
    let mut resolver = SimpleResolver { imports };
    unsafe {
        let instance = compiled_module
            .instantiate(
                config.validating_config.operator_config.enable_bulk_memory,
                &mut resolver,
                sig_registry,
            )
            .map_err(|e| -> Error {
                match e {
                    InstantiationError::StartTrap(trap) | InstantiationError::Trap(trap) => {
                        Trap::from_jit(trap).into()
                    }
                    other => other.into(),
                }
            })?;
        Ok(instance)
    }
}

/// An instantiated WebAssembly module.
///
/// This type represents the instantiation of a [`Module`]. Once instantiated
/// you can access the [`exports`](Instance::exports) which are of type
/// [`Extern`] and provide the ability to call functions, set globals, read
/// memory, etc. This is where all the fun stuff happens!
///
/// An [`Instance`] is created from two inputs, a [`Module`] and a list of
/// imports, provided as a list of [`Extern`] values. The [`Module`] is the wasm
/// code that was compiled and we're instantiating, and the [`Extern`] imports
/// are how we're satisfying the imports of the module provided. On successful
/// instantiation an [`Instance`] will automatically invoke the wasm `start`
/// function.
///
/// When interacting with any wasm code you'll want to make an [`Instance`] to
/// call any code or execute anything!
#[derive(Clone)]
pub struct Instance {
    pub(crate) instance_handle: InstanceHandle,
    module: Module,
    exports: Box<[Extern]>,
}

impl Instance {
    /// Creates a new [`Instance`] from the previously compiled [`Module`] and
    /// list of `imports` specified.
    ///
    /// This method instantiates the `module` provided with the `imports`,
    /// following the procedure in the [core specification][inst] to
    /// instantiate. Instantiation can fail for a number of reasons (many
    /// specified below), but if successful the `start` function will be
    /// automatically run (if provided) and then the [`Instance`] will be
    /// returned.
    ///
    /// ## Providing Imports
    ///
    /// The `imports` array here is a bit tricky. The entries in the list of
    /// `imports` are intended to correspond 1:1 with the list of imports
    /// returned by [`Module::imports`]. Before calling [`Instance::new`] you'll
    /// want to inspect the return value of [`Module::imports`] and, for each
    /// import type, create an [`Extern`] which corresponds to that type.
    /// These [`Extern`] values are all then collected into a list and passed to
    /// this function.
    ///
    /// Note that this function is intentionally relatively low level. It is the
    /// intention that we'll soon provide a [higher level API][issue] which will
    /// be much more ergonomic for instantiating modules. If you need the full
    /// power of customization of imports, though, this is the method for you!
    ///
    /// ## Errors
    ///
    /// This function can fail for a number of reasons, including, but not
    /// limited to:
    ///
    /// * The number of `imports` provided doesn't match the number of imports
    ///   returned by the `module`'s [`Module::imports`] method.
    /// * The type of any [`Extern`] doesn't match the corresponding
    ///   [`ExternType`] entry that it maps to.
    /// * The `start` function in the instance, if present, traps.
    /// * Module/instance resource limits are exceeded.
    ///
    /// When instantiation fails it's recommended to inspect the return value to
    /// see why it failed, or bubble it upwards. If you'd like to specifically
    /// check for trap errors, you can use `error.downcast::<Trap>()`.
    ///
    /// [inst]: https://webassembly.github.io/spec/core/exec/modules.html#exec-instantiation
    /// [issue]: https://github.com/bytecodealliance/wasmtime/issues/727
    /// [`ExternType`]: crate::ExternType
    pub fn new(module: &Module, imports: &[Extern]) -> Result<Instance, Error> {
        let store = module.store();

        // For now we have a restriction that the `Store` that we're working
        // with is the same for everything involved here.
        for import in imports {
            if !import.comes_from_same_store(store) {
                bail!("cross-`Store` instantiation is not currently supported");
            }
        }

        let config = store.engine().config();
        let instance_handle = instantiate(
            config,
            module.compiled_module(),
            imports,
            store.compiler().signatures(),
        )?;

        let mut exports = Vec::with_capacity(module.exports().len());
        for export in module.exports() {
            let name = export.name().to_string();
            let export = instance_handle.lookup(&name).expect("export");
            exports.push(Extern::from_wasmtime_export(
                store,
                instance_handle.clone(),
                export,
            ));
        }
        module.register_frame_info();
        Ok(Instance {
            instance_handle,
            module: module.clone(),
            exports: exports.into_boxed_slice(),
        })
    }

    /// Returns the associated [`Store`] that this `Instance` is compiled into.
    ///
    /// This is the [`Store`] that generally serves as a sort of global cache
    /// for various instance-related things.
    pub fn store(&self) -> &Store {
        self.module.store()
    }

    /// Returns the associated [`Module`] that this `Instance` instantiated.
    ///
    /// The corresponding [`Module`] here is a static version of this `Instance`
    /// which can be used to learn information such as naming information about
    /// various functions.
    pub fn module(&self) -> &Module {
        &self.module
    }

    /// Returns the list of exported items from this [`Instance`].
    ///
    /// Note that the exports here do not have names associated with them,
    /// they're simply the values that are exported. To learn the value of each
    /// export you'll need to consult [`Module::exports`]. The list returned
    /// here maps 1:1 with the list that [`Module::exports`] returns, and
    /// [`ExportType`](crate::ExportType) contains the name of each export.
    pub fn exports(&self) -> &[Extern] {
        &self.exports
    }

    /// Looks up an exported [`Extern`] value by name.
    ///
    /// This method will search the module for an export named `name` and return
    /// the value, if found.
    ///
    /// Returns `None` if there was no export named `name`.
    pub fn get_export(&self, name: &str) -> Option<&Extern> {
        let (i, _) = self
            .module
            .exports()
            .iter()
            .enumerate()
            .find(|(_, e)| e.name() == name)?;
        Some(&self.exports()[i])
    }

    #[doc(hidden)]
    pub fn handle(&self) -> &InstanceHandle {
        &self.instance_handle
    }
}
