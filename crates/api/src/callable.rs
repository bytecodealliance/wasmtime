use crate::r#ref::HostRef;
use crate::runtime::Store;
use crate::trampoline::{generate_func_export, take_api_trap};
use crate::trap::{Trap, TrapInfo};
use crate::types::FuncType;
use crate::values::Val;
use std::rc::Rc;
use wasmtime_environ::ir;
use wasmtime_jit::InstanceHandle;
use wasmtime_runtime::Export;

/// A trait representing a function that can be imported and called from inside
/// WebAssembly.
/// # Example
/// ```
/// use wasmtime::{HostRef, Val};
///
/// struct TimesTwo;
///
/// impl wasmtime::Callable for TimesTwo {
///     fn call(&self, params: &[Val], results: &mut [Val]) -> Result<(), wasmtime::Trap> {
///         let mut value = params[0].unwrap_i32();
///         value *= 2;
///         results[0] = value.into();
///
///         Ok(())
///     }
/// }
///
/// # fn main () -> Result<(), Box<dyn std::error::Error>> {
/// // Simple module that imports our host function ("times_two") and re-exports
/// // it as "run".
/// let binary = wat::parse_str(r#"
///    (module
///      (func $times_two (import "" "times_two") (param i32) (result i32))
///      (func
///        (export "run")
///        (param i32)
///        (result i32)
///        (local.get 0)
///        (call $times_two))
///    )
/// "#)?;
///
/// // Initialise environment and our module.
/// let engine = HostRef::new(wasmtime::Engine::default());
/// let store = HostRef::new(wasmtime::Store::new(&engine));
/// let module = HostRef::new(wasmtime::Module::new(&store, &binary)?);
///
/// // Define the type of the function we're going to call.
/// let times_two_type = wasmtime::FuncType::new(
///     // Parameters
///     Box::new([wasmtime::ValType::I32]),
///     // Results
///     Box::new([wasmtime::ValType::I32])
/// );
///
/// // Build a reference to the "times_two" function that can be used.
/// let times_two_function = HostRef::new(
///     wasmtime::Func::new(&store, times_two_type, std::rc::Rc::new(TimesTwo))
/// );
///
/// // Create module instance that imports our function
/// let instance = wasmtime::Instance::new(
///     &store,
///     &module,
///     &[times_two_function.into()]
/// )?;
///
/// // Get "run" function from the exports.
/// let run_function = instance.exports()[0].func().unwrap();
///
/// // Borrow and call "run". Returning any error message from Wasm as a string.
/// let original = 5i32;
/// let results = run_function
///     .borrow()
///     .call(&[original.into()])
///     .map_err(|trap| trap.to_string())?;
///
/// // Compare that the results returned matches what we expect.
/// assert_eq!(original * 2, results[0].unwrap_i32());
/// # Ok(())
/// # }
/// ```
pub trait Callable {
    /// What is called when the function is invoked in WebAssembly.
    /// `params` is an immutable list of parameters provided to the function.
    /// `results` is mutable list of results to be potentially set by your
    /// function. Produces a `Trap` if the function encounters any errors.
    fn call(&self, params: &[Val], results: &mut [Val]) -> Result<(), Trap>;
}

pub(crate) trait WrappedCallable {
    fn call(&self, params: &[Val], results: &mut [Val]) -> Result<(), Trap>;
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
    fn call(&self, params: &[Val], results: &mut [Val]) -> Result<(), Trap> {
        use std::cmp::max;
        use std::mem;

        let (vmctx, body, signature) = match self.wasmtime_export() {
            Export::Function {
                vmctx,
                address,
                signature,
            } => (*vmctx, *address, signature.clone()),
            _ => panic!("unexpected export type in Callable"),
        };

        let value_size = mem::size_of::<u128>();
        let mut values_vec = vec![0; max(params.len(), results.len())];

        // Store the argument values into `values_vec`.
        for (index, arg) in params.iter().enumerate() {
            unsafe {
                let ptr = values_vec.as_mut_ptr().add(index);
                arg.write_value_to(ptr);
            }
        }

        // Get the trampoline to call for this function.
        let exec_code_buf = self
            .store
            .borrow_mut()
            .context()
            .compiler()
            .get_published_trampoline(body, &signature, value_size)
            .map_err(|e| Trap::new(format!("trampoline error: {:?}", e)))?;

        // Call the trampoline.
        if let Err(message) = unsafe {
            wasmtime_runtime::wasmtime_call_trampoline(
                vmctx,
                exec_code_buf,
                values_vec.as_mut_ptr() as *mut u8,
            )
        } {
            let trap = take_api_trap()
                .unwrap_or_else(|| HostRef::new(TrapInfo::new(format!("call error: {}", message))));
            return Err(trap.into());
        }

        // Load the return values out of `values_vec`.
        for (index, abi_param) in signature.returns.iter().enumerate() {
            unsafe {
                let ptr = values_vec.as_ptr().add(index);

                results[index] = Val::read_value_from(ptr, abi_param.value_type);
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
    fn call(&self, params: &[Val], results: &mut [Val]) -> Result<(), Trap> {
        self.callable.call(params, results)
    }
    fn wasmtime_handle(&self) -> &InstanceHandle {
        &self.instance
    }
    fn wasmtime_export(&self) -> &Export {
        &self.export
    }
}
