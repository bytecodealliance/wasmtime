use anyhow::Result;
use wasmtime::component::{Component, ComponentParams, Lift, Lower, TypedFunc};
use wasmtime::{AsContextMut, Config, Engine};

mod func;
mod import;
mod instance;
mod macros;
mod nested;
mod post_return;

trait TypedFuncExt<P, R> {
    fn call_and_post_return(&self, store: impl AsContextMut, params: P) -> Result<R>;
}

impl<P, R> TypedFuncExt<P, R> for TypedFunc<P, R>
where
    P: ComponentParams + Lower,
    R: Lift,
{
    fn call_and_post_return(&self, mut store: impl AsContextMut, params: P) -> Result<R> {
        let result = self.call(&mut store, params)?;
        self.post_return(&mut store)?;
        Ok(result)
    }
}

// A simple bump allocator which can be used with modules
const REALLOC_AND_FREE: &str = r#"
    (global $last (mut i32) (i32.const 8))
    (func $realloc (export "realloc")
        (param $old_ptr i32)
        (param $old_size i32)
        (param $align i32)
        (param $new_size i32)
        (result i32)

        ;; Test if the old pointer is non-null
        local.get $old_ptr
        if
            ;; If the old size is bigger than the new size then
            ;; this is a shrink and transparently allow it
            local.get $old_size
            local.get $new_size
            i32.gt_u
            if
                local.get $old_ptr
                return
            end

            ;; ... otherwise this is unimplemented
            unreachable
        end

        ;; align up `$last`
        (global.set $last
            (i32.and
                (i32.add
                    (global.get $last)
                    (i32.add
                        (local.get $align)
                        (i32.const -1)))
                (i32.xor
                    (i32.add
                        (local.get $align)
                        (i32.const -1))
                    (i32.const -1))))

        ;; save the current value of `$last` as the return value
        global.get $last

        ;; ensure anything necessary is set to valid data by spraying a bit
        ;; pattern that is invalid
        global.get $last
        i32.const 0xde
        local.get $new_size
        memory.fill

        ;; bump our pointer
        (global.set $last
            (i32.add
                (global.get $last)
                (local.get $new_size)))
    )
"#;

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
                (import "" (core module))
            )
        "#,
    )?;

    Component::new(
        &engine,
        r#"
            (component
                (import "" (core module $m1
                    (import "" "" (func))
                    (import "" "x" (global i32))

                    (export "a" (table 1 funcref))
                    (export "b" (memory 1))
                    (export "c" (func (result f32)))
                    (export "d" (global i64))
                ))

                (core module $m2
                    (func (export ""))
                    (global (export "x") i32 i32.const 0)
                )
                (core instance $i2 (instantiate (module $m2)))
                (core instance $i1 (instantiate (module $m1) (with "" (instance $i2))))

                (core module $m3
                    (import "mod" "1" (memory 1))
                    (import "mod" "2" (table 1 funcref))
                    (import "mod" "3" (global i64))
                    (import "mod" "4" (func (result f32)))
                )

                (core instance $i3 (instantiate (module $m3)
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
