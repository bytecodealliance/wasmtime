use crate::data_structures::ir;
use crate::r#ref::HostRef;
use crate::runtime::Store;
use crate::trampoline::{generate_func_export, take_api_trap};
use crate::trap::Trap;
use crate::types::FuncType;
use crate::values::Val;
use std::rc::Rc;
use wasmtime_jit::InstanceHandle;
use wasmtime_runtime::Export;

pub trait Callable {
    fn call(&self, params: &[Val], results: &mut [Val]) -> Result<(), HostRef<Trap>>;
}

pub(crate) trait WrappedCallable {
    fn call(&self, params: &[Val], results: &mut [Val]) -> Result<(), HostRef<Trap>>;
    fn signature(&self) -> &ir::Signature {
        match self.wasmtime_export() {
            Export::Function { signature, .. } => signature,
            _ => panic!("unexpected export type in Callable"),
        }
    }
    fn wasmtime_handle(&self) -> &InstanceHandle;
    fn wasmtime_export(&self) -> &Export;
}

pub(crate) struct WasmtimeFn {
    store: HostRef<Store>,
    instance: InstanceHandle,
    export: Export,
}

impl WasmtimeFn {
    pub fn new(store: &HostRef<Store>, instance: InstanceHandle, export: Export) -> WasmtimeFn {
        WasmtimeFn {
            store: store.clone(),
            instance,
            export,
        }
    }
}

impl WrappedCallable for WasmtimeFn {
    fn call(&self, params: &[Val], results: &mut [Val]) -> Result<(), HostRef<Trap>> {
        use std::cmp::max;
        use std::{mem, ptr};

        let (vmctx, body, signature) = match self.wasmtime_export() {
            Export::Function {
                vmctx,
                address,
                signature,
            } => (*vmctx, *address, signature.clone()),
            _ => panic!("unexpected export type in Callable"),
        };

        let value_size = mem::size_of::<u64>();
        let mut values_vec: Vec<u64> = vec![0; max(params.len(), results.len())];

        // Store the argument values into `values_vec`.
        for (index, arg) in params.iter().enumerate() {
            unsafe {
                let ptr = values_vec.as_mut_ptr().add(index);

                match arg {
                    Val::I32(x) => ptr::write(ptr as *mut i32, *x),
                    Val::I64(x) => ptr::write(ptr as *mut i64, *x),
                    Val::F32(x) => ptr::write(ptr as *mut u32, *x),
                    Val::F64(x) => ptr::write(ptr as *mut u64, *x),
                    _ => unimplemented!("WasmtimeFn arg"),
                }
            }
        }

        // Get the trampoline to call for this function.
        let exec_code_buf = self
            .store
            .borrow_mut()
            .context()
            .compiler()
            .get_published_trampoline(body, &signature, value_size)
            .map_err(|_| HostRef::new(Trap::fake()))?; //was ActionError::Setup)?;

        // Call the trampoline.
        if let Err(message) = unsafe {
            wasmtime_runtime::wasmtime_call_trampoline(
                vmctx,
                exec_code_buf,
                values_vec.as_mut_ptr() as *mut u8,
            )
        } {
            let trap = take_api_trap().unwrap_or_else(|| HostRef::new(Trap::new(message)));
            return Err(trap);
        }

        // Load the return values out of `values_vec`.
        for (index, abi_param) in signature.returns.iter().enumerate() {
            unsafe {
                let ptr = values_vec.as_ptr().add(index);

                results[index] = match abi_param.value_type {
                    ir::types::I32 => Val::I32(ptr::read(ptr as *const i32)),
                    ir::types::I64 => Val::I64(ptr::read(ptr as *const i64)),
                    ir::types::F32 => Val::F32(ptr::read(ptr as *const u32)),
                    ir::types::F64 => Val::F64(ptr::read(ptr as *const u64)),
                    other => panic!("unsupported value type {:?}", other),
                }
            }
        }

        Ok(())
    }
    fn wasmtime_handle(&self) -> &InstanceHandle {
        &self.instance
    }
    fn wasmtime_export(&self) -> &Export {
        &self.export
    }
}

pub struct NativeCallable {
    callable: Rc<dyn Callable + 'static>,
    instance: InstanceHandle,
    export: Export,
}

impl NativeCallable {
    pub(crate) fn new(
        callable: Rc<dyn Callable + 'static>,
        ft: &FuncType,
        store: &HostRef<Store>,
    ) -> Self {
        let (instance, export) =
            generate_func_export(ft, &callable, store).expect("generated func");
        NativeCallable {
            callable,
            instance,
            export,
        }
    }
}

impl WrappedCallable for NativeCallable {
    fn call(&self, params: &[Val], results: &mut [Val]) -> Result<(), HostRef<Trap>> {
        self.callable.call(params, results)
    }
    fn wasmtime_handle(&self) -> &InstanceHandle {
        &self.instance
    }
    fn wasmtime_export(&self) -> &Export {
        &self.export
    }
}
