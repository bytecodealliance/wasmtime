use wasmtime::*;
use wasmtime_test_macros::wasmtime_test;

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

#[wasmtime_test(wasm_features(tail_call))]
#[cfg_attr(miri, ignore)]
fn unreachable_control_operator_cost(config: &mut Config) -> Result<()> {
    // Use nonzero costs so charging unreachable control operators is observable.
    config.consume_fuel(true).operator_cost(OperatorCost {
        Block: 1,
        Loop: 1,
        If: 1,
        Else: 1,
        End: 1,
        ..Default::default()
    });
    let engine = Engine::new(config)?;
    for terminator in [
        "return",
        "unreachable",
        "return_call $leaf",
        "i32.const 0 return_call_indirect (type $t)",
        "br $exit",
    ] {
        let body = |dead: &str| {
            format!(
                r#"(param i32)
                    block $exit
                        local.get 0
                        if
                            {terminator}
                            {dead}
                        else
                            i32.const 3 drop
                        end
                        i32.const 5 drop
                    end
                    i32.const 7 drop"#
            )
        };
        let module = Module::new(
            &engine,
            format!(
                r#"(module
                    (type $t (func))
                    (func $leaf (type $t))
                    (table funcref (elem $leaf))
                    (func (export "with_dead") {})
                    (func (export "without_dead") {}))"#,
                body("block loop end i32.const 0 if else end end"),
                body(""),
            ),
        )?;
        let mut store = Store::new(&engine, ());
        let instance = Instance::new(&mut store, &module, &[])?;
        // Exercise both the terminating path and the reachable else/end joins.
        for condition in [0, 1] {
            let mut consumed = Vec::new();
            for export in ["with_dead", "without_dead"] {
                store.set_fuel(1_000)?;
                let run = instance.get_typed_func::<i32, ()>(&mut store, export)?;
                let result = run.call(&mut store, condition);
                if terminator == "unreachable" && condition == 1 {
                    assert_eq!(
                        result.unwrap_err().downcast::<Trap>()?,
                        Trap::UnreachableCodeReached
                    );
                } else {
                    result?;
                }
                consumed.push(1_000 - store.get_fuel()?);
            }
            assert_eq!(consumed[0], consumed[1], "{terminator}, {condition}");
        }
    }
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

#[wasmtime_test(wasm_features(extended_const))]
#[cfg_attr(miri, ignore)]
fn const_expr_honors_operator_cost(config: &mut Config) -> Result<()> {
    const WAT: &str = r#"
        (module
          (global $g i32 (i32.add (i32.const 1) (i32.const 2)))
          (export "g" (global $g))
          (func $start)
          (start $start))
    "#;

    fn instantiation_fuel(config: &mut Config, op_cost: OperatorCost) -> Result<u64> {
        config.consume_fuel(true).operator_cost(op_cost);
        let engine = Engine::new(config)?;
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

    assert_eq!(instantiation_fuel(config, OperatorCost::default())?, 6);

    let custom = OperatorCost {
        I32Const: 7,
        I32Add: 100,
        ..Default::default()
    };
    assert_eq!(instantiation_fuel(config, custom)?, 117);

    Ok(())
}

#[wasmtime_test]
#[cfg_attr(miri, ignore)]
fn module_start_call_honors_operator_cost(config: &mut Config) -> Result<()> {
    const WAT: &str = r#"
        (module
          (func $start)
          (start $start))
    "#;

    fn instantiation_fuel(config: &mut Config, op_cost: OperatorCost) -> Result<u64> {
        config.consume_fuel(true).operator_cost(op_cost);
        let engine = Engine::new(config)?;
        let module = Module::new(&engine, WAT)?;

        let mut store = Store::new(&engine, ());
        store.set_fuel(10_000)?;
        Instance::new(&mut store, &module, &[])?;

        Ok(10_000 - store.get_fuel()?)
    }

    assert_eq!(instantiation_fuel(config, OperatorCost::default())?, 3);

    let custom = OperatorCost {
        Call: 50,
        ..Default::default()
    };
    assert_eq!(instantiation_fuel(config, custom)?, 52);

    Ok(())
}

#[wasmtime_test(strategies(not(Winch)))]
#[cfg_attr(miri, ignore)]
fn else_and_end_operator_cost_on_taken_paths(config: &mut Config) -> Result<()> {
    config.consume_fuel(true).operator_cost(OperatorCost {
        If: 0,
        Else: 10,
        End: 100,
        Nop: 0,
        Return: 0,
        LocalGet: 0,
        ..Default::default()
    });
    let engine = Engine::new(config)?;
    let module = Module::new(
        &engine,
        r#"
        (module
          (func (export "if_else") (param i32)
            (if (local.get 0)
              (then nop)
              (else nop)))
          (func (export "if_then_returns") (param i32)
            (if (local.get 0)
              (then return)
              (else nop)))
          (func (export "if_else_returns") (param i32)
            (if (local.get 0)
              (then nop)
              (else return)))
        )
        "#,
    )?;

    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[])?;

    let call_and_measure = |store: &mut Store<()>, name: &str, cond: i32| -> Result<u64> {
        store.set_fuel(10_000)?;
        let f = instance.get_typed_func::<i32, ()>(&mut *store, name)?;
        f.call(&mut *store, cond)?;
        Ok(10_000 - store.get_fuel()?)
    };

    // `then` taken: 1 (func entry) + 100 (`if` end) + 100 (`func` end) = 201.
    // `Else` (10) must NOT be charged when the `then` branch is taken.
    assert_eq!(call_and_measure(&mut store, "if_else", 1)?, 201);

    // `else` taken: 1 (func entry) + 10 (`else`) + 100 (`if` end) + 100 (`func` end) = 211.
    assert_eq!(call_and_measure(&mut store, "if_else", 0)?, 211);

    // `then` taken when `then` ends in `return`: only function entry (1) is charged;
    // neither `Else` (10) nor either `End` (100) is reached.
    assert_eq!(call_and_measure(&mut store, "if_then_returns", 1)?, 1);

    // `else` taken when `then` ends in `return`: still charges `Else` (10) + both `End`s (200) + entry (1) = 211.
    assert_eq!(call_and_measure(&mut store, "if_then_returns", 0)?, 211);

    // `then` taken when `else` ends in `return`: still charges both `End`s (200) + entry (1) = 201.
    assert_eq!(call_and_measure(&mut store, "if_else_returns", 1)?, 201);

    // `else` taken when `else` ends in `return`: charges entry (1) + `Else` (10) = 11;
    // neither `End` (100) is reached.
    assert_eq!(call_and_measure(&mut store, "if_else_returns", 0)?, 11);

    Ok(())
}

#[wasmtime_test(wasm_features(extended_const))]
#[cfg_attr(miri, ignore)]
fn static_memory_segment_preserves_startup_fuel(config: &mut Config) -> Result<()> {
    const WAT: &str = r#"
        (module
          (global $g i32 (i32.add (i32.const 1) (i32.const 2)))
          (memory 1)
          (data (i32.const 0) "hello")
          (export "g" (global $g)))
    "#;

    config.consume_fuel(true);
    let engine = Engine::new(config)?;
    let module = Module::new(&engine, WAT)?;

    let mut store = Store::new(&engine, ());
    store.set_fuel(10_000)?;
    let instance = Instance::new(&mut store, &module, &[])?;

    let g = instance
        .get_global(&mut store, "g")
        .unwrap()
        .get(&mut store);
    assert_eq!(g.i32(), Some(3));

    // At least the 4 units from startup entry + `i32.const` + `i32.const` +
    // `i32.add` must be accounted for even when static memory initialization
    // takes the CoW fast path.
    let consumed = 10_000 - store.get_fuel()?;
    assert!(
        consumed >= 4,
        "startup fuel before static data segment was lost: consumed={consumed}"
    );

    Ok(())
}
