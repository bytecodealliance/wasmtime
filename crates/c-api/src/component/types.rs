macro_rules! type_wrapper {
    (
        $(#[$attr:meta])*
        pub struct $name:ident { pub(crate) ty: $ty:ty, }

        clone: $clone:ident,
        delete: $delete:ident,
        $(equal: $equal:ident,)?
    ) => {
        #[derive(Clone)]
        $(#[$attr])*
        pub struct $name {
            pub(crate) ty: $ty,
        }

        impl From<$ty> for $name {
            fn from(ty: $ty) -> Self {
                $name { ty }
            }
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn $clone(ty: &$name) -> Box<$name> {
            Box::new(ty.clone())
        }

        $(
            #[unsafe(no_mangle)]
            pub extern "C" fn $equal(a: &$name, b: &$name) -> bool {
                a.ty == b.ty
            }
        )?

        #[unsafe(no_mangle)]
        pub extern "C" fn $delete(_ty: Option<Box<$name>>) {}
    };
}

mod component;
mod func;
mod instance;
mod module;
mod resource;
mod val;

pub use component::*;
pub use func::*;
pub use instance::*;
pub use module::*;
pub use resource::*;
pub use val::*;
