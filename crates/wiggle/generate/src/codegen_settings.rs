use crate::config::{AsyncConf, ErrorConf};
use anyhow::{anyhow, Error};
use proc_macro2::TokenStream;
use quote::quote;
use std::collections::HashMap;
use std::rc::Rc;
use witx::{Document, Id, InterfaceFunc, Module, NamedType, TypeRef};

pub use crate::config::Asyncness;

pub struct CodegenSettings {
    pub errors: ErrorTransform,
    pub async_: AsyncConf,
    pub wasmtime: bool,
}
impl CodegenSettings {
    pub fn new(
        error_conf: &ErrorConf,
        async_: &AsyncConf,
        doc: &Document,
        wasmtime: bool,
    ) -> Result<Self, Error> {
        let errors = ErrorTransform::new(error_conf, doc)?;
        Ok(Self {
            errors,
            async_: async_.clone(),
            wasmtime,
        })
    }
    pub fn get_async(&self, module: &Module, func: &InterfaceFunc) -> Asyncness {
        self.async_.get(module.name.as_str(), func.name.as_str())
    }
}

pub struct ErrorTransform {
    m: Vec<UserErrorType>,
}

impl ErrorTransform {
    pub fn empty() -> Self {
        Self { m: Vec::new() }
    }
    pub fn new(conf: &ErrorConf, doc: &Document) -> Result<Self, Error> {
        let mut richtype_identifiers = HashMap::new();
        let m = conf.iter().map(|(ident, field)|
            if let Some(abi_type) = doc.typename(&Id::new(ident.to_string())) {
                    if let Some(ident) = field.rich_error.get_ident() {
                        if let Some(prior_def) = richtype_identifiers.insert(ident.clone(), field.err_loc.clone())
                         {
                            return Err(anyhow!(
                                    "duplicate rich type identifier of {:?} not allowed. prior definition at {:?}",
                                    ident, prior_def
                                ));
                        }
                        Ok(UserErrorType {
                            abi_type,
                            rich_type: field.rich_error.clone(),
                            method_fragment: ident.to_string()
                        })
                    } else {
                        return Err(anyhow!(
                            "rich error type must be identifier for now - TODO add ability to provide a corresponding identifier: {:?}",
                            field.err_loc
                        ))
                    }
                }
                else { Err(anyhow!("No witx typename \"{}\" found", ident.to_string())) }
        ).collect::<Result<Vec<_>, Error>>()?;
        Ok(Self { m })
    }

    pub fn iter(&self) -> impl Iterator<Item = &UserErrorType> {
        self.m.iter()
    }

    pub fn for_abi_error(&self, tref: &TypeRef) -> Option<&UserErrorType> {
        match tref {
            TypeRef::Name(nt) => self.for_name(nt),
            TypeRef::Value { .. } => None,
        }
    }

    pub fn for_name(&self, nt: &NamedType) -> Option<&UserErrorType> {
        self.m.iter().find(|u| u.abi_type.name == nt.name)
    }
}

pub struct UserErrorType {
    abi_type: Rc<NamedType>,
    rich_type: syn::Path,
    method_fragment: String,
}

impl UserErrorType {
    pub fn abi_type(&self) -> TypeRef {
        TypeRef::Name(self.abi_type.clone())
    }
    pub fn typename(&self) -> TokenStream {
        let t = &self.rich_type;
        quote!(#t)
    }
    pub fn method_fragment(&self) -> &str {
        &self.method_fragment
    }
}
