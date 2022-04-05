//! Implements module serialization.
//!
//! This module implements the serialization format for `wasmtime::Module`.
//! This includes both the binary format of the final artifact as well as
//! validation on ingestion of artifacts.
//!
//! There are two main pieces of data associated with a binary artifact:
//!
//! 1. The compiled module image, currently an ELF file.
//! 2. Compilation metadata for the module, including the `TypeTables`
//!    information. This metadata is validated for compilation settings.
//!
//! Compiled modules are, at this time, represented as an ELF file. This ELF
//! file contains all the necessary data needed to decode a module, and
//! conveniently also handles things like alignment so we can actually directly
//! `mmap` compilation artifacts from disk.
//!
//! With this in mind, the current serialization format is as follows:
//!
//! * First the ELF image for the compiled module starts the artifact. This
//!   helps developers use standard ELF-reading utilities like `objdump` to poke
//!   around and see what's inside the compiled image.
//!
//! * After the ELF file is a number of fields:
//!
//!   1. The `HEADER` value
//!   2. A byte indicating how long the next field is
//!   3. A version string of the length of the previous byte value
//!   4. A `bincode`-encoded `Metadata` structure.
//!
//!   This is hoped to help distinguish easily Wasmtime-based ELF files from
//!   other random ELF files, as well as provide better error messages for
//!   using wasmtime artifacts across versions.
//!
//! Note that the structure of the ELF format is what enables this
//! representation. We can have trailing data after an ELF file which isn't read
//! by any parsing of the ELF itself, which provides a convenient location for
//! the metadata information to go.
//!
//! This format is implemented by the `to_bytes` and `from_mmap` function.

use crate::{Engine, Module, ModuleVersionStrategy};
use anyhow::{anyhow, bail, Context, Result};
use object::read::elf::FileHeader;
use object::Bytes;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;
use wasmtime_environ::{FlagValue, Tunables, TypeTables};
use wasmtime_jit::{subslice_range, CompiledModuleInfo};
use wasmtime_runtime::MmapVec;

const HEADER: &[u8] = b"\0wasmtime-aot";

// This exists because `wasmparser::WasmFeatures` isn't serializable
#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
struct WasmFeatures {
    pub reference_types: bool,
    pub multi_value: bool,
    pub bulk_memory: bool,
    pub component_model: bool,
    pub simd: bool,
    pub threads: bool,
    pub tail_call: bool,
    pub deterministic_only: bool,
    pub multi_memory: bool,
    pub exceptions: bool,
    pub memory64: bool,
    pub relaxed_simd: bool,
    pub extended_const: bool,
}

impl From<&wasmparser::WasmFeatures> for WasmFeatures {
    fn from(other: &wasmparser::WasmFeatures) -> Self {
        let wasmparser::WasmFeatures {
            reference_types,
            multi_value,
            bulk_memory,
            component_model,
            simd,
            threads,
            tail_call,
            deterministic_only,
            multi_memory,
            exceptions,
            memory64,
            relaxed_simd,
            extended_const,

            // Always on; we don't currently have knobs for these.
            mutable_global: _,
            saturating_float_to_int: _,
            sign_extension: _,
        } = *other;

        Self {
            reference_types,
            multi_value,
            bulk_memory,
            component_model,
            simd,
            threads,
            tail_call,
            deterministic_only,
            multi_memory,
            exceptions,
            memory64,
            relaxed_simd,
            extended_const,
        }
    }
}

// This is like `std::borrow::Cow` but it doesn't have a `Clone` bound on `T`
enum MyCow<'a, T> {
    Borrowed(&'a T),
    Owned(T),
}

impl<'a, T> MyCow<'a, T> {
    fn as_ref(&self) -> &T {
        match self {
            MyCow::Owned(val) => val,
            MyCow::Borrowed(val) => val,
        }
    }
    fn unwrap_owned(self) -> T {
        match self {
            MyCow::Owned(val) => val,
            MyCow::Borrowed(_) => unreachable!(),
        }
    }
}

impl<'a, T: Serialize> Serialize for MyCow<'a, T> {
    fn serialize<S>(&self, dst: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        match self {
            MyCow::Borrowed(val) => val.serialize(dst),
            MyCow::Owned(val) => val.serialize(dst),
        }
    }
}

impl<'a, 'b, T: Deserialize<'a>> Deserialize<'a> for MyCow<'b, T> {
    fn deserialize<D>(src: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'a>,
    {
        Ok(MyCow::Owned(T::deserialize(src)?))
    }
}

pub struct SerializedModule<'a> {
    artifacts: MyCow<'a, MmapVec>,
    metadata: Metadata<'a>,
}

#[derive(Serialize, Deserialize)]
struct Metadata<'a> {
    target: String,
    shared_flags: BTreeMap<String, FlagValue>,
    isa_flags: BTreeMap<String, FlagValue>,
    tunables: Tunables,
    features: WasmFeatures,
    types: MyCow<'a, TypeTables>,
}

impl<'a> SerializedModule<'a> {
    #[cfg(compiler)]
    pub fn new(module: &'a Module) -> Self {
        Self::with_data(
            module.engine(),
            MyCow::Borrowed(module.compiled_module().mmap()),
            MyCow::Borrowed(module.types()),
        )
    }

    #[cfg(compiler)]
    pub fn from_artifacts(engine: &Engine, artifacts: &'a MmapVec, types: &'a TypeTables) -> Self {
        Self::with_data(engine, MyCow::Borrowed(artifacts), MyCow::Borrowed(types))
    }

    #[cfg(compiler)]
    fn with_data(
        engine: &Engine,
        artifacts: MyCow<'a, MmapVec>,
        types: MyCow<'a, TypeTables>,
    ) -> Self {
        Self {
            artifacts,
            metadata: Metadata {
                target: engine.compiler().triple().to_string(),
                shared_flags: engine.compiler().flags(),
                isa_flags: engine.compiler().isa_flags(),
                tunables: engine.config().tunables.clone(),
                features: (&engine.config().features).into(),
                types,
            },
        }
    }

    pub fn into_module(self, engine: &Engine) -> Result<Module> {
        let (mmap, info, types) = self.into_parts(engine)?;
        Module::from_parts(engine, mmap, info, Arc::new(types))
    }

    pub fn into_parts(
        mut self,
        engine: &Engine,
    ) -> Result<(MmapVec, Option<CompiledModuleInfo>, TypeTables)> {
        // Verify that the compilation settings in the engine match the
        // compilation settings of the module that's being loaded.
        self.check_triple(engine)?;
        self.check_shared_flags(engine)?;
        self.check_isa_flags(engine)?;

        self.check_tunables(&engine.config().tunables)?;
        self.check_features(&engine.config().features)?;

        let module = self.artifacts.unwrap_owned();

        Ok((module, None, self.metadata.types.unwrap_owned()))
    }

    pub fn to_bytes(&self, version_strat: &ModuleVersionStrategy) -> Result<Vec<u8>> {
        // Start off with a copy of the ELF image.
        let mut ret = self.artifacts.as_ref().to_vec();

        // Append the bincode-encoded `Metadata` section with a few other guards
        // to help give better error messages during deserialization if
        // something goes wrong.
        ret.extend_from_slice(HEADER);
        let version = match version_strat {
            ModuleVersionStrategy::WasmtimeVersion => env!("CARGO_PKG_VERSION"),
            ModuleVersionStrategy::Custom(c) => &c,
            ModuleVersionStrategy::None => "",
        };
        // This precondition is checked in Config::module_version:
        assert!(
            version.len() < 256,
            "package version must be less than 256 bytes"
        );
        ret.push(version.len() as u8);
        ret.extend_from_slice(version.as_bytes());
        bincode::serialize_into(&mut ret, &self.metadata)?;

        Ok(ret)
    }

    pub fn from_bytes(bytes: &[u8], version_strat: &ModuleVersionStrategy) -> Result<Self> {
        Self::from_mmap(MmapVec::from_slice(bytes)?, version_strat)
    }

    pub fn from_file(path: &Path, version_strat: &ModuleVersionStrategy) -> Result<Self> {
        Self::from_mmap(
            MmapVec::from_file(path).with_context(|| {
                format!("failed to create file mapping for: {}", path.display())
            })?,
            version_strat,
        )
    }

    pub fn from_mmap(mmap: MmapVec, version_strat: &ModuleVersionStrategy) -> Result<Self> {
        // First validate that this is at least somewhat an elf file within
        // `mmap` and additionally skip to the end of the elf file to find our
        // metadata.
        let metadata = data_after_elf(&mmap)?;

        // The metadata has a few guards up front which we process first, and
        // eventually this bottoms out in a `bincode::deserialize` call.
        let metadata = metadata
            .strip_prefix(HEADER)
            .ok_or_else(|| anyhow!("bytes are not a compatible serialized wasmtime module"))?;
        if metadata.is_empty() {
            bail!("serialized data data is empty");
        }
        let version_len = metadata[0] as usize;
        if metadata.len() < version_len + 1 {
            bail!("serialized data is malformed");
        }

        match version_strat {
            ModuleVersionStrategy::WasmtimeVersion => {
                let version = std::str::from_utf8(&metadata[1..1 + version_len])?;
                if version != env!("CARGO_PKG_VERSION") {
                    bail!(
                        "Module was compiled with incompatible Wasmtime version '{}'",
                        version
                    );
                }
            }
            ModuleVersionStrategy::Custom(v) => {
                let version = std::str::from_utf8(&metadata[1..1 + version_len])?;
                if version != v {
                    bail!(
                        "Module was compiled with incompatible version '{}'",
                        version
                    );
                }
            }
            ModuleVersionStrategy::None => { /* ignore the version info, accept all */ }
        }

        let metadata = bincode::deserialize::<Metadata>(&metadata[1 + version_len..])
            .context("deserialize compilation artifacts")?;

        return Ok(SerializedModule {
            artifacts: MyCow::Owned(mmap),
            metadata,
        });

        /// This function will return the trailing data behind the ELF file
        /// parsed from `data` which is where we find our metadata section.
        fn data_after_elf(data: &[u8]) -> Result<&[u8]> {
            use object::NativeEndian as NE;
            // There's not actually a great utility for figuring out where
            // the end of an ELF file is in the `object` crate. In lieu of that
            // we build our own which leverages the format of ELF files, which
            // is that the header comes first, that tells us where the section
            // headers are, and for our ELF files the end of the file is the
            // end of the section headers.
            let mut bytes = Bytes(data);
            let header = bytes
                .read::<object::elf::FileHeader64<NE>>()
                .map_err(|()| anyhow!("artifact truncated, can't read header"))?;
            if !header.is_supported() {
                bail!("invalid elf header");
            }
            let sections = header
                .section_headers(NE, data)
                .context("failed to read section headers")?;
            let range = subslice_range(object::bytes_of_slice(sections), data);
            Ok(&data[range.end..])
        }
    }

    fn check_triple(&self, engine: &Engine) -> Result<()> {
        let engine_target = engine.target();
        let module_target =
            target_lexicon::Triple::from_str(&self.metadata.target).map_err(|e| anyhow!(e))?;

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
        for (name, val) in self.metadata.shared_flags.iter() {
            engine
                .check_compatible_with_shared_flag(name, val)
                .map_err(|s| anyhow::Error::msg(s))
                .context("compilation settings of module incompatible with native host")?;
        }
        Ok(())
    }

    fn check_isa_flags(&mut self, engine: &Engine) -> Result<()> {
        for (name, val) in self.metadata.isa_flags.iter() {
            engine
                .check_compatible_with_isa_flag(name, val)
                .map_err(|s| anyhow::Error::msg(s))
                .context("compilation settings of module incompatible with native host")?;
        }
        Ok(())
    }

    fn check_int<T: Eq + std::fmt::Display>(found: T, expected: T, feature: &str) -> Result<()> {
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
            static_memory_bound,
            static_memory_offset_guard_size,
            dynamic_memory_offset_guard_size,
            generate_native_debuginfo,
            parse_wasm_debuginfo,
            consume_fuel,
            epoch_interruption,
            static_memory_bound_is_maximum,
            guard_before_linear_memory,

            // This doesn't affect compilation, it's just a runtime setting.
            dynamic_memory_growth_reserve: _,

            // This does technically affect compilation but modules with/without
            // trap information can be loaded into engines with the opposite
            // setting just fine (it's just a section in the compiled file and
            // whether it's present or not)
            generate_address_map: _,
        } = self.metadata.tunables;

        Self::check_int(
            static_memory_bound,
            other.static_memory_bound,
            "static memory bound",
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

        Ok(())
    }

    fn check_features(&mut self, other: &wasmparser::WasmFeatures) -> Result<()> {
        let WasmFeatures {
            reference_types,
            multi_value,
            bulk_memory,
            component_model,
            simd,
            threads,
            tail_call,
            deterministic_only,
            multi_memory,
            exceptions,
            memory64,
            relaxed_simd,
            extended_const,
        } = self.metadata.features;

        Self::check_bool(
            reference_types,
            other.reference_types,
            "WebAssembly reference types support",
        )?;
        Self::check_bool(
            multi_value,
            other.multi_value,
            "WebAssembly multi-value support",
        )?;
        Self::check_bool(
            bulk_memory,
            other.bulk_memory,
            "WebAssembly bulk memory support",
        )?;
        Self::check_bool(
            component_model,
            other.component_model,
            "WebAssembly component model support",
        )?;
        Self::check_bool(simd, other.simd, "WebAssembly SIMD support")?;
        Self::check_bool(threads, other.threads, "WebAssembly threads support")?;
        Self::check_bool(tail_call, other.tail_call, "WebAssembly tail-call support")?;
        Self::check_bool(
            deterministic_only,
            other.deterministic_only,
            "WebAssembly deterministic-only support",
        )?;
        Self::check_bool(
            multi_memory,
            other.multi_memory,
            "WebAssembly multi-memory support",
        )?;
        Self::check_bool(
            exceptions,
            other.exceptions,
            "WebAssembly exceptions support",
        )?;
        Self::check_bool(
            memory64,
            other.memory64,
            "WebAssembly 64-bit memory support",
        )?;
        Self::check_bool(
            extended_const,
            other.extended_const,
            "WebAssembly extended-const support",
        )?;
        Self::check_bool(
            relaxed_simd,
            other.relaxed_simd,
            "WebAssembly relaxed-simd support",
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::Config;

    #[test]
    fn test_architecture_mismatch() -> Result<()> {
        let engine = Engine::default();
        let module = Module::new(&engine, "(module)")?;

        let mut serialized = SerializedModule::new(&module);
        serialized.metadata.target = "unknown-generic-linux".to_string();

        match serialized.into_module(&engine) {
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
        let module = Module::new(&engine, "(module)")?;

        let mut serialized = SerializedModule::new(&module);
        serialized.metadata.target = format!(
            "{}-generic-unknown",
            target_lexicon::Triple::host().architecture
        );

        match serialized.into_module(&engine) {
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
        let module = Module::new(&engine, "(module)")?;

        let mut serialized = SerializedModule::new(&module);
        serialized
            .metadata
            .shared_flags
            .insert("avoid_div_traps".to_string(), FlagValue::Bool(false));

        match serialized.into_module(&engine) {
            Ok(_) => unreachable!(),
            Err(e) => assert_eq!(
                format!("{:?}", e),
                "\
compilation settings of module incompatible with native host

Caused by:
    setting \"avoid_div_traps\" is configured to Bool(false) which is not supported"
            ),
        }

        Ok(())
    }

    #[test]
    fn test_isa_flags_mismatch() -> Result<()> {
        let engine = Engine::default();
        let module = Module::new(&engine, "(module)")?;

        let mut serialized = SerializedModule::new(&module);

        serialized
            .metadata
            .isa_flags
            .insert("not_a_flag".to_string(), FlagValue::Bool(true));

        match serialized.into_module(&engine) {
            Ok(_) => unreachable!(),
            Err(e) => assert_eq!(
                format!("{:?}", e),
                "\
compilation settings of module incompatible with native host

Caused by:
    cannot test if target-specific flag \"not_a_flag\" is available at runtime",
            ),
        }

        Ok(())
    }

    #[test]
    fn test_tunables_int_mismatch() -> Result<()> {
        let engine = Engine::default();
        let module = Module::new(&engine, "(module)")?;

        let mut serialized = SerializedModule::new(&module);
        serialized.metadata.tunables.static_memory_offset_guard_size = 0;

        match serialized.into_module(&engine) {
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
        let module = Module::new(&engine, "(module)")?;

        let mut serialized = SerializedModule::new(&module);
        serialized.metadata.tunables.epoch_interruption = false;

        match serialized.into_module(&engine) {
            Ok(_) => unreachable!(),
            Err(e) => assert_eq!(
                e.to_string(),
                "Module was compiled without epoch interruption but it is enabled for the host"
            ),
        }

        let mut config = Config::new();
        config.epoch_interruption(false);

        let engine = Engine::new(&config)?;
        let module = Module::new(&engine, "(module)")?;

        let mut serialized = SerializedModule::new(&module);
        serialized.metadata.tunables.epoch_interruption = true;

        match serialized.into_module(&engine) {
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
        config.wasm_simd(true);

        let engine = Engine::new(&config)?;
        let module = Module::new(&engine, "(module)")?;

        let mut serialized = SerializedModule::new(&module);
        serialized.metadata.features.simd = false;

        match serialized.into_module(&engine) {
            Ok(_) => unreachable!(),
            Err(e) => assert_eq!(e.to_string(), "Module was compiled without WebAssembly SIMD support but it is enabled for the host"),
        }

        let mut config = Config::new();
        config.wasm_simd(false);

        let engine = Engine::new(&config)?;
        let module = Module::new(&engine, "(module)")?;

        let mut serialized = SerializedModule::new(&module);
        serialized.metadata.features.simd = true;

        match serialized.into_module(&engine) {
            Ok(_) => unreachable!(),
            Err(e) => assert_eq!(e.to_string(), "Module was compiled with WebAssembly SIMD support but it is not enabled for the host"),
        }

        Ok(())
    }
}
