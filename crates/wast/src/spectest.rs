use std::collections::HashMap;
use wasmtime::*;

/// Return an instance implementing the "spectest" interface used in the
/// spec testsuite.
pub fn instantiate_spectest(store: &Store) -> HashMap<&'static str, Extern> {
    let mut ret = HashMap::new();

    let func = Func::wrap(store, || {});
    ret.insert("print", Extern::Func(func));

    let func = Func::wrap(store, |val: i32| println!("{}: i32", val));
    ret.insert("print_i32", Extern::Func(func));

    let func = Func::wrap(store, |val: i64| println!("{}: i64", val));
    ret.insert("print_i64", Extern::Func(func));

    let func = Func::wrap(store, |val: f32| println!("{}: f32", val));
    ret.insert("print_f32", Extern::Func(func));

    let func = Func::wrap(store, |val: f64| println!("{}: f64", val));
    ret.insert("print_f64", Extern::Func(func));

    let func = Func::wrap(store, |i: i32, f: f32| {
        println!("{}: i32", i);
        println!("{}: f32", f);
    });
    ret.insert("print_i32_f32", Extern::Func(func));

    let func = Func::wrap(store, |f1: f64, f2: f64| {
        println!("{}: f64", f1);
        println!("{}: f64", f2);
    });
    ret.insert("print_f64_f64", Extern::Func(func));

    let ty = GlobalType::new(ValType::I32, Mutability::Const);
    let g = Global::new(store, ty, Val::I32(666)).unwrap();
    ret.insert("global_i32", Extern::Global(g));

    let ty = GlobalType::new(ValType::I64, Mutability::Const);
    let g = Global::new(store, ty, Val::I64(666)).unwrap();
    ret.insert("global_i64", Extern::Global(g));

    let ty = GlobalType::new(ValType::F32, Mutability::Const);
    let g = Global::new(store, ty, Val::F32(0x4426_8000)).unwrap();
    ret.insert("global_f32", Extern::Global(g));

    let ty = GlobalType::new(ValType::F64, Mutability::Const);
    let g = Global::new(store, ty, Val::F64(0x4084_d000_0000_0000)).unwrap();
    ret.insert("global_f64", Extern::Global(g));

    let ty = TableType::new(ValType::FuncRef, Limits::new(10, Some(20)));
    let table = Table::new(store, ty, Val::AnyRef(AnyRef::Null)).unwrap();
    ret.insert("table", Extern::Table(table));

    let ty = MemoryType::new(Limits::new(1, Some(2)));
    let memory = Memory::new(store, ty);
    ret.insert("memory", Extern::Memory(memory));

    return ret;
}
