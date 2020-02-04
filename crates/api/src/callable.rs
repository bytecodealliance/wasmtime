use crate::runtime::Store;
use crate::trampoline::generate_func_export;
use crate::trap::Trap;
use crate::types::FuncType;
use crate::values::Val;
use std::ptr;
use std::rc::Rc;
use wasmtime_environ::ir;
use wasmtime_jit::InstanceHandle;
use wasmtime_runtime::Export;

/// A trait representing a function that can be imported and called from inside
/// WebAssembly.
/// # Example
/// ```
/// use wasmtime::Val;
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
/// let wat = r#"
///    (module
///      (func $times_two (import "" "times_two") (param i32) (result i32))
///      (func
///        (export "run")
///        (param i32)
///        (result i32)
///        (local.get 0)
///        (call $times_two))
///    )
/// "#;
///
/// // Initialise environment and our module.
/// let store = wasmtime::Store::default();
/// let module = wasmtime::Module::new(&store, wat)?;
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
/// let times_two_function =
///     wasmtime::Func::new(&store, times_two_type, std::rc::Rc::new(TimesTwo));
///
/// // Create module instance that imports our function
/// let instance = wasmtime::Instance::new(
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
    store: Store,
    instance: InstanceHandle,
    export: Export,
}

impl WasmtimeFn {
    pub fn new(store: &Store, instance: InstanceHandle, export: Export) -> WasmtimeFn {
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
        if signature.params.len() - 2 != params.len() {
            return Err(Trap::new(format!(
                "expected {} arguments, got {}",
                signature.params.len() - 2,
                params.len()
            )));
        }
        if signature.returns.len() != results.len() {
            return Err(Trap::new(format!(
                "expected {} results, got {}",
                signature.returns.len(),
                results.len()
            )));
        }

        let value_size = mem::size_of::<u128>();
        let mut values_vec = vec![0; max(params.len(), results.len())];

        // Store the argument values into `values_vec`.
        let param_tys = signature.params.iter().skip(2);
        for ((arg, slot), ty) in params.iter().zip(&mut values_vec).zip(param_tys) {
            if arg.ty().get_wasmtime_type() != Some(ty.value_type) {
                return Err(Trap::new("argument type mismatch"));
            }
            unsafe {
                arg.write_value_to(slot);
            }
        }

        // Get the trampoline to call for this function.
        let exec_code_buf = self
            .store
            .compiler_mut()
            .get_published_trampoline(body, &signature, value_size)
            .map_err(|e| Trap::new(format!("trampoline error: {:?}", e)))?;

        // Call the trampoline.
        if let Err(error) = unsafe {
            wasmtime_runtime::wasmtime_call_trampoline(
                vmctx,
                ptr::null_mut(),
                exec_code_buf,
                values_vec.as_mut_ptr() as *mut u8,
            )
        } {
            return Err(Trap::from_jit(error));
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
    pub(crate) fn new(callable: Rc<dyn Callable + 'static>, ft: &FuncType, store: &Store) -> Self {
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
