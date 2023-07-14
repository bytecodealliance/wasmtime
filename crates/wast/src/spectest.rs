use anyhow::Result;
use wasmtime::*;

/// Return an instance implementing the "spectest" interface used in the
/// spec testsuite.
pub fn link_spectest<T>(
    linker: &mut Linker<T>,
    store: &mut Store<T>,
    use_shared_memory: bool,
) -> Result<()> {
    linker.func_wrap("spectest", "print", || {})?;
    linker.func_wrap("spectest", "print_i32", |val: i32| println!("{}: i32", val))?;
    linker.func_wrap("spectest", "print_i64", |val: i64| println!("{}: i64", val))?;
    linker.func_wrap("spectest", "print_f32", |val: f32| println!("{}: f32", val))?;
    linker.func_wrap("spectest", "print_f64", |val: f64| println!("{}: f64", val))?;
    linker.func_wrap("spectest", "print_i32_f32", |i: i32, f: f32| {
        println!("{}: i32", i);
        println!("{}: f32", f);
    })?;
    linker.func_wrap("spectest", "print_f64_f64", |f1: f64, f2: f64| {
        println!("{}: f64", f1);
        println!("{}: f64", f2);
    })?;

    let ty = GlobalType::new(ValType::I32, Mutability::Const);
    let g = Global::new(&mut *store, ty, Val::I32(666))?;
    linker.define(&mut *store, "spectest", "global_i32", g)?;

    let ty = GlobalType::new(ValType::I64, Mutability::Const);
    let g = Global::new(&mut *store, ty, Val::I64(666))?;
    linker.define(&mut *store, "spectest", "global_i64", g)?;

    let ty = GlobalType::new(ValType::F32, Mutability::Const);
    let g = Global::new(&mut *store, ty, Val::F32(0x4426_8000))?;
    linker.define(&mut *store, "spectest", "global_f32", g)?;

    let ty = GlobalType::new(ValType::F64, Mutability::Const);
    let g = Global::new(&mut *store, ty, Val::F64(0x4084_d000_0000_0000))?;
    linker.define(&mut *store, "spectest", "global_f64", g)?;

    let ty = TableType::new(ValType::FuncRef, 10, Some(20));
    let table = Table::new(&mut *store, ty, Val::FuncRef(None))?;
    linker.define(&mut *store, "spectest", "table", table)?;

    let ty = MemoryType::new(1, Some(2));
    let memory = Memory::new(&mut *store, ty)?;
    linker.define(&mut *store, "spectest", "memory", memory)?;

    if use_shared_memory {
        let ty = MemoryType::shared(1, 1);
        let memory = Memory::new(&mut *store, ty)?;
        linker.define(&mut *store, "spectest", "shared_memory", memory)?;
    }

    Ok(())
}

#[cfg(feature = "component-model")]
pub fn link_component_spectest<T>(linker: &mut component::Linker<T>) -> Result<()> {
    use std::sync::atomic::{AtomicU32, Ordering::SeqCst};
    use std::sync::Arc;
    use wasmtime::component::Resource;

    let engine = linker.engine().clone();
    linker
        .root()
        .func_wrap("host-return-two", |_, _: ()| Ok((2u32,)))?;
    let mut i = linker.instance("host")?;
    i.func_wrap("return-three", |_, _: ()| Ok((3u32,)))?;
    i.instance("nested")?
        .func_wrap("return-four", |_, _: ()| Ok((4u32,)))?;

    let module = Module::new(
        &engine,
        r#"
            (module
                (global (export "g") i32 i32.const 100)
                (func (export "f") (result i32) i32.const 101)
            )
        "#,
    )?;
    i.module("simple-module", &module)?;

    struct Resource1;
    struct Resource2;

    #[derive(Default)]
    struct ResourceState {
        drops: AtomicU32,
        last_drop: AtomicU32,
    }

    let state = Arc::new(ResourceState::default());

    i.resource::<Resource1>("resource1", {
        let state = state.clone();
        move |_, rep| {
            state.drops.fetch_add(1, SeqCst);
            state.last_drop.store(rep, SeqCst);
        }
    })?;
    i.resource::<Resource2>("resource2", |_, _| {})?;
    // Currently the embedder API requires redefining the resource destructor
    // here despite this being the same type as before, and fixing that is left
    // for a future refactoring.
    i.resource::<Resource1>("resource1-again", |_, _| {
        panic!("shouldn't be destroyed");
    })?;

    i.func_wrap("[constructor]resource1", |_cx, (rep,): (u32,)| {
        Ok((Resource::<Resource1>::new_own(rep),))
    })?;
    i.func_wrap(
        "[static]resource1.assert",
        |_cx, (resource, rep): (Resource<Resource1>, u32)| {
            assert_eq!(resource.rep(), rep);
            Ok(())
        },
    )?;
    i.func_wrap("[static]resource1.last-drop", {
        let state = state.clone();
        move |_, (): ()| Ok((state.last_drop.load(SeqCst),))
    })?;
    i.func_wrap("[static]resource1.drops", {
        let state = state.clone();
        move |_, (): ()| Ok((state.drops.load(SeqCst),))
    })?;
    i.func_wrap(
        "[method]resource1.simple",
        |_cx, (resource, rep): (Resource<Resource1>, u32)| {
            assert!(!resource.owned());
            assert_eq!(resource.rep(), rep);
            Ok(())
        },
    )?;

    i.func_wrap(
        "[method]resource1.take-borrow",
        |_, (a, b): (Resource<Resource1>, Resource<Resource1>)| {
            assert!(!a.owned());
            assert!(!b.owned());
            Ok(())
        },
    )?;
    i.func_wrap(
        "[method]resource1.take-own",
        |_cx, (a, b): (Resource<Resource1>, Resource<Resource1>)| {
            assert!(!a.owned());
            assert!(b.owned());
            Ok(())
        },
    )?;
    Ok(())
}
