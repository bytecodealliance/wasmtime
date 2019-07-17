use crate::address_map::FunctionAddressMap;
use crate::compilation;
use directories::ProjectDirs;
use lazy_static::lazy_static;
use log::debug;
use serde::de::{Deserialize, Deserializer, SeqAccess, Visitor};
use serde::ser::{Serialize, SerializeSeq, Serializer};
use sha2::{Digest, Sha256};
use std::fmt;
use std::fs;
use std::marker::PhantomData;
use std::path::PathBuf;
use std::vec::Vec;

use crate::module_environ::FunctionBodyData;

lazy_static! {
    static ref CACHE_DIR: Option<PathBuf> =
        match ProjectDirs::from("org", "CraneStation", "wasmtime") {
            Some(proj_dirs) => {
                let cache_dir = proj_dirs.cache_dir();
                match fs::create_dir_all(cache_dir) {
                    Ok(()) => (),
                    Err(err) => debug!("Unable to create cache directory, failed with: {}", err),
                };
                Some(cache_dir.to_path_buf())
            }
            None => {
                debug!("Unable to find cache directory");
                None
            }
        };
}

pub struct FuncCacheEntry {
    func_cache_path: Option<PathBuf>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct FuncCacheData {
    code_buf: Vec<u8>,
    jt_offsets: JtOffsetsSerdeWrapper,
    func_relocs: Vec<compilation::Relocation>,
    address_transform: Option<FunctionAddressMap>,
}
type JtOffsetsType =
    cranelift_entity::SecondaryMap<cranelift_codegen::ir::entities::JumpTable, u32>;
struct JtOffsetsSerdeWrapper(JtOffsetsType);
type FuncCacheDataTupleType = (
    Vec<u8>,
    JtOffsetsType,
    Vec<compilation::Relocation>,
    Option<FunctionAddressMap>,
);
struct JtOffsetsSerdeWrapperVisitor {
    marker: PhantomData<fn() -> JtOffsetsSerdeWrapper>,
}

impl FuncCacheEntry {
    pub fn new(input: &FunctionBodyData) -> Self {
        let mut hasher = Sha256::new();
        hasher.input(input.data);
        let hash = hasher.result();

        let func_cache_path = CACHE_DIR
            .clone()
            .map(|p| p.join(format!("func-{}", hex::encode(hash))));

        FuncCacheEntry { func_cache_path }
    }

    pub fn get_data(&self) -> Option<FuncCacheData> {
        if let Some(p) = &self.func_cache_path {
            match fs::read(p) {
                Ok(cache_bytes) => match bincode::deserialize(&cache_bytes[..]) {
                    Ok(data) => Some(data),
                    Err(err) => {
                        debug!("Failed to deserialize cached code: {}", err);
                        None
                    }
                },
                Err(_) => None,
            }
        } else {
            None
        }
    }

    pub fn update_data(&self, data: &FuncCacheData) {
        if let Some(p) = &self.func_cache_path {
            let cache_buf = match bincode::serialize(&data) {
                Ok(data) => data,
                Err(err) => {
                    debug!("Failed to serialize cached code: {}", err);
                    return;
                }
            };
            match fs::write(p, &cache_buf) {
                Ok(()) => (),
                Err(err) => debug!("Failed to write cached code to disk: {}", err),
            }
        }
    }
}

impl FuncCacheData {
    pub fn from_tuple(data: FuncCacheDataTupleType) -> Self {
        Self {
            code_buf: data.0,
            jt_offsets: JtOffsetsSerdeWrapper(data.1),
            func_relocs: data.2,
            address_transform: data.3,
        }
    }

    pub fn to_tuple(self) -> FuncCacheDataTupleType {
        (
            self.code_buf,
            self.jt_offsets.0,
            self.func_relocs,
            self.address_transform,
        )
    }
}

impl Serialize for JtOffsetsSerdeWrapper {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let default_val = self.0.get_default();
        let mut seq = serializer.serialize_seq(Some(1 + self.0.len()))?;
        seq.serialize_element(&Some(default_val))?;
        for e in self.0.values() {
            let some_e = Some(e);
            seq.serialize_element(if e == default_val { &None } else { &some_e })?;
        }
        seq.end()
    }
}

impl<'de> Deserialize<'de> for JtOffsetsSerdeWrapper {
    fn deserialize<D>(deserializer: D) -> Result<JtOffsetsSerdeWrapper, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_seq(JtOffsetsSerdeWrapperVisitor::new())
    }
}

impl JtOffsetsSerdeWrapperVisitor {
    fn new() -> Self {
        JtOffsetsSerdeWrapperVisitor {
            marker: PhantomData,
        }
    }
}

impl<'de> Visitor<'de> for JtOffsetsSerdeWrapperVisitor {
    type Value = JtOffsetsSerdeWrapper;
    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("JtOffsetsSerdeWrapper")
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
                    match cranelift_codegen::ir::JumpTable::with_number(idx) {
                        Some(jt_idx) => m[jt_idx] = val.unwrap_or(default_val),
                        None => {
                            return Err(serde::de::Error::custom("Invalid JumpTable reference"))
                        }
                    };
                    idx += 1;
                }
                Ok(JtOffsetsSerdeWrapper(m))
            }
            _ => Err(serde::de::Error::custom("Default value required")),
        }
    }
}
