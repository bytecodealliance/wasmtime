use crate::externals::Extern;
use crate::module::Module;
use crate::runtime::Store;
use crate::trampoline::take_api_trap;
use crate::trap::Trap;
use crate::types::{ExportType, ExternType};
use anyhow::{Error, Result};
use wasmtime_jit::{CompiledModule, Resolver};
use wasmtime_runtime::{Export, InstanceHandle, InstantiationError};

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
    compiled_module: &CompiledModule,
    imports: &[Extern],
) -> Result<InstanceHandle, Error> {
    let mut resolver = SimpleResolver { imports };
    unsafe {
        let instance = compiled_module
            .instantiate(&mut resolver)
            .map_err(|e| -> Error {
                if let Some(trap) = take_api_trap() {
                    trap.into()
                } else if let InstantiationError::StartTrap(trap) = e {
                    Trap::from_jit(trap).into()
                } else {
                    e.into()
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
    pub fn new(module: &Module, imports: &[Extern]) -> Result<Instance, Error> {
        let store = module.store();
        let instance_handle = instantiate(module.compiled_module(), imports)?;

        let exports = {
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
            exports.into_boxed_slice()
        };
        module.register_names();
        Ok(Instance {
            instance_handle,
            module: module.clone(),
            exports,
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
    /// [`ExportType`] contains the name of each export.
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
    pub fn from_handle(store: &Store, instance_handle: InstanceHandle) -> Instance {
        let mut exports = Vec::new();
        let mut exports_types = Vec::new();
        for (name, _) in instance_handle.exports() {
            let export = instance_handle.lookup(name).expect("export");
            if let wasmtime_runtime::Export::Function { signature, .. } = &export {
                // HACK ensure all handles, instantiated outside Store, present in
                // the store's SignatureRegistry, e.g. WASI instances that are
                // imported into this store using the from_handle() method.
                store.compiler().signatures().register(signature);
            }

            // We should support everything supported by wasmtime_runtime, or
            // otherwise we've got a bug in this crate, so panic if anything
            // fails to convert here.
            let extern_type = match ExternType::from_wasmtime_export(&export) {
                Some(ty) => ty,
                None => panic!("unsupported core wasm external type {:?}", export),
            };
            exports_types.push(ExportType::new(name, extern_type));
            exports.push(Extern::from_wasmtime_export(
                store,
                instance_handle.clone(),
                export.clone(),
            ));
        }

        let module = Module::from_exports(store, exports_types.into_boxed_slice());

        Instance {
            instance_handle,
            module,
            exports: exports.into_boxed_slice(),
        }
    }

    #[doc(hidden)]
    pub fn handle(&self) -> &InstanceHandle {
        &self.instance_handle
    }

    #[doc(hidden)]
    pub fn get_wasmtime_memory(&self) -> Option<wasmtime_runtime::Export> {
        self.instance_handle.lookup("memory")
    }
}
