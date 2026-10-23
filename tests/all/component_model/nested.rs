#![cfg(not(miri))]

use super::REALLOC_AND_FREE;
use wasmtime::Result;
use wasmtime::component::*;
use wasmtime::{Config, Engine, Module, Store};

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
fn thread_options_through_inner() -> Result<()> {
    let component = format!(
        r#"
(component
  (import "hostfn" (func $host (param "a" u32) (result string)))

  (component $c
    (import "hostfn" (func $host (param "a" u32) (result string)))

    (core module $libc
        (memory (export "memory") 1)
        {REALLOC_AND_FREE}
    )
    (core instance $libc (instantiate $libc))

    (core func $host_lower
        (canon lower
            (func $host)
            (memory (core memory $libc "memory"))
            (realloc (core func $libc "realloc"))
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

    (func (export "run") (param "a" u32) (result string)
        (canon lift
            (core func $m "run")
            (memory (core memory $m "memory"))
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
        .func_wrap("hostfn", |_, (param,): (u32,)| Ok((param.to_string(),)))?;
    let instance = linker.instantiate(&mut store, &component)?;
    let result = instance
        .get_typed_func::<(u32,), (WasmStr,)>(&mut store, "run")?
        .call(&mut store, (43,))?
        .0;
    assert_eq!(result.to_str(&store)?, "42");
    Ok(())
}

#[test]
fn issue_14506_long_reexport_chain_compiles_quickly() -> Result<()> {
    const FUNCS: usize = 500;
    const CHAIN: usize = 500;

    let mut wat = String::from(
        "                       (component\n\
                                  (core module $D\n",
    );
    for i in 0..FUNCS {
        wat.push_str(&format!("     (func (export \"f{i}\"))\n"));
    }
    wat.push_str(
        "                         )\n\
                                  (core module $M\n",
    );
    for i in 0..FUNCS {
        wat.push_str(&format!("     (import \"\" \"f{i}\" (func))\n"));
    }
    for i in 0..FUNCS {
        wat.push_str(&format!("     (export \"f{i}\" (func {i}))\n"));
    }
    wat.push_str(
        "                         )\n\
                                  (core instance $i0 (instantiate $D))\n",
    );
    for i in 1..=CHAIN {
        let prev = i - 1;
        wat.push_str(&format!(
            "                     (core instance $i{i} (instantiate $M (with \"\" (instance $i{prev}))))\n"
        ));
    }
    wat.push_str("              )\n");

    let engine = super::engine();
    Component::new(&engine, wat)?;
    Ok(())
}

#[test]
fn instance_passed_to_nested_component_has_declared_type() -> Result<()> {
    let component = r#"
(component
  (import "host" (instance $host
    (export "n" (implements "p:q/r") (instance))
    (export "extra" (func (result u32)))
  ))

  (core module $m (func (export "f") (result i32) i32.const 7))
  (core instance $i (instantiate $m))
  (func $f (result u32) (canon lift (core func $i "f")))
  (instance $n)
  (instance $arg
    (export "n" (implements "p:q/r") (instance $n))
    (export "extra" (func $f))
  )

  (component $inner
    (import "x" (instance $x (export "n" (implements "x:y/z") (instance))))
    (export "y" (instance $x))
  )
  (instance $a (instantiate $inner (with "x" (instance $arg))))
  (instance $b (instantiate $inner (with "x" (instance $host))))
  (export "a" (instance $a "y"))
  (export "b" (instance $b "y"))
  (export "c" (instance $a))
)
    "#;
    let mut config = Config::new();
    config.wasm_component_model_implements(true);
    let engine = Engine::new(&config)?;
    let component = Component::new(&engine, component)?;

    // The instances exported by `$inner` have the type that `$inner` declared
    // for its import, not the type of the instances passed to it.
    let instance = |item: Option<types::ComponentExtern<'_>>| match item.unwrap().ty {
        types::ComponentItem::ComponentInstance(i) => i,
        _ => panic!("expected an instance"),
    };
    let ty = component.component_type();
    let c = instance(ty.get_export(&engine, "c"));
    for i in [
        instance(ty.get_export(&engine, "a")),
        instance(ty.get_export(&engine, "b")),
        instance(c.get_export(&engine, "y")),
    ] {
        let exports = i
            .exports(&engine)
            .map(|(name, e)| (name, e.implements))
            .collect::<Vec<_>>();
        assert_eq!(exports, [("n", Some("x:y/z"))]);
    }

    let mut store = Store::new(&engine, ());
    let mut linker = Linker::new(&engine);
    let mut host = linker.instance("host")?;
    host.instance("n")?;
    host.func_wrap("extra", |_, ()| Ok((1u32,)))?;
    let instance = linker.instantiate(&mut store, &component)?;
    let c = instance.get_export_index(&mut store, None, "c").unwrap();
    for (parent, name) in [(None, "a"), (None, "b"), (Some(&c), "y")] {
        let i = instance.get_export_index(&mut store, parent, name).unwrap();
        assert!(
            instance
                .get_export_index(&mut store, Some(&i), "n")
                .is_some()
        );
        assert!(
            instance
                .get_export_index(&mut store, Some(&i), "extra")
                .is_none()
        );
    }

    Ok(())
}
