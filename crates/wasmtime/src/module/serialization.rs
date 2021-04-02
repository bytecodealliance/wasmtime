//! Implements module serialization.

use super::ModuleInner;
use crate::{Engine, Module, OptLevel};
use anyhow::{anyhow, bail, Context, Result};
use bincode::Options;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::fmt;
use std::str::FromStr;
use std::sync::Arc;
use std::{collections::HashMap, fmt::Display};
use wasmtime_environ::Tunables;
use wasmtime_environ::{isa::TargetIsa, settings};
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

/// A small helper struct which defines modules are serialized.
#[derive(Serialize, Deserialize)]
struct SerializedModuleData<'a> {
    /// All compiled artifacts needed by this module, where the last entry in
    /// this list is the artifacts for the module itself.
    artifacts: Vec<MyCow<'a, CompilationArtifacts>>,
    /// Closed-over module values that are also needed for this module.
    modules: Vec<SerializedModuleData<'a>>,
    /// The index into the list of type tables that are used for this module's
    /// type tables.
    type_tables: usize,
}

impl<'a> SerializedModuleData<'a> {
    pub fn new(module: &'a Module) -> (Self, Vec<MyCow<'a, TypeTables>>) {
        let mut pushed = HashMap::new();
        let mut tables = Vec::new();
        return (module_data(module, &mut pushed, &mut tables), tables);

        fn module_data<'a>(
            module: &'a Module,
            type_tables_pushed: &mut HashMap<usize, usize>,
            type_tables: &mut Vec<MyCow<'a, TypeTables>>,
        ) -> SerializedModuleData<'a> {
            // Deduplicate `Arc<TypeTables>` using our two parameters to ensure we
            // serialize type tables as little as possible.
            let ptr = Arc::as_ptr(module.types());
            let type_tables_idx = *type_tables_pushed.entry(ptr as usize).or_insert_with(|| {
                type_tables.push(MyCow::Borrowed(module.types()));
                type_tables.len() - 1
            });
            SerializedModuleData {
                artifacts: module
                    .inner
                    .artifact_upvars
                    .iter()
                    .map(|i| MyCow::Borrowed(i.compilation_artifacts()))
                    .chain(Some(MyCow::Borrowed(
                        module.compiled_module().compilation_artifacts(),
                    )))
                    .collect(),
                modules: module
                    .inner
                    .module_upvars
                    .iter()
                    .map(|i| module_data(i, type_tables_pushed, type_tables))
                    .collect(),
                type_tables: type_tables_idx,
            }
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
    data: SerializedModuleData<'a>,
    tables: Vec<MyCow<'a, TypeTables>>,
}

impl<'a> SerializedModule<'a> {
    pub fn new(module: &'a Module) -> Self {
        let (data, tables) = SerializedModuleData::new(module);
        Self::with_data(module.engine().compiler(), data, tables)
    }

    pub fn from_artifacts(
        compiler: &Compiler,
        artifacts: &'a Vec<CompilationArtifacts>,
        types: &'a TypeTables,
    ) -> Self {
        Self::with_data(
            compiler,
            SerializedModuleData {
                artifacts: artifacts.iter().map(MyCow::Borrowed).collect(),
                modules: Vec::new(),
                type_tables: 0,
            },
            vec![MyCow::Borrowed(types)],
        )
    }

    fn with_data(
        compiler: &Compiler,
        data: SerializedModuleData<'a>,
        tables: Vec<MyCow<'a, TypeTables>>,
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
            data,
            tables,
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

        let types = self
            .tables
            .into_iter()
            .map(|t| Arc::new(t.unwrap_owned()))
            .collect::<Vec<_>>();
        let module = mk(engine, &types, self.data)?;

        // Validate the module can be used with the current allocator
        engine.allocator().validate(module.inner.module.module())?;

        return Ok(module);

        fn mk(
            engine: &Engine,
            types: &Vec<Arc<TypeTables>>,
            data: SerializedModuleData<'_>,
        ) -> Result<Module> {
            let mut artifacts = CompiledModule::from_artifacts_list(
                data.artifacts
                    .into_iter()
                    .map(|i| i.unwrap_owned())
                    .collect(),
                engine.compiler().isa(),
                &*engine.config().profiler,
            )?;
            let inner = ModuleInner {
                engine: engine.clone(),
                types: types[data.type_tables].clone(),
                module: artifacts.pop().unwrap(),
                artifact_upvars: artifacts,
                module_upvars: data
                    .modules
                    .into_iter()
                    .map(|m| mk(engine, types, m))
                    .collect::<Result<Vec<_>>>()?,
            };

            Ok(Module {
                inner: Arc::new(inner),
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
