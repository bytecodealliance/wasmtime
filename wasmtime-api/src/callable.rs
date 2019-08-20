use crate::runtime::Store;
use crate::trap::Trap;
use crate::values::Val;
use core::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

use cranelift_codegen::ir;
use wasmtime_runtime::{VMContext, VMFunctionBody};

pub trait Callable: Any {
    fn call(&self, params: &[Val], results: &mut [Val]) -> Result<(), Rc<RefCell<Trap>>>;
}

pub(crate) struct WasmtimeFn {
    store: Rc<RefCell<Store>>,
    signature: ir::Signature,
    body: *const VMFunctionBody,
    vmctx: *mut VMContext,
}

impl WasmtimeFn {
    pub fn new(
        store: Rc<RefCell<Store>>,
        signature: ir::Signature,
        body: *const VMFunctionBody,
        vmctx: *mut VMContext,
    ) -> WasmtimeFn {
        WasmtimeFn {
            store,
            signature,
            body,
            vmctx,
        }
    }
}
impl Callable for WasmtimeFn {
    fn call(&self, params: &[Val], results: &mut [Val]) -> Result<(), Rc<RefCell<Trap>>> {
        use core::cmp::max;
        use core::{mem, ptr};

        let mut store = self.store.borrow_mut();

        let context = store.context();
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
        let exec_code_buf = context
            .compiler()
            .get_published_trampoline(self.body, &self.signature, value_size)
            .map_err(|_| Rc::new(RefCell::new(Trap::fake())))?; //was ActionError::Setup)?;

        // Call the trampoline.
        if let Err(message) = unsafe {
            wasmtime_runtime::wasmtime_call_trampoline(
                self.vmctx,
                exec_code_buf,
                values_vec.as_mut_ptr() as *mut u8,
            )
        } {
            return Err(Rc::new(RefCell::new(Trap::new(message))));
        }

        // Load the return values out of `values_vec`.
        for (index, abi_param) in self.signature.returns.iter().enumerate() {
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
}
