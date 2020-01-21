#![cfg_attr(test, feature(test))]
#![feature(proc_macro_hygiene, type_alias_impl_trait)]

#[cfg(test)]
extern crate test;

mod backend;
mod disassemble;
mod error;
mod function_body;
mod microwasm;
mod module;
mod translate_sections;

#[cfg(test)]
mod benches;

pub use crate::backend::CodeGenSession;
pub use crate::function_body::translate_wasm as translate_function;
pub use crate::module::{
    translate, ExecutableModule, ExecutionError, ModuleContext, Signature, TranslatedModule,
};

pub struct StrErr {
    pub message: std::borrow::Cow<'static, str>,
}

impl From<&'static str> for StrErr {
    fn from(message: &'static str) -> Self {
        StrErr {
            message: message.into(),
        }
    }
}

impl From<String> for StrErr {
    fn from(message: String) -> Self {
        StrErr {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for StrErr {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.message.fmt(f)
    }
}

impl std::fmt::Debug for StrErr {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.message.fmt(f)
    }
}

impl std::error::Error for StrErr {
    fn description(&self) -> &str {
        self.message.as_ref()
    }
}
