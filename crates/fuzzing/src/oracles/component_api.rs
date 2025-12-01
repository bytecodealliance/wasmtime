//! This module generates test cases for the Wasmtime component model function APIs,
//! e.g. `wasmtime::component::func::Func` and `TypedFunc`.
//!
//! Each case includes a list of arbitrary interface types to use as parameters, plus another one to use as a
//! result, and a component which exports a function and imports a function.  The exported function forwards its
//! parameters to the imported one and forwards the result back to the caller.  This serves to exercise Wasmtime's
//! lifting and lowering code and verify the values remain intact during both processes.

use crate::block_on;
use crate::generators::{self, CompilerStrategy, InstanceAllocationStrategy};
use crate::oracles::log_wasm;
use arbitrary::{Arbitrary, Unstructured};
use std::any::Any;
use std::fmt::Debug;
use std::ops::ControlFlow;
use wasmtime::component::{
    self, Accessor, Component, ComponentNamedList, Lift, Linker, Lower, Val,
};
use wasmtime::{AsContextMut, Enabled, Engine, Result, Store, StoreContextMut};
use wasmtime_test_util::component_fuzz::{
    Declarations, EXPORT_FUNCTION, IMPORT_FUNCTION, MAX_TYPE_DEPTH, TestCase, Type,
};

/// Minimum length of an arbitrary list value generated for a test case
const MIN_LIST_LENGTH: u32 = 0;

/// Maximum length of an arbitrary list value generated for a test case
const MAX_LIST_LENGTH: u32 = 10;

/// Maximum number of invocations of one fuzz case.
const MAX_ITERS: usize = 1_000;

/// Generate an arbitrary instance of the specified type.
fn arbitrary_val(ty: &component::Type, input: &mut Unstructured) -> arbitrary::Result<Val> {
    use component::Type;

    Ok(match ty {
        Type::Bool => Val::Bool(input.arbitrary()?),
        Type::S8 => Val::S8(input.arbitrary()?),
        Type::U8 => Val::U8(input.arbitrary()?),
        Type::S16 => Val::S16(input.arbitrary()?),
        Type::U16 => Val::U16(input.arbitrary()?),
        Type::S32 => Val::S32(input.arbitrary()?),
        Type::U32 => Val::U32(input.arbitrary()?),
        Type::S64 => Val::S64(input.arbitrary()?),
        Type::U64 => Val::U64(input.arbitrary()?),
        Type::Float32 => Val::Float32(input.arbitrary()?),
        Type::Float64 => Val::Float64(input.arbitrary()?),
        Type::Char => Val::Char(input.arbitrary()?),
        Type::String => Val::String(input.arbitrary()?),
        Type::List(list) => {
            let mut values = Vec::new();
            input.arbitrary_loop(Some(MIN_LIST_LENGTH), Some(MAX_LIST_LENGTH), |input| {
                values.push(arbitrary_val(&list.ty(), input)?);

                Ok(ControlFlow::Continue(()))
            })?;

            Val::List(values)
        }
        Type::Record(record) => Val::Record(
            record
                .fields()
                .map(|field| Ok((field.name.to_string(), arbitrary_val(&field.ty, input)?)))
                .collect::<arbitrary::Result<_>>()?,
        ),
        Type::Tuple(tuple) => Val::Tuple(
            tuple
                .types()
                .map(|ty| arbitrary_val(&ty, input))
                .collect::<arbitrary::Result<_>>()?,
        ),
        Type::Variant(variant) => {
            let cases = variant.cases().collect::<Vec<_>>();
            let case = input.choose(&cases)?;
            let payload = match &case.ty {
                Some(ty) => Some(Box::new(arbitrary_val(ty, input)?)),
                None => None,
            };
            Val::Variant(case.name.to_string(), payload)
        }
        Type::Enum(en) => {
            let names = en.names().collect::<Vec<_>>();
            let name = input.choose(&names)?;
            Val::Enum(name.to_string())
        }
        Type::Option(option) => {
            let discriminant = input.int_in_range(0..=1)?;
            Val::Option(match discriminant {
                0 => None,
                1 => Some(Box::new(arbitrary_val(&option.ty(), input)?)),
                _ => unreachable!(),
            })
        }
        Type::Result(result) => {
            let discriminant = input.int_in_range(0..=1)?;
            Val::Result(match discriminant {
                0 => Ok(match result.ok() {
                    Some(ty) => Some(Box::new(arbitrary_val(&ty, input)?)),
                    None => None,
                }),
                1 => Err(match result.err() {
                    Some(ty) => Some(Box::new(arbitrary_val(&ty, input)?)),
                    None => None,
                }),
                _ => unreachable!(),
            })
        }
        Type::Flags(flags) => Val::Flags(
            flags
                .names()
                .filter_map(|name| {
                    input
                        .arbitrary()
                        .map(|p| if p { Some(name.to_string()) } else { None })
                        .transpose()
                })
                .collect::<arbitrary::Result<_>>()?,
        ),

        // Resources, futures, streams, and error contexts aren't fuzzed at this time.
        Type::Own(_) | Type::Borrow(_) | Type::Future(_) | Type::Stream(_) | Type::ErrorContext => {
            unreachable!()
        }
    })
}

fn store<T>(input: &mut Unstructured<'_>, val: T) -> arbitrary::Result<Store<T>> {
    crate::init_fuzzing();

    let mut config = input.arbitrary::<generators::Config>()?;
    config.enable_async(input)?;
    config.module_config.config.multi_value_enabled = true;
    config.module_config.config.reference_types_enabled = true;
    config.module_config.config.max_memories = 2;
    config.module_config.component_model_async = true;
    config.module_config.component_model_async_stackful = true;
    if config.wasmtime.compiler_strategy == CompilerStrategy::Winch {
        config.wasmtime.compiler_strategy = CompilerStrategy::CraneliftNative;
    }

    fn set_min<T>(a: &mut T, min: T)
    where
        T: Ord + Copy,
    {
        if *a < min {
            *a = min;
        }
    }

    fn set_opt_min<T>(a: &mut Option<T>, min: T)
    where
        T: Ord + Copy,
    {
        if let Some(a) = a {
            set_min(a, min);
        }
    }

    if let InstanceAllocationStrategy::Pooling(p) = &mut config.wasmtime.strategy {
        set_min(&mut p.total_component_instances, 5);
        set_min(&mut p.total_core_instances, 5);
        set_min(&mut p.total_memories, 2);
        set_min(&mut p.total_stacks, 4);
        set_min(&mut p.max_memories_per_component, 2);
        set_min(&mut p.max_memories_per_module, 2);
        set_min(&mut p.component_instance_size, 64 << 10);
        set_min(&mut p.core_instance_size, 64 << 10);
        p.memory_protection_keys = Enabled::No;
        p.max_memory_size = 10 << 20; // 10 MiB
    }
    set_opt_min(
        &mut config.wasmtime.memory_config.memory_reservation,
        10 << 20,
    );

    let engine = Engine::new(
        config
            .to_wasmtime()
            .debug_adapter_modules(input.arbitrary()?),
    )
    .unwrap();
    let mut store = Store::new(&engine, val);
    config.configure_store_epoch_and_fuel(&mut store);
    Ok(store)
}

/// Generate zero or more sets of arbitrary argument and result values and execute the test using those
/// values, asserting that they flow from host-to-guest and guest-to-host unchanged.
pub fn static_api_test<'a, P, R>(
    input: &mut Unstructured<'a>,
    declarations: &Declarations,
) -> arbitrary::Result<()>
where
    P: ComponentNamedList
        + Lift
        + Lower
        + Clone
        + PartialEq
        + Debug
        + Arbitrary<'a>
        + Send
        + Sync
        + 'static,
    R: ComponentNamedList
        + Lift
        + Lower
        + Clone
        + PartialEq
        + Debug
        + Arbitrary<'a>
        + Send
        + Sync
        + 'static,
{
    crate::init_fuzzing();

    let mut store = store::<Box<dyn Any + Send>>(input, Box::new(()))?;
    let engine = store.engine();
    let wat = declarations.make_component();
    let wat = wat.as_bytes();
    crate::oracles::log_wasm(wat);
    let component = Component::new(&engine, wat).unwrap();
    let mut linker = Linker::new(&engine);

    fn host_function<P, R>(
        cx: StoreContextMut<'_, Box<dyn Any + Send>>,
        params: P,
    ) -> anyhow::Result<R>
    where
        P: Debug + PartialEq + 'static,
        R: Debug + Clone + 'static,
    {
        log::trace!("received parameters {params:?}");
        let data: &(P, R) = cx.data().downcast_ref().unwrap();
        let (expected_params, result) = data;
        assert_eq!(params, *expected_params);
        log::trace!("returning result {result:?}");
        Ok(result.clone())
    }

    if declarations.options.host_async {
        linker
            .root()
            .func_wrap_concurrent(IMPORT_FUNCTION, |a, params| {
                Box::pin(async move {
                    a.with(|mut cx| host_function::<P, R>(cx.as_context_mut(), params))
                })
            })
            .unwrap();
    } else {
        linker
            .root()
            .func_wrap(IMPORT_FUNCTION, |cx, params| {
                host_function::<P, R>(cx, params)
            })
            .unwrap();
    }

    block_on(async {
        let instance = linker
            .instantiate_async(&mut store, &component)
            .await
            .unwrap();
        let func = instance
            .get_typed_func::<P, R>(&mut store, EXPORT_FUNCTION)
            .unwrap();

        let mut iters = 0..MAX_ITERS;
        while iters.next().is_some() && input.arbitrary()? {
            let params = input.arbitrary::<P>()?;
            let result = input.arbitrary::<R>()?;
            *store.data_mut() = Box::new((params.clone(), result.clone()));
            log::trace!("passing in parameters {params:?}");
            let actual = if declarations.options.guest_caller_async {
                store
                    .run_concurrent(async |a| func.call_concurrent(a, params).await.unwrap().0)
                    .await
                    .unwrap()
            } else {
                let result = func.call_async(&mut store, params).await.unwrap();
                func.post_return_async(&mut store).await.unwrap();
                result
            };
            log::trace!("got result {actual:?}");
            assert_eq!(actual, result);
        }

        Ok(())
    })
}

/// Generate and execute a `crate::generators::component_types::TestCase` using the specified `input` to create
/// arbitrary types and values.
pub fn dynamic_component_api_target(input: &mut arbitrary::Unstructured) -> arbitrary::Result<()> {
    crate::init_fuzzing();

    let mut types = Vec::new();
    let mut type_fuel = 500;

    for _ in 0..5 {
        types.push(Type::generate(input, MAX_TYPE_DEPTH, &mut type_fuel)?);
    }

    let case = TestCase::generate(&types, input)?;

    let mut store = store(input, (Vec::new(), None))?;
    let engine = store.engine();
    let wat = case.declarations().make_component();
    let wat = wat.as_bytes();
    log_wasm(wat);
    let component = Component::new(&engine, wat).unwrap();
    let mut linker = Linker::new(&engine);

    fn host_function(
        mut cx: StoreContextMut<'_, (Vec<Val>, Option<Vec<Val>>)>,
        params: &[Val],
        results: &mut [Val],
    ) -> Result<()> {
        log::trace!("received params {params:?}");
        let (expected_args, expected_results) = cx.data_mut();
        assert_eq!(params.len(), expected_args.len());
        for (expected, actual) in expected_args.iter().zip(params) {
            assert_eq!(expected, actual);
        }
        results.clone_from_slice(&expected_results.take().unwrap());
        log::trace!("returning results {results:?}");
        Ok(())
    }

    if case.options.host_async {
        linker
            .root()
            .func_new_concurrent(IMPORT_FUNCTION, {
                move |cx: &Accessor<_, _>, _, params: &[Val], results: &mut [Val]| {
                    Box::pin(async move {
                        cx.with(|mut store| host_function(store.as_context_mut(), params, results))
                    })
                }
            })
            .unwrap();
    } else {
        linker
            .root()
            .func_new(IMPORT_FUNCTION, {
                move |cx, _, params, results| host_function(cx, params, results)
            })
            .unwrap();
    }

    block_on(async {
        let instance = linker
            .instantiate_async(&mut store, &component)
            .await
            .unwrap();
        let func = instance.get_func(&mut store, EXPORT_FUNCTION).unwrap();
        let ty = func.ty(&store);

        let mut iters = 0..MAX_ITERS;
        while iters.next().is_some() && input.arbitrary()? {
            let params = ty
                .params()
                .map(|(_, ty)| arbitrary_val(&ty, input))
                .collect::<arbitrary::Result<Vec<_>>>()?;
            let results = ty
                .results()
                .map(|ty| arbitrary_val(&ty, input))
                .collect::<arbitrary::Result<Vec<_>>>()?;

            *store.data_mut() = (params.clone(), Some(results.clone()));

            log::trace!("passing params {params:?}");
            let mut actual = vec![Val::Bool(false); results.len()];
            if case.options.guest_caller_async {
                store
                    .run_concurrent(async |a| {
                        func.call_concurrent(a, &params, &mut actual).await.unwrap();
                    })
                    .await
                    .unwrap();
            } else {
                func.call_async(&mut store, &params, &mut actual)
                    .await
                    .unwrap();
                func.post_return_async(&mut store).await.unwrap();
            }
            log::trace!("received results {actual:?}");
            assert_eq!(actual, results);
        }
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::test_n_times;
    use wasmtime_test_util::component_fuzz::{TestCase, Type};

    #[test]
    fn dynamic_component_api_smoke_test() {
        test_n_times(50, |(), u| super::dynamic_component_api_target(u));
    }

    #[test]
    fn static_api_smoke_test() {
        test_n_times(10, |(), u| {
            let mut case = TestCase::generate(&[], u)?;
            case.params = vec![&Type::S32, &Type::Bool, &Type::String];
            case.result = Some(&Type::String);

            let declarations = case.declarations();
            static_api_test::<(i32, bool, String), (String,)>(u, &declarations)
        });
    }
}
