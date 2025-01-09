fn main() -> anyhow::Result<()> {
    component::generate_static_api_tests()?;

    Ok(())
}

mod component {
    use anyhow::{anyhow, Context, Error, Result};
    use arbitrary::Unstructured;
    use component_fuzz_util::{Declarations, TestCase, Type, MAX_TYPE_DEPTH};
    use proc_macro2::TokenStream;
    use quote::quote;
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};
    use std::env;
    use std::fmt::Write;
    use std::fs;
    use std::iter;
    use std::path::PathBuf;
    use std::process::Command;

    pub fn generate_static_api_tests() -> Result<()> {
        println!("cargo:rerun-if-changed=build.rs");
        let out_dir = PathBuf::from(
            env::var_os("OUT_DIR").expect("The OUT_DIR environment variable must be set"),
        );

        let mut out = String::new();
        write_static_api_tests(&mut out)?;

        let output = out_dir.join("static_component_api.rs");
        fs::write(&output, out)?;

        drop(Command::new("rustfmt").arg(&output).status());

        Ok(())
    }

    fn write_static_api_tests(out: &mut String) -> Result<()> {
        let seed = if let Ok(seed) = env::var("WASMTIME_FUZZ_SEED") {
            seed.parse::<u64>()
                .with_context(|| anyhow!("expected u64 in WASMTIME_FUZZ_SEED"))?
        } else {
            StdRng::from_entropy().r#gen()
        };

        eprintln!(
            "using seed {seed} (set WASMTIME_FUZZ_SEED={seed} in your environment to reproduce)"
        );

        let mut rng = StdRng::seed_from_u64(seed);

        const TYPE_COUNT: usize = 50;
        const MAX_ARITY: u32 = 5;
        const TEST_CASE_COUNT: usize = 100;

        let mut type_fuel = 1000;
        let mut types = Vec::new();
        let name_counter = &mut 0;
        let mut declarations = TokenStream::new();
        let mut tests = TokenStream::new();

        // First generate a set of type to select from.
        for _ in 0..TYPE_COUNT {
            let ty = generate(&mut rng, |u| {
                // Only discount fuel if the generation was successful,
                // otherwise we'll get more random data and try again.
                let mut fuel = type_fuel;
                let ret = Type::generate(u, MAX_TYPE_DEPTH, &mut fuel);
                if ret.is_ok() {
                    type_fuel = fuel;
                }
                ret
            })?;

            let name = component_fuzz_util::rust_type(&ty, name_counter, &mut declarations);
            types.push((name, ty));
        }

        // Next generate a set of static API test cases driven by the above
        // types.
        for index in 0..TEST_CASE_COUNT {
            let (case, rust_params, rust_results) = generate(&mut rng, |u| {
                let mut params = Vec::new();
                let mut results = Vec::new();
                let mut rust_params = TokenStream::new();
                let mut rust_results = TokenStream::new();
                for _ in 0..u.int_in_range(0..=MAX_ARITY)? {
                    let (name, ty) = u.choose(&types)?;
                    params.push(ty);
                    rust_params.extend(name.clone());
                    rust_params.extend(quote!(,));
                }
                for _ in 0..u.int_in_range(0..=MAX_ARITY)? {
                    let (name, ty) = u.choose(&types)?;
                    results.push(ty);
                    rust_results.extend(name.clone());
                    rust_results.extend(quote!(,));
                }

                let case = TestCase {
                    params,
                    results,
                    encoding1: u.arbitrary()?,
                    encoding2: u.arbitrary()?,
                };
                Ok((case, rust_params, rust_results))
            })?;

            let Declarations {
                types,
                type_instantiation_args,
                params,
                results,
                import_and_export,
                encoding1,
                encoding2,
            } = case.declarations();

            let test = quote!(#index => component_types::static_api_test::<(#rust_params), (#rust_results)>(
                input,
                {
                    static DECLS: Declarations = Declarations {
                        types: Cow::Borrowed(#types),
                        type_instantiation_args: Cow::Borrowed(#type_instantiation_args),
                        params: Cow::Borrowed(#params),
                        results: Cow::Borrowed(#results),
                        import_and_export: Cow::Borrowed(#import_and_export),
                        encoding1: #encoding1,
                        encoding2: #encoding2,
                    };
                    &DECLS
                }
            ),);

            tests.extend(test);
        }

        let module = quote! {
            #[allow(unused_imports)]
            fn static_component_api_target(input: &mut libfuzzer_sys::arbitrary::Unstructured) -> libfuzzer_sys::arbitrary::Result<()> {
                use anyhow::Result;
                use component_fuzz_util::Declarations;
                use component_test_util::{self, Float32, Float64};
                use libfuzzer_sys::arbitrary::{self, Arbitrary};
                use std::borrow::Cow;
                use std::sync::{Arc, Once};
                use wasmtime::component::{ComponentType, Lift, Lower};
                use wasmtime_fuzzing::generators::component_types;

                const SEED: u64 = #seed;

                static ONCE: Once = Once::new();

                ONCE.call_once(|| {
                    eprintln!(
                        "Seed {SEED} was used to generate static component API fuzz tests.\n\
                         Set WASMTIME_FUZZ_SEED={SEED} in your environment at build time to reproduce."
                    );
                });

                #declarations

                match input.int_in_range(0..=(#TEST_CASE_COUNT-1))? {
                    #tests
                    _ => unreachable!()
                }
            }
        };

        write!(out, "{module}")?;

        Ok(())
    }

    fn generate<T>(
        rng: &mut StdRng,
        mut f: impl FnMut(&mut Unstructured<'_>) -> arbitrary::Result<T>,
    ) -> Result<T> {
        let mut bytes = Vec::new();
        loop {
            let count = rng.gen_range(1000..2000);
            bytes.extend(iter::repeat_with(|| rng.r#gen::<u8>()).take(count));

            match f(&mut Unstructured::new(&bytes)) {
                Ok(ret) => break Ok(ret),
                Err(arbitrary::Error::NotEnoughData) => (),
                Err(error) => break Err(Error::from(error)),
            }
        }
    }
}
