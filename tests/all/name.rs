use wasmtime::*;

#[test]
fn test_module_no_name() -> anyhow::Result<()> {
    let engine = Engine::default();
    let wat = r#"
        (module
        (func (export "run") (nop))
        )
    "#;

    let module = Module::new(&engine, wat)?;
    assert_eq!(module.name(), None);

    Ok(())
}

#[test]
fn test_module_name() -> anyhow::Result<()> {
    let engine = Engine::default();
    let wat = r#"
        (module $from_name_section
        (func (export "run") (nop))
        )
    "#;

    let module = Module::new(&engine, wat)?;
    assert_eq!(module.name(), Some("from_name_section"));

    Ok(())
}
