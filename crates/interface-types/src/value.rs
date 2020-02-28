use std::convert::TryFrom;
use std::fmt;

/// The set of all possible WebAssembly Interface Types
#[derive(Debug, Clone)]
#[allow(missing_docs)]
pub enum Value {
    String(String),
    I32(i32),
    U32(u32),
    I64(i64),
    U64(u64),
    F32(f32),
    F64(f64),
}

macro_rules! from {
    ($($a:ident => $b:ident,)*) => ($(
        impl From<$a> for Value {
            fn from(val: $a) -> Value {
                Value::$b(val)
            }
        }

        impl TryFrom<Value> for $a {
            type Error = anyhow::Error;

            fn try_from(val: Value) -> Result<$a, Self::Error> {
                match val {
                    Value::$b(v) => Ok(v),
                    v => anyhow::bail!("cannot convert {:?} to {}", v, stringify!($a)),
                }
            }
        }
    )*)
}

from! {
    String => String,
    i32 => I32,
    u32 => U32,
    i64 => I64,
    u64 => U64,
    f32 => F32,
    f64 => F64,
}

impl<'a> From<&'a str> for Value {
    fn from(x: &'a str) -> Value {
        x.to_string().into()
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::String(s) => s.fmt(f),
            Value::I32(s) => s.fmt(f),
            Value::U32(s) => s.fmt(f),
            Value::I64(s) => s.fmt(f),
            Value::U64(s) => s.fmt(f),
            Value::F32(s) => s.fmt(f),
            Value::F64(s) => s.fmt(f),
        }
    }
}
