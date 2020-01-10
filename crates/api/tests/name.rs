use wasmtime::*;
use wat::parse_str;

#[test]
fn test_module_no_name() -> Result<(), String> {
    let store = Store::default();
    let binary = parse_str(
        r#"
                (module
                (func (export "run") (nop))
                )
            "#,
    )
    .map_err(|e| format!("failed to parse WebAssembly text source: {}", e))?;

    let module = HostRef::new(
        Module::new(&store, &binary).map_err(|e| format!("failed to compile module: {}", e))?,
    );
    assert_eq!(module.borrow().name(), None);

    Ok(())
}

#[test]
fn test_module_name() -> Result<(), String> {
    let store = Store::default();
    let binary = parse_str(
        r#"
                (module $from_name_section
                (func (export "run") (nop))
                )
            "#,
    )
    .map_err(|e| format!("failed to parse WebAssembly text source: {}", e))?;

    let module = HostRef::new(
        Module::new(&store, &binary).map_err(|e| format!("failed to compile module: {}", e))?,
    );
    assert_eq!(module.borrow().name(), Some("from_name_section"));

    let module = HostRef::new(
        Module::new_with_name(&store, &binary, "override")
            .map_err(|e| format!("failed to compile module: {}", e))?,
    );
    assert_eq!(module.borrow().name(), Some("override"));

    Ok(())
}
