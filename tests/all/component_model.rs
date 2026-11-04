use std::fmt::Write;
use std::iter;
use wasmtime::component::{
    Component, ComponentNamedList, Instance, Lift, Linker, Lower, ResourceAny, TypedFunc,
};
use wasmtime::{Config, Result, Store};
use wasmtime_component_util::REALLOC_AND_FREE;
use wasmtime_test_util::component::{async_engine, config, engine, map_engine};

mod aot;
mod r#async;
mod async_dynamic;
mod bindgen;
mod call_hook;
mod dynamic;
mod fixed_length_list;
mod func;
mod handle_table_limit;
mod import;
mod instance;
mod linker;
mod macros;
mod map;
mod missing_async;
mod nested;
mod post_return;
mod resources;
mod stream_contract;
mod stream_sync_dropped;
mod strings;
mod sync_call_inline;

#[derive(Copy, Clone)]
enum ApiStyle {
    Sync,
    Async,
    AsyncNotConcurrent,
    Concurrent,
}

impl ApiStyle {
    fn config(self) -> Config {
        let mut config = config();
        match self {
            ApiStyle::AsyncNotConcurrent => {
                config.concurrency_support(false);
            }
            ApiStyle::Sync | ApiStyle::Async | ApiStyle::Concurrent => {
                config.wasm_component_model_threading(true);
            }
        }
        config
    }

    async fn instantiate<T: Send>(
        self,
        store: &mut Store<T>,
        linker: &Linker<T>,
        component: &Component,
    ) -> Result<Instance> {
        match self {
            ApiStyle::Sync => linker.instantiate(store, &component),
            ApiStyle::Async | ApiStyle::AsyncNotConcurrent | ApiStyle::Concurrent => {
                linker.instantiate_async(store, &component).await
            }
        }
    }

    async fn call<
        P: ComponentNamedList + Lower + 'static,
        R: ComponentNamedList + Lift + 'static,
        T: Send,
    >(
        self,
        store: &mut Store<T>,
        func: TypedFunc<P, R>,
        params: P,
    ) -> Result<R> {
        match self {
            ApiStyle::Sync => func.call(&mut *store, params),
            ApiStyle::Async | ApiStyle::AsyncNotConcurrent => {
                func.call_async(&mut *store, params).await
            }
            ApiStyle::Concurrent => Ok(store
                .run_concurrent(async |access| func.call_concurrent(access, params).await)
                .await??),
        }
    }

    async fn resource_drop<T: Send>(
        self,
        store: &mut Store<T>,
        resource: ResourceAny,
    ) -> Result<()> {
        match self {
            ApiStyle::Sync => resource.resource_drop(store),
            ApiStyle::Async | ApiStyle::AsyncNotConcurrent => {
                resource.resource_drop_async(store).await
            }
            ApiStyle::Concurrent => {
                store
                    .run_concurrent(async |accessor| {
                        resource.resource_drop_concurrent(accessor).await
                    })
                    .await??;
                Ok(())
            }
        }
    }
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

#[derive(Copy, Clone, PartialEq, Eq)]
struct Param(Type, Option<usize>);

fn make_echo_component(type_definition: &str, type_size: u32) -> String {
    let mut offset = 0;
    make_echo_component_with_params(
        type_definition,
        &iter::repeat(Type::I32)
            .map(|ty| {
                let param = Param(ty, Some(offset));
                offset += 4;
                param
            })
            .take(usize::try_from(type_size).unwrap() / 4)
            .collect::<Vec<_>>(),
    )
}

fn make_echo_component_with_params(type_definition: &str, params: &[Param]) -> String {
    let func = if params.len() == 0 {
        format!("(func (export \"echo\"))")
    } else if params.len() == 1 || params.len() > 16 {
        let primitive = if params.len() == 1 {
            params[0].0.primitive()
        } else {
            "i32"
        };

        format!(
            r#"
            (func (export "echo") (param {primitive}) (result {primitive})
                 local.get 0
            )"#,
        )
    } else {
        let mut param_string = String::new();
        let mut store = String::new();
        let mut size = 8;

        for (index, Param(ty, offset)) in params.iter().enumerate() {
            let primitive = ty.primitive();

            write!(&mut param_string, " {primitive}").unwrap();
            if let Some(offset) = offset {
                write!(
                    &mut store,
                    "({primitive}.{} offset={offset} (local.get $base) (local.get {index}))",
                    ty.store(),
                )
                .unwrap();

                size = size.max(offset + 8);
            }
        }

        format!(
            r#"
            (func (export "echo") (param{param_string}) (result i32)
                (local $base i32)
                (local.set $base
                    (call $realloc
                        (i32.const 0)
                        (i32.const 0)
                        (i32.const 4)
                        (i32.const {size})))
                {store}
                local.get $base
            )"#
        )
    };

    let type_section = if type_definition.contains("(type ") {
        type_definition.to_string()
    } else {
        format!("(type $Foo' {type_definition})")
    };

    format!(
        r#"
        (component
            (core module $m
                {func}

                (memory (export "memory") 1)
                {REALLOC_AND_FREE}
            )

            (core instance $i (instantiate $m))

            {type_section}
            (export $Foo "foo" (type $Foo'))

            (func (export "echo") (param "a" $Foo) (result $Foo)
                (canon lift
                    (core func $i "echo")
                    (memory (core memory $i "memory"))
                    (realloc (core func $i "realloc"))
                )
            )
        )"#
    )
}
