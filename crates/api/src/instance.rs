use crate::context::Context;
use crate::externals::Extern;
use crate::module::Module;
use crate::runtime::Store;
use crate::trampoline::take_api_trap;
use crate::trap::Trap;
use crate::types::{ExportType, ExternType};
use anyhow::{Error, Result};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use wasmtime_jit::{instantiate, Resolver, SetupError};
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

pub fn instantiate_in_context(
    data: &[u8],
    imports: &[Extern],
    module_name: Option<String>,
    context: Context,
    exports: Rc<RefCell<HashMap<String, Option<wasmtime_runtime::Export>>>>,
) -> Result<(InstanceHandle, HashSet<Context>), Error> {
    let mut contexts = HashSet::new();
    let debug_info = context.debug_info();
    let mut resolver = SimpleResolver { imports };
    let instance = instantiate(
        &mut context.compiler(),
        data,
        module_name,
        &mut resolver,
        exports,
        debug_info,
    )
    .map_err(|e| -> Error {
        if let Some(trap) = take_api_trap() {
            trap.into()
        } else if let SetupError::Instantiate(InstantiationError::StartTrap(msg)) = e {
            Trap::new(msg).into()
        } else {
            e.into()
        }
    })?;
    contexts.insert(context);
    Ok((instance, contexts))
}

#[derive(Clone)]
pub struct Instance {
    instance_handle: InstanceHandle,

    module: Module,

    // We need to keep CodeMemory alive.
    contexts: HashSet<Context>,

    exports: Box<[Extern]>,
}

impl Instance {
    pub fn new(store: &Store, module: &Module, externs: &[Extern]) -> Result<Instance, Error> {
        let context = store.context().clone();
        let exports = store.global_exports().clone();
        let (mut instance_handle, contexts) = instantiate_in_context(
            module.binary().expect("binary"),
            externs,
            module.name().cloned(),
            context,
            exports,
        )?;

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
        Ok(Instance {
            instance_handle,
            module: module.clone(),
            contexts,
            exports,
        })
    }

    pub fn exports(&self) -> &[Extern] {
        &self.exports
    }

    pub fn module(&self) -> &Module {
        &self.module
    }

    pub fn find_export_by_name(&self, name: &str) -> Option<&Extern> {
        let (i, _) = self
            .module
            .exports()
            .iter()
            .enumerate()
            .find(|(_, e)| e.name() == name)?;
        Some(&self.exports()[i])
    }

    pub fn from_handle(store: &Store, instance_handle: InstanceHandle) -> Instance {
        let contexts = HashSet::new();

        let mut exports = Vec::new();
        let mut exports_types = Vec::new();
        let mut mutable = instance_handle.clone();
        for (name, _) in instance_handle.clone().exports() {
            let export = mutable.lookup(name).expect("export");
            if let wasmtime_runtime::Export::Function { signature, .. } = &export {
                // HACK ensure all handles, instantiated outside Store, present in
                // the store's SignatureRegistry, e.g. WASI instances that are
                // imported into this store using the from_handle() method.
                let _ = store.register_wasmtime_signature(signature);
            }
            let extern_type = ExternType::from_wasmtime_export(&export);
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
            contexts,
            exports: exports.into_boxed_slice(),
        }
    }

    pub fn handle(&self) -> &InstanceHandle {
        &self.instance_handle
    }

    pub fn get_wasmtime_memory(&self) -> Option<wasmtime_runtime::Export> {
        let mut instance_handle = self.instance_handle.clone();
        instance_handle.lookup("memory")
    }
}

// OS-specific signal handling
cfg_if::cfg_if! {
    if #[cfg(target_os = "linux")] {
        impl Instance {
            /// The signal handler must be
            /// [async-signal-safe](http://man7.org/linux/man-pages/man7/signal-safety.7.html).
            pub fn set_signal_handler<H>(&self, handler: H)
            where
                H: 'static + Fn(libc::c_int, *const libc::siginfo_t, *const libc::c_void) -> bool,
            {
                self.instance_handle.clone().set_signal_handler(handler);
            }
        }
    } else if #[cfg(target_os = "windows")] {
        impl Instance {
            pub fn set_signal_handler<H>(&self, handler: H)
            where
                H: 'static + Fn(winapi::um::winnt::EXCEPTION_POINTERS) -> bool,
            {
                self.instance_handle.clone().set_signal_handler(handler);
            }
        }
    } else if #[cfg(target_os = "macos")] {
        impl Instance {
            pub fn set_signal_handler<H>(&self, handler: H)
            where
                H: 'static + Fn(libc::c_int, *const libc::siginfo_t, *const libc::c_void) -> bool,
            {
                self.instance_handle.clone().set_signal_handler(handler);
            }
        }
    }
}
