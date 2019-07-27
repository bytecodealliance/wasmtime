use crate::address_map::{ModuleAddressMap, ValueLabelsRanges};
use crate::compilation::{CodeAndJTOffsets, Compilation, Relocations};
use crate::module::Module;
use crate::module_environ::FunctionBodyData;
use core::hash::Hasher;
use cranelift_codegen::{ir, isa};
use cranelift_entity::PrimaryMap;
use cranelift_wasm::DefinedFuncIndex;
use lazy_static::lazy_static;
use log::{debug, warn};
use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde::ser::{self, Serialize, SerializeSeq, SerializeStruct, Serializer};
use sha2::{Digest, Sha256};
use std::fmt;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::string::{String, ToString};

/// Module for configuring the cache system.
pub mod conf {
    use directories::ProjectDirs;
    use log::{debug, warn};
    use spin::Once;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicBool, Ordering};

    struct Config {
        pub cache_enabled: bool,
        pub cache_dir: PathBuf,
    }

    // Private static, so only internal function can access it.
    static CONFIG: Once<Config> = Once::new();
    static INIT_CALLED: AtomicBool = AtomicBool::new(false);

    /// Returns true if and only if the cache is enabled.
    pub fn cache_enabled() -> bool {
        // Not everyone knows about the cache system, i.e. the tests,
        // so the default is cache disabled.
        CONFIG
            .call_once(|| Config::new_cache_disabled())
            .cache_enabled
    }

    /// Returns path to the cache directory.
    ///
    /// Panics if the cache is disabled.
    pub fn cache_directory() -> &'static PathBuf {
        &CONFIG
            .r#try()
            .expect("Cache system must be initialized")
            .cache_dir
    }

    /// Initializes the cache system. Should be called exactly once,
    /// and before using the cache system. Otherwise it can panic.
    pub fn init<P: AsRef<Path>>(enabled: bool, dir: Option<P>) {
        INIT_CALLED
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .expect("Cache system init must be called at most once");
        assert!(
            CONFIG.r#try().is_none(),
            "Cache system init must be called before using the system."
        );
        let conf = CONFIG.call_once(|| Config::new(enabled, dir));
        debug!(
            "Cache init(): enabled={}, cache-dir={:?}",
            conf.cache_enabled, conf.cache_dir
        );
    }

    impl Config {
        pub fn new_cache_disabled() -> Self {
            Self {
                cache_enabled: false,
                cache_dir: PathBuf::new(),
            }
        }

        pub fn new<P: AsRef<Path>>(enabled: bool, dir: Option<P>) -> Self {
            if enabled {
                match dir {
                    Some(dir) => Self::new_step2(dir.as_ref()),
                    None => match ProjectDirs::from("", "CraneStation", "wasmtime") {
                        Some(proj_dirs) => Self::new_step2(proj_dirs.cache_dir()),
                        None => {
                            warn!("Cache directory not specified and failed to find the default. Disabling cache.");
                            Self::new_cache_disabled()
                        }
                    },
                }
            } else {
                Self::new_cache_disabled()
            }
        }

        fn new_step2(dir: &Path) -> Self {
            // On Windows, if we want long paths, we need '\\?\' prefix, but it doesn't work
            // with relative paths. One way to get absolute path (the only one?) is to use
            // fs::canonicalize, but it requires that given path exists. The extra advantage
            // of this method is fact that the method prepends '\\?\' on Windows.
            match fs::create_dir_all(dir) {
                Ok(()) => match fs::canonicalize(dir) {
                    Ok(p) => Self {
                        cache_enabled: true,
                        cache_dir: p,
                    },
                    Err(err) => {
                        warn!(
                            "Failed to canonicalize the cache directory. Disabling cache. \
                             Message: {}",
                            err
                        );
                        Self::new_cache_disabled()
                    }
                },
                Err(err) => {
                    warn!(
                        "Failed to create the cache directory. Disabling cache. Message: {}",
                        err
                    );
                    Self::new_cache_disabled()
                }
            }
        }
    }
}

lazy_static! {
    static ref SELF_MTIME: String = {
        match std::env::current_exe() {
            Ok(path) => match fs::metadata(&path) {
                Ok(metadata) => match metadata.modified() {
                    Ok(mtime) => match mtime.duration_since(std::time::UNIX_EPOCH) {
                        Ok(duration) => format!("{}", duration.as_millis()),
                        Err(err) => format!("m{}", err.duration().as_millis()),
                    },
                    Err(_) => {
                        warn!("Failed to get modification time of current executable");
                        "no-mtime".to_string()
                    }
                },
                Err(_) => {
                    warn!("Failed to get metadata of current executable");
                    "no-mtime".to_string()
                }
            },
            Err(_) => {
                warn!("Failed to get path of current executable");
                "no-mtime".to_string()
            }
        }
    };
}

pub struct ModuleCacheEntry {
    mod_cache_path: Option<PathBuf>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ModuleCacheData {
    compilation: Compilation,
    relocations: Relocations,
    address_transforms: ModuleAddressMap,
    value_ranges: ValueLabelsRanges,
    stack_slots: PrimaryMap<DefinedFuncIndex, ir::StackSlots>,
}

type ModuleCacheDataTupleType = (
    Compilation,
    Relocations,
    ModuleAddressMap,
    ValueLabelsRanges,
    PrimaryMap<DefinedFuncIndex, ir::StackSlots>,
);

struct Sha256Hasher(Sha256);

impl ModuleCacheEntry {
    pub fn new<'data>(
        module: &Module,
        function_body_inputs: &PrimaryMap<DefinedFuncIndex, FunctionBodyData<'data>>,
        isa: &dyn isa::TargetIsa,
        compiler_name: &str,
        generate_debug_info: bool,
    ) -> Self {
        let mod_cache_path = if conf::cache_enabled() {
            let hash = Sha256Hasher::digest(module, function_body_inputs);
            let compiler_dir = if cfg!(debug_assertions) {
                format!(
                    "{comp_name}-{comp_ver}-{comp_mtime}",
                    comp_name = compiler_name,
                    comp_ver = env!("GIT_REV"),
                    comp_mtime = *SELF_MTIME,
                )
            } else {
                format!(
                    "{comp_name}-{comp_ver}",
                    comp_name = compiler_name,
                    comp_ver = env!("GIT_REV"),
                )
            };
            let mod_filename = format!(
                "mod-{mod_hash}{mod_dbg}",
                mod_hash = base64::encode_config(&hash, base64::URL_SAFE_NO_PAD), // standard encoding uses '/' which can't be used for filename
                mod_dbg = if generate_debug_info { ".d" } else { "" },
            );
            Some(
                conf::cache_directory()
                    .join(isa.name())
                    .join(compiler_dir)
                    .join(mod_filename),
            )
        } else {
            None
        };

        ModuleCacheEntry { mod_cache_path }
    }

    pub fn get_data(&self) -> Option<ModuleCacheData> {
        if let Some(p) = &self.mod_cache_path {
            match fs::read(p) {
                Ok(cache_bytes) => match bincode::deserialize(&cache_bytes[..]) {
                    Ok(data) => Some(data),
                    Err(err) => {
                        warn!("Failed to deserialize cached code: {}", err);
                        None
                    }
                },
                Err(_) => None,
            }
        } else {
            None
        }
    }

    pub fn update_data(&self, data: &ModuleCacheData) {
        if let Some(p) = &self.mod_cache_path {
            let cache_buf = match bincode::serialize(&data) {
                Ok(data) => data,
                Err(err) => {
                    warn!("Failed to serialize cached code: {}", err);
                    return;
                }
            };
            // Optimize syscalls: first, try writing to disk. It should succeed in most cases.
            // Otherwise, try creating the cache directory and retry writing to the file.
            match fs::write(p, &cache_buf) {
                Ok(()) => (),
                Err(err) => {
                    debug!(
                        "Attempting to create the cache directory, because \
                         failed to write cached code to disk, path: {}, message: {}",
                        p.display(),
                        err,
                    );
                    let cache_dir = p.parent().unwrap();
                    match fs::create_dir_all(cache_dir) {
                        Ok(()) => match fs::write(p, &cache_buf) {
                            Ok(()) => (),
                            Err(err) => {
                                warn!(
                                    "Failed to write cached code to disk, path: {}, message: {}",
                                    p.display(),
                                    err
                                );
                                match fs::remove_file(p) {
                                    Ok(()) => (),
                                    Err(err) => {
                                        if err.kind() != io::ErrorKind::NotFound {
                                            warn!(
                                                "Failed to cleanup invalid cache, path: {}, message: {}",
                                                p.display(),
                                                err
                                            );
                                        }
                                    }
                                }
                            }
                        },
                        Err(err) => warn!(
                            "Failed to create cache directory, path: {}, message: {}",
                            cache_dir.display(),
                            err
                        ),
                    }
                }
            }
        }
    }
}

impl ModuleCacheData {
    pub fn from_tuple(data: ModuleCacheDataTupleType) -> Self {
        Self {
            compilation: data.0,
            relocations: data.1,
            address_transforms: data.2,
            value_ranges: data.3,
            stack_slots: data.4,
        }
    }

    pub fn to_tuple(self) -> ModuleCacheDataTupleType {
        (
            self.compilation,
            self.relocations,
            self.address_transforms,
            self.value_ranges,
            self.stack_slots,
        )
    }
}

impl Sha256Hasher {
    pub fn digest<'data>(
        module: &Module,
        function_body_inputs: &PrimaryMap<DefinedFuncIndex, FunctionBodyData<'data>>,
    ) -> [u8; 32] {
        let mut hasher = Self(Sha256::new());
        module.hash_for_cache(function_body_inputs, &mut hasher);
        hasher.0.result().into()
    }
}

impl Hasher for Sha256Hasher {
    fn finish(&self) -> u64 {
        panic!("Sha256Hasher doesn't support finish!");
    }

    fn write(&mut self, bytes: &[u8]) {
        self.0.input(bytes);
    }
}

//-////////////////////////////////////////////////////////////////////
// Serialization and deserialization of type containing SecondaryMap //
//-////////////////////////////////////////////////////////////////////

enum JtOffsetsWrapper<'a> {
    Ref(&'a ir::JumpTableOffsets), // for serialization
    Data(ir::JumpTableOffsets),    // for deserialization
}

impl Serialize for CodeAndJTOffsets {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut cajto = serializer.serialize_struct("CodeAndJTOffsets", 2)?;
        cajto.serialize_field("body", &self.body)?;
        cajto.serialize_field("jt_offsets", &JtOffsetsWrapper::Ref(&self.jt_offsets))?;
        cajto.end()
    }
}

impl<'de> Deserialize<'de> for CodeAndJTOffsets {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        #[serde(field_identifier, rename_all = "lowercase")]
        enum Field {
            Body,
            JtOffsets,
        };

        struct CodeAndJTOffsetsVisitor;

        impl<'de> Visitor<'de> for CodeAndJTOffsetsVisitor {
            type Value = CodeAndJTOffsets;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct CodeAndJTOffsets")
            }

            fn visit_seq<V>(self, mut seq: V) -> Result<Self::Value, V::Error>
            where
                V: SeqAccess<'de>,
            {
                let body = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(0, &self))?;
                let jt_offsets = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(1, &self))?;
                match jt_offsets {
                    JtOffsetsWrapper::Data(jt_offsets) => Ok(CodeAndJTOffsets { body, jt_offsets }),
                    JtOffsetsWrapper::Ref(_) => Err(de::Error::custom(
                        "Received invalid variant of JtOffsetsWrapper",
                    )),
                }
            }

            fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut body = None;
                let mut jt_offsets = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Body => {
                            if body.is_some() {
                                return Err(de::Error::duplicate_field("body"));
                            }
                            body = Some(map.next_value()?);
                        }
                        Field::JtOffsets => {
                            if jt_offsets.is_some() {
                                return Err(de::Error::duplicate_field("jt_offsets"));
                            }
                            jt_offsets = Some(map.next_value()?);
                        }
                    }
                }

                let body = body.ok_or_else(|| de::Error::missing_field("body"))?;
                let jt_offsets =
                    jt_offsets.ok_or_else(|| de::Error::missing_field("jt_offsets"))?;
                match jt_offsets {
                    JtOffsetsWrapper::Data(jt_offsets) => Ok(CodeAndJTOffsets { body, jt_offsets }),
                    JtOffsetsWrapper::Ref(_) => Err(de::Error::custom(
                        "Received invalid variant of JtOffsetsWrapper",
                    )),
                }
            }
        }

        const FIELDS: &'static [&'static str] = &["body", "jt_offsets"];
        deserializer.deserialize_struct("CodeAndJTOffsets", FIELDS, CodeAndJTOffsetsVisitor)
    }
}

impl Serialize for JtOffsetsWrapper<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            JtOffsetsWrapper::Ref(data) => {
                // TODO: bincode encodes option as "byte for Some/None" and then optionally the content
                // TODO: we can actually optimize it by encoding manually bitmask, then elements
                let default_val = data.get_default();
                let mut seq = serializer.serialize_seq(Some(1 + data.len()))?;
                seq.serialize_element(&Some(default_val))?;
                for e in data.values() {
                    let some_e = Some(e);
                    seq.serialize_element(if e == default_val { &None } else { &some_e })?;
                }
                seq.end()
            }
            JtOffsetsWrapper::Data(_) => Err(ser::Error::custom(
                "Received invalid variant of JtOffsetsWrapper",
            )),
        }
    }
}

impl<'de> Deserialize<'de> for JtOffsetsWrapper<'_> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct JtOffsetsWrapperVisitor;

        impl<'de> Visitor<'de> for JtOffsetsWrapperVisitor {
            type Value = JtOffsetsWrapper<'static>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct JtOffsetsWrapper")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                match seq.next_element()? {
                    Some(Some(default_val)) => {
                        let mut m = cranelift_entity::SecondaryMap::with_default(default_val);
                        let mut idx = 0;
                        while let Some(val) = seq.next_element()? {
                            let val: Option<_> = val; // compiler can't infer the type, and this line is needed
                            match ir::JumpTable::with_number(idx) {
                                Some(jt_idx) => m[jt_idx] = val.unwrap_or(default_val),
                                None => {
                                    return Err(serde::de::Error::custom(
                                        "Invalid JumpTable reference",
                                    ))
                                }
                            };
                            idx += 1;
                        }
                        Ok(JtOffsetsWrapper::Data(m))
                    }
                    _ => Err(serde::de::Error::custom("Default value required")),
                }
            }
        }

        deserializer.deserialize_seq(JtOffsetsWrapperVisitor {})
    }
}
