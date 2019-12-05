pub use wasmtime_rust_macro::wasmtime;

// modules used by the macro
#[doc(hidden)]
pub mod __rt {
    pub use anyhow;
    pub use wasmtime;
    pub use wasmtime_wasi;

    use std::convert::{TryFrom, TryInto};

    pub trait FromVecValue: Sized {
        fn from(list: Vec<wasmtime::Val>) -> anyhow::Result<Self>;
    }

    macro_rules! tuple {
        ($(($($a:ident),*),)*) => ($(
            impl<$($a: TryFrom<wasmtime::Val>),*> FromVecValue for ($($a,)*)
                where $(anyhow::Error: From<$a::Error>,)*
            {
                #[allow(non_snake_case)]
                fn from(list: Vec<wasmtime::Val>) -> anyhow::Result<Self> {
                    let mut iter = list.into_iter();
                    $(
                        let $a = iter.next()
                            .ok_or_else(|| anyhow::format_err!("not enough values"))?
                            .try_into()?;
                    )*
                    if iter.next().is_some() {
                        anyhow::bail!("too many return values");
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
