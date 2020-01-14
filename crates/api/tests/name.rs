use wasmtime::*;

#[test]
fn test_module_no_name() -> anyhow::Result<()> {
    let store = Store::default();
    let binary = wat::parse_str(
        r#"
            (module
            (func (export "run") (nop))
            )
        "#,
    )?;

    let module = Module::new(&store, &binary)?;
    assert_eq!(module.name(), None);

    Ok(())
}

#[test]
fn test_module_name() -> anyhow::Result<()> {
    let store = Store::default();
    let binary = wat::parse_str(
        r#"
            (module $from_name_section
            (func (export "run") (nop))
            )
        "#,
    )?;

    let module = Module::new(&store, &binary)?;
    assert_eq!(module.name(), Some("from_name_section"));

    let module = Module::new_with_name(&store, &binary, "override")?;
    assert_eq!(module.name(), Some("override"));

    Ok(())
}
