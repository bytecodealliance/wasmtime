pub use wasmtime_rust_macro::wasmtime;

// modules used by the macro
#[doc(hidden)]
pub mod __rt {
    pub use cranelift_codegen;
    pub use cranelift_native;
    pub use failure;
    pub use wasmtime_interface_types;
    pub use wasmtime_jit;

    use std::convert::{TryFrom, TryInto};
    use wasmtime_interface_types::Value;

    pub trait FromVecValue: Sized {
        fn from(list: Vec<Value>) -> Result<Self, failure::Error>;
    }

    macro_rules! tuple {
        ($(($($a:ident),*),)*) => ($(
            impl<$($a: TryFrom<Value>),*> FromVecValue for ($($a,)*)
                where $(failure::Error: From<$a::Error>,)*
            {
                #[allow(non_snake_case)]
                fn from(list: Vec<Value>) -> Result<Self, failure::Error> {
                    let mut iter = list.into_iter();
                    $(
                        let $a = iter.next()
                            .ok_or_else(|| failure::format_err!("not enough values"))?
                            .try_into()?;
                    )*
                    if iter.next().is_some() {
                        failure::format_err!("too many return values");
                    }
                    Ok(($($a,)*))
                }
            }
        )*)
    }

    tuple! {
        (),
        (A),
        (A, B),
        (A, B, C),
    }
}
