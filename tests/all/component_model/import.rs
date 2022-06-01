use anyhow::Result;
use wasmtime::component::Component;

#[test]
fn can_compile() -> Result<()> {
    let engine = super::engine();
    let libc = r#"
        (module $libc
            (memory (export "memory") 1)
            (func (export "canonical_abi_realloc") (param i32 i32 i32 i32) (result i32)
                unreachable)
            (func (export "canonical_abi_free") (param i32 i32 i32)
                unreachable)
        )
        (instance $libc (instantiate (module $libc)))
    "#;
    Component::new(
        &engine,
        r#"(component
            (import "" (func $f))
            (func (canon.lower (func $f)))
        )"#,
    )?;
    Component::new(
        &engine,
        format!(
            r#"(component
                (import "" (func $f (param string)))
                {libc}
                (func (canon.lower (into $libc) (func $f)))
            )"#
        ),
    )?;
    Component::new(
        &engine,
        format!(
            r#"(component
                (import "f1" (func $f1 (param string) (result string)))
                {libc}
                (func (canon.lower (into $libc) (func $f1)))

                (import "f2" (func $f2 (param u32) (result (list u8))))
                (instance $libc2 (instantiate (module $libc)))
                (func (canon.lower (into $libc2) (func $f2)))

                (func (canon.lower (into $libc2) (func $f1)))
                (func (canon.lower (into $libc) (func $f2)))
            )"#
        ),
    )?;
    Component::new(
        &engine,
        format!(
            r#"(component
                (import "log" (func $log (param string)))
                {libc}
                (func $log_lower (canon.lower (into $libc) (func $log)))

                (module $logger
                    (import "host" "log" (func $log (param i32 i32)))
                    (import "libc" "memory" (memory 1))

                    (func (export "call")
                        i32.const 0
                        i32.const 0
                        call $log)
                )
                (instance $logger (instantiate (module $logger)
                    (with "host" (instance (export "log" (func $log_lower))))
                    (with "libc" (instance $libc))
                ))

                (func (export "call")
                    (canon.lift (func) (func $logger "call"))
                )
            )"#
        ),
    )?;
    Ok(())
}
