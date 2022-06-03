use anyhow::Result;
use wasmtime::component::Component;
use wasmtime::{Config, Engine};

mod func;
mod import;

fn engine() -> Engine {
    let mut config = Config::new();
    config.wasm_component_model(true);
    Engine::new(&config).unwrap()
}

#[test]
fn components_importing_modules() -> Result<()> {
    let engine = engine();

    // FIXME: these components should actually get instantiated in `*.wast`
    // tests once supplying imports has actually been implemented.

    Component::new(
        &engine,
        r#"
            (component
                (import "" (module))
            )
        "#,
    )?;

    Component::new(
        &engine,
        r#"
            (component
                (import "" (module $m1
                    (import "" "" (func))
                    (import "" "x" (global i32))

                    (export "a" (table 1 funcref))
                    (export "b" (memory 1))
                    (export "c" (func (result f32)))
                    (export "d" (global i64))
                ))

                (module $m2
                    (func (export ""))
                    (global (export "x") i32 i32.const 0)
                )
                (instance $i2 (instantiate (module $m2)))
                (instance $i1 (instantiate (module $m1) (with "" (instance $i2))))

                (module $m3
                    (import "mod" "1" (memory 1))
                    (import "mod" "2" (table 1 funcref))
                    (import "mod" "3" (global i64))
                    (import "mod" "4" (func (result f32)))
                )

                (instance $i3 (instantiate (module $m3)
                    (with "mod" (instance
                        (export "1" (memory $i1 "b"))
                        (export "2" (table $i1 "a"))
                        (export "3" (global $i1 "d"))
                        (export "4" (func $i1 "c"))
                    ))
                ))
            )
        "#,
    )?;

    Ok(())
}
