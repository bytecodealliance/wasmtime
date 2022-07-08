use anyhow::Result;
use std::fmt::Write;
use std::iter;
use wasmtime::component::__internal::align_to;
use wasmtime::component::{Component, ComponentParams, Lift, Lower, TypedFunc};
use wasmtime::{AsContextMut, Config, Engine};

mod dynamic;
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

#[derive(Copy, Clone, PartialEq, Eq)]
enum Type {
    S8,
    U8,
    S16,
    U16,
    I32,
    I64,
    F32,
    F64,
}

impl Type {
    fn size(&self) -> u32 {
        match self {
            Self::S8 | Self::U8 => 1,
            Self::S16 | Self::U16 => 2,
            Self::I32 | Self::F32 => 4,
            Self::I64 | Self::F64 => 8,
        }
    }

    fn store(&self) -> &'static str {
        match self {
            Self::S8 | Self::U8 => "store8",
            Self::S16 | Self::U16 => "store16",
            Self::I32 | Self::F32 | Self::I64 | Self::F64 => "store",
        }
    }

    fn primitive(&self) -> &'static str {
        match self {
            Self::S8 | Self::U8 | Self::S16 | Self::U16 | Self::I32 => "i32",
            Self::I64 => "i64",
            Self::F32 => "f32",
            Self::F64 => "f64",
        }
    }
}

fn make_echo_component(type_definition: &str, type_size: u32) -> String {
    make_echo_component_with_params(
        type_definition,
        &iter::repeat(Type::I32)
            .take(usize::try_from(type_size).unwrap() / 4)
            .collect::<Vec<_>>(),
    )
}

fn make_echo_component_with_params(type_definition: &str, params: &[Type]) -> String {
    if params.len() == 1 || params.len() > 16 {
        let primitive = if params.len() == 1 {
            params[0].primitive()
        } else {
            "i32"
        };

        format!(
            r#"
            (component
                (core module $m
                    (func (export "echo") (param {primitive}) (result {primitive})
                        local.get 0
                    )

                    (memory (export "memory") 1)
                    {REALLOC_AND_FREE}
                )

                (core instance $i (instantiate $m))

                (type $Foo {type_definition})

                (func (export "echo") (param $Foo) (result $Foo)
                    (canon lift
                        (core func $i "echo")
                        (memory $i "memory")
                        (realloc (func $i "realloc"))
                    )
                )
            )"#,
        )
    } else {
        let mut param_string = String::new();
        let mut store = String::new();
        let mut offset = 0;

        for (index, param) in params.iter().enumerate() {
            let primitive = param.primitive();
            let size = param.size();
            offset = align_to(offset, size);

            write!(&mut param_string, " {primitive}").unwrap();
            write!(
                &mut store,
                "({primitive}.{} offset={} (local.get $base) (local.get {index}))",
                param.store(),
                offset,
            )
            .unwrap();

            offset += usize::try_from(size).unwrap();
        }

        format!(
            r#"
            (component
                (core module $m
                    (func (export "echo") (param{param_string}) (result i32)
                        (local $base i32)
                        (local.set $base
                            (call $realloc
                                (i32.const 0)
                                (i32.const 0)
                                (i32.const 4)
                                (i32.const {offset})))
                        {store}
                        local.get $base
                    )

                    (memory (export "memory") 1)
                    {REALLOC_AND_FREE}
                )

                (core instance $i (instantiate $m))

                (type $Foo {type_definition})

                (func (export "echo") (param $Foo) (result $Foo)
                    (canon lift
                        (core func $i "echo")
                        (memory $i "memory")
                        (realloc (func $i "realloc"))
                    )
                )
            )"#
        )
    }
}
