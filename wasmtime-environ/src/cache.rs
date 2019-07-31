use crate::address_map::ModuleAddressMap;
use crate::compilation::{CodeAndJTOffsets, Compilation, Relocations};
use crate::module::Module;
use cranelift_codegen::ir;
use cranelift_codegen::isa;
use directories::ProjectDirs;
use lazy_static::lazy_static;
use log::{debug, warn};
use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde::ser::{self, Serialize, SerializeSeq, SerializeStruct, Serializer};
use std::ffi::OsString;
use std::fmt;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::string::{String, ToString};

/// Module for configuring the cache system.
pub mod conf {
    use spin::Once;

    // Private static, so only internal function can access it.
    static CACHE_ENABLED: Once<bool> = Once::new();

    /// Returns true if and only if the cache is enabled.
    pub fn cache_enabled() -> bool {
        // Not everyone knows about the cache system, i.e. the tests,
        // so the default is false.
        *CACHE_ENABLED.call_once(|| false)
    }

    /// Initializes the cache system. Should be called exactly once,
    /// and before using the cache system. Otherwise it can panic.
    pub fn init(enabled: bool) {
        // init() should be called exactly once
        assert!(CACHE_ENABLED.r#try().is_none());
        let val = *CACHE_ENABLED.call_once(|| enabled);
        // But multiple threads can pass the first assertion, so let's guarantee consistency:
        assert!(val == enabled);
    }
}

lazy_static! {
    static ref CACHE_DIR: Option<PathBuf> =
        match ProjectDirs::from("", "CraneStation", "wasmtime") {
            Some(proj_dirs) => {
                let cache_dir = proj_dirs.cache_dir();
                // Temporary workaround for: https://github.com/rust-lang/rust/issues/32689
                if cfg!(windows) {
                    let mut long_path = OsString::new();
                    if !cache_dir.starts_with("\\\\?\\") {
                        long_path.push("\\\\?\\");
                    }
                    long_path.push(cache_dir.as_os_str());
                    Some(PathBuf::from(long_path))
                }
                else {
                    Some(cache_dir.to_path_buf())
                }
            }
            None => {
                warn!("Failed to find proper cache directory location.");
                None
            }
        };
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
}

type ModuleCacheDataTupleType = (Compilation, Relocations, ModuleAddressMap);

impl ModuleCacheEntry {
    pub fn new(
        module: &Module,
        isa: &dyn isa::TargetIsa,
        compiler_name: &str,
        generate_debug_info: bool,
    ) -> Self {
        let mod_cache_path = if conf::cache_enabled() {
            CACHE_DIR.clone().and_then(|p| {
                module.hash.map(|hash| {
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
                    p.join(isa.name()).join(compiler_dir).join(format!(
                        "mod-{mod_hash}{mod_dbg}",
                        mod_hash = base64::encode_config(&hash, base64::URL_SAFE_NO_PAD), // standard encoding uses '/' which can't be used for filename
                        mod_dbg = if generate_debug_info { ".d" } else { "" },
                    ))
                })
            })
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
        }
    }

    pub fn to_tuple(self) -> ModuleCacheDataTupleType {
        (self.compilation, self.relocations, self.address_transforms)
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
