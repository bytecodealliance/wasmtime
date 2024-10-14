//! Generate Cranelift compiler settings.

use arbitrary::{Arbitrary, Unstructured};

/// Choose between matching the host architecture or a cross-compilation target.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum CodegenSettings {
    /// Use the host's feature set.
    Native,
    /// Generate a modified flag set for the current host.
    #[allow(dead_code)]
    Target {
        /// The target triple of the host.
        target: String,
        /// A list of CPU features to enable, e.g., `("has_avx", "false")`.
        flags: Vec<(String, String)>,
    },
}

impl CodegenSettings {
    /// Configure Wasmtime with these codegen settings.
    pub fn configure(&self, config: &mut wasmtime::Config) {
        match self {
            CodegenSettings::Native => {}
            CodegenSettings::Target { target, flags } => {
                config.target(target).unwrap();
                for (key, value) in flags {
                    unsafe {
                        config.cranelift_flag_set(key, value);
                    }
                }
            }
        }
    }
}

impl<'a> Arbitrary<'a> for CodegenSettings {
    #[allow(unused_macros, unused_variables)]
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        // Helper macro to enable clif features based on what the native host
        // supports. If the input says to enable a feature and the host doesn't
        // support it then that test case is rejected with a warning.
        //
        // Note that this specifically consumes bytes from the fuzz input for
        // features for all targets, discarding anything which isn't applicable
        // to the current target. The theory behind this is that most fuzz bugs
        // won't be related to this feature selection so by consistently
        // consuming input irrespective of the current platform reproducing fuzz
        // bugs should be easier between different architectures.
        macro_rules! target_features {
            (
                $(
                    $arch:tt => {
                        test:$test:ident,
                        $(std: $std:tt => clif: $clif:tt $(ratio: $a:tt in $b:tt)?,)*
                    },
                )*
            ) => ({
                let mut flags = Vec::new();
                $( // for each `$arch`
                    $( // for each `$std`/`$clif` pair
                        // Use the input to generate whether `$clif` will be
                        // enabled. By default this is a 1 in 2 chance but each
                        // feature supports a custom ratio as well which shadows
                        // the (low, hi)
                        let (low, hi) = (1, 2);
                        $(let (low, hi) = ($a, $b);)?
                        let enable = u.ratio(low, hi)?;

                        // If we're actually on the relevant platform and the
                        // feature is enabled be sure to check that this host
                        // supports it. If the host doesn't support it then
                        // print a warning and return an error because this fuzz
                        // input must be discarded.
                        #[cfg(target_arch = $arch)]
                        if enable && !std::arch::$test!($std) {
                            log::warn!("want to enable clif `{}` but host doesn't support it",
                                $clif);
                            return Err(arbitrary::Error::EmptyChoose)
                        }

                        // And finally actually push the feature into the set of
                        // flags to enable, but only if we're on the right
                        // architecture.
                        if cfg!(target_arch = $arch) {
                            flags.push((
                                $clif.to_string(),
                                enable.to_string(),
                            ));
                        }
                    )*
                )*
                flags
            })
        }
        if u.ratio(1, 10)? {
            let flags = target_features! {
                "x86_64" => {
                    test: is_x86_feature_detected,

                    std:"cmpxchg16b" => clif:"has_cmpxchg16b",
                    std:"sse3" => clif:"has_sse3",
                    std:"ssse3" => clif:"has_ssse3",
                    std:"sse4.1" => clif:"has_sse41",
                    std:"sse4.2" => clif:"has_sse42",
                    std:"popcnt" => clif:"has_popcnt",
                    std:"avx" => clif:"has_avx",
                    std:"avx2" => clif:"has_avx2",
                    std:"fma" => clif:"has_fma",
                    std:"bmi1" => clif:"has_bmi1",
                    std:"bmi2" => clif:"has_bmi2",
                    std:"lzcnt" => clif:"has_lzcnt",

                    // not a lot of of cpus support avx512 so these are weighted
                    // to get enabled much less frequently.
                    std:"avx512bitalg" => clif:"has_avx512bitalg" ratio:1 in 1000,
                    std:"avx512dq" => clif:"has_avx512dq" ratio: 1 in 1000,
                    std:"avx512f" => clif:"has_avx512f" ratio: 1 in 1000,
                    std:"avx512vl" => clif:"has_avx512vl" ratio: 1 in 1000,
                    std:"avx512vbmi" => clif:"has_avx512vbmi" ratio: 1 in 1000,
                },
                "aarch64" => {
                    test: is_aarch64_feature_detected,

                    std: "bti" => clif: "use_bti",
                    std: "lse" => clif: "has_lse",
                    std: "fp16" => clif: "has_fp16",
                    // even though the natural correspondence seems to be
                    // between "paca" and "has_pauth", the latter has no effect
                    // in isolation, so we actually use the setting that affects
                    // code generation
                    std: "paca" => clif: "sign_return_address",
                    // "paca" and "pacg" check for the same underlying
                    // architectural feature, so we use the latter to cover more
                    // code generation settings, of which we have chosen the one
                    // with the most significant effect
                    std: "pacg" => clif: "sign_return_address_all" ratio: 1 in 2,
                },
            };
            return Ok(CodegenSettings::Target {
                target: target_lexicon::Triple::host().to_string(),
                flags,
            });
        }
        Ok(CodegenSettings::Native)
    }
}
