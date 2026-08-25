use wasmtime::*;
use wasmtime_test_macros::wasmtime_test;
use wast::parser::{self, Parse, ParseBuffer, Parser};
use wast::token::Span;

mod kw {
    wast::custom_keyword!(assert_fuel);
}

struct FuelWast<'a> {
    assertions: Vec<(Span, u64, wast::core::Module<'a>)>,
}

impl<'a> Parse<'a> for FuelWast<'a> {
    fn parse(parser: Parser<'a>) -> parser::Result<Self> {
        let mut assertions = Vec::new();
        while !parser.is_empty() {
            assertions.push(parser.parens(|p| {
                let span = p.parse::<kw::assert_fuel>()?.0;
                Ok((span, p.parse()?, p.parens(|p| p.parse())?))
            })?);
        }
        Ok(FuelWast { assertions })
    }
}

#[wasmtime_test(wasm_features(bulk_memory, reference_types, gc))]
#[cfg_attr(miri, ignore)]
fn run(config: &mut Config) -> Result<()> {
    config.consume_fuel(true);
    let test = std::fs::read_to_string("tests/all/fuel.wast")?;
    let buf = ParseBuffer::new(&test)?;
    let mut wast = parser::parse::<FuelWast<'_>>(&buf)?;
    for (span, fuel, module) in wast.assertions.iter_mut() {
        let consumed = fuel_consumed(&config, &module.encode()?)?;
        if consumed == *fuel {
            continue;
        }
        let (line, col) = span.linecol_in(&test);
        panic!(
            "tests/all/fuel.wast:{}:{} - expected {} fuel, found {}",
            line + 1,
            col + 1,
            fuel,
            consumed
        );
    }
    Ok(())
}

fn fuel_consumed(config: &Config, wasm: &[u8]) -> Result<u64> {
    let engine = Engine::new(&config)?;
    let module = Module::new(&engine, wasm)?;
    let mut store = Store::new(&engine, ());
    store.set_fuel(u64::MAX)?;
    drop(Instance::new(&mut store, &module, &[]));
    Ok(u64::MAX - store.get_fuel()?)
}

#[wasmtime_test(wasm_features(gc, function_references, bulk_memory))]
#[cfg_attr(miri, ignore)]
fn iloop(config: &mut Config) -> Result<()> {
    let _ = env_logger::try_init();

    config.consume_fuel(true);
    iloop_aborts(
        &config,
        r#"
            (module
                (start 0)
                (func loop br 0 end)
            )
        "#,
    )?;
    iloop_aborts(
        &config,
        r#"
            (module
                (start 0)
                (func loop i32.const 1 br_if 0 end)
            )
        "#,
    )?;
    iloop_aborts(
        &config,
        r#"
            (module
                (start 0)
                (func loop i32.const 0 br_table 0 end)
            )
        "#,
    )?;
    iloop_aborts(
        &config,
        r#"
            (module
                (start 0)
                (func $f0 call $f1 call $f1)
                (func $f1 call $f2 call $f2)
                (func $f2 call $f3 call $f3)
                (func $f3 call $f4 call $f4)
                (func $f4 call $f5 call $f5)
                (func $f5 call $f6 call $f6)
                (func $f6 call $f7 call $f7)
                (func $f7 call $f8 call $f8)
                (func $f8 call $f9 call $f9)
                (func $f9 call $f10 call $f10)
                (func $f10 call $f11 call $f11)
                (func $f11 call $f12 call $f12)
                (func $f12 call $f13 call $f13)
                (func $f13 call $f14 call $f14)
                (func $f14 call $f15 call $f15)
                (func $f15 call $f16 call $f16)
                (func $f16)
            )
        "#,
    )?;
    iloop_aborts(
        &config,
        r#"
            (module
                (start 0)
                (func loop ref.null func br_on_null 0 drop end)
            )
        "#,
    )?;
    iloop_aborts(
        &config,
        r#"
            (module
                (start 0)
                (func
                    ref.func 0
                    loop (param (ref func))
                        br_on_non_null 0
                        unreachable
                    end
                )
                (elem declare func 0)
            )
        "#,
    )?;
    iloop_aborts(
        &config,
        r#"
            (module
                (start 0)
                (func
                    i32.const 0
                    ref.i31
                    loop (param (ref i31))
                        br_on_cast 0 anyref (ref i31)
                        unreachable
                    end
                )
                (elem declare func 0)
            )
        "#,
    )?;
    iloop_aborts(
        &config,
        r#"
            (module
                (start 0)
                (func
                    ref.null any
                    loop (param anyref)
                        br_on_cast_fail 0 anyref (ref i31)
                        unreachable
                    end
                )
                (elem declare func 0)
            )
        "#,
    )?;

    iloop_aborts(
        &config,
        r#"
            (module
                (memory 1)
                (start 0)
                (func
                    i32.const 0
                    i32.const 0
                    i32.const 65536
                    memory.copy
                )
            )
        "#,
    )?;

    iloop_aborts(
        &config,
        r#"
            (module
                (memory 1)
                (start 0)
                (func
                    i32.const 0
                    i32.const 0
                    i32.const 65536
                    memory.fill
                )
            )
        "#,
    )?;

    let data = "a".repeat(65536);
    iloop_aborts(
        &config,
        &format!(
            r#"
            (module
                (memory 1)
                (start 0)
                (func
                    i32.const 0
                    i32.const 0
                    i32.const 65536
                    memory.init $d
                )

                (data $d "{data}")
            )
            "#
        ),
    )?;

    iloop_aborts(
        &config,
        r#"
            (module
                (table 20000 funcref)
                (start 0)
                (func
                    i32.const 0
                    i32.const 0
                    i32.const 20000
                    table.copy
                )
            )
        "#,
    )?;

    iloop_aborts(
        &config,
        r#"
            (module
                (table 20000 funcref)
                (start 0)
                (func
                    i32.const 0
                    ref.null func
                    i32.const 20000
                    table.fill
                )
            )
        "#,
    )?;

    iloop_aborts(
        &config,
        r#"
            (module
                (table 0 funcref)
                (start 0)
                (func
                    ref.null func
                    i32.const 20000
                    table.grow
                    drop
                )
            )
        "#,
    )?;

    let elems = "$f ".repeat(20000);
    iloop_aborts(
        &config,
        &format!(
            r#"
            (module
                (table 20000 funcref)
                (start 0)
                (func
                    i32.const 0
                    i32.const 0
                    i32.const 20000
                    table.init $e
                )
                (func $f)
                (elem $e func {elems})
            )
            "#
        ),
    )?;

    iloop_aborts(
        &config,
        r#"
            (module
                (type $a (array i8))
                (start 0)
                (func
                    i32.const 2_0000
                    array.new_default $a
                    drop
                )
            )
        "#,
    )?;

    iloop_aborts(
        &config,
        r#"
            (module
                (type $a (array (mut i8)))
                (start 0)
                (global $a (ref $a) i32.const 20000 array.new_default $a)
                (global $b (ref $a) i32.const 20000 array.new_default $a)
                (func
                    global.get $a
                    i32.const 0
                    global.get $b
                    i32.const 0
                    i32.const 20000
                    array.copy $a $a
                )
            )
        "#,
    )?;

    iloop_aborts(
        &config,
        r#"
            (module
                (type $a (array (mut i8)))
                (start 0)
                (global $a (ref $a) i32.const 20000 array.new_default $a)
                (func
                    global.get $a
                    i32.const 0
                    i32.const 0
                    i32.const 20000
                    array.fill $a
                )
            )
        "#,
    )?;

    iloop_aborts(
        &config,
        &format!(
            r#"
            (module
                (type $a (array (mut i8)))
                (start 0)
                (func
                    i32.const 0
                    i32.const 65536
                    array.new_data $a $d
                    drop
                )

                (data $d "{data}")
            )
            "#
        ),
    )?;

    iloop_aborts(
        &config,
        &format!(
            r#"
            (module
                (type $a (array (mut i8)))
                (start 0)
                (global $a (ref $a) i32.const 20000 array.new_default $a)
                (func
                    global.get $a
                    i32.const 0
                    i32.const 0
                    i32.const 20000
                    array.init_data $a $d
                )

                (data $d "{data}")
            )
            "#
        ),
    )?;

    iloop_aborts(
        &config,
        &format!(
            r#"
            (module
                (type $a (array (mut funcref)))
                (start 0)
                (func
                    i32.const 0
                    i32.const 20000
                    array.new_elem $a $e
                    drop
                )
                (func $f)
                (elem $e func {elems})
            )
            "#
        ),
    )?;

    iloop_aborts(
        &config,
        &format!(
            r#"
            (module
                (type $a (array (mut funcref)))
                (start 0)
                (global $a (ref $a) i32.const 20000 array.new_default $a)
                (func
                    global.get $a
                    i32.const 0
                    i32.const 0
                    i32.const 20000
                    array.init_elem $a $e
                )
                (func $f)
                (elem $e func {elems})
            )
            "#
        ),
    )?;

    iloop_aborts(
        &config,
        r#"
            (module
                (type $a (array (mut i8)))
                (start 0)
                (func
                    i32.const 0
                    i32.const 20000
                    array.new $a
                    drop
                )
            )
        "#,
    )?;

    fn iloop_aborts(config: &Config, wat: &str) -> Result<()> {
        log::debug!("Testing infinite loop:\n{wat}");
        let engine = Engine::new(&config)?;
        let module = Module::new(&engine, wat)?;
        let mut store = Store::new(&engine, ());
        store.set_fuel(10_000)?;
        let error = Instance::new(&mut store, &module, &[]).err().unwrap();
        assert_eq!(error.downcast::<Trap>().unwrap(), Trap::OutOfFuel);
        Ok(())
    }

    Ok(())
}

#[wasmtime_test]
fn manual_fuel(config: &mut Config) -> Result<()> {
    config.consume_fuel(true);
    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, ());
    store.set_fuel(10_000).unwrap();
    assert_eq!(store.get_fuel().ok(), Some(10_000));
    assert_eq!(store.set_fuel(1).ok(), Some(()));
    assert_eq!(store.get_fuel().ok(), Some(1));
    Ok(())
}

#[wasmtime_test]
#[cfg_attr(miri, ignore)]
fn host_function_consumes_all(config: &mut Config) -> Result<()> {
    const FUEL: u64 = 10_000;
    config.consume_fuel(true);
    let engine = Engine::new(&config)?;
    let module = Module::new(
        &engine,
        r#"
            (module
                (import "" "" (func))
                (func (export "")
                    call 0
                    call $other)
                (func $other))
        "#,
    )
    .unwrap();
    let mut store = Store::new(&engine, ());
    store.set_fuel(FUEL).unwrap();
    let func = Func::wrap(&mut store, |mut caller: Caller<'_, ()>| {
        let remaining = caller.get_fuel().unwrap();
        assert_eq!(remaining, FUEL - 2);
        assert!(caller.set_fuel(1).is_ok());
    });

    let instance = Instance::new(&mut store, &module, &[func.into()]).unwrap();
    let export = instance.get_typed_func::<(), ()>(&mut store, "").unwrap();
    let trap = export.call(&mut store, ()).unwrap_err();
    assert_eq!(trap.downcast::<Trap>().unwrap(), Trap::OutOfFuel);
    Ok(())
}

#[wasmtime_test]
fn manual_edge_cases(config: &mut Config) -> Result<()> {
    config.consume_fuel(true);
    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, ());
    store.set_fuel(u64::MAX).unwrap();
    assert_eq!(store.get_fuel().unwrap(), u64::MAX);
    Ok(())
}

#[wasmtime_test]
#[cfg_attr(miri, ignore)]
fn unconditionally_trapping_memory_accesses_save_fuel_before_trapping(
    config: &mut Config,
) -> Result<()> {
    config.consume_fuel(true);
    config.memory_reservation(0x1_0000);

    let engine = Engine::new(&config)?;

    let module = Module::new(
        &engine,
        r#"
            (module
              (memory 1 1)
              (func (export "f") (param i32) (result i32)
                local.get 0
                local.get 0
                i32.add
                ;; This offset is larger than our memory max size and therefore
                ;; will unconditionally trap.
                i32.load8_s offset=0xffffffff))
        "#,
    )
    .unwrap();

    let mut store = Store::new(&engine, ());
    let init_fuel = 1_000;
    store.set_fuel(init_fuel).unwrap();
    assert_eq!(init_fuel, store.get_fuel().unwrap());

    let instance = Instance::new(&mut store, &module, &[]).unwrap();
    let f = instance
        .get_typed_func::<i32, i32>(&mut store, "f")
        .unwrap();

    let trap = f.call(&mut store, 0).unwrap_err();
    assert_eq!(trap.downcast::<Trap>().unwrap(), Trap::MemoryOutOfBounds);

    // The `i32.add` consumed some fuel before the unconditionally trapping
    // memory access.
    let consumed_fuel = init_fuel - store.get_fuel().unwrap();
    assert!(consumed_fuel > 0);
    Ok(())
}

#[wasmtime_test]
#[cfg_attr(miri, ignore)]
fn get_fuel_clamps_at_zero(config: &mut Config) -> Result<()> {
    config.consume_fuel(true);
    let engine = Engine::new(config)?;
    let mut store = Store::new(&engine, ());
    let module = Module::new(
        &engine,
        r#"
(module
  (func $add2 (export "add2") (param $n i32) (result i32)
    (i32.add (local.get $n) (i32.const 2))
  )
)
        "#,
    )?;
    let instance = Instance::new(&mut store, &module, &[])?;

    let add2 = instance.get_typed_func::<i32, i32>(&mut store, "add2")?;

    // Start with 6 fuel and one invocation of this function should cost 4 fuel
    store.set_fuel(6)?;
    assert_eq!(store.get_fuel()?, 6);
    add2.call(&mut store, 10)?;
    assert_eq!(store.get_fuel()?, 2);

    // One more invocation of the function would technically take us to -2 fuel,
    // but that's not representable, so the store should report 0 fuel after
    // this completes.
    add2.call(&mut store, 10)?;
    assert_eq!(store.get_fuel()?, 0);

    // Any further attempts should fail.
    assert!(add2.call(&mut store, 10).is_err());

    Ok(())
}

#[wasmtime_test]
#[cfg_attr(miri, ignore)]
fn immediate_trap_with_fuel1(config: &mut Config) -> Result<()> {
    config.consume_fuel(true);
    let engine = Engine::new(config)?;
    let mut store = Store::new(&engine, ());

    let module = Module::new(
        &engine,
        r#"
            (module
                (func (export "main"))
            )
        "#,
    )?;

    let instance = Instance::new(&mut store, &module, &[])?;
    let main = instance.get_typed_func::<(), ()>(&mut store, "main")?;
    store.set_fuel(1)?;

    assert!(main.call(&mut store, ()).is_err());

    Ok(())
}

#[wasmtime_test(strategies(only(Winch)))]
#[cfg_attr(miri, ignore)]
fn ensure_stack_alignment(config: &mut Config) -> Result<()> {
    config.consume_fuel(true);
    let engine = Engine::new(config)?;
    let mut store = Store::new(&engine, ());
    store.set_fuel(100000000)?;

    let bytes = include_bytes!("../misc_testsuite/winch/fuel_stack_alignment.wat");
    let module = Module::new(&engine, bytes)?;
    let instance = Instance::new(&mut store, &module, &[])?;
    let func = instance.get_typed_func::<f32, ()>(&mut store, "")?;
    let trap = func.call(&mut store, 50397184.0).unwrap_err();
    assert_eq!(
        trap.downcast::<Trap>().unwrap(),
        Trap::UnreachableCodeReached
    );
    Ok(())
}

#[wasmtime_test]
#[cfg_attr(miri, ignore)]
fn custom_operator_cost(config: &mut Config) -> Result<()> {
    config.consume_fuel(true);
    let op_cost = OperatorCost {
        I32Const: 12,
        I32Add: 23,
        I64Const: 64,
        I64Add: 128,
        Drop: 5,
        ..Default::default()
    };
    config.operator_cost(op_cost.clone());
    let engine = Engine::new(config)?;
    let module = Module::new(
        &engine,
        r#"
            (module
              (func (export "main")
                ;; i32: 1 + 2
                (drop (i32.add (i32.const 1) (i32.const 2)))

                ;; i64: 3 + 4
                (drop (i64.add (i64.const 3) (i64.const 4)))
              )
            )
        "#,
    )?;
    let mut store = Store::new(&engine, ());
    store.set_fuel(10_000)?;

    let instance = Instance::new(&mut store, &module, &[])?;
    let main = instance.get_typed_func::<(), ()>(&mut store, "main")?;

    let initial_fuel = store.get_fuel()?;
    main.call(&mut store, ())?;
    let cost_of_execution = u64::from(op_cost.I32Add)
        + u64::from(op_cost.I64Add)
        + u64::from(op_cost.I32Const) * 2
        + u64::from(op_cost.I64Const) * 2
        + u64::from(op_cost.Drop) * 2
        + 1;
    assert_eq!(store.get_fuel()?, initial_fuel - cost_of_execution);

    Ok(())
}

#[wasmtime_test(wasm_features(bulk_memory, reference_types, gc, function_references))]
#[cfg_attr(miri, ignore)]
fn custom_variable_operator_cost(config: &mut Config) -> Result<()> {
    config.consume_fuel(true);

    let mut op_cost = OperatorCost {
        I32Const: 0,
        I64Const: 0,
        RefNull: 0,
        LocalGet: 0,
        LocalSet: 0,
        MemoryCopy: 59,
        MemoryFill: 0,
        MemoryInit: 0,
        MemoryGrow: 61,
        TableCopy: 0,
        TableFill: 67,
        TableInit: 0,
        TableGrow: 0,
        ArrayCopy: 73,
        ArrayFill: 0,
        ArrayNewData: 71,
        ArrayInitData: 0,
        ArrayNewElem: 0,
        ArrayInitElem: 0,
        ArrayNewDefault: 0,
        ArrayNew: 0,
        ..Default::default()
    };
    op_cost.variable = VariableOperatorCost {
        memory_copy_per_byte: 2,
        memory_fill_per_byte: 3,
        memory_init_per_byte: 5,
        memory_grow_per_page: 7,
        table_copy_per_element: 11,
        table_fill_per_element: 13,
        table_init_per_element: 17,
        table_grow_per_element: 19,
        array_copy_per_element: 23,
        array_fill_per_element: 29,
        array_new_data_per_element: 31,
        array_init_data_per_element: 37,
        array_new_elem_per_element: 41,
        array_init_elem_per_element: 43,
        array_new_default_per_element: 47,
        array_new_per_element: 53,
    };
    config.operator_cost(op_cost.clone());

    let wasm = wat::parse_str(
        r#"(module
            (type $i64_array (array (mut i64)))
            (type $i32_array (array (mut i32)))
            (type $ref_array (array (mut funcref)))
            (memory 1 6)
            (table 5 10 funcref)
            (data $data "abcdefghijklmnopqrstuvwxyz")
            (func $f)
            (elem $elem func $f $f $f $f $f)

            (func (export "main")
                (local $i64_values (ref null $i64_array))
                (local $i32_values (ref null $i32_array))
                (local $ref_values (ref null $ref_array))

                i32.const 0 i32.const 0 i32.const 5 memory.copy
                i32.const 0 i32.const 0 i32.const 5 memory.fill
                i32.const 0 i32.const 0 i32.const 5 memory.init $data
                i32.const 5 memory.grow drop

                i32.const 0 i32.const 0 i32.const 5 table.copy
                i32.const 0 ref.null func i32.const 5 table.fill
                i32.const 0 i32.const 0 i32.const 5 table.init $elem
                ref.null func i32.const 5 table.grow drop

                i64.const 0 i32.const 5 array.new $i64_array
                local.set $i64_values
                i32.const 5 array.new_default $i64_array drop
                i32.const 0 i32.const 5 array.new_data $i32_array $data
                local.set $i32_values
                i32.const 0 i32.const 5 array.new_elem $ref_array $elem
                local.set $ref_values

                local.get $i64_values i32.const 0
                local.get $i64_values i32.const 0
                i32.const 5 array.copy $i64_array $i64_array
                local.get $i64_values i32.const 0 i64.const 0 i32.const 5
                array.fill $i64_array
                local.get $i32_values i32.const 0 i32.const 0 i32.const 5
                array.init_data $i32_array $data
                local.get $ref_values i32.const 0 i32.const 0 i32.const 5
                array.init_elem $ref_array $elem)
        )"#,
    )?;
    let engine = Engine::new(config)?;
    let module = Module::new(&engine, wasm)?;
    let mut store = Store::new(&engine, ());
    store.set_fuel(10_000)?;
    let instance = Instance::new(&mut store, &module, &[])?;
    let main = instance.get_typed_func::<(), ()>(&mut store, "main")?;
    let initial_fuel = store.get_fuel()?;
    main.call(&mut store, ())?;
    let consumed = initial_fuel - store.get_fuel()?;

    const UNITS: u64 = 5;
    let variable = &op_cost.variable;
    let base_cost = u64::from(op_cost.MemoryCopy)
        + u64::from(op_cost.MemoryGrow)
        + u64::from(op_cost.TableFill)
        + u64::from(op_cost.ArrayCopy)
        + u64::from(op_cost.ArrayNewData);
    let cost_of_execution = base_cost
        + UNITS
            * (u64::from(variable.memory_copy_per_byte)
                + u64::from(variable.memory_fill_per_byte)
                + u64::from(variable.memory_init_per_byte)
                + u64::from(variable.memory_grow_per_page)
                + u64::from(variable.table_copy_per_element)
                + u64::from(variable.table_fill_per_element)
                + u64::from(variable.table_init_per_element)
                + u64::from(variable.table_grow_per_element)
                + u64::from(variable.array_copy_per_element)
                + u64::from(variable.array_fill_per_element)
                + u64::from(variable.array_new_data_per_element)
                + u64::from(variable.array_init_data_per_element)
                + u64::from(variable.array_new_elem_per_element)
                + u64::from(variable.array_init_elem_per_element)
                + u64::from(variable.array_new_default_per_element)
                + u64::from(variable.array_new_per_element))
        + 1;
    assert_eq!(consumed, cost_of_execution);

    Ok(())
}

#[wasmtime_test(wasm_features(reference_types), strategies(not(Winch)))]
#[cfg_attr(miri, ignore)]
fn variable_operator_cost_failed_growth(config: &mut Config) -> Result<()> {
    config.consume_fuel(true);
    let mut op_cost = OperatorCost {
        I32Const: 0,
        RefNull: 0,
        MemoryGrow: 0,
        TableGrow: 0,
        ..Default::default()
    };
    op_cost.variable.memory_grow_per_page = 7;
    op_cost.variable.table_grow_per_element = 19;
    config.operator_cost(op_cost.clone());

    let engine = Engine::new(config)?;
    let module = Module::new(
        &engine,
        r#"(module
            (memory 1 1)
            (table 0 0 funcref)
            (func (export "main")
                ;; these exceed the max limits so they fail
                i32.const 5 memory.grow drop
                ref.null func i32.const 5 table.grow drop)
        )"#,
    )?;
    let mut store = Store::new(&engine, ());
    store.set_fuel(1_000)?;
    let instance = Instance::new(&mut store, &module, &[])?;
    let main = instance.get_typed_func::<(), ()>(&mut store, "main")?;

    let initial_fuel = store.get_fuel()?;
    main.call(&mut store, ())?;
    let cost_of_execution = 5 * u64::from(op_cost.variable.memory_grow_per_page)
        + 5 * u64::from(op_cost.variable.table_grow_per_element)
        + 1;
    assert_eq!(store.get_fuel()?, initial_fuel - cost_of_execution);

    Ok(())
}

#[wasmtime_test(wasm_features(bulk_memory), strategies(not(Winch)))]
#[cfg_attr(miri, ignore)]
fn variable_operator_cost_follows_bounds_check(config: &mut Config) -> Result<()> {
    config.consume_fuel(true);
    let op_cost = OperatorCost {
        I32Const: 0,
        MemoryFill: 0,
        variable: VariableOperatorCost {
            memory_fill_per_byte: 7,
            ..Default::default()
        },
        ..Default::default()
    };
    config.operator_cost(op_cost);

    let engine = Engine::new(config)?;
    let module = Module::new(
        &engine,
        r#"(module
            (memory 1)
            (func (export "main")
                ;; out of bounds fill
                i32.const 65535 i32.const 0 i32.const 5 memory.fill)
        )"#,
    )?;
    let mut store = Store::new(&engine, ());
    store.set_fuel(1_000)?;
    let instance = Instance::new(&mut store, &module, &[])?;
    let main = instance.get_typed_func::<(), ()>(&mut store, "main")?;

    let initial_fuel = store.get_fuel()?;
    let error = main.call(&mut store, ()).unwrap_err();
    assert_eq!(error.downcast::<Trap>().unwrap(), Trap::MemoryOutOfBounds);
    // The operation traps during bounds validation, before its five-byte
    // variable charge or the pending function-entry unit is flushed.
    assert_eq!(store.get_fuel()?, initial_fuel);

    Ok(())
}

#[wasmtime_test(wasm_features(memory64), strategies(not(Winch)))]
#[cfg_attr(miri, ignore)]
fn memory64_variable_operator_cost_saturates(config: &mut Config) -> Result<()> {
    config.consume_fuel(true);
    let mut operator_cost = OperatorCost::default();
    operator_cost.variable.memory_grow_per_page = 2;
    config.operator_cost(operator_cost);

    let engine = Engine::new(config)?;
    let module = Module::new(
        &engine,
        r#"(module
            (memory i64 0 0)
            (func (export "grow") (param i64) (result i64)
                local.get 0 memory.grow)
        )"#,
    )?;
    let mut store = Store::new(&engine, ());
    store.set_fuel(10_000)?;
    let instance = Instance::new(&mut store, &module, &[])?;
    let grow = instance.get_typed_func::<i64, i64>(&mut store, "grow")?;

    // i64::MAX * 2 must saturate at i64::MAX rather than wrap to -2.
    let error = grow.call(&mut store, i64::MAX).unwrap_err();
    assert_eq!(error.downcast::<Trap>().unwrap(), Trap::OutOfFuel);

    Ok(())
}

#[wasmtime_test(wasm_features(memory64, reference_types), strategies(not(Winch)))]
#[cfg_attr(miri, ignore)]
#[cfg(target_pointer_width = "64")]
fn table64_variable_operator_cost_saturates(config: &mut Config) -> Result<()> {
    config.consume_fuel(true);
    let mut operator_cost = OperatorCost::default();
    operator_cost.variable.table_grow_per_element = 2;
    config.operator_cost(operator_cost);

    let engine = Engine::new(config)?;
    let module = Module::new(
        &engine,
        r#"(module
            (table i64 0 0 funcref)
            (func (export "grow") (param i64) (result i64)
                ref.null func local.get 0 table.grow)
        )"#,
    )?;
    let mut store = Store::new(&engine, ());
    store.set_fuel(10_000)?;
    let instance = Instance::new(&mut store, &module, &[])?;
    let grow = instance.get_typed_func::<i64, i64>(&mut store, "grow")?;

    // i64::MAX * 2 must saturate at i64::MAX rather than wrap to -2.
    let error = grow.call(&mut store, i64::MAX).unwrap_err();
    assert_eq!(error.downcast::<Trap>().unwrap(), Trap::OutOfFuel);
    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn huge_table64_grow_cannot_mint_fuel() -> Result<()> {
    huge_table64_grow_cannot_mint_fuel_impl(
        r#"
        (module
          (table $t i64 0 0x10000 (ref null func))
          (func (export "run") (param $delta i64)
            (loop $l
              (drop (table.grow $t (ref.null func) (local.get $delta)))
              (br $l))))
        "#,
    )
}

#[test]
#[cfg_attr(miri, ignore)]
fn huge_table64_grow_cannot_mint_fuel_const() -> Result<()> {
    huge_table64_grow_cannot_mint_fuel_impl(
        r#"
        (module
          (table $t i64 0 0x10000 (ref null func))
          (func (export "run") (param $delta i64)
            (loop $l
              (drop (table.grow $t (ref.null func) (i64.const -500)))
              (br $l))))
        "#,
    )
}

fn huge_table64_grow_cannot_mint_fuel_impl(wat: &str) -> Result<()> {
    let mut config = Config::new();
    config.consume_fuel(true);
    let engine = Engine::new(&config)?;
    let module = Module::new(&engine, wat)?;

    let mut store = Store::new(&engine, ());
    store.set_fuel(100_000)?;
    let instance = Instance::new(&mut store, &module, &[])?;
    let run = instance.get_typed_func::<i64, ()>(&mut store, "run")?;

    let trap = run.call(&mut store, -500).unwrap_err().downcast::<Trap>()?;
    assert_eq!(trap, Trap::OutOfFuel);
    assert_eq!(store.get_fuel()?, 0);
    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn fuel_around_table_grow() -> Result<()> {
    let mut config = Config::new();
    config.consume_fuel(true);
    let engine = Engine::new(&config)?;
    let module = Module::new(
        &engine,
        r#"
            (module
              (type $ft (func))
              (func $f (type $ft))
              (table $t 1 10000000 (ref $ft) (ref.func $f))
              (func (export "grow") (result i32)
                (table.grow $t (ref.func $f) (i32.const 9999999)))
              (func (export "call") (param i32)
                (call_indirect $t (type $ft) (local.get 0))))
        "#,
    )?;

    let mut store = Store::new(&engine, ());
    store.set_fuel(2)?;
    let instance = Instance::new(&mut store, &module, &[])?;
    let grow = instance.get_typed_func::<(), i32>(&mut store, "grow")?;
    let trap = grow.call(&mut store, ()).unwrap_err().downcast::<Trap>()?;
    assert_eq!(trap, Trap::OutOfFuel);

    store.set_fuel(u64::MAX)?;
    let call = instance.get_typed_func::<i32, ()>(&mut store, "call")?;
    let trap = call
        .call(&mut store, 9999999)
        .unwrap_err()
        .downcast::<Trap>()?;
    assert_eq!(trap, Trap::TableOutOfBounds);
    Ok(())
}

/// A const-expr operator must be charged its configured cost, not a hardcoded 1.
///
/// The module's global initializer is a three-op const-expr. Wasmtime does not
/// constant-fold `i32.add` at compile time, so the expression is translated into
/// the synthesized module-startup function and executed at instantiation. The
/// `(start ...)` function is what flushes the buffered charges into the fuel
/// counter, so it must be present for the charges to be observable.
///
/// Expected accounting with `I32Const = 7`, `I32Add = 100`, and every other
/// cost left at its default of 1:
///
/// | module-startup function entry | 1   |
/// | `i32.const 1`                 | 7   |
/// | `i32.const 2`                 | 7   |
/// | `i32.add`                     | 100 |
/// | synthesized `call $start`     | 1   |
/// | `$start` function entry       | 1   |
/// | total                         | 117 |
///
/// With the default table every op costs 1, so the same module totals 6.
#[test]
#[cfg_attr(miri, ignore)]
fn const_expr_honors_operator_cost() -> Result<()> {
    const WAT: &str = r#"
        (module
          (global $g i32 (i32.add (i32.const 1) (i32.const 2)))
          (export "g" (global $g))
          (func $start)
          (start $start))
    "#;

    fn instantiation_fuel(op_cost: OperatorCost) -> Result<u64> {
        let mut config = Config::new();
        config.consume_fuel(true).operator_cost(op_cost);
        let engine = Engine::new(&config)?;
        let module = Module::new(&engine, WAT)?;

        let mut store = Store::new(&engine, ());
        store.set_fuel(10_000)?;
        let instance = Instance::new(&mut store, &module, &[])?;

        let g = instance
            .get_global(&mut store, "g")
            .unwrap()
            .get(&mut store);
        assert_eq!(g.i32(), Some(3), "global initializer did not run");

        Ok(10_000 - store.get_fuel()?)
    }

    assert_eq!(instantiation_fuel(OperatorCost::default())?, 6);

    let custom = OperatorCost {
        I32Const: 7,
        I32Add: 100,
        ..Default::default()
    };
    assert_eq!(instantiation_fuel(custom)?, 117);

    Ok(())
}

/// `module_start` synthesizes the call to the wasm `(start ...)` function and
/// hand-rolls the fuel accounting that `Operator::Call` normally receives from
/// `fuel_before_op`. That accounting must use the configured `Call` cost rather
/// than a hardcoded 1.
///
/// The module has no globals and no segments, and an empty start function, so
/// the only charges are:
///
/// | module-startup function entry | 1  |
/// | synthesized `call $start`     | 50 |
/// | `$start` function entry       | 1  |
/// | total                         | 52 |
///
/// The two entry rows are the flat per-function entry charge
/// (`FuncEnvironment::new`'s `fuel_consumed: 1`, flushed by `fuel_check` on
/// function entry), not derived from any operator's cost.
#[test]
#[cfg_attr(miri, ignore)]
fn module_start_call_honors_operator_cost() -> Result<()> {
    const WAT: &str = r#"
        (module
          (func $start)
          (start $start))
    "#;

    let mut config = Config::new();
    config.consume_fuel(true).operator_cost(OperatorCost {
        Call: 50,
        ..Default::default()
    });
    let engine = Engine::new(&config)?;
    let module = Module::new(&engine, WAT)?;

    let mut store = Store::new(&engine, ());
    store.set_fuel(10_000)?;
    Instance::new(&mut store, &module, &[])?;

    assert_eq!(10_000 - store.get_fuel()?, 52);
    Ok(())
}
