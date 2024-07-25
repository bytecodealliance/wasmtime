//! This module generates test cases for the Wasmtime component model function APIs,
//! e.g. `wasmtime::component::func::Func` and `TypedFunc`.
//!
//! Each case includes a list of arbitrary interface types to use as parameters, plus another one to use as a
//! result, and a component which exports a function and imports a function.  The exported function forwards its
//! parameters to the imported one and forwards the result back to the caller.  This serves to exercise Wasmtime's
//! lifting and lowering code and verify the values remain intact during both processes.

use arbitrary::{Arbitrary, Unstructured};
use component_fuzz_util::{Declarations, EXPORT_FUNCTION, IMPORT_FUNCTION};
use std::any::Any;
use std::fmt::Debug;
use std::ops::ControlFlow;
use wasmtime::component::{self, Component, ComponentNamedList, Lift, Linker, Lower, Val};
use wasmtime::{Config, Engine, Store, StoreContextMut};

/// Minimum length of an arbitrary list value generated for a test case
const MIN_LIST_LENGTH: u32 = 0;

/// Maximum length of an arbitrary list value generated for a test case
const MAX_LIST_LENGTH: u32 = 10;

/// Generate an arbitrary instance of the specified type.
pub fn arbitrary_val(ty: &component::Type, input: &mut Unstructured) -> arbitrary::Result<Val> {
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

            Val::List(values.into())
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

        // Resources aren't fuzzed at this time.
        Type::Own(_) | Type::Borrow(_) => unreachable!(),
    })
}

/// Generate zero or more sets of arbitrary argument and result values and execute the test using those
/// values, asserting that they flow from host-to-guest and guest-to-host unchanged.
pub fn static_api_test<'a, P, R>(
    input: &mut Unstructured<'a>,
    declarations: &Declarations,
) -> arbitrary::Result<()>
where
    P: ComponentNamedList + Lift + Lower + Clone + PartialEq + Debug + Arbitrary<'a> + 'static,
    R: ComponentNamedList + Lift + Lower + Clone + PartialEq + Debug + Arbitrary<'a> + 'static,
{
    crate::init_fuzzing();

    let mut config = Config::new();
    config.wasm_component_model(true);
    config.wasm_component_model_multiple_returns(true);
    config.debug_adapter_modules(input.arbitrary()?);
    let engine = Engine::new(&config).unwrap();
    let wat = declarations.make_component();
    let wat = wat.as_bytes();
    crate::oracles::log_wasm(wat);
    let component = Component::new(&engine, wat).unwrap();
    let mut linker = Linker::new(&engine);
    linker
        .root()
        .func_wrap(
            IMPORT_FUNCTION,
            |cx: StoreContextMut<'_, Box<dyn Any>>, params: P| {
                log::trace!("received parameters {params:?}");
                let data: &(P, R) = cx.data().downcast_ref().unwrap();
                let (expected_params, result) = data;
                assert_eq!(params, *expected_params);
                log::trace!("returning result {:?}", result);
                Ok(result.clone())
            },
        )
        .unwrap();
    let mut store: Store<Box<dyn Any>> = Store::new(&engine, Box::new(()));
    let instance = linker.instantiate(&mut store, &component).unwrap();
    let func = instance
        .get_typed_func::<P, R>(&mut store, EXPORT_FUNCTION)
        .unwrap();

    while input.arbitrary()? {
        let params = input.arbitrary::<P>()?;
        let result = input.arbitrary::<R>()?;
        *store.data_mut() = Box::new((params.clone(), result.clone()));
        log::trace!("passing in parameters {params:?}");
        let actual = func.call(&mut store, params).unwrap();
        log::trace!("got result {:?}", actual);
        assert_eq!(actual, result);
        func.post_return(&mut store).unwrap();
    }

    Ok(())
}
