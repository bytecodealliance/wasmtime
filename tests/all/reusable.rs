use anyhow::Result;
use std::sync::Arc;
use wasmtime::*;

fn dynamic_memory_config() -> Config {
    let mut config = Config::new();
    config.static_memory_maximum_size(0);
    config.dynamic_memory_reserved_for_growth(0);
    config
}

fn clears_memory_on_reset(config: &Config) -> Result<()> {
    let engine = Engine::new(&config)?;
    let linker = Linker::new(&engine);

    let module = Module::new(
        &engine,
        r#"(module
            (memory $0 1)
            (export "write_memory" (func $write_memory))
            (export "read_memory" (func $read_memory))
            (func $write_memory
              (i32.store offset=1024
                (i32.const 0)
                (i32.const 10)
              )
            )
            (func $read_memory (result i32)
              (i32.load offset=1024 (i32.const 0))
            )
        )"#,
    )?;

    let mut store = Store::new(&engine, ());
    let instance_pre = linker.instantiate_pre(&mut store, &module)?;
    let instance = instance_pre.instantiate_reusable(&mut store)?;

    let read_memory = instance
        .get_func(&mut store, "read_memory")
        .unwrap()
        .typed::<(), i32, _>(&store)?;
    let write_memory = instance
        .get_func(&mut store, "write_memory")
        .unwrap()
        .typed::<(), (), _>(&store)?;

    // Modify the memory and make sure it's actually modified.
    assert_eq!(read_memory.call(&mut store, ())?, 0);
    write_memory.call(&mut store, ())?;
    assert_eq!(read_memory.call(&mut store, ())?, 10);

    // Reset the instance and make sure the memory was actually cleared.
    instance.reset(&mut store)?;
    assert_eq!(read_memory.call(&mut store, ())?, 0);

    // Do it all over again to make sure the instance can be reset multiple times.
    write_memory.call(&mut store, ())?;
    assert_eq!(read_memory.call(&mut store, ())?, 10);
    instance.reset(&mut store)?;
    assert_eq!(read_memory.call(&mut store, ())?, 0);

    Ok(())
}

#[test]
fn clears_memory_on_reset_static_memory() -> Result<()> {
    clears_memory_on_reset(&Config::new())
}

#[test]
fn clears_memory_on_reset_dynamic_memory() -> Result<()> {
    clears_memory_on_reset(&dynamic_memory_config())
}

fn restores_original_data_segment_on_reset(config: &Config) -> Result<()> {
    let engine = Engine::new(&config)?;
    let linker = Linker::new(&engine);

    let module = Module::new(
        &engine,
        r#"(module
            (memory $0 1)
            (data $.data (i32.const 1024) "A\00\00\00")
            (export "write_memory" (func $write_memory))
            (export "grow_memory" (func $grow_memory))
            (export "read_memory" (func $read_memory))
            (func $write_memory
              (i32.store offset=1024
                (i32.const 0)
                (i32.const 66)
              )
            )
            (func $grow_memory (param $0 i32) (result i32)
              (memory.grow
               (local.get $0)
              )
            )
            (func $read_memory (result i32)
              (i32.load offset=1024 (i32.const 0))
            )
        )"#,
    )?;

    let mut store = Store::new(&engine, ());
    let instance_pre = linker.instantiate_pre(&mut store, &module)?;
    let instance = instance_pre.instantiate_reusable(&mut store)?;

    let grow_memory = instance
        .get_func(&mut store, "grow_memory")
        .unwrap()
        .typed::<i32, i32, _>(&store)?;
    let read_memory = instance
        .get_func(&mut store, "read_memory")
        .unwrap()
        .typed::<(), i32, _>(&store)?;
    let write_memory = instance
        .get_func(&mut store, "write_memory")
        .unwrap()
        .typed::<(), (), _>(&store)?;

    // Modify the memory and make sure it's actually modified.
    assert_eq!(read_memory.call(&mut store, ())?, 65);
    write_memory.call(&mut store, ())?;
    assert_eq!(read_memory.call(&mut store, ())?, 66);

    // Reset the memory and make sure it was actually reset to its initial value.
    instance.reset(&mut store)?;
    assert_eq!(read_memory.call(&mut store, ())?, 65);

    // Do it all over again to make sure the instance can be reset multiple times.
    write_memory.call(&mut store, ())?;
    assert_eq!(read_memory.call(&mut store, ())?, 66);
    instance.reset(&mut store)?;
    assert_eq!(read_memory.call(&mut store, ())?, 65);

    // Do it once again, but this time grow the memory after writing to it.
    //
    // If the memory is dynamic this will reallocate it from scratch somewhere
    // else in memory, so we want to make sure that it still can be reset.
    write_memory.call(&mut store, ())?;
    assert_eq!(grow_memory.call(&mut store, 1)?, 1);
    assert_eq!(read_memory.call(&mut store, ())?, 66);
    instance.reset(&mut store)?;
    assert_eq!(read_memory.call(&mut store, ())?, 65);

    Ok(())
}

#[test]
fn restores_original_data_segment_on_reset_static_memory() -> Result<()> {
    restores_original_data_segment_on_reset(&Config::new())
}

#[test]
fn restores_original_data_segment_on_reset_dynamic_memory() -> Result<()> {
    restores_original_data_segment_on_reset(&dynamic_memory_config())
}

fn restores_memory_size_on_reset_after_grow(config: &Config) -> Result<()> {
    let engine = Engine::new(&config)?;
    let linker = Linker::new(&engine);

    let module = Module::new(
        &engine,
        r#"(module
            (memory $0 1)
            (export "write_memory" (func $write_memory))
            (export "grow_memory" (func $grow_memory))
            (export "read_memory" (func $read_memory))
            (func $write_memory
              (i32.store offset=65536
                (i32.const 0)
                (i32.const 10)
              )
            )
            (func $grow_memory (param $0 i32) (result i32)
              (memory.grow
               (local.get $0)
              )
            )
            (func $read_memory (result i32)
              (i32.load offset=65536 (i32.const 0))
            )
        )"#,
    )?;

    let mut store = Store::new(&engine, ());
    let instance_pre = linker.instantiate_pre(&mut store, &module)?;
    let instance = instance_pre.instantiate_reusable(&mut store)?;

    let grow_memory = instance
        .get_func(&mut store, "grow_memory")
        .unwrap()
        .typed::<i32, i32, _>(&store)?;
    let write_memory = instance
        .get_func(&mut store, "write_memory")
        .unwrap()
        .typed::<(), (), _>(&store)?;
    let read_memory = instance
        .get_func(&mut store, "read_memory")
        .unwrap()
        .typed::<(), i32, _>(&store)?;

    // The memory should initially be one WASM page big.
    assert_eq!(grow_memory.call(&mut store, 0)?, 1);
    assert_eq!(
        read_memory
            .call(&mut store, ())
            .unwrap_err()
            .trap_code()
            .unwrap(),
        TrapCode::MemoryOutOfBounds
    );
    assert_eq!(
        write_memory
            .call(&mut store, ())
            .unwrap_err()
            .trap_code()
            .unwrap(),
        TrapCode::MemoryOutOfBounds
    );

    // ...then we grow it, which should make it accessible.
    assert_eq!(grow_memory.call(&mut store, 1)?, 1);

    // Make sure that we can access it and that it's actually zero'd.
    assert_eq!(read_memory.call(&mut store, ())?, 0);

    // Now we can dirty it.
    write_memory.call(&mut store, ())?;
    assert_eq!(read_memory.call(&mut store, ())?, 10);

    // Then we reset it to its initial state (both its size and its contents).
    instance.reset(&mut store)?;

    // Make sure that the memory size was reset, and that it isn't actually accessible.
    assert_eq!(grow_memory.call(&mut store, 0)?, 1);
    assert_eq!(
        read_memory
            .call(&mut store, ())
            .unwrap_err()
            .trap_code()
            .unwrap(),
        TrapCode::MemoryOutOfBounds
    );
    assert_eq!(
        write_memory
            .call(&mut store, ())
            .unwrap_err()
            .trap_code()
            .unwrap(),
        TrapCode::MemoryOutOfBounds
    );

    // Now we can grow it again.
    assert_eq!(grow_memory.call(&mut store, 1)?, 1);

    // Let's make sure the old value doesn't linger (that is - that the memory wasn't simply only made unaccessible).
    assert_eq!(read_memory.call(&mut store, ())?, 0);

    // Now make sure we can write to it again and read the value back.
    write_memory.call(&mut store, ())?;
    assert_eq!(read_memory.call(&mut store, ())?, 10);

    Ok(())
}

#[test]
fn restores_memory_size_on_reset_after_grow_static_memory() -> Result<()> {
    restores_memory_size_on_reset_after_grow(&Config::new())
}

#[test]
fn restores_memory_size_on_reset_after_grow_dynamic_memory() -> Result<()> {
    restores_memory_size_on_reset_after_grow(&dynamic_memory_config())
}

#[test]
fn restores_table_on_reset() -> Result<()> {
    let engine = Engine::default();
    let linker = Linker::new(&engine);

    let module = Module::new(
        &engine,
        r#"(module
            (type $none_=>_i32 (func (result i32)))
            (memory $0 1)
            (table $table 1 2 funcref)
            (export "return_100" (func $return_100))
            (export "return_200" (func $return_200))
            (export "call_table" (func $call_table))
            (export "table" (table $table))
            (elem (i32.const 0) $return_100)
            (func $call_table (result i32)
              (call_indirect (type $none_=>_i32)
                (i32.const 0)
              )
            )
            (func $return_100 (result i32)
              (i32.const 100)
            )
            (func $return_200 (result i32)
              (i32.const 200)
            )
        )"#,
    )?;

    let mut store = Store::new(&engine, ());
    let instance_pre = linker.instantiate_pre(&mut store, &module)?;
    let instance = instance_pre.instantiate_reusable(&mut store)?;

    let call_table = instance
        .get_func(&mut store, "call_table")
        .unwrap()
        .typed::<(), i32, _>(&store)?;
    let return_200 = instance.get_func(&mut store, "return_200").unwrap();
    let table = instance.get_table(&mut store, "table").unwrap();

    // Sanity checks on the initial state.
    assert_eq!(call_table.call(&mut store, ())?, 100);
    assert_eq!(table.size(&mut store), 1);

    // Replace the function in the table, and grow the table.
    table.set(&mut store, 0, return_200.into())?;
    assert_eq!(call_table.call(&mut store, ())?, 200);
    assert_eq!(table.grow(&mut store, 1, return_200.into())?, 1);

    // Reset the instance, and make sure the table was restored (both its size and its contents).
    instance.reset(&mut store)?;
    assert_eq!(call_table.call(&mut store, ())?, 100);
    assert_eq!(table.size(&mut store), 1);

    Ok(())
}

#[test]
fn snapshot_is_taken_after_start() -> Result<()> {
    let engine = Engine::default();
    let linker = Linker::new(&engine);

    let module = Module::new(
        &engine,
        r#"(module
            (memory $0 1)
            (export "write_memory" (func $write_memory))
            (export "read_memory" (func $read_memory))
            (start $start)
            (func $write_memory
              (i32.store offset=1024
                (i32.const 0)
                (i32.const 10)
              )
            )
            (func $read_memory (result i32)
              (i32.load offset=1024 (i32.const 0))
            )
            (func $start
              (i32.store offset=1024
                (i32.const 0)
                (i32.const 5)
              )
            )
        )"#,
    )?;

    let mut store = Store::new(&engine, ());
    let instance_pre = linker.instantiate_pre(&mut store, &module)?;
    let instance = instance_pre.instantiate_reusable(&mut store)?;

    let read_memory = instance
        .get_func(&mut store, "read_memory")
        .unwrap()
        .typed::<(), i32, _>(&store)?;
    let write_memory = instance
        .get_func(&mut store, "write_memory")
        .unwrap()
        .typed::<(), (), _>(&store)?;

    // Make sure the start function was actually run.
    assert_eq!(read_memory.call(&mut store, ())?, 5);

    // Overwrite the memory.
    write_memory.call(&mut store, ())?;
    assert_eq!(read_memory.call(&mut store, ())?, 10);

    // Reset the memory and make sure it was restored to what the start function wrote.
    instance.reset(&mut store)?;
    assert_eq!(read_memory.call(&mut store, ())?, 5);

    Ok(())
}

#[test]
fn globals_are_restored_on_reset() -> Result<()> {
    let engine = Engine::default();
    let linker = Linker::new(&engine);

    let module = Module::new(
        &engine,
        r#"(module
            (memory $0 1)
            (global $1 (mut i32) (i32.const 123))
            (export "write_global" (func $write_global))
            (export "read_global" (func $read_global))
            (func $write_global
              (global.set $1 (i32.const 124))
            )
            (func $read_global (result i32)
              (global.get $1)
            )
        )"#,
    )?;

    let mut store = Store::new(&engine, ());
    let instance_pre = linker.instantiate_pre(&mut store, &module)?;
    let instance = instance_pre.instantiate_reusable(&mut store)?;

    let read_global = instance
        .get_func(&mut store, "read_global")
        .unwrap()
        .typed::<(), i32, _>(&store)?;
    let write_global = instance
        .get_func(&mut store, "write_global")
        .unwrap()
        .typed::<(), (), _>(&store)?;

    // Make sure the global was properly initialized.
    assert_eq!(read_global.call(&mut store, ())?, 123);

    // Overwrite the global.
    write_global.call(&mut store, ())?;
    assert_eq!(read_global.call(&mut store, ())?, 124);

    // Reset the instance and make sure the global was also restored.
    instance.reset(&mut store)?;
    assert_eq!(read_global.call(&mut store, ())?, 123);

    Ok(())
}

#[test]
fn non_reusable_instance_cannot_be_reset() -> Result<()> {
    let engine = Engine::default();
    let linker = Linker::new(&engine);

    let module = Module::new(&engine, r#"(module)"#)?;

    let mut store = Store::new(&engine, ());
    let instance_pre = linker.instantiate_pre(&mut store, &module)?;
    let instance = instance_pre.instantiate(&mut store)?;

    assert!(instance.reset(&mut store).is_err());

    Ok(())
}

#[test]
fn reusable_instances_are_not_compatible_with_instance_pooling_strategy() -> Result<()> {
    let mut config = Config::new();
    config.allocation_strategy(InstanceAllocationStrategy::Pooling {
        strategy: Default::default(),
        module_limits: Default::default(),
        instance_limits: Default::default(),
    });

    let engine = Engine::new(&config)?;
    let linker = Linker::new(&engine);

    let module = Module::new(&engine, r#"(module)"#)?;

    let mut store = Store::new(&engine, ());
    let instance_pre = linker.instantiate_pre(&mut store, &module)?;
    assert!(instance_pre.instantiate_reusable(&mut store).is_err());

    Ok(())
}

struct DummyMemoryCreator;

unsafe impl MemoryCreator for DummyMemoryCreator {
    fn new_memory(
        &self,
        _ty: MemoryType,
        _minimum: usize,
        _maximum: Option<usize>,
        _reserved_size_in_bytes: Option<usize>,
        _guard_size_in_bytes: usize,
    ) -> Result<Box<dyn LinearMemory>, String> {
        unimplemented!();
    }
}

#[test]
fn reusable_instances_do_not_use_a_custom_memory_creator() -> Result<()> {
    let mut config = Config::new();
    config.with_host_memory(Arc::new(DummyMemoryCreator));

    let engine = Engine::new(&config)?;
    let linker = Linker::new(&engine);

    let module = Module::new(
        &engine,
        r#"(module
            (memory $0 1)
            (export "grow_memory" (func $grow_memory))
            (func $grow_memory (param $0 i32) (result i32)
              (memory.grow
               (local.get $0)
              )
            )
           )"#,
    )?;

    let mut store = Store::new(&engine, ());
    let instance_pre = linker.instantiate_pre(&mut store, &module)?;
    let instance = instance_pre.instantiate_reusable(&mut store)?;

    let grow_memory = instance
        .get_func(&mut store, "grow_memory")
        .unwrap()
        .typed::<i32, i32, _>(&store)?;

    assert_eq!(grow_memory.call(&mut store, 1)?, 1);
    assert_eq!(grow_memory.call(&mut store, 1)?, 2);

    instance.reset(&mut store)?;

    Ok(())
}

#[test]
fn incompatible_with_imported_memories() -> Result<()> {
    let engine = Engine::default();
    let mut linker = Linker::new(&engine);

    let module = Module::new(
        &engine,
        r#"(module
            (import "env" "memory" (memory $memoryimport 1))
        )"#,
    )?;

    let mut store = Store::new(&engine, ());
    let memory_ty = MemoryType::new(1, None);
    let memory = Memory::new(&mut store, memory_ty)?;
    linker.define("env", "memory", memory)?;

    let instance_pre = linker.instantiate_pre(&mut store, &module)?;
    assert!(instance_pre.instantiate(&mut store).is_ok());
    assert!(instance_pre.instantiate_reusable(&mut store).is_err());

    Ok(())
}

#[test]
fn incompatible_with_imported_tables() -> Result<()> {
    let engine = Engine::default();
    let mut linker = Linker::new(&engine);

    let module = Module::new(
        &engine,
        r#"(module
            (import "env" "table" (table $tableimport 1 2 funcref))
        )"#,
    )?;

    let mut store = Store::new(&engine, ());
    let table_ty = TableType::new(ValType::FuncRef, 1, Some(2));
    let table = Table::new(&mut store, table_ty, Val::FuncRef(None))?;
    linker.define("env", "table", table)?;

    let instance_pre = linker.instantiate_pre(&mut store, &module)?;
    assert!(instance_pre.instantiate(&mut store).is_ok());
    assert!(instance_pre.instantiate_reusable(&mut store).is_err());

    Ok(())
}
