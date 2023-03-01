use wasmtime::{ValType, RefType, HeapType};

pub const EXTERN_REF: RefType = RefType {
    nullable: true,
    heap_type: HeapType::Extern,
};
pub const FUNC_REF: RefType = RefType {
    nullable: true,
    heap_type: HeapType::Func,
};

pub fn valtype_eq(x: &ValType, y: &ValType) -> bool {
    ValType::is_subtype(x, y) && ValType::is_subtype(y, x)
}

pub fn pointwise_eq(ts1: Vec<ValType>, ts2: Vec<ValType>) -> bool {
    if ts1.len() != ts2.len() {
        return false;
    }

    for (t1, t2) in ts1.iter().zip(ts2.iter()) {
        if !valtype_eq(t1, t2) {
            return false;
        }
    }

    return true;
}
