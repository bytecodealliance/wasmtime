//! Tests for `Unrooted<T>` GC references and `MaybeRooted::Unrooted` values.

use super::{ErrorExt, gc_engine, gc_store};
use wasmtime::*;

fn struct_pre(store: impl AsContextMut, fields: &[StorageType]) -> Result<StructRefPre> {
    let ty = StructType::new(
        store.as_context().engine(),
        fields
            .iter()
            .map(|ty| FieldType::new(Mutability::Var, ty.clone())),
    )?;
    Ok(StructRefPre::new(store, ty))
}

fn anyref_field() -> StorageType {
    StorageType::ValType(ValType::ANYREF)
}

fn new_struct(mut store: impl AsContextMut, value: i32) -> Result<Rooted<StructRef>> {
    let pre = struct_pre(&mut store, &[StorageType::ValType(ValType::I32)])?;
    StructRef::new(&mut store, &pre, &[Val::I32(value)])
}

/// Assert that `val` holds an unrooted reference to the same object as `expected`.
fn assert_unrooted_ref_to(
    store: &mut Store<()>,
    val: Option<&MaybeRooted<AnyRef>>,
    expected: Rooted<StructRef>,
) -> Result<()> {
    let val = val.expect("non-null");
    assert!(val.is_unrooted());
    let val = val.to_rooted(&mut *store)?;
    assert!(Rooted::ref_eq(&*store, &val, &expected.to_anyref())?);
    Ok(())
}

#[test]
fn read_through_unrooted_with_shared_store() -> Result<()> {
    let mut store = gc_store()?;
    let inner = new_struct(&mut store, 42)?;
    let pre = struct_pre(&mut store, &[anyref_field()])?;
    let outer = StructRef::new(&mut store, &pre, &[inner.into()])?;

    // Everything below only needs a shared borrow of the store.
    let store_ref: &Store<()> = &store;
    let outer = outer.to_unrooted(store_ref)?;
    assert!(outer.is_valid(store_ref));
    assert_eq!(outer.ty(store_ref)?.fields().len(), 1);

    // Reading a GC reference through an unrooted reference gives an unrooted
    // reference.
    let field = outer.field(store_ref, 0)?;
    let inner = field.unwrap_anyref().unwrap();
    assert!(inner.is_unrooted());
    let inner = inner.unwrap_unrooted().as_struct(store_ref)?.unwrap();
    assert_eq!(inner.field(store_ref, 0)?.unwrap_i32(), 42);
    assert_eq!(inner.fields(store_ref)?.len(), 1);
    Ok(())
}

#[test]
fn read_gc_heap_graph_with_shared_store() -> Result<()> {
    let mut store = gc_store()?;
    let module = Module::new(
        store.engine(),
        r#"
            (module
                (type $leaf (struct (field i32)))
                (type $arr (array (mut anyref)))
                (type $node (struct (field i32) (field (ref null $arr))))
                (global (export "g") (mut anyref)
                    (struct.new $node
                        (i32.const 1)
                        (array.new_fixed $arr 3
                            (struct.new $leaf (i32.const 10))
                            (ref.i31 (i32.const 20))
                            (ref.null any))))
            )
        "#,
    )?;
    let instance = Instance::new(&mut store, &module, &[])?;
    let global = instance.get_global(&mut store, "g").unwrap();

    // Walk the whole graph with only a shared borrow of the store.
    let store: &Store<()> = &store;
    let root = global.get(store);
    let node = root.unwrap_anyref().unwrap().unwrap_unrooted();
    let node = node.as_struct(store)?.unwrap();
    assert_eq!(node.field(store, 0)?.unwrap_i32(), 1);
    let arr = node.field(store, 1)?;
    let arr = arr.unwrap_anyref().unwrap().unwrap_unrooted();
    let arr = arr.as_array(store)?.unwrap();
    assert_eq!(arr.len(store)?, 3);
    let leaf = arr.get(store, 0)?;
    let leaf = leaf.unwrap_anyref().unwrap().unwrap_unrooted();
    let leaf = leaf.as_struct(store)?.unwrap();
    assert_eq!(leaf.field(store, 0)?.unwrap_i32(), 10);
    let i31 = arr.get(store, 1)?;
    let i31 = i31.unwrap_anyref().unwrap().unwrap_unrooted();
    assert_eq!(i31.as_i31(store)?.unwrap().get_u32(), 20);
    assert!(arr.get(store, 2)?.unwrap_anyref().is_none());

    // The same, but starting from rooted references.
    let mut store_mut = gc_store()?;
    let s = new_struct(&mut store_mut, 7)?;
    let pre = struct_pre(&mut store_mut, &[anyref_field()])?;
    let outer = StructRef::new(&mut store_mut, &pre, &[s.into()])?;
    let array_ty = ArrayType::new(
        store_mut.engine(),
        FieldType::new(Mutability::Var, anyref_field()),
    );
    let array_pre = ArrayRefPre::new(&mut store_mut, array_ty);
    let array = ArrayRef::new(&mut store_mut, &array_pre, &outer.into(), 2)?;
    let store: &Store<()> = &store_mut;
    assert_eq!(array.elems(store)?.len(), 2);
    let elem = array.get(store, 1)?;
    let elem = elem.unwrap_anyref().unwrap().unwrap_unrooted();
    let elem = elem.as_struct(store)?.unwrap();
    let fields = elem.fields(store)?;
    let inner = fields[0].unwrap_anyref().unwrap().unwrap_unrooted();
    let inner = inner.as_struct(store)?.unwrap();
    assert_eq!(inner.field(store, 0)?.unwrap_i32(), 7);
    let first = outer.fields(store)?.next().unwrap();
    assert!(first.unwrap_anyref().unwrap().is_unrooted());
    assert!(outer.to_anyref().as_eqref(store)?.is_some());
    Ok(())
}

#[test]
fn unrooted_is_invalidated_by_heap_mutation() -> Result<()> {
    let mut store = gc_store()?;
    let s = new_struct(&mut store, 1)?;

    // Writing to an object invalidates unrooted references.
    let u = s.to_unrooted(&store)?;
    s.set_field(&mut store, 0, Val::I32(2))?;
    assert!(!u.is_valid(&store));
    u.field(&store, 0)
        .unwrap_err()
        .assert_contains("after the GC heap changed");

    // As does allocating.
    let u = s.to_unrooted(&store)?;
    new_struct(&mut store, 3)?;
    assert!(!u.is_valid(&store));

    // As does collecting garbage.
    let u = s.to_unrooted(&store)?;
    store.gc(None)?;
    assert!(!u.is_valid(&store));

    // As does rooting any unrooted reference, even another one.
    let u = s.to_unrooted(&store)?;
    let u2 = s.to_unrooted(&store)?;
    assert_eq!(u, u2);
    let r = u.to_rooted(&mut store)?;
    assert!(!u.is_valid(&store));
    assert!(!u2.is_valid(&store));
    u2.to_owned_rooted(&mut store)
        .unwrap_err()
        .assert_contains("after the GC heap changed");

    // But reading does not, even through an API that takes `&mut Store`.
    let u = s.to_unrooted(&store)?;
    assert_eq!(r.field(&mut store, 0)?.unwrap_i32(), 2);
    assert!(u.is_valid(&store));
    Ok(())
}

#[test]
fn unrooted_is_invalidated_by_running_wasm() -> Result<()> {
    let mut store = gc_store()?;
    let module = Module::new(store.engine(), r#"(module (func (export "f")))"#)?;
    let instance = Instance::new(&mut store, &module, &[])?;
    let f = instance.get_typed_func::<(), ()>(&mut store, "f")?;

    let s = new_struct(&mut store, 1)?;
    let u = s.to_unrooted(&store)?;
    f.call(&mut store, ())?;
    assert!(!u.is_valid(&store));
    Ok(())
}

#[test]
fn unrooted_from_host_call_is_invalidated_once_wasm_runs() -> Result<()> {
    use std::sync::{Arc, Mutex};

    let mut store = gc_store()?;
    let s = new_struct(&mut store, 1)?;
    let global = Global::new(
        &mut store,
        GlobalType::new(ValType::ANYREF, Mutability::Var),
        s.into(),
    )?;

    // A host function creates an unrooted reference and stashes it, then
    // returns to Wasm; Wasm writes to the struct and calls another host
    // function, which observes that the stashed reference is now invalid.
    let stash = Arc::new(Mutex::new(None));
    let stash2 = stash.clone();
    let make = Func::wrap(&mut store, move |caller: Caller<'_, ()>| {
        let u = global
            .get(&caller)
            .unwrap_anyref()
            .unwrap()
            .unwrap_unrooted();
        assert!(u.is_valid(&caller));
        *stash2.lock().unwrap() = Some(u);
    });
    let stash3 = stash.clone();
    let check = Func::wrap(&mut store, move |caller: Caller<'_, ()>| {
        let u = stash3.lock().unwrap().unwrap();
        assert!(!u.is_valid(&caller));
    });
    let module = Module::new(
        store.engine(),
        r#"
            (module
                (type $s (struct (field (mut i32))))
                (import "" "make" (func $make))
                (import "" "check" (func $check))
                (import "" "g" (global $g (mut anyref)))
                (func (export "run")
                    call $make
                    (struct.set $s 0 (ref.cast (ref $s) (global.get $g)) (i32.const 2))
                    call $check
                    call $make
                )
            )
        "#,
    )?;
    let instance = Instance::new(
        &mut store,
        &module,
        &[make.into(), check.into(), global.into()],
    )?;
    let run = instance.get_typed_func::<(), ()>(&mut store, "run")?;
    run.call(&mut store, ())?;
    assert_eq!(s.field(&store, 0)?.unwrap_i32(), 2);

    // The reference stashed by the last host call is invalid once Wasm has
    // returned, since Wasm may have run after the host call returned.
    let u = stash.lock().unwrap().unwrap();
    assert!(!u.is_valid(&store));
    Ok(())
}

#[test]
fn unrooted_is_invalidated_when_its_root_is_dropped() -> Result<()> {
    let mut store = gc_store()?;
    let u = {
        let mut scope = RootScope::new(&mut store);
        let s = new_struct(&mut scope, 1)?;
        let u = s.to_unrooted(&scope)?;
        assert!(u.is_valid(&scope));
        u
    };
    // Exiting the scope may have freed the object.
    assert!(!u.is_valid(&store));
    u.to_rooted(&mut store)
        .unwrap_err()
        .assert_contains("after the GC heap changed");
    Ok(())
}

#[test]
fn unrooted_i31_is_always_valid() -> Result<()> {
    let mut store = gc_store()?;
    let i31 = AnyRef::from_i31(&mut store, I31::wrapping_u32(7)).to_unrooted(&store)?;
    new_struct(&mut store, 1)?;
    store.gc(None)?;
    assert!(i31.is_valid(&store));
    assert_eq!(i31.as_i31(&store)?.unwrap().get_u32(), 7);
    Ok(())
}

#[test]
fn val_rooting_conversions() -> Result<()> {
    let mut store = gc_store()?;
    let s = new_struct(&mut store, 1)?;

    let rooted = Val::from(s);
    rooted.validate_rooted()?;

    let mut val = rooted.to_unrooted(&store)?;
    assert!(val.unwrap_anyref().unwrap().is_unrooted());
    val.validate_rooted()
        .unwrap_err()
        .assert_contains("expected a rooted GC reference");

    // Non-reference values and null references are trivially rooted.
    Val::I32(1).validate_rooted()?;
    Val::null_any_ref().validate_rooted()?;

    val.ensure_rooted(&mut store)?;
    val.validate_rooted()?;
    let r = val.unwrap_anyref().unwrap().unwrap_rooted();
    assert!(r.is_struct(&store)?);

    // Rooting a stale value fails.
    let mut stale = rooted.to_unrooted(&store)?;
    new_struct(&mut store, 2)?;
    stale
        .ensure_rooted(&mut store)
        .unwrap_err()
        .assert_contains("after the GC heap changed");
    Ok(())
}

#[test]
fn getters_return_unrooted_references() -> Result<()> {
    let mut store = gc_store()?;
    let inner = new_struct(&mut store, 5)?;

    // Globals.
    let global = Global::new(
        &mut store,
        GlobalType::new(ValType::ANYREF, Mutability::Var),
        inner.into(),
    )?;
    let val = global.get(&mut store);
    assert_unrooted_ref_to(&mut store, val.unwrap_anyref(), inner)?;

    // Tables.
    let table = Table::new(
        &mut store,
        TableType::new(RefType::ANYREF, 3, None)?,
        inner.into(),
    )?;
    let elem = table.get(&mut store, 1).unwrap();
    assert_unrooted_ref_to(&mut store, elem.unwrap_any(), inner)?;

    // Struct fields.
    let pre = struct_pre(&mut store, &[anyref_field(), anyref_field()])?;
    let outer = StructRef::new(&mut store, &pre, &[inner.into(), inner.into()])?;
    let field = outer.field(&mut store, 0)?;
    assert_unrooted_ref_to(&mut store, field.unwrap_anyref(), inner)?;

    // Array elements.
    let array_ty = ArrayType::new(
        store.engine(),
        FieldType::new(Mutability::Var, anyref_field()),
    );
    let array_pre = ArrayRefPre::new(&mut store, array_ty);
    let array = ArrayRef::new(&mut store, &array_pre, &inner.into(), 2)?;
    let elem = array.get(&mut store, 1)?;
    assert_unrooted_ref_to(&mut store, elem.unwrap_anyref(), inner)?;

    // Values read through an iterator stay valid after the iterator is done.
    let fields = outer.fields(&mut store)?.collect::<Vec<_>>();
    let elems = array.elems(&mut store)?.collect::<Vec<_>>();
    for val in fields.iter().chain(&elems) {
        let val = val.unwrap_anyref().unwrap();
        assert!(val.is_unrooted());
        assert!(val.unwrap_unrooted().is_valid(&store));
    }
    Ok(())
}

#[test]
fn mutating_apis_reject_unrooted_values() -> Result<()> {
    let mut store = gc_store()?;
    let inner = new_struct(&mut store, 1)?;
    let unrooted = Val::from(inner).to_unrooted(&store)?;
    let unrooted_ref = Ref::from(inner).to_unrooted(&store)?;

    // Allocating constructors.
    let pre = struct_pre(&mut store, &[anyref_field()])?;
    StructRef::new(&mut store, &pre, &[unrooted])
        .unwrap_err()
        .assert_contains("unrooted field value");
    let array_ty = ArrayType::new(
        store.engine(),
        FieldType::new(Mutability::Var, anyref_field()),
    );
    let array_pre = ArrayRefPre::new(&mut store, array_ty);
    ArrayRef::new(&mut store, &array_pre, &unrooted, 3)
        .unwrap_err()
        .assert_contains("unrooted element value");
    ArrayRef::new_fixed(&mut store, &array_pre, &[Val::null_any_ref(), unrooted])
        .unwrap_err()
        .assert_contains("unrooted element value");

    // Setters.
    let outer = StructRef::new(&mut store, &pre, &[Val::null_any_ref()])?;
    outer
        .set_field(&mut store, 0, unrooted)
        .unwrap_err()
        .assert_contains("unrooted value");
    let array = ArrayRef::new(&mut store, &array_pre, &Val::null_any_ref(), 2)?;
    array
        .set(&mut store, 1, unrooted)
        .unwrap_err()
        .assert_contains("unrooted value");

    // Globals.
    let global_ty = GlobalType::new(ValType::ANYREF, Mutability::Var);
    Global::new(&mut store, global_ty.clone(), unrooted)
        .unwrap_err()
        .assert_contains("expected a rooted GC reference");
    let global = Global::new(&mut store, global_ty, Val::null_any_ref())?;
    global
        .set(&mut store, unrooted)
        .unwrap_err()
        .assert_contains("expected a rooted GC reference");

    // Tables.
    let table_ty = TableType::new(RefType::ANYREF, 3, None)?;
    Table::new(&mut store, table_ty.clone(), unrooted_ref.clone())
        .unwrap_err()
        .assert_contains("expected a rooted GC reference");
    let table = Table::new(&mut store, table_ty, Ref::Any(None))?;
    for result in [
        table.set(&mut store, 0, unrooted_ref.clone()),
        table.fill(&mut store, 0, unrooted_ref.clone(), 2),
        table.grow(&mut store, 2, unrooted_ref).map(|_| ()),
    ] {
        result
            .unwrap_err()
            .assert_contains("expected a rooted GC reference");
    }
    // A failed `grow` does not grow the table.
    assert_eq!(table.size(&store), 3);

    // Converting to raw values.
    unrooted
        .to_raw(&mut store)
        .unwrap_err()
        .assert_contains("expected a rooted GC reference");

    // None of the above changed anything, and rooted values are acceptable.
    assert!(outer.field(&mut store, 0)?.unwrap_anyref().is_none());
    assert!(array.get(&mut store, 1)?.unwrap_anyref().is_none());
    assert!(global.get(&mut store).unwrap_anyref().is_none());
    let rooted = Val::from(inner);
    outer.set_field(&mut store, 0, rooted)?;
    array.set(&mut store, 1, rooted)?;
    global.set(&mut store, rooted)?;
    table.fill(&mut store, 0, rooted.unwrap_anyref().copied().into(), 3)?;
    StructRef::new(&mut store, &pre, &[rooted])?;
    Ok(())
}

#[test]
fn func_call_takes_rooted_and_returns_unrooted() -> Result<()> {
    let mut store = gc_store()?;
    let module = Module::new(
        store.engine(),
        r#"
            (module
                (type $s (struct (field (mut i32))))
                (func (export "swap") (param anyref anyref) (result anyref anyref)
                    (local.get 1)
                    (local.get 0)
                )
                (func (export "sum") (param anyref anyref) (result i32)
                    (struct.get $s 0 (ref.cast (ref $s) (local.get 0)))
                    (struct.get $s 0 (ref.cast (ref $s) (local.get 1)))
                    i32.add
                )
            )
        "#,
    )?;
    let instance = Instance::new(&mut store, &module, &[])?;
    let swap = instance.get_func(&mut store, "swap").unwrap();
    let sum = instance.get_func(&mut store, "sum").unwrap();

    // Note that the struct type here must be the same as the module's for the
    // casts to succeed, which they are because of canonicalization.
    let a = new_struct(&mut store, 2)?;
    let b = new_struct(&mut store, 3)?;

    // Unrooted arguments are rejected.
    let unrooted_args = [
        Val::from(a).to_unrooted(&store)?,
        Val::from(b).to_unrooted(&store)?,
    ];
    let mut results = [Val::null_any_ref(), Val::null_any_ref()];
    swap.call(&mut store, &unrooted_args, &mut results)
        .unwrap_err()
        .assert_contains("unrooted argument");

    // Results are unrooted.
    swap.call(&mut store, &[a.into(), b.into()], &mut results)?;
    for result in &results {
        assert!(result.unwrap_anyref().unwrap().is_unrooted());
    }

    // So they must be rooted before being passed back in. Rooting them one at
    // a time does not work, because rooting the first invalidates the second.
    results[0].ensure_rooted(&mut store)?;
    results[1]
        .ensure_rooted(&mut store)
        .unwrap_err()
        .assert_contains("after the GC heap changed");

    // But rooting them all at once does.
    swap.call(&mut store, &[a.into(), b.into()], &mut results)?;
    Val::ensure_all_rooted(&mut store, &mut results)?;
    let mut sum_results = [Val::I32(0)];
    sum.call(&mut store, &results, &mut sum_results)?;
    assert_eq!(sum_results[0].unwrap_i32(), 5);
    let swapped_a = results[1].unwrap_anyref().unwrap().unwrap_rooted();
    assert!(Rooted::ref_eq(&store, &swapped_a, &a.to_anyref())?);
    Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn func_call_async_returns_unrooted() -> Result<()> {
    let mut store = gc_store()?;
    let module = Module::new(
        store.engine(),
        r#"
            (module
                (func (export "id2") (param anyref anyref) (result anyref anyref)
                    (local.get 0)
                    (local.get 1)
                )
            )
        "#,
    )?;
    let instance = Instance::new_async(&mut store, &module, &[]).await?;
    let id2 = instance.get_func(&mut store, "id2").unwrap();
    let a = new_struct(&mut store, 1)?;
    let b = new_struct(&mut store, 2)?;
    let mut results = [Val::null_any_ref(), Val::null_any_ref()];
    id2.call_async(&mut store, &[a.into(), b.into()], &mut results)
        .await?;
    Val::ensure_all_rooted(&mut store, &mut results)?;
    let a2 = results[0].unwrap_anyref().unwrap().unwrap_rooted();
    let b2 = results[1].unwrap_anyref().unwrap().unwrap_rooted();
    assert!(Rooted::ref_eq(&store, &a2, &a.to_anyref())?);
    assert!(Rooted::ref_eq(&store, &b2, &b.to_anyref())?);
    Ok(())
}

#[test]
fn batch_rooting() -> Result<()> {
    let mut store = gc_store()?;
    let a = new_struct(&mut store, 1)?;
    let b = new_struct(&mut store, 2)?;
    let unrooted = [a.to_unrooted(&store)?, b.to_unrooted(&store)?];

    // Rooting several unrooted references at once.
    let rooted = Rooted::to_all_rooted(&mut store, &unrooted)?;
    assert_eq!(rooted.len(), 2);
    assert!(Rooted::ref_eq(&store, &rooted[0], &a)?);
    assert!(Rooted::ref_eq(&store, &rooted[1], &b)?);

    let unrooted = [a.to_unrooted(&store)?, b.to_unrooted(&store)?];
    let owned = OwnedRooted::to_all_owned_rooted(&mut store, &unrooted)?;
    assert_eq!(owned.len(), 2);
    assert!(Rooted::ref_eq(&store, &owned[0], &a)?);
    assert!(Rooted::ref_eq(&store, &owned[1], &b)?);

    // Values can mix rooted, unrooted, and non-reference values.
    let mut vals = [
        Val::I32(7),
        Val::from(a).to_unrooted(&store)?,
        Val::from(b),
        Val::null_any_ref(),
        Val::from(b).to_unrooted(&store)?,
    ];
    Val::ensure_all_rooted(&mut store, &mut vals)?;
    for val in &vals {
        val.validate_rooted()?;
    }
    assert_eq!(vals[0].unwrap_i32(), 7);
    assert!(vals[3].unwrap_anyref().is_none());

    // If any reference is stale, nothing is rooted.
    let fresh = Val::from(a).to_unrooted(&store)?;
    let stale_u = b.to_unrooted(&store)?;
    let stale = Val::from(b).to_unrooted(&store)?;
    new_struct(&mut store, 3)?;
    let fresh_u = a.to_unrooted(&store)?;
    let mut vals = [fresh, stale];
    Val::ensure_all_rooted(&mut store, &mut vals)
        .unwrap_err()
        .assert_contains("after the GC heap changed");
    assert!(vals[0].unwrap_anyref().unwrap().is_unrooted());
    Rooted::to_all_rooted(&mut store, &[fresh_u, stale_u])
        .unwrap_err()
        .assert_contains("after the GC heap changed");
    OwnedRooted::to_all_owned_rooted(&mut store, &[fresh_u, stale_u])
        .unwrap_err()
        .assert_contains("after the GC heap changed");

    // Rooting nothing is fine.
    assert!(Rooted::<AnyRef>::to_all_rooted(&mut store, &[])?.is_empty());
    Val::ensure_all_rooted(&mut store, &mut [])?;
    Ok(())
}

#[test]
#[should_panic(expected = "wrong store")]
fn unrooted_used_with_wrong_store() {
    let engine = gc_engine().unwrap();
    let mut store1 = Store::new(&engine, ());
    let mut store2 = Store::new(&engine, ());
    let s = new_struct(&mut store1, 1).unwrap();
    new_struct(&mut store2, 1).unwrap();
    let u = s.to_unrooted(&store1).unwrap();
    let _ = u.is_valid(&store2);
}

#[test]
#[should_panic(expected = "wrong store")]
fn unrooted_values_rooted_in_wrong_store() {
    let engine = gc_engine().unwrap();
    let mut store1 = Store::new(&engine, ());
    let mut store2 = Store::new(&engine, ());
    let s = new_struct(&mut store1, 1).unwrap();
    new_struct(&mut store2, 1).unwrap();
    let mut vals = [Val::from(s).to_unrooted(&store1).unwrap()];
    let _ = Val::ensure_all_rooted(&mut store2, &mut vals);
}

#[test]
fn unrooted_externref_data() -> Result<()> {
    let mut store = gc_store()?;
    let e = ExternRef::new(&mut store, 1234_u32)?;
    let u = e.to_unrooted(&store)?;
    let data = u.data(&store)?.unwrap().downcast_ref::<u32>().copied();
    assert_eq!(data, Some(1234));
    Ok(())
}
