use wasmtime_hostmodule::{
    exports, BindArgType, Func, Global, Instantiate, InstantiationError, Memory, Table,
    TableElementType, Value,
};
use wasmtime_jit::InstanceHandle;

fn spectest_print() {}

#[allow(clippy::print_stdout)]
fn spectest_print_i32(x: i32) {
    println!("{}: i32", x);
}

#[allow(clippy::print_stdout)]
fn spectest_print_i64(x: i64) {
    println!("{}: i64", x);
}

#[allow(clippy::print_stdout)]
fn spectest_print_f32(x: f32) {
    println!("{}: f32", x);
}

#[allow(clippy::print_stdout)]
fn spectest_print_f64(x: f64) {
    println!("{}: f64", x);
}

#[allow(clippy::print_stdout)]
fn spectest_print_i32_f32(x: i32, y: f32) {
    println!("{}: i32", x);
    println!("{}: f32", y);
}

#[allow(clippy::print_stdout)]
fn spectest_print_f64_f64(x: f64, y: f64) {
    println!("{}: f64", x);
    println!("{}: f64", y);
}

/// Return an instance implementing the "spectest" interface used in the
/// spec testsuite.
pub fn instantiate_spectest() -> Result<InstanceHandle, InstantiationError> {
    (exports! {
        print: Func(spectest_print.bind::<()>()),
        print_i32: Func(spectest_print_i32.bind::<(i32,)>()),
        print_i64: Func(spectest_print_i64.bind::<(i64,)>()),
        print_f32: Func(spectest_print_f32.bind::<(f32,)>()),
        print_f64: Func(spectest_print_f64.bind::<(f64,)>()),
        print_i32_f32: Func(spectest_print_i32_f32.bind::<(i32, f32)>()),
        print_f64_f64: Func(spectest_print_f64_f64.bind::<(f64, f64)>()),
        global_i32: Global(666i32, Default::default()),
        global_i64: Global(666i64, Default::default()),
        global_f32: Global(Value::F32(0x44268000), Default::default()),
        global_f64: Global(Value::F64(0x4084d00000000000), Default::default()),
        memory: Memory {
            minimum: 1,
            maximum: Some(2),
            shared: false,
        },
        table: Table {
            ty: TableElementType::Func,
            minimum: 10,
            maximum: Some(20),
        },
    })
    .instantiate()
}
