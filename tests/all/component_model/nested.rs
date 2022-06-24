use super::REALLOC_AND_FREE;
use anyhow::Result;
use wasmtime::component::*;
use wasmtime::{Module, Store, StoreContextMut};

#[test]
fn top_level_instance_two_level() -> Result<()> {
    let component = r#"
(component
  (import "c" (instance $i
    (export "c" (instance
      (export "m" (core module
        (export "g" (global i32))
      ))
    ))
  ))
  (component $c1
    (import "c" (instance $i
      (export "c" (instance
        (export "m" (core module
          (export "g" (global i32))
        ))
      ))
    ))
    (core module $verify
      (import "" "g" (global i32))
      (func $start
        global.get 0
        i32.const 101
        i32.ne
        if unreachable end
      )

      (start $start)
    )
    (core instance $m (instantiate (module $i "c" "m")))
    (core instance (instantiate $verify (with "" (instance $m))))
  )
  (instance (instantiate $c1 (with "c" (instance $i))))
)
    "#;
    let module = r#"
(module
  (global (export "g") i32 i32.const 101)
)
    "#;

    let engine = super::engine();
    let module = Module::new(&engine, module)?;
    let component = Component::new(&engine, component)?;
    let mut store = Store::new(&engine, ());
    let mut linker = Linker::new(&engine);
    linker.instance("c")?.instance("c")?.module("m", &module)?;
    linker.instantiate(&mut store, &component)?;
    Ok(())
}

#[test]
fn nested_many_instantiations() -> Result<()> {
    let component = r#"
(component
  (import "count" (func $count))
  (component $c1
    (import "count" (func $count))
    (core func $count_lower (canon lower (func $count)))
    (core module $m
        (import "" "" (func $count))
        (start $count)
    )
    (core instance (instantiate $m (with "" (instance (export "" (func $count_lower))))))
    (core instance (instantiate $m (with "" (instance (export "" (func $count_lower))))))
  )
  (component $c2
    (import "count" (func $count))
    (instance (instantiate $c1 (with "count" (func $count))))
    (instance (instantiate $c1 (with "count" (func $count))))
  )
  (component $c3
    (import "count" (func $count))
    (instance (instantiate $c2 (with "count" (func $count))))
    (instance (instantiate $c2 (with "count" (func $count))))
  )
  (component $c4
    (import "count" (func $count))
    (instance (instantiate $c3 (with "count" (func $count))))
    (instance (instantiate $c3 (with "count" (func $count))))
  )

  (instance (instantiate $c4 (with "count" (func $count))))
)
    "#;
    let engine = super::engine();
    let component = Component::new(&engine, component)?;
    let mut store = Store::new(&engine, 0);
    let mut linker = Linker::new(&engine);
    linker
        .root()
        .func_wrap("count", |mut store: StoreContextMut<'_, u32>| {
            *store.data_mut() += 1;
            Ok(())
        })?;
    linker.instantiate(&mut store, &component)?;
    assert_eq!(*store.data(), 16);
    Ok(())
}

#[test]
fn thread_options_through_inner() -> Result<()> {
    let component = format!(
        r#"
(component
  (import "hostfn" (func $host (param u32) (result string)))

  (component $c
    (import "hostfn" (func $host (param u32) (result string)))

    (core module $libc
        (memory (export "memory") 1)
        {REALLOC_AND_FREE}
    )
    (core instance $libc (instantiate $libc))

    (core func $host_lower
        (canon lower
            (func $host)
            (memory $libc "memory")
            (realloc (func $libc "realloc"))
        )
    )

    (core module $m
        (import "" "host" (func $host (param i32 i32)))
        (import "libc" "memory" (memory 1))
        (func (export "run") (param i32) (result i32)
            i32.const 42
            i32.const 100
            call $host
            i32.const 100
        )
        (export "memory" (memory 0))
    )
    (core instance $m (instantiate $m
        (with "" (instance (export "host" (func $host_lower))))
        (with "libc" (instance $libc))
    ))

    (func (export "run") (param u32) (result string)
        (canon lift
            (core func $m "run")
            (memory $m "memory")
        )
    )
  )
  (instance $c (instantiate $c (with "hostfn" (func $host))))
  (export "run" (func $c "run"))
)
    "#
    );
    let engine = super::engine();
    let component = Component::new(&engine, component)?;
    let mut store = Store::new(&engine, 0);
    let mut linker = Linker::new(&engine);
    linker
        .root()
        .func_wrap("hostfn", |param: u32| Ok(param.to_string()))?;
    let instance = linker.instantiate(&mut store, &component)?;
    let result = instance
        .get_typed_func::<(u32,), WasmStr, _>(&mut store, "run")?
        .call(&mut store, (43,))?;
    assert_eq!(result.to_str(&store)?, "42");
    Ok(())
}
