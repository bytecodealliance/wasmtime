//! This module generates test cases for the Wasmtime component model function APIs,
//! e.g. `wasmtime::component::func::Func` and `TypedFunc`.
//!
//! Each case includes a list of arbitrary interface types to use as parameters, plus another one to use as a
//! result, and a component which exports a function and imports a function.  The exported function forwards its
//! parameters to the imported one and forwards the result back to the caller.  This serves to excercise Wasmtime's
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
        Type::Float32 => Val::Float32(input.arbitrary::<f32>()?.to_bits()),
        Type::Float64 => Val::Float64(input.arbitrary::<f64>()?.to_bits()),
        Type::Char => Val::Char(input.arbitrary()?),
        Type::String => Val::String(input.arbitrary()?),
        Type::List(list) => {
            let mut values = Vec::new();
            input.arbitrary_loop(Some(MIN_LIST_LENGTH), Some(MAX_LIST_LENGTH), |input| {
                values.push(arbitrary_val(&list.ty(), input)?);

                Ok(ControlFlow::Continue(()))
            })?;

            list.new_val(values.into()).unwrap()
        }
        Type::Record(record) => record
            .new_val(
                record
                    .fields()
                    .map(|field| Ok((field.name, arbitrary_val(&field.ty, input)?)))
                    .collect::<arbitrary::Result<Vec<_>>>()?,
            )
            .unwrap(),
        Type::Tuple(tuple) => tuple
            .new_val(
                tuple
                    .types()
                    .map(|ty| arbitrary_val(&ty, input))
                    .collect::<arbitrary::Result<_>>()?,
            )
            .unwrap(),
        Type::Variant(variant) => {
            let cases = variant.cases().collect::<Vec<_>>();
            let case = input.choose(&cases)?;
            let payload = match &case.ty {
                Some(ty) => Some(arbitrary_val(ty, input)?),
                None => None,
            };
            variant.new_val(case.name, payload).unwrap()
        }
        Type::Enum(en) => {
            let names = en.names().collect::<Vec<_>>();
            let name = input.choose(&names)?;
            en.new_val(name).unwrap()
        }
        Type::Union(un) => {
            let mut types = un.types();
            let discriminant = input.int_in_range(0..=types.len() - 1)?;
            un.new_val(
                discriminant.try_into().unwrap(),
                arbitrary_val(&types.nth(discriminant).unwrap(), input)?,
            )
            .unwrap()
        }
        Type::Option(option) => {
            let discriminant = input.int_in_range(0..=1)?;
            option
                .new_val(match discriminant {
                    0 => None,
                    1 => Some(arbitrary_val(&option.ty(), input)?),
                    _ => unreachable!(),
                })
                .unwrap()
        }
        Type::Result(result) => {
            let discriminant = input.int_in_range(0..=1)?;
            result
                .new_val(match discriminant {
                    0 => Ok(match result.ok() {
                        Some(ty) => Some(arbitrary_val(&ty, input)?),
                        None => None,
                    }),
                    1 => Err(match result.err() {
                        Some(ty) => Some(arbitrary_val(&ty, input)?),
                        None => None,
                    }),
                    _ => unreachable!(),
                })
                .unwrap()
        }
        Type::Flags(flags) => flags
            .new_val(
                &flags
                    .names()
                    .filter_map(|name| {
                        input
                            .arbitrary()
                            .map(|p| if p { Some(name) } else { None })
                            .transpose()
                    })
                    .collect::<arbitrary::Result<Box<[_]>>>()?,
            )
            .unwrap(),
    })
}

macro_rules! define_static_api_test {
    ($name:ident $(($param:ident $param_name:ident $param_expected_name:ident))*) => {
        #[allow(unused_parens)]
        /// Generate zero or more sets of arbitrary argument and result values and execute the test using those
        /// values, asserting that they flow from host-to-guest and guest-to-host unchanged.
        pub fn $name<'a, $($param,)* R>(
            input: &mut Unstructured<'a>,
            declarations: &Declarations,
        ) -> arbitrary::Result<()>
        where
            $($param: Lift + Lower + Clone + PartialEq + Debug + Arbitrary<'a> + 'static,)*
            R: ComponentNamedList + Lift + Lower + Clone + PartialEq + Debug + Arbitrary<'a> + 'static
        {
            crate::init_fuzzing();

            let mut config = Config::new();
            config.wasm_component_model(true);
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
                    |cx: StoreContextMut<'_, Box<dyn Any>>,
                    ($($param_name,)*): ($($param,)*)|
                    {
                        log::trace!("received parameters {:?}", ($(&$param_name,)*));
                        let data: &($($param,)* R,) =
                            cx.data().downcast_ref().unwrap();
                        let ($($param_expected_name,)* result,) = data;
                        $(assert_eq!($param_name, *$param_expected_name);)*
                        log::trace!("returning result {:?}", result);
                        Ok(result.clone())
                    },
                )
                .unwrap();
            let mut store: Store<Box<dyn Any>> = Store::new(&engine, Box::new(()));
            let instance = linker.instantiate(&mut store, &component).unwrap();
            let func = instance
                .get_typed_func::<($($param,)*), R, _>(&mut store, EXPORT_FUNCTION)
                .unwrap();

            while input.arbitrary()? {
                $(let $param_name = input.arbitrary::<$param>()?;)*
                let result = input.arbitrary::<R>()?;
                *store.data_mut() = Box::new((
                    $($param_name.clone(),)*
                    result.clone(),
                ));
                log::trace!(
                    "passing in parameters {:?}",
                    ($(&$param_name,)*),
                );
                let actual = func.call(&mut store, ($($param_name,)*)).unwrap();
                log::trace!("got result {:?}", actual);
                assert_eq!(actual, result);
                func.post_return(&mut store).unwrap();
            }

            Ok(())
        }
    }
}

define_static_api_test!(static_api_test0);
define_static_api_test!(static_api_test1 (P0 p0 p0_expected));
define_static_api_test!(static_api_test2 (P0 p0 p0_expected) (P1 p1 p1_expected));
define_static_api_test!(static_api_test3 (P0 p0 p0_expected) (P1 p1 p1_expected) (P2 p2 p2_expected));
define_static_api_test!(static_api_test4 (P0 p0 p0_expected) (P1 p1 p1_expected) (P2 p2 p2_expected)
                        (P3 p3 p3_expected));
define_static_api_test!(static_api_test5 (P0 p0 p0_expected) (P1 p1 p1_expected) (P2 p2 p2_expected)
                        (P3 p3 p3_expected) (P4 p4 p4_expected));
