fn main() -> anyhow::Result<()> {
    component::generate_static_api_tests()?;

    Ok(())
}

mod component {
    use anyhow::{anyhow, Context, Error, Result};
    use arbitrary::{Arbitrary, Unstructured};
    use component_fuzz_util::{self, Declarations, TestCase};
    use proc_macro2::TokenStream;
    use quote::{format_ident, quote};
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
            StdRng::from_entropy().gen()
        };

        eprintln!(
            "using seed {seed} (set WASMTIME_FUZZ_SEED={seed} in your environment to reproduce)"
        );

        let mut rng = StdRng::seed_from_u64(seed);

        const TEST_CASE_COUNT: usize = 100;

        let mut tests = TokenStream::new();

        let name_counter = &mut 0;

        let mut declarations = TokenStream::new();

        for index in 0..TEST_CASE_COUNT {
            let mut bytes = Vec::new();

            let case = loop {
                let count = rng.gen_range(1000..2000);
                bytes.extend(iter::repeat_with(|| rng.gen::<u8>()).take(count));

                match TestCase::arbitrary(&mut Unstructured::new(&bytes)) {
                    Ok(case) => break case,
                    Err(arbitrary::Error::NotEnoughData) => (),
                    Err(error) => return Err(Error::from(error)),
                }
            };

            let Declarations {
                types,
                params,
                results,
                import_and_export,
                encoding1,
                encoding2,
            } = case.declarations();

            let test = format_ident!("static_api_test{}", case.params.len());

            let rust_params = case
                .params
                .iter()
                .map(|ty| {
                    let ty = component_fuzz_util::rust_type(&ty, name_counter, &mut declarations);
                    quote!(#ty,)
                })
                .collect::<TokenStream>();

            let rust_results = case
                .results
                .iter()
                .map(|ty| {
                    let ty = component_fuzz_util::rust_type(&ty, name_counter, &mut declarations);
                    quote!(#ty,)
                })
                .collect::<TokenStream>();

            let test = quote!(#index => component_types::#test::<#rust_params (#rust_results)>(
                input,
                {
                    static DECLS: Declarations = Declarations {
                        types: Cow::Borrowed(#types),
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
}
