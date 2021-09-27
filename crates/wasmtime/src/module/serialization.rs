//! Implements module serialization.
//!
//! This module implements the serialization format for `wasmtime::Module`.
//! This includes both the binary format of the final artifact as well as
//! validation on ingestion of artifacts.
//!
//! There are two main pieces of data associated with a binary artifact:
//!
//! 1. A list of compiled modules. The reason this is a list as opposed to one
//!    singular module is that a module-linking module may encompass a number
//!    of other modules.
//! 2. Compilation metadata shared by all modules, including the global
//!    `TypeTables` information. This metadata is validated for compilation
//!    settings and also has information shared by all modules (such as the
//!    shared `TypeTables`).
//!
//! Compiled modules are, at this time, represented as an ELF file. This ELF
//! file contains all the necessary data needed to decode each individual
//! module, and conveniently also handles things like alignment so we can
//! actually directly `mmap` compilation artifacts from disk.
//!
//! With all this in mind, the current serialization format is as follows:
//!
//! * The first, primary, module starts the final artifact. This means that the
//!   final artifact is actually, and conveniently, a valid ELF file. ELF files
//!   don't place any restrictions on data coming after the ELF file itself,
//!   so that's where everything else will go. Another reason for using this
//!   format is that our compilation artifacts are then consumable by standard
//!   debugging tools like `objdump` to poke around and see what's what.
//!
//! * Next, all other modules are encoded. Each module has its own alignment,
//!   though, so modules aren't simply concatenated. Instead directly after an
//!   ELF file there is a 64-bit little-endian integer which is the offset,
//!   from the end of the previous ELF file, to the next ELF file.
//!
//! * Finally, once all modules have been encoded (there's always at least
//!   one), the 8-byte value `u64::MAX` is encoded. Following this is a
//!   number of fields:
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
//! This format is implemented by the `to_bytes` and `from_mmap` function.

use crate::{Engine, Module, ModuleVersionStrategy};
use anyhow::{anyhow, bail, Context, Result};
use object::read::elf::FileHeader;
use object::{Bytes, File, Object, ObjectSection};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::convert::TryFrom;
use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;
use wasmtime_environ::{Compiler, FlagValue, Tunables};
use wasmtime_jit::{subslice_range, CompiledModule, CompiledModuleInfo, MmapVec, TypeTables};

const HEADER: &[u8] = b"\0wasmtime-aot";

// This exists because `wasmparser::WasmFeatures` isn't serializable
#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
struct WasmFeatures {
    pub reference_types: bool,
    pub multi_value: bool,
    pub bulk_memory: bool,
    pub module_linking: bool,
    pub simd: bool,
    pub threads: bool,
    pub tail_call: bool,
    pub deterministic_only: bool,
    pub multi_memory: bool,
    pub exceptions: bool,
    pub memory64: bool,
}

impl From<&wasmparser::WasmFeatures> for WasmFeatures {
    fn from(other: &wasmparser::WasmFeatures) -> Self {
        let wasmparser::WasmFeatures {
            reference_types,
            multi_value,
            bulk_memory,
            module_linking,
            simd,
            threads,
            tail_call,
            deterministic_only,
            multi_memory,
            exceptions,
            memory64,
        } = other;

        Self {
            reference_types: *reference_types,
            multi_value: *multi_value,
            bulk_memory: *bulk_memory,
            module_linking: *module_linking,
            simd: *simd,
            threads: *threads,
            tail_call: *tail_call,
            deterministic_only: *deterministic_only,
            multi_memory: *multi_memory,
            exceptions: *exceptions,
            memory64: *memory64,
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

/// A small helper struct for serialized module upvars.
#[derive(Serialize, Deserialize)]
pub struct SerializedModuleUpvar {
    /// The module's index into the compilation artifact.
    pub index: usize,
    /// Indexes into the list of all compilation artifacts for this module.
    pub artifact_upvars: Vec<usize>,
    /// Closed-over module values that are also needed for this module.
    pub module_upvars: Vec<SerializedModuleUpvar>,
}

impl SerializedModuleUpvar {
    pub fn new(module: &Module, artifacts: &[Arc<CompiledModule>]) -> Self {
        // TODO: improve upon the linear searches in the artifact list
        let index = artifacts
            .iter()
            .position(|a| Arc::as_ptr(a) == Arc::as_ptr(&module.inner.module))
            .expect("module should be in artifacts list");

        SerializedModuleUpvar {
            index,
            artifact_upvars: module
                .inner
                .artifact_upvars
                .iter()
                .map(|m| {
                    artifacts
                        .iter()
                        .position(|a| Arc::as_ptr(a) == Arc::as_ptr(m))
                        .expect("artifact should be in artifacts list")
                })
                .collect(),
            module_upvars: module
                .inner
                .module_upvars
                .iter()
                .map(|m| SerializedModuleUpvar::new(m, artifacts))
                .collect(),
        }
    }
}

pub struct SerializedModule<'a> {
    artifacts: Vec<MyCow<'a, MmapVec>>,
    metadata: Metadata<'a>,
}

#[derive(Serialize, Deserialize)]
struct Metadata<'a> {
    target: String,
    shared_flags: BTreeMap<String, FlagValue>,
    isa_flags: BTreeMap<String, FlagValue>,
    tunables: Tunables,
    features: WasmFeatures,
    module_upvars: Vec<SerializedModuleUpvar>,
    types: MyCow<'a, TypeTables>,
}

impl<'a> SerializedModule<'a> {
    #[cfg(compiler)]
    pub fn new(module: &'a Module) -> Self {
        let artifacts = module
            .inner
            .artifact_upvars
            .iter()
            .map(|m| MyCow::Borrowed(m.mmap()))
            .chain(Some(MyCow::Borrowed(module.inner.module.mmap())))
            .collect::<Vec<_>>();
        let module_upvars = module
            .inner
            .module_upvars
            .iter()
            .map(|m| SerializedModuleUpvar::new(m, &module.inner.artifact_upvars))
            .collect::<Vec<_>>();

        Self::with_data(
            module.engine(),
            artifacts,
            module_upvars,
            MyCow::Borrowed(module.types()),
        )
    }

    #[cfg(compiler)]
    pub fn from_artifacts(
        engine: &Engine,
        artifacts: impl IntoIterator<Item = &'a MmapVec>,
        types: &'a TypeTables,
    ) -> Self {
        Self::with_data(
            engine,
            artifacts.into_iter().map(MyCow::Borrowed).collect(),
            Vec::new(),
            MyCow::Borrowed(types),
        )
    }

    #[cfg(compiler)]
    fn with_data(
        engine: &Engine,
        artifacts: Vec<MyCow<'a, MmapVec>>,
        module_upvars: Vec<SerializedModuleUpvar>,
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
                module_upvars,
                types,
            },
        }
    }

    pub fn into_module(self, engine: &Engine) -> Result<Module> {
        let (main_module, modules, types, upvars) = self.into_parts(engine)?;
        let modules = engine.run_maybe_parallel(modules, |(i, m)| {
            CompiledModule::from_artifacts(i, m, &*engine.config().profiler)
        })?;

        Module::from_parts(engine, modules, main_module, Arc::new(types), &upvars)
    }

    pub fn into_parts(
        mut self,
        engine: &Engine,
    ) -> Result<(
        usize,
        Vec<(MmapVec, Option<CompiledModuleInfo>)>,
        TypeTables,
        Vec<SerializedModuleUpvar>,
    )> {
        // Verify that the module we're loading matches the triple that `engine`
        // is configured for. If compilation is disabled within engine then the
        // assumed triple is the host itself.
        #[cfg(compiler)]
        let engine_triple = engine.compiler().triple();
        #[cfg(not(compiler))]
        let engine_triple = &target_lexicon::Triple::host();
        self.check_triple(engine_triple)?;

        // FIXME: Similar to `Module::from_binary` it should likely be validated
        // here that when `cfg(not(compiler))` is true the isa/shared flags
        // enabled for this precompiled module are compatible with the host
        // itself, which `engine` is assumed to be running code for.
        #[cfg(compiler)]
        {
            let compiler = engine.compiler();
            self.check_shared_flags(compiler)?;
            self.check_isa_flags(compiler)?;
        }

        self.check_tunables(&engine.config().tunables)?;
        self.check_features(&engine.config().features)?;

        assert!(!self.artifacts.is_empty());
        let modules = self.artifacts.into_iter().map(|i| (i.unwrap_owned(), None));

        let main_module = modules.len() - 1;

        Ok((
            main_module,
            modules.collect(),
            self.metadata.types.unwrap_owned(),
            self.metadata.module_upvars,
        ))
    }

    pub fn to_bytes(&self, version_strat: &ModuleVersionStrategy) -> Result<Vec<u8>> {
        // First up, create a linked-ish list of ELF files. For more
        // information on this format, see the doc comment on this module.
        // The only semi-tricky bit here is that we leave an
        // offset-to-the-next-file between each set of ELF files. The list
        // is then terminated with `u64::MAX`.
        let mut ret = Vec::new();
        for (i, obj) in self.artifacts.iter().enumerate() {
            // Anything after the first object needs to respect the alignment of
            // the object's sections, so insert padding as necessary. Note that
            // the +8 to the length here is to accomodate the size we'll write
            // to get to the next object.
            if i > 0 {
                let obj = File::parse(&obj.as_ref()[..])?;
                let align = obj.sections().map(|s| s.align()).max().unwrap_or(0).max(1);
                let align = usize::try_from(align).unwrap();
                let new_size = align_to(ret.len() + 8, align);
                ret.extend_from_slice(&(new_size as u64).to_le_bytes());
                ret.resize(new_size, 0);
            }
            ret.extend_from_slice(obj.as_ref());
        }
        ret.extend_from_slice(&[0xff; 8]);

        // The last part of our artifact is the bincode-encoded `Metadata`
        // section with a few other guards to help give better error messages.
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

    pub fn from_mmap(mut mmap: MmapVec, version_strat: &ModuleVersionStrategy) -> Result<Self> {
        // Artifacts always start with an ELF file, so read that first.
        // Afterwards we continually read ELF files until we see the `u64::MAX`
        // marker, meaning we've reached the end.
        let first_module = read_file(&mut mmap)?;
        let mut pos = first_module.len();
        let mut artifacts = vec![MyCow::Owned(first_module)];

        let metadata = loop {
            if mmap.len() < 8 {
                bail!("invalid serialized data");
            }
            let next_file_start = u64::from_le_bytes([
                mmap[0], mmap[1], mmap[2], mmap[3], mmap[4], mmap[5], mmap[6], mmap[7],
            ]);
            if next_file_start == u64::MAX {
                mmap.drain(..8);
                break mmap;
            }

            // Remove padding leading up to the next file
            let next_file_start = usize::try_from(next_file_start).unwrap();
            let _padding = mmap.drain(..next_file_start - pos);
            let data = read_file(&mut mmap)?;
            pos = next_file_start + data.len();
            artifacts.push(MyCow::Owned(data));
        };

        // Once we've reached the end we parse a `Metadata` object. This has a
        // few guards up front which we process first, and eventually this
        // bottoms out in a `bincode::deserialize` call.
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
            artifacts,
            metadata,
        });

        /// This function will drain the beginning contents of `mmap` which
        /// correspond to an ELF object file. The ELF file is only very lightly
        /// validated.
        ///
        /// The `mmap` passed in will be reset to just after the ELF file, and
        /// the `MmapVec` returned represents the extend of the ELF file
        /// itself.
        fn read_file(mmap: &mut MmapVec) -> Result<MmapVec> {
            use object::NativeEndian as NE;
            // There's not actually a great utility for figuring out where
            // the end of an ELF file is in the `object` crate. In lieu of that
            // we build our own which leverages the format of ELF files, which
            // is that the header comes first, that tells us where the section
            // headers are, and for our ELF files the end of the file is the
            // end of the section headers.
            let mut bytes = Bytes(mmap);
            let header = bytes
                .read::<object::elf::FileHeader64<NE>>()
                .map_err(|()| anyhow!("artifact truncated, can't read header"))?;
            if !header.is_supported() {
                bail!("invalid elf header");
            }
            let sections = header
                .section_headers(NE, &mmap[..])
                .context("failed to read section headers")?;
            let range = subslice_range(object::bytes_of_slice(sections), mmap);
            Ok(mmap.drain(..range.end))
        }
    }

    fn check_triple(&self, other: &target_lexicon::Triple) -> Result<()> {
        let triple =
            target_lexicon::Triple::from_str(&self.metadata.target).map_err(|e| anyhow!(e))?;

        if triple.architecture != other.architecture {
            bail!(
                "Module was compiled for architecture '{}'",
                triple.architecture
            );
        }

        if triple.operating_system != other.operating_system {
            bail!(
                "Module was compiled for operating system '{}'",
                triple.operating_system
            );
        }

        Ok(())
    }

    fn check_shared_flags(&mut self, compiler: &dyn Compiler) -> Result<()> {
        let mut shared_flags = std::mem::take(&mut self.metadata.shared_flags);
        for (name, host) in compiler.flags() {
            match shared_flags.remove(&name) {
                Some(v) => {
                    if v != host {
                        bail!("Module was compiled with a different '{}' setting: expected '{}' but host has '{}'", name, v, host);
                    }
                }
                None => bail!("Module was compiled without setting '{}'", name),
            }
        }

        for (name, _) in shared_flags {
            bail!(
                "Module was compiled with setting '{}' but it is not present for the host",
                name
            );
        }

        Ok(())
    }

    fn check_isa_flags(&mut self, compiler: &dyn Compiler) -> Result<()> {
        let mut isa_flags = std::mem::take(&mut self.metadata.isa_flags);
        for (name, host) in compiler.isa_flags() {
            match isa_flags.remove(&name) {
                Some(v) => match (&v, &host) {
                    (FlagValue::Bool(v), FlagValue::Bool(host)) => {
                        // ISA flags represent CPU features; for boolean values, only
                        // treat it as an error if the module was compiled with the setting enabled
                        // but the host does not have it enabled.
                        if *v && !*host {
                            bail!("Module was compiled with setting '{}' enabled but the host does not support it", name);
                        }
                    }
                    _ => {
                        if v != host {
                            bail!("Module was compiled with a different '{}' setting: expected '{}' but host has '{}'", name, v, host);
                        }
                    }
                },
                None => bail!("Module was compiled without setting '{}'", name),
            }
        }

        for (name, _) in isa_flags {
            bail!(
                "Module was compiled with setting '{}' but it is not present for the host",
                name
            );
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
            interruptable,
            consume_fuel,
            static_memory_bound_is_maximum,
            guard_before_linear_memory,

            // This doesn't affect compilation, it's just a runtime setting.
            dynamic_memory_growth_reserve: _,
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
        Self::check_bool(interruptable, other.interruptable, "interruption support")?;
        Self::check_bool(consume_fuel, other.consume_fuel, "fuel support")?;
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
            module_linking,
            simd,
            threads,
            tail_call,
            deterministic_only,
            multi_memory,
            exceptions,
            memory64,
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
            module_linking,
            other.module_linking,
            "WebAssembly module linking support",
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

        Ok(())
    }
}

/// Aligns the `val` specified up to `align`, which must be a power of two
fn align_to(val: usize, align: usize) -> usize {
    debug_assert!(align.is_power_of_two());
    (val + (align - 1)) & (!(align - 1))
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::Config;
    use std::borrow::Cow;

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
        serialized.metadata.shared_flags.insert(
            "opt_level".to_string(),
            FlagValue::Enum(Cow::Borrowed("none")),
        );

        match serialized.into_module(&engine) {
            Ok(_) => unreachable!(),
            Err(e) => assert_eq!(
                e.to_string(),
                "Module was compiled with a different 'opt_level' setting: expected 'none' but host has 'speed'"
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
                e.to_string(),
                "Module was compiled with setting 'not_a_flag' but it is not present for the host",
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
        config.interruptable(true);

        let engine = Engine::new(&config)?;
        let module = Module::new(&engine, "(module)")?;

        let mut serialized = SerializedModule::new(&module);
        serialized.metadata.tunables.interruptable = false;

        match serialized.into_module(&engine) {
            Ok(_) => unreachable!(),
            Err(e) => assert_eq!(
                e.to_string(),
                "Module was compiled without interruption support but it is enabled for the host"
            ),
        }

        let mut config = Config::new();
        config.interruptable(false);

        let engine = Engine::new(&config)?;
        let module = Module::new(&engine, "(module)")?;

        let mut serialized = SerializedModule::new(&module);
        serialized.metadata.tunables.interruptable = true;

        match serialized.into_module(&engine) {
            Ok(_) => unreachable!(),
            Err(e) => assert_eq!(
                e.to_string(),
                "Module was compiled with interruption support but it is not enabled for the host"
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
