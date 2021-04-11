//! Implements module serialization.

use super::{ModuleInner, ModuleSharedSignatures};
use crate::{Engine, Module, OptLevel};
use anyhow::{anyhow, bail, Context, Result};
use bincode::Options;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::fmt;
use std::str::FromStr;
use std::sync::Arc;
use std::{collections::HashMap, fmt::Display};
use wasmtime_environ::{isa::TargetIsa, settings, Tunables};
use wasmtime_jit::{
    CompilationArtifacts, CompilationStrategy, CompiledModule, Compiler, TypeTables,
};

const HEADER: &[u8] = b"\0wasmtime-aot";

fn bincode_options() -> impl Options {
    // Use a variable-length integer encoding instead of fixed length. The
    // module shown on #2318 gets compressed from ~160MB to ~110MB simply using
    // this, presumably because there's a lot of 8-byte integers which generally
    // have small values. Local testing shows that the deserialization
    // performance, while higher, is in the few-percent range. For huge size
    // savings this seems worthwhile to lose a small percentage of
    // deserialization performance.
    bincode::DefaultOptions::new().with_varint_encoding()
}

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

impl From<settings::OptLevel> for OptLevel {
    fn from(level: settings::OptLevel) -> Self {
        match level {
            settings::OptLevel::Speed => OptLevel::Speed,
            settings::OptLevel::SpeedAndSize => OptLevel::SpeedAndSize,
            settings::OptLevel::None => OptLevel::None,
        }
    }
}

/// A small helper struct for serialized module upvars.
#[derive(Serialize, Deserialize)]
struct SerializedModuleUpvar {
    /// The module's index into the compilation artifact.
    index: usize,
    /// Indexes into the list of all compilation artifacts for this module.
    artifact_upvars: Vec<usize>,
    /// Closed-over module values that are also needed for this module.
    module_upvars: Vec<SerializedModuleUpvar>,
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

#[derive(Serialize, Deserialize, Eq, PartialEq)]
enum FlagValue {
    Enum(Cow<'static, str>),
    Num(u8),
    Bool(bool),
}

impl From<settings::Value> for FlagValue {
    fn from(v: settings::Value) -> Self {
        match v.kind() {
            settings::SettingKind::Enum => Self::Enum(v.as_enum().unwrap().into()),
            settings::SettingKind::Num => Self::Num(v.as_num().unwrap()),
            settings::SettingKind::Bool => Self::Bool(v.as_bool().unwrap()),
            settings::SettingKind::Preset => unreachable!(),
        }
    }
}

impl Display for FlagValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Enum(v) => v.fmt(f),
            Self::Num(v) => v.fmt(f),
            Self::Bool(v) => v.fmt(f),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct SerializedModule<'a> {
    target: String,
    shared_flags: HashMap<String, FlagValue>,
    isa_flags: HashMap<String, FlagValue>,
    strategy: CompilationStrategy,
    tunables: Tunables,
    features: WasmFeatures,
    artifacts: Vec<MyCow<'a, CompilationArtifacts>>,
    module_upvars: Vec<SerializedModuleUpvar>,
    types: MyCow<'a, TypeTables>,
}

impl<'a> SerializedModule<'a> {
    pub fn new(module: &'a Module) -> Self {
        let compiler = module.engine().compiler();
        let artifacts = module
            .inner
            .artifact_upvars
            .iter()
            .map(|m| MyCow::Borrowed(m.compilation_artifacts()))
            .chain(Some(MyCow::Borrowed(
                module.inner.module.compilation_artifacts(),
            )))
            .collect::<Vec<_>>();
        let module_upvars = module
            .inner
            .module_upvars
            .iter()
            .map(|m| SerializedModuleUpvar::new(m, &module.inner.artifact_upvars))
            .collect::<Vec<_>>();

        Self::with_data(
            compiler,
            artifacts,
            module_upvars,
            MyCow::Borrowed(module.types()),
        )
    }

    pub fn from_artifacts(
        compiler: &Compiler,
        artifacts: &'a Vec<CompilationArtifacts>,
        types: &'a TypeTables,
    ) -> Self {
        Self::with_data(
            compiler,
            artifacts.iter().map(MyCow::Borrowed).collect(),
            Vec::new(),
            MyCow::Borrowed(types),
        )
    }

    fn with_data(
        compiler: &Compiler,
        artifacts: Vec<MyCow<'a, CompilationArtifacts>>,
        module_upvars: Vec<SerializedModuleUpvar>,
        types: MyCow<'a, TypeTables>,
    ) -> Self {
        let isa = compiler.isa();

        Self {
            target: isa.triple().to_string(),
            shared_flags: isa
                .flags()
                .iter()
                .map(|v| (v.name.to_owned(), v.into()))
                .collect(),
            isa_flags: isa
                .isa_flags()
                .into_iter()
                .map(|v| (v.name.to_owned(), v.into()))
                .collect(),
            strategy: compiler.strategy(),
            tunables: compiler.tunables().clone(),
            features: compiler.features().into(),
            artifacts,
            module_upvars,
            types,
        }
    }

    pub fn into_module(mut self, engine: &Engine) -> Result<Module> {
        let compiler = engine.compiler();
        let isa = compiler.isa();

        self.check_triple(isa)?;
        self.check_shared_flags(isa)?;
        self.check_isa_flags(isa)?;
        self.check_strategy(compiler)?;
        self.check_tunables(compiler)?;
        self.check_features(compiler)?;

        let types = Arc::new(self.types.unwrap_owned());
        let mut modules = CompiledModule::from_artifacts_list(
            self.artifacts
                .into_iter()
                .map(|i| i.unwrap_owned())
                .collect(),
            engine.compiler().isa(),
            &*engine.config().profiler,
        )?;

        // Validate the module can be used with the current allocator
        engine
            .allocator()
            .validate(modules.last().unwrap().module())?;

        let (signatures, trampolines) = engine.register_module_signatures(
            &types.wasm_signatures,
            modules.iter().flat_map(|m| m.trampolines().iter().cloned()),
        );

        let signatures = Arc::new(ModuleSharedSignatures {
            engine: engine.clone(),
            signatures,
            trampolines,
        });

        let module = modules.pop().unwrap();

        let module_upvars = self
            .module_upvars
            .iter()
            .map(|m| {
                mk(
                    engine,
                    &modules,
                    &types,
                    m.index,
                    &m.artifact_upvars,
                    &m.module_upvars,
                    &signatures,
                )
            })
            .collect::<Result<Vec<_>>>()?;

        return Ok(Module {
            inner: Arc::new(ModuleInner {
                engine: engine.clone(),
                types,
                module,
                artifact_upvars: modules,
                module_upvars,
                signatures,
            }),
        });

        fn mk(
            engine: &Engine,
            artifacts: &[Arc<CompiledModule>],
            types: &Arc<TypeTables>,
            module_index: usize,
            artifact_upvars: &[usize],
            module_upvars: &[SerializedModuleUpvar],
            signatures: &Arc<ModuleSharedSignatures>,
        ) -> Result<Module> {
            Ok(Module {
                inner: Arc::new(ModuleInner {
                    engine: engine.clone(),
                    types: types.clone(),
                    module: artifacts[module_index].clone(),
                    artifact_upvars: artifact_upvars
                        .iter()
                        .map(|i| artifacts[*i].clone())
                        .collect(),
                    module_upvars: module_upvars
                        .into_iter()
                        .map(|m| {
                            mk(
                                engine,
                                artifacts,
                                types,
                                m.index,
                                &m.artifact_upvars,
                                &m.module_upvars,
                                signatures,
                            )
                        })
                        .collect::<Result<Vec<_>>>()?,
                    signatures: signatures.clone(),
                }),
            })
        }
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        use std::io::Write;

        let mut bytes = Vec::new();

        bytes.write_all(HEADER)?;

        // Preface the data with a version so we can do a version check independent
        // of the serialized data.
        let version = env!("CARGO_PKG_VERSION");
        assert!(
            version.len() < 256,
            "package version must be less than 256 bytes"
        );
        bytes.write(&[version.len() as u8])?;

        bytes.write_all(version.as_bytes())?;

        bincode_options().serialize_into(&mut bytes, self)?;

        Ok(bytes)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Option<Self>> {
        if !bytes.starts_with(HEADER) {
            return Ok(None);
        }

        let bytes = &bytes[HEADER.len()..];

        if bytes.is_empty() {
            bail!("serialized data data is empty");
        }

        let version_len = bytes[0] as usize;
        if bytes.len() < version_len + 1 {
            bail!("serialized data is malformed");
        }

        let version = std::str::from_utf8(&bytes[1..1 + version_len])?;
        if version != env!("CARGO_PKG_VERSION") {
            bail!(
                "Module was compiled with incompatible Wasmtime version '{}'",
                version
            );
        }

        Ok(Some(
            bincode_options()
                .deserialize::<SerializedModule<'_>>(&bytes[1 + version_len..])
                .context("deserialize compilation artifacts")?,
        ))
    }

    fn check_triple(&self, isa: &dyn TargetIsa) -> Result<()> {
        let triple = target_lexicon::Triple::from_str(&self.target).map_err(|e| anyhow!(e))?;

        if triple.architecture != isa.triple().architecture {
            bail!(
                "Module was compiled for architecture '{}'",
                triple.architecture
            );
        }

        if triple.operating_system != isa.triple().operating_system {
            bail!(
                "Module was compiled for operating system '{}'",
                triple.operating_system
            );
        }

        Ok(())
    }

    fn check_shared_flags(&mut self, isa: &dyn TargetIsa) -> Result<()> {
        let mut shared_flags = std::mem::take(&mut self.shared_flags);
        for value in isa.flags().iter() {
            let name = value.name;
            match shared_flags.remove(name) {
                Some(v) => {
                    let host: FlagValue = value.into();
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

    fn check_isa_flags(&mut self, isa: &dyn TargetIsa) -> Result<()> {
        let mut isa_flags = std::mem::take(&mut self.isa_flags);
        for value in isa.isa_flags().into_iter() {
            let name = value.name;
            let host: FlagValue = value.into();
            match isa_flags.remove(name) {
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

    fn check_strategy(&self, compiler: &Compiler) -> Result<()> {
        #[allow(unreachable_patterns)]
        let matches = match (self.strategy, compiler.strategy()) {
            (CompilationStrategy::Auto, CompilationStrategy::Auto)
            | (CompilationStrategy::Auto, CompilationStrategy::Cranelift)
            | (CompilationStrategy::Cranelift, CompilationStrategy::Auto)
            | (CompilationStrategy::Cranelift, CompilationStrategy::Cranelift) => true,
            #[cfg(feature = "lightbeam")]
            (CompilationStrategy::Lightbeam, CompilationStrategy::Lightbeam) => true,
            _ => false,
        };

        if !matches {
            bail!("Module was compiled with strategy '{:?}'", self.strategy);
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

    fn check_tunables(&self, compiler: &Compiler) -> Result<()> {
        let Tunables {
            static_memory_bound,
            static_memory_offset_guard_size,
            dynamic_memory_offset_guard_size,
            generate_native_debuginfo,
            parse_wasm_debuginfo,
            interruptable,
            consume_fuel,
            static_memory_bound_is_maximum,
        } = self.tunables;

        let other = compiler.tunables();

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

        Ok(())
    }

    fn check_features(&self, compiler: &Compiler) -> Result<()> {
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
        } = self.features;

        let other = compiler.features();
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

#[cfg(test)]
mod test {
    use super::*;
    use crate::Config;

    #[test]
    fn test_architecture_mismatch() -> Result<()> {
        let engine = Engine::default();
        let module = Module::new(&engine, "(module)")?;

        let mut serialized = SerializedModule::new(&module);
        serialized.target = "unknown-generic-linux".to_string();

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
        serialized.target = format!(
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
        serialized.shared_flags.insert(
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

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn test_isa_flags_mismatch() -> Result<()> {
        let engine = Engine::default();
        let module = Module::new(&engine, "(module)")?;

        let mut serialized = SerializedModule::new(&module);

        serialized
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

    #[cfg(feature = "lightbeam")]
    #[test]
    fn test_compilation_strategy_mismatch() -> Result<()> {
        let engine = Engine::default();
        let module = Module::new(&engine, "(module)")?;

        let mut serialized = SerializedModule::new(&module);
        serialized.strategy = CompilationStrategy::Lightbeam;

        match serialized.into_module(&engine) {
            Ok(_) => unreachable!(),
            Err(e) => assert_eq!(
                e.to_string(),
                "Module was compiled with strategy 'Cranelift'",
            ),
        }

        Ok(())
    }

    #[test]
    fn test_tunables_int_mismatch() -> Result<()> {
        let engine = Engine::default();
        let module = Module::new(&engine, "(module)")?;

        let mut serialized = SerializedModule::new(&module);
        serialized.tunables.static_memory_offset_guard_size = 0;

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
        serialized.tunables.interruptable = false;

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
        serialized.tunables.interruptable = true;

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
        serialized.features.simd = false;

        match serialized.into_module(&engine) {
            Ok(_) => unreachable!(),
            Err(e) => assert_eq!(e.to_string(), "Module was compiled without WebAssembly SIMD support but it is enabled for the host"),
        }

        let mut config = Config::new();
        config.wasm_simd(false);

        let engine = Engine::new(&config)?;
        let module = Module::new(&engine, "(module)")?;

        let mut serialized = SerializedModule::new(&module);
        serialized.features.simd = true;

        match serialized.into_module(&engine) {
            Ok(_) => unreachable!(),
            Err(e) => assert_eq!(e.to_string(), "Module was compiled with WebAssembly SIMD support but it is not enabled for the host"),
        }

        Ok(())
    }
}
