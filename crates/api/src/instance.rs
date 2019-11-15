use crate::context::Context;
use crate::externals::Extern;
use crate::module::Module;
use crate::r#ref::HostRef;
use crate::runtime::Store;
use crate::types::{ExportType, ExternType, Name};
use crate::{HashMap, HashSet};
use alloc::string::{String, ToString};
use alloc::{boxed::Box, rc::Rc, vec::Vec};
use anyhow::Result;
use core::cell::RefCell;
use wasmtime_jit::{instantiate, Resolver};
use wasmtime_runtime::{Export, InstanceHandle};

struct SimpleResolver {
    imports: Vec<(String, String, Extern)>,
}

impl Resolver for SimpleResolver {
    fn resolve(&mut self, name: &str, field: &str) -> Option<Export> {
        // TODO speedup lookup
        self.imports
            .iter_mut()
            .find(|(n, f, _)| name == n && field == f)
            .map(|(_, _, e)| e.get_wasmtime_export())
    }
}

pub fn instantiate_in_context(
    data: &[u8],
    imports: Vec<(String, String, Extern)>,
    mut context: Context,
    exports: Rc<RefCell<HashMap<String, Option<wasmtime_runtime::Export>>>>,
) -> Result<(InstanceHandle, HashSet<Context>)> {
    let mut contexts = HashSet::new();
    let debug_info = context.debug_info();
    let mut resolver = SimpleResolver { imports };
    let instance = instantiate(
        &mut context.compiler(),
        data,
        &mut resolver,
        exports,
        debug_info,
    )?;
    contexts.insert(context);
    Ok((instance, contexts))
}

#[derive(Clone)]
pub struct Instance {
    instance_handle: InstanceHandle,

    module: HostRef<Module>,

    // We need to keep CodeMemory alive.
    contexts: HashSet<Context>,

    exports: Box<[Extern]>,
}

impl Instance {
    pub fn new(
        store: &HostRef<Store>,
        module: &HostRef<Module>,
        externs: &[Extern],
    ) -> Result<Instance> {
        let context = store.borrow_mut().context().clone();
        let exports = store.borrow_mut().global_exports().clone();
        let imports = module
            .borrow()
            .imports()
            .iter()
            .zip(externs.iter())
            .map(|(i, e)| (i.module().to_string(), i.name().to_string(), e.clone()))
            .collect::<Vec<_>>();
        let (mut instance_handle, contexts) = instantiate_in_context(
            module.borrow().binary().expect("binary"),
            imports,
            context,
            exports,
        )?;

        let exports = {
            let module = module.borrow();
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

    pub fn module(&self) -> &HostRef<Module> {
        &self.module
    }

    pub fn find_export_by_name(&self, name: &str) -> Option<&Extern> {
        let (i, _) = self
            .module
            .borrow()
            .exports()
            .iter()
            .enumerate()
            .find(|(_, e)| e.name().as_str() == name)?;
        Some(&self.exports()[i])
    }

    pub fn from_handle(store: &HostRef<Store>, instance_handle: InstanceHandle) -> Instance {
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
                let _ = store.borrow_mut().register_cranelift_signature(signature);
            }
            let extern_type = ExternType::from_wasmtime_export(&export);
            exports_types.push(ExportType::new(Name::new(name), extern_type));
            exports.push(Extern::from_wasmtime_export(
                store,
                instance_handle.clone(),
                export.clone(),
            ));
        }

        let module = HostRef::new(Module::from_exports(
            store,
            exports_types.into_boxed_slice(),
        ));

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
