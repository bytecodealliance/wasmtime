//! In-progress implementation of the WebAssembly component model
//!
//! This module is a work-in-progress and currently represents an incomplete and
//! probably buggy implementation of the component model.

mod component;
mod func;
mod instance;
mod store;
pub use self::component::Component;
pub use self::func::Func;
pub use self::instance::Instance;

pub(crate) use self::store::ComponentStoreData;
