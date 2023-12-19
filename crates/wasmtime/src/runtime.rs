#[macro_use]
pub(crate) mod func;

pub(crate) mod code;
pub(crate) mod code_memory;
pub(crate) mod externals;
pub(crate) mod instance;
pub(crate) mod instantiate;
pub(crate) mod linker;
pub(crate) mod memory;
pub(crate) mod module;
pub(crate) mod r#ref;
pub(crate) mod runtime_engine;
pub(crate) mod signatures;
pub(crate) mod store;
pub(crate) mod trampoline;
pub(crate) mod trap;
pub(crate) mod types;
pub(crate) mod v128;
pub(crate) mod values;

#[cfg(feature = "component-model")]
pub mod component;

cfg_if::cfg_if! {
    if #[cfg(miri)] {
        // no extensions on miri
    } else if #[cfg(unix)] {
        pub mod unix;
    } else if #[cfg(windows)] {
        pub mod windows;
    } else {
        // ... unknown os!
    }
}
