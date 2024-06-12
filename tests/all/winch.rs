use anyhow::bail;
use wasmtime::*;

#[test]
#[cfg_attr(miri, ignore)]
fn dynamic_heap() -> Result<()> {
    let mut c = Config::new();

    c.strategy(Strategy::Winch);
    c.static_memory_maximum_size(0);
    c.static_memory_guard_size(0);
    c.guard_before_linear_memory(false);
    c.dynamic_memory_guard_size(0);

    let engine = Engine::new(&c)?;
    let wat = r#"
        (module
          (type (;0;) (func (result i32)))
          (func (;0;) (type 0) (result i32)
            (local i32 i64)
            global.get 0
            i32.eqz
            if ;; label = @1
              unreachable
            end
            global.get 0
            i32.const 1
            i32.sub
            global.set 0
            memory.size
            local.set 0
            block ;; label = @1
              block ;; label = @2
                memory.size
                i32.const 65536
                i32.mul
                i32.const 65511
                local.get 0
                i32.add
                i32.le_u
                br_if 0 (;@2;)
                local.get 0
                i32.const 0
                i32.le_s
                br_if 0 (;@2;)
                local.get 0
                i64.load8_s offset=65503
                local.set 1
                br 1 (;@1;)
              end
              i64.const 0
              local.set 1
            end
            local.get 1
            drop
            i32.const 0
          )
          (memory (;0;) 1 3)
          (global (;0;) (mut i32) i32.const 1000)
          (export " " (func 0))
          (export "" (memory 0))
        )
    "#;
    let mut store = Store::new(&engine, ());
    let module = Module::new(&engine, wat)?;
    let instance = Instance::new(&mut store, &module, &[])?;
    let f = instance.get_typed_func::<(), i32>(&mut store, " ")?;
    let result = f.call(&mut store, ())?;

    assert!(result == 0);

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
#[cfg_attr(windows, ignore)]
fn static_oob() -> Result<()> {
    let mut c = Config::new();
    c.static_memory_maximum_size(65536);
    c.strategy(Strategy::Winch);
    let engine = Engine::new(&c)?;
    let wat = r#"
        (module
          (memory 0 1)
          (func (export "") (result i32)
            (i32.const 0)
            (i32.const 1)
            (i32.store offset=726020653)
            (i32.const 1)
            (memory.grow)
          )
        )
    "#;
    let mut store = Store::new(&engine, ());
    let module = Module::new(&engine, wat)?;
    let instance = Instance::new(&mut store, &module, &[])?;
    let f = instance.get_typed_func::<(), i32>(&mut store, "")?;
    let result = f.call(&mut store, ()).unwrap_err();
    assert!(result.downcast_ref::<WasmBacktrace>().is_some());

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
#[cfg_attr(windows, ignore)]
fn dynamic_heap_with_zero_max_size() -> Result<()> {
    let mut c = Config::new();
    c.static_memory_maximum_size(0);
    c.strategy(Strategy::Winch);
    let engine = Engine::new(&c)?;
    let wat = r#"
        (module
          (type (;0;) (func (result i64)))
          (type (;1;) (func (param f32)))
          (func (;0;) (type 0) (result i64)
            (local i32 i32 i32 i32 i32 i32 f32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32)
            global.get 0
            i32.eqz
            if ;; label = @1
              unreachable
            end
            global.get 0
            i32.const 1
            i32.sub
            global.set 0
            memory.size
            f64.load offset=3598476644
            loop (result i32) ;; label = @1
              global.get 0
              i32.eqz
              if ;; label = @2
                unreachable
              end
              global.get 0
              i32.const 1
              i32.sub
              global.set 0
              unreachable
            end
            unreachable
          )
          (memory (;0;) 0 8)
          (global (;0;) (mut i32) i32.const 1000)
          (export "" (func 0))
        )
    "#;
    let mut store = Store::new(&engine, ());
    let module = Module::new(&engine, wat)?;
    let instance = Instance::new(&mut store, &module, &[])?;
    let f = instance.get_typed_func::<(), i64>(&mut store, "")?;
    let result = f.call(&mut store, ()).unwrap_err();
    assert!(result.downcast_ref::<WasmBacktrace>().is_some());

    Ok(())
}
