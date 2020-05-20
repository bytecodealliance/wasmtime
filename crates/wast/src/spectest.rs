use anyhow::Result;
use wasmtime::*;

/// Return an instance implementing the "spectest" interface used in the
/// spec testsuite.
pub fn link_spectest(linker: &mut Linker) -> Result<()> {
    linker.func("spectest", "print", || {})?;
    linker.func("spectest", "print_i32", |val: i32| println!("{}: i32", val))?;
    linker.func("spectest", "print_i64", |val: i64| println!("{}: i64", val))?;
    linker.func("spectest", "print_f32", |val: f32| println!("{}: f32", val))?;
    linker.func("spectest", "print_f64", |val: f64| println!("{}: f64", val))?;
    linker.func("spectest", "print_i32_f32", |i: i32, f: f32| {
        println!("{}: i32", i);
        println!("{}: f32", f);
    })?;
    linker.func("spectest", "print_f64_f64", |f1: f64, f2: f64| {
        println!("{}: f64", f1);
        println!("{}: f64", f2);
    })?;

    let ty = GlobalType::new(ValType::I32, Mutability::Const);
    let g = Global::new(linker.store(), ty, Val::I32(666))?;
    linker.define("spectest", "global_i32", g)?;

    let ty = GlobalType::new(ValType::I64, Mutability::Const);
    let g = Global::new(linker.store(), ty, Val::I64(666))?;
    linker.define("spectest", "global_i64", g)?;

    let ty = GlobalType::new(ValType::F32, Mutability::Const);
    let g = Global::new(linker.store(), ty, Val::F32(0x4426_8000))?;
    linker.define("spectest", "global_f32", g)?;

    let ty = GlobalType::new(ValType::F64, Mutability::Const);
    let g = Global::new(linker.store(), ty, Val::F64(0x4084_d000_0000_0000))?;
    linker.define("spectest", "global_f64", g)?;

    let ty = TableType::new(ValType::FuncRef, Limits::new(10, Some(20)));
    let table = Table::new(linker.store(), ty, Val::ExternRef(ExternRef::Null))?;
    linker.define("spectest", "table", table)?;

    let ty = MemoryType::new(Limits::new(1, Some(2)));
    let memory = Memory::new(linker.store(), ty);
    linker.define("spectest", "memory", memory)?;

    Ok(())
}
