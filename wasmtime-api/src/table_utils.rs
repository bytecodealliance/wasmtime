use std::cell::RefCell;
use std::mem;
use std::ptr;
use std::rc::Rc;
use std::slice;
use wasmtime_runtime::{
    InstanceHandle, VMCallerCheckedAnyfunc, VMSharedSignatureIndex, VMTableDefinition,
};

use crate::callable::WasmtimeFn;
use crate::runtime::SignatureRegistry;
use crate::runtime::Store;
use crate::types::TableType;
use crate::values::{AnyRef, FuncRef, Val};

fn into_checked_anyfunc(val: Val, store: &Rc<RefCell<Store>>) -> VMCallerCheckedAnyfunc {
    match val {
        Val::AnyRef(AnyRef::Null) => VMCallerCheckedAnyfunc {
            func_ptr: ptr::null(),
            type_index: VMSharedSignatureIndex::default(),
            vmctx: ptr::null_mut(),
        },
        Val::AnyRef(AnyRef::Func(f)) | Val::FuncRef(f) => {
            let (vmctx, func_ptr, signature) = match f.0.wasmtime_export() {
                wasmtime_runtime::Export::Function {
                    vmctx,
                    address,
                    signature,
                } => (*vmctx, *address, signature),
                _ => panic!("expected function export"),
            };
            let type_index = store.borrow_mut().register_cranelift_signature(signature);
            VMCallerCheckedAnyfunc {
                func_ptr,
                type_index,
                vmctx,
            }
        }
        _ => panic!("val is not funcref"),
    }
}

unsafe fn from_checked_anyfunc(item: &VMCallerCheckedAnyfunc, store: &Rc<RefCell<Store>>) -> Val {
    if item.type_index == VMSharedSignatureIndex::default() {
        return Val::AnyRef(AnyRef::Null);
    }
    let signature = store
        .borrow()
        .lookup_cranelift_signature(item.type_index)
        .expect("signature")
        .clone();
    let instance_handle = InstanceHandle::from_vmctx(item.vmctx);
    let export = wasmtime_runtime::Export::Function {
        address: item.func_ptr,
        signature,
        vmctx: item.vmctx,
    };
    let f = WasmtimeFn::new(store.clone(), instance_handle, export);
    Val::FuncRef(FuncRef(Rc::new(f)))
}

pub unsafe fn get_item(
    table: *mut VMTableDefinition,
    store: &Rc<RefCell<Store>>,
    index: u32,
) -> Val {
    let base = slice::from_raw_parts(
        (*table).base as *const VMCallerCheckedAnyfunc,
        (*table).current_elements,
    );

    from_checked_anyfunc(&base[index as usize], store)
}

pub unsafe fn set_item(
    table: *mut VMTableDefinition,
    store: &Rc<RefCell<Store>>,
    index: u32,
    val: Val,
) -> bool {
    let base = slice::from_raw_parts_mut(
        (*table).base as *mut VMCallerCheckedAnyfunc,
        (*table).current_elements,
    );
    if index as usize >= base.len() {
        return false;
    }

    base[index as usize] = into_checked_anyfunc(val, store);
    true
}

pub unsafe fn get_size(table: *mut VMTableDefinition) -> u32 {
    (*table).current_elements as u32
}

pub unsafe fn grow_table(
    table: *mut VMTableDefinition,
    table_type: &TableType,
    store: &Rc<RefCell<Store>>,
    delta: u32,
    init: Val,
) -> bool {
    let new_len = (*table).current_elements + delta as usize;
    if (table_type.limits().max() as usize) < new_len {
        return false;
    }

    let mut buffer = Vec::from_raw_parts(
        (*table).base as *mut VMCallerCheckedAnyfunc,
        (*table).current_elements,
        (*table).current_elements,
    );
    buffer.resize(new_len, into_checked_anyfunc(init, store));
    buffer.shrink_to_fit();
    assert!(buffer.capacity() == new_len);

    (*table).base = buffer.as_mut_ptr() as *mut u8;
    (*table).current_elements = new_len;
    mem::forget(buffer);

    true
}
