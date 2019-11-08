use lightbeam::{translate, ExecutableModule, ExecutionError};

fn translate_wat(wat: &str) -> ExecutableModule {
    let wasm = wat::parse_str(wat).unwrap();
    let compiled = translate(&wasm).unwrap();
    compiled
}

#[test]
fn wrong_type() {
    let code = r#"
(module
  (func (param i32) (param i64) (result i32)
    (i32.const 228)
  )
)
    "#;

    let translated = translate_wat(code);
    assert_eq!(
        translated
            .execute_func::<_, ()>(0, (0u32, 0u32))
            .unwrap_err(),
        ExecutionError::TypeMismatch
    );
}

#[test]
fn wrong_index() {
    let code = r#"
(module
  (func (param i32) (param i64) (result i32)
    (i32.const 228)
  )
)
    "#;

    let translated = translate_wat(code);
    assert_eq!(
        translated
            .execute_func::<_, ()>(10, (0u32, 0u32))
            .unwrap_err(),
        ExecutionError::FuncIndexOutOfBounds
    );
}
