//! This module implements serialization and deserialization of `Engine`
//! configuration data which is embedded into compiled artifacts of Wasmtime.
//!
//! The data serialized here is used to double-check that when a module is
//! loaded from one host onto another that it's compatible with the target host.
//! Additionally though this data is the first data read from a precompiled
//! artifact so it's "extra hardened" to provide reasonable-ish error messages
//! for mismatching wasmtime versions. Once something successfully deserializes
//! here it's assumed it's meant for this wasmtime so error messages are in
//! general much worse afterwards.
//!
//! Wasmtime AOT artifacts are ELF files so the data for the engine here is
//! stored into a section of the output file. The structure of this section is:
//!
//! 1. A version byte, currently `VERSION`.
//! 2. A byte indicating how long the next field is.
//! 3. A version string of the length of the previous byte value.
//! 4. A `postcard`-encoded `Metadata` structure.
//!
//! This is hoped to help distinguish easily Wasmtime-based ELF files from
//! other random ELF files, as well as provide better error messages for
//! using wasmtime artifacts across versions.

use crate::prelude::*;
use crate::{Engine, ModuleVersionStrategy, Precompiled};
use core::str::FromStr;
use object::endian::NativeEndian;
#[cfg(any(feature = "cranelift", feature = "winch"))]
use object::write::{Object, StandardSegment};
use object::{read::elf::ElfFile64, FileFlags, Object as _, ObjectSection, SectionKind};
use serde_derive::{Deserialize, Serialize};
use wasmtime_environ::obj;
use wasmtime_environ::{FlagValue, ObjectKind, Tunables};

const VERSION: u8 = 0;

/// Verifies that the serialized engine in `mmap` is compatible with the
/// `engine` provided.
///
/// This function will verify that the `mmap` provided can be deserialized
/// successfully and that the contents are all compatible with the `engine`
/// provided here, notably compatible wasm features are enabled, compatible
/// compiler options, etc. If a mismatch is found and the compilation metadata
/// specified is incompatible then an error is returned.
pub fn check_compatible(engine: &Engine, mmap: &[u8], expected: ObjectKind) -> Result<()> {
    // Parse the input `mmap` as an ELF file and see if the header matches the
    // Wasmtime-generated header. This includes a Wasmtime-specific `os_abi` and
    // the `e_flags` field should indicate whether `expected` matches or not.
    //
    // Note that errors generated here could mean that a precompiled module was
    // loaded as a component, or vice versa, both of which aren't supposed to
    // work.
    //
    // Ideally we'd only `File::parse` once and avoid the linear
    // `section_by_name` search here but the general serialization code isn't
    // structured well enough to make this easy and additionally it's not really
    // a perf issue right now so doing that is left for another day's
    // refactoring.
    let obj = ElfFile64::<NativeEndian>::parse(mmap)
        .err2anyhow()
        .context("failed to parse precompiled artifact as an ELF")?;
    let expected_e_flags = match expected {
        ObjectKind::Module => obj::EF_WASMTIME_MODULE,
        ObjectKind::Component => obj::EF_WASMTIME_COMPONENT,
    };
    match obj.flags() {
        FileFlags::Elf {
            os_abi: obj::ELFOSABI_WASMTIME,
            abi_version: 0,
            e_flags,
        } if e_flags == expected_e_flags => {}
        _ => bail!("incompatible object file format"),
    }

    let data = obj
        .section_by_name(obj::ELF_WASM_ENGINE)
        .ok_or_else(|| anyhow!("failed to find section `{}`", obj::ELF_WASM_ENGINE))?
        .data()
        .err2anyhow()?;
    let (first, data) = data
        .split_first()
        .ok_or_else(|| anyhow!("invalid engine section"))?;
    if *first != VERSION {
        bail!("mismatched version in engine section");
    }
    let (len, data) = data
        .split_first()
        .ok_or_else(|| anyhow!("invalid engine section"))?;
    let len = usize::from(*len);
    let (version, data) = if data.len() < len + 1 {
        bail!("engine section too small")
    } else {
        data.split_at(len)
    };

    match &engine.config().module_version {
        ModuleVersionStrategy::WasmtimeVersion => {
            let version = core::str::from_utf8(version).err2anyhow()?;
            if version != env!("CARGO_PKG_VERSION") {
                bail!(
                    "Module was compiled with incompatible Wasmtime version '{}'",
                    version
                );
            }
        }
        ModuleVersionStrategy::Custom(v) => {
            let version = core::str::from_utf8(&version).err2anyhow()?;
            if version != v {
                bail!(
                    "Module was compiled with incompatible version '{}'",
                    version
                );
            }
        }
        ModuleVersionStrategy::None => { /* ignore the version info, accept all */ }
    }
    postcard::from_bytes::<Metadata<'_>>(data)
        .err2anyhow()?
        .check_compatible(engine)
}

#[cfg(any(feature = "cranelift", feature = "winch"))]
pub fn append_compiler_info(engine: &Engine, obj: &mut Object<'_>, metadata: &Metadata<'_>) {
    let section = obj.add_section(
        obj.segment_name(StandardSegment::Data).to_vec(),
        obj::ELF_WASM_ENGINE.as_bytes().to_vec(),
        SectionKind::ReadOnlyData,
    );
    let mut data = Vec::new();
    data.push(VERSION);
    let version = match &engine.config().module_version {
        ModuleVersionStrategy::WasmtimeVersion => env!("CARGO_PKG_VERSION"),
        ModuleVersionStrategy::Custom(c) => c,
        ModuleVersionStrategy::None => "",
    };
    // This precondition is checked in Config::module_version:
    assert!(
        version.len() < 256,
        "package version must be less than 256 bytes"
    );
    data.push(version.len() as u8);
    data.extend_from_slice(version.as_bytes());
    data.extend(postcard::to_allocvec(metadata).unwrap());
    obj.set_section_data(section, data, 1);
}

fn detect_precompiled<'data, R: object::ReadRef<'data>>(
    obj: ElfFile64<'data, NativeEndian, R>,
) -> Option<Precompiled> {
    match obj.flags() {
        FileFlags::Elf {
            os_abi: obj::ELFOSABI_WASMTIME,
            abi_version: 0,
            e_flags: obj::EF_WASMTIME_MODULE,
        } => Some(Precompiled::Module),
        FileFlags::Elf {
            os_abi: obj::ELFOSABI_WASMTIME,
            abi_version: 0,
            e_flags: obj::EF_WASMTIME_COMPONENT,
        } => Some(Precompiled::Component),
        _ => None,
    }
}

pub fn detect_precompiled_bytes(bytes: &[u8]) -> Option<Precompiled> {
    detect_precompiled(ElfFile64::parse(bytes).ok()?)
}

#[cfg(feature = "std")]
pub fn detect_precompiled_file(path: impl AsRef<std::path::Path>) -> Result<Option<Precompiled>> {
    let read_cache = object::ReadCache::new(std::fs::File::open(path)?);
    let obj = ElfFile64::parse(&read_cache)?;
    Ok(detect_precompiled(obj))
}

#[derive(Serialize, Deserialize)]
pub struct Metadata<'a> {
    target: String,
    #[serde(borrow)]
    shared_flags: Vec<(&'a str, FlagValue<'a>)>,
    #[serde(borrow)]
    isa_flags: Vec<(&'a str, FlagValue<'a>)>,
    tunables: Tunables,
    features: WasmFeatures,
}

// This exists because `wasmparser::WasmFeatures` isn't serializable
#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
struct WasmFeatures {
    reference_types: bool,
    multi_value: bool,
    bulk_memory: bool,
    component_model: bool,
    simd: bool,
    tail_call: bool,
    threads: bool,
    multi_memory: bool,
    exceptions: bool,
    memory64: bool,
    relaxed_simd: bool,
    extended_const: bool,
    function_references: bool,
    gc: bool,
    custom_page_sizes: bool,
    component_model_more_flags: bool,
    component_model_multiple_returns: bool,
}

impl Metadata<'_> {
    #[cfg(any(feature = "cranelift", feature = "winch"))]
    pub fn new(engine: &Engine) -> Metadata<'static> {
        let wasmparser::WasmFeaturesInflated {
            reference_types,
            multi_value,
            bulk_memory,
            component_model,
            simd,
            threads,
            tail_call,
            multi_memory,
            exceptions,
            memory64,
            relaxed_simd,
            extended_const,
            memory_control,
            function_references,
            gc,
            custom_page_sizes,
            shared_everything_threads,
            component_model_values,
            component_model_nested_names,
            component_model_more_flags,
            component_model_multiple_returns,
            legacy_exceptions,

            // Always on; we don't currently have knobs for these.
            mutable_global: _,
            saturating_float_to_int: _,
            sign_extension: _,
            floats: _,
        } = engine.config().features.inflate();

        // These features are not implemented in Wasmtime yet. We match on them
        // above so that once we do implement support for them, we won't
        // silently ignore them during serialization.
        assert!(!memory_control);
        assert!(!component_model_values);
        assert!(!component_model_nested_names);
        assert!(!shared_everything_threads);
        assert!(!legacy_exceptions);

        Metadata {
            target: engine.compiler().triple().to_string(),
            shared_flags: engine.compiler().flags(),
            isa_flags: engine.compiler().isa_flags(),
            tunables: engine.tunables().clone(),
            features: WasmFeatures {
                reference_types,
                multi_value,
                bulk_memory,
                component_model,
                simd,
                threads,
                tail_call,
                multi_memory,
                exceptions,
                memory64,
                relaxed_simd,
                extended_const,
                function_references,
                gc,
                custom_page_sizes,
                component_model_more_flags,
                component_model_multiple_returns,
            },
        }
    }

    fn check_compatible(mut self, engine: &Engine) -> Result<()> {
        self.check_triple(engine)?;
        self.check_shared_flags(engine)?;
        self.check_isa_flags(engine)?;
        self.check_tunables(&engine.tunables())?;
        self.check_features(&engine.config().features)?;
        Ok(())
    }

    fn check_triple(&self, engine: &Engine) -> Result<()> {
        let engine_target = engine.target();
        let module_target =
            target_lexicon::Triple::from_str(&self.target).map_err(|e| anyhow!(e))?;

        if module_target.architecture != engine_target.architecture {
            bail!(
                "Module was compiled for architecture '{}'",
                module_target.architecture
            );
        }

        if module_target.operating_system != engine_target.operating_system {
            bail!(
                "Module was compiled for operating system '{}'",
                module_target.operating_system
            );
        }

        Ok(())
    }

    fn check_shared_flags(&mut self, engine: &Engine) -> Result<()> {
        for (name, val) in self.shared_flags.iter() {
            engine
                .check_compatible_with_shared_flag(name, val)
                .map_err(|s| anyhow::Error::msg(s))
                .context("compilation settings of module incompatible with native host")?;
        }
        Ok(())
    }

    fn check_isa_flags(&mut self, engine: &Engine) -> Result<()> {
        for (name, val) in self.isa_flags.iter() {
            engine
                .check_compatible_with_isa_flag(name, val)
                .map_err(|s| anyhow::Error::msg(s))
                .context("compilation settings of module incompatible with native host")?;
        }
        Ok(())
    }

    fn check_int<T: Eq + core::fmt::Display>(found: T, expected: T, feature: &str) -> Result<()> {
        if found == expected {
            return Ok(());
        }

        bail!(
            "Module was compiled with a {} of '{}' but '{}' is expected for the host",
            feature,
            found,
            expected
        );
    }

    fn check_bool(found: bool, expected: bool, feature: &str) -> Result<()> {
        if found == expected {
            return Ok(());
        }

        bail!(
            "Module was compiled {} {} but it {} enabled for the host",
            if found { "with" } else { "without" },
            feature,
            if expected { "is" } else { "is not" }
        );
    }

    fn check_tunables(&mut self, other: &Tunables) -> Result<()> {
        let Tunables {
            static_memory_reservation,
            static_memory_offset_guard_size,
            dynamic_memory_offset_guard_size,
            generate_native_debuginfo,
            parse_wasm_debuginfo,
            consume_fuel,
            epoch_interruption,
            static_memory_bound_is_maximum,
            guard_before_linear_memory,
            table_lazy_init,
            relaxed_simd_deterministic,
            tail_callable,
            winch_callable,

            // This doesn't affect compilation, it's just a runtime setting.
            dynamic_memory_growth_reserve: _,

            // This does technically affect compilation but modules with/without
            // trap information can be loaded into engines with the opposite
            // setting just fine (it's just a section in the compiled file and
            // whether it's present or not)
            generate_address_map: _,

            // Just a debugging aid, doesn't affect functionality at all.
            debug_adapter_modules: _,
        } = self.tunables;

        Self::check_int(
            static_memory_reservation,
            other.static_memory_reservation,
            "static memory reservation",
        )?;
        Self::check_int(
            static_memory_offset_guard_size,
            other.static_memory_offset_guard_size,
            "static memory guard size",
        )?;
        Self::check_int(
            dynamic_memory_offset_guard_size,
            other.dynamic_memory_offset_guard_size,
            "dynamic memory guard size",
        )?;
        Self::check_bool(
            generate_native_debuginfo,
            other.generate_native_debuginfo,
            "debug information support",
        )?;
        Self::check_bool(
            parse_wasm_debuginfo,
            other.parse_wasm_debuginfo,
            "WebAssembly backtrace support",
        )?;
        Self::check_bool(consume_fuel, other.consume_fuel, "fuel support")?;
        Self::check_bool(
            epoch_interruption,
            other.epoch_interruption,
            "epoch interruption",
        )?;
        Self::check_bool(
            static_memory_bound_is_maximum,
            other.static_memory_bound_is_maximum,
            "pooling allocation support",
        )?;
        Self::check_bool(
            guard_before_linear_memory,
            other.guard_before_linear_memory,
            "guard before linear memory",
        )?;
        Self::check_bool(table_lazy_init, other.table_lazy_init, "table lazy init")?;
        Self::check_bool(
            relaxed_simd_deterministic,
            other.relaxed_simd_deterministic,
            "relaxed simd deterministic semantics",
        )?;
        Self::check_bool(tail_callable, other.tail_callable, "WebAssembly tail calls")?;
        Self::check_bool(
            winch_callable,
            other.winch_callable,
            "Winch calling convention",
        )?;

        Ok(())
    }

    fn check_cfg_bool(
        cfg: bool,
        cfg_str: &str,
        found: bool,
        expected: bool,
        feature: &str,
    ) -> Result<()> {
        if cfg {
            Self::check_bool(found, expected, feature)
        } else {
            assert!(!expected);
            ensure!(
                !found,
                "Module was compiled with {feature} but support in the host \
                 was disabled at compile time because the `{cfg_str}` Cargo \
                 feature was not enabled",
            );
            Ok(())
        }
    }

    fn check_features(&mut self, other: &wasmparser::WasmFeatures) -> Result<()> {
        let WasmFeatures {
            reference_types,
            multi_value,
            bulk_memory,
            component_model,
            simd,
            tail_call,
            threads,
            multi_memory,
            exceptions,
            memory64,
            relaxed_simd,
            extended_const,
            function_references,
            gc,
            custom_page_sizes,
            component_model_more_flags,
            component_model_multiple_returns,
        } = self.features;

        use wasmparser::WasmFeatures as F;
        Self::check_cfg_bool(
            cfg!(feature = "gc"),
            "gc",
            reference_types,
            other.contains(F::REFERENCE_TYPES),
            "WebAssembly reference types support",
        )?;
        Self::check_cfg_bool(
            cfg!(feature = "gc"),
            "gc",
            function_references,
            other.contains(F::FUNCTION_REFERENCES),
            "WebAssembly function-references support",
        )?;
        Self::check_cfg_bool(
            cfg!(feature = "gc"),
            "gc",
            gc,
            other.contains(F::GC),
            "WebAssembly garbage collection support",
        )?;

        Self::check_bool(
            multi_value,
            other.contains(F::MULTI_VALUE),
            "WebAssembly multi-value support",
        )?;
        Self::check_bool(
            bulk_memory,
            other.contains(F::BULK_MEMORY),
            "WebAssembly bulk memory support",
        )?;
        Self::check_bool(
            component_model,
            other.contains(F::COMPONENT_MODEL),
            "WebAssembly component model support",
        )?;
        Self::check_bool(simd, other.contains(F::SIMD), "WebAssembly SIMD support")?;
        Self::check_bool(
            tail_call,
            other.contains(F::TAIL_CALL),
            "WebAssembly tail calls support",
        )?;
        Self::check_bool(
            threads,
            other.contains(F::THREADS),
            "WebAssembly threads support",
        )?;
        Self::check_bool(
            multi_memory,
            other.contains(F::MULTI_MEMORY),
            "WebAssembly multi-memory support",
        )?;
        Self::check_bool(
            exceptions,
            other.contains(F::EXCEPTIONS),
            "WebAssembly exceptions support",
        )?;
        Self::check_bool(
            memory64,
            other.contains(F::MEMORY64),
            "WebAssembly 64-bit memory support",
        )?;
        Self::check_bool(
            extended_const,
            other.contains(F::EXTENDED_CONST),
            "WebAssembly extended-const support",
        )?;
        Self::check_bool(
            relaxed_simd,
            other.contains(F::RELAXED_SIMD),
            "WebAssembly relaxed-simd support",
        )?;
        Self::check_bool(
            custom_page_sizes,
            other.contains(F::CUSTOM_PAGE_SIZES),
            "WebAssembly custom-page-sizes support",
        )?;
        Self::check_bool(
            component_model_more_flags,
            other.contains(F::COMPONENT_MODEL_MORE_FLAGS),
            "WebAssembly component model support for more than 32 flags",
        )?;
        Self::check_bool(
            component_model_multiple_returns,
            other.contains(F::COMPONENT_MODEL_MULTIPLE_RETURNS),
            "WebAssembly component model support for multiple returns",
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{Config, Module, OptLevel};
    use std::{
        collections::hash_map::DefaultHasher,
        hash::{Hash, Hasher},
    };
    use tempfile::TempDir;

    #[test]
    fn test_architecture_mismatch() -> Result<()> {
        let engine = Engine::default();
        let mut metadata = Metadata::new(&engine);
        metadata.target = "unknown-generic-linux".to_string();

        match metadata.check_compatible(&engine) {
            Ok(_) => unreachable!(),
            Err(e) => assert_eq!(
                e.to_string(),
                "Module was compiled for architecture 'unknown'",
            ),
        }

        Ok(())
    }

    #[test]
    fn test_os_mismatch() -> Result<()> {
        let engine = Engine::default();
        let mut metadata = Metadata::new(&engine);

        metadata.target = format!(
            "{}-generic-unknown",
            target_lexicon::Triple::host().architecture
        );

        match metadata.check_compatible(&engine) {
            Ok(_) => unreachable!(),
            Err(e) => assert_eq!(
                e.to_string(),
                "Module was compiled for operating system 'unknown'",
            ),
        }

        Ok(())
    }

    #[test]
    fn test_cranelift_flags_mismatch() -> Result<()> {
        let engine = Engine::default();
        let mut metadata = Metadata::new(&engine);

        metadata
            .shared_flags
            .push(("preserve_frame_pointers", FlagValue::Bool(false)));

        match metadata.check_compatible(&engine) {
            Ok(_) => unreachable!(),
            Err(e) => assert!(format!("{:?}", e).starts_with(
                "\
compilation settings of module incompatible with native host

Caused by:
    setting \"preserve_frame_pointers\" is configured to Bool(false) which is not supported"
            )),
        }

        Ok(())
    }

    #[test]
    fn test_isa_flags_mismatch() -> Result<()> {
        let engine = Engine::default();
        let mut metadata = Metadata::new(&engine);

        metadata
            .isa_flags
            .push(("not_a_flag", FlagValue::Bool(true)));

        match metadata.check_compatible(&engine) {
            Ok(_) => unreachable!(),
            Err(e) => assert!(
                format!("{e:?}").starts_with(
                    "\
compilation settings of module incompatible with native host

Caused by:
    don't know how to test for target-specific flag \"not_a_flag\" at runtime",
                ),
                "bad error {e:?}",
            ),
        }

        Ok(())
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn test_tunables_int_mismatch() -> Result<()> {
        let engine = Engine::default();
        let mut metadata = Metadata::new(&engine);

        metadata.tunables.static_memory_offset_guard_size = 0;

        match metadata.check_compatible(&engine) {
            Ok(_) => unreachable!(),
            Err(e) => assert_eq!(e.to_string(), "Module was compiled with a static memory guard size of '0' but '2147483648' is expected for the host"),
        }

        Ok(())
    }

    #[test]
    fn test_tunables_bool_mismatch() -> Result<()> {
        let mut config = Config::new();
        config.epoch_interruption(true);

        let engine = Engine::new(&config)?;
        let mut metadata = Metadata::new(&engine);
        metadata.tunables.epoch_interruption = false;

        match metadata.check_compatible(&engine) {
            Ok(_) => unreachable!(),
            Err(e) => assert_eq!(
                e.to_string(),
                "Module was compiled without epoch interruption but it is enabled for the host"
            ),
        }

        let mut config = Config::new();
        config.epoch_interruption(false);

        let engine = Engine::new(&config)?;
        let mut metadata = Metadata::new(&engine);
        metadata.tunables.epoch_interruption = true;

        match metadata.check_compatible(&engine) {
            Ok(_) => unreachable!(),
            Err(e) => assert_eq!(
                e.to_string(),
                "Module was compiled with epoch interruption but it is not enabled for the host"
            ),
        }

        Ok(())
    }

    #[test]
    fn test_feature_mismatch() -> Result<()> {
        let mut config = Config::new();
        config.wasm_threads(true);

        let engine = Engine::new(&config)?;
        let mut metadata = Metadata::new(&engine);
        metadata.features.threads = false;

        match metadata.check_compatible(&engine) {
            Ok(_) => unreachable!(),
            Err(e) => assert_eq!(e.to_string(), "Module was compiled without WebAssembly threads support but it is enabled for the host"),
        }

        let mut config = Config::new();
        config.wasm_threads(false);

        let engine = Engine::new(&config)?;
        let mut metadata = Metadata::new(&engine);
        metadata.features.threads = true;

        match metadata.check_compatible(&engine) {
            Ok(_) => unreachable!(),
            Err(e) => assert_eq!(e.to_string(), "Module was compiled with WebAssembly threads support but it is not enabled for the host"),
        }

        Ok(())
    }

    #[test]
    fn engine_weak_upgrades() {
        let engine = Engine::default();
        let weak = engine.weak();
        weak.upgrade()
            .expect("engine is still alive, so weak reference can upgrade");
        drop(engine);
        assert!(
            weak.upgrade().is_none(),
            "engine was dropped, so weak reference cannot upgrade"
        );
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn cache_accounts_for_opt_level() -> Result<()> {
        let td = TempDir::new()?;
        let config_path = td.path().join("config.toml");
        std::fs::write(
            &config_path,
            &format!(
                "
                    [cache]
                    enabled = true
                    directory = '{}'
                ",
                td.path().join("cache").display()
            ),
        )?;
        let mut cfg = Config::new();
        cfg.cranelift_opt_level(OptLevel::None)
            .cache_config_load(&config_path)?;
        let engine = Engine::new(&cfg)?;
        Module::new(&engine, "(module (func))")?;
        assert_eq!(engine.config().cache_config.cache_hits(), 0);
        assert_eq!(engine.config().cache_config.cache_misses(), 1);
        Module::new(&engine, "(module (func))")?;
        assert_eq!(engine.config().cache_config.cache_hits(), 1);
        assert_eq!(engine.config().cache_config.cache_misses(), 1);

        let mut cfg = Config::new();
        cfg.cranelift_opt_level(OptLevel::Speed)
            .cache_config_load(&config_path)?;
        let engine = Engine::new(&cfg)?;
        Module::new(&engine, "(module (func))")?;
        assert_eq!(engine.config().cache_config.cache_hits(), 0);
        assert_eq!(engine.config().cache_config.cache_misses(), 1);
        Module::new(&engine, "(module (func))")?;
        assert_eq!(engine.config().cache_config.cache_hits(), 1);
        assert_eq!(engine.config().cache_config.cache_misses(), 1);

        let mut cfg = Config::new();
        cfg.cranelift_opt_level(OptLevel::SpeedAndSize)
            .cache_config_load(&config_path)?;
        let engine = Engine::new(&cfg)?;
        Module::new(&engine, "(module (func))")?;
        assert_eq!(engine.config().cache_config.cache_hits(), 0);
        assert_eq!(engine.config().cache_config.cache_misses(), 1);
        Module::new(&engine, "(module (func))")?;
        assert_eq!(engine.config().cache_config.cache_hits(), 1);
        assert_eq!(engine.config().cache_config.cache_misses(), 1);

        let mut cfg = Config::new();
        cfg.debug_info(true).cache_config_load(&config_path)?;
        let engine = Engine::new(&cfg)?;
        Module::new(&engine, "(module (func))")?;
        assert_eq!(engine.config().cache_config.cache_hits(), 0);
        assert_eq!(engine.config().cache_config.cache_misses(), 1);
        Module::new(&engine, "(module (func))")?;
        assert_eq!(engine.config().cache_config.cache_hits(), 1);
        assert_eq!(engine.config().cache_config.cache_misses(), 1);

        Ok(())
    }

    #[test]
    fn precompile_compatibility_key_accounts_for_opt_level() {
        fn hash_for_config(cfg: &Config) -> u64 {
            let engine = Engine::new(cfg).expect("Config should be valid");
            let mut hasher = DefaultHasher::new();
            engine.precompile_compatibility_hash().hash(&mut hasher);
            hasher.finish()
        }
        let mut cfg = Config::new();
        cfg.cranelift_opt_level(OptLevel::None);
        let opt_none_hash = hash_for_config(&cfg);
        cfg.cranelift_opt_level(OptLevel::Speed);
        let opt_speed_hash = hash_for_config(&cfg);
        assert_ne!(opt_none_hash, opt_speed_hash)
    }

    #[test]
    fn precompile_compatibility_key_accounts_for_module_version_strategy() -> Result<()> {
        fn hash_for_config(cfg: &Config) -> u64 {
            let engine = Engine::new(cfg).expect("Config should be valid");
            let mut hasher = DefaultHasher::new();
            engine.precompile_compatibility_hash().hash(&mut hasher);
            hasher.finish()
        }
        let mut cfg_custom_version = Config::new();
        cfg_custom_version.module_version(ModuleVersionStrategy::Custom("1.0.1111".to_string()))?;
        let custom_version_hash = hash_for_config(&cfg_custom_version);

        let mut cfg_default_version = Config::new();
        cfg_default_version.module_version(ModuleVersionStrategy::WasmtimeVersion)?;
        let default_version_hash = hash_for_config(&cfg_default_version);

        let mut cfg_none_version = Config::new();
        cfg_none_version.module_version(ModuleVersionStrategy::None)?;
        let none_version_hash = hash_for_config(&cfg_none_version);

        assert_ne!(custom_version_hash, default_version_hash);
        assert_ne!(custom_version_hash, none_version_hash);
        assert_ne!(default_version_hash, none_version_hash);

        Ok(())
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    #[cfg(feature = "component-model")]
    fn components_are_cached() -> Result<()> {
        use crate::component::Component;

        let td = TempDir::new()?;
        let config_path = td.path().join("config.toml");
        std::fs::write(
            &config_path,
            &format!(
                "
                    [cache]
                    enabled = true
                    directory = '{}'
                ",
                td.path().join("cache").display()
            ),
        )?;
        let mut cfg = Config::new();
        cfg.cache_config_load(&config_path)?;
        let engine = Engine::new(&cfg)?;
        Component::new(&engine, "(component (core module (func)))")?;
        assert_eq!(engine.config().cache_config.cache_hits(), 0);
        assert_eq!(engine.config().cache_config.cache_misses(), 1);
        Component::new(&engine, "(component (core module (func)))")?;
        assert_eq!(engine.config().cache_config.cache_hits(), 1);
        assert_eq!(engine.config().cache_config.cache_misses(), 1);

        Ok(())
    }
}
