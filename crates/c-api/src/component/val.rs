use wasmtime::component::Val;

#[repr(C, u8)]
pub enum wasmtime_component_val_t {
    Bool(bool),
    S8(i8),
    U8(u8),
    S16(i16),
    U16(u16),
    S32(i32),
    U32(u32),
    S64(i64),
    U64(u64),
    F32(f32),
    F64(f64),
}

impl From<&mut wasmtime_component_val_t> for Val {
    fn from(value: &mut wasmtime_component_val_t) -> Self {
        match value {
            wasmtime_component_val_t::Bool(x) => Val::Bool(*x),
            wasmtime_component_val_t::S8(x) => Val::S8(*x),
            wasmtime_component_val_t::U8(x) => Val::U8(*x),
            wasmtime_component_val_t::S16(x) => Val::S16(*x),
            wasmtime_component_val_t::U16(x) => Val::U16(*x),
            wasmtime_component_val_t::S32(x) => Val::S32(*x),
            wasmtime_component_val_t::U32(x) => Val::U32(*x),
            wasmtime_component_val_t::S64(x) => Val::S64(*x),
            wasmtime_component_val_t::U64(x) => Val::U64(*x),
            wasmtime_component_val_t::F32(x) => Val::Float32(*x),
            wasmtime_component_val_t::F64(x) => Val::Float64(*x),
        }
    }
}

impl From<Val> for wasmtime_component_val_t {
    fn from(value: Val) -> Self {
        match value {
            Val::Bool(x) => wasmtime_component_val_t::Bool(x),
            Val::S8(x) => wasmtime_component_val_t::S8(x),
            Val::U8(x) => wasmtime_component_val_t::U8(x),
            Val::S16(x) => wasmtime_component_val_t::S16(x),
            Val::U16(x) => wasmtime_component_val_t::U16(x),
            Val::S32(x) => wasmtime_component_val_t::S32(x),
            Val::U32(x) => wasmtime_component_val_t::U32(x),
            Val::S64(x) => wasmtime_component_val_t::S64(x),
            Val::U64(x) => wasmtime_component_val_t::U64(x),
            Val::Float32(x) => wasmtime_component_val_t::F32(x),
            Val::Float64(x) => wasmtime_component_val_t::F64(x),
            Val::Char(_) => todo!(),
            Val::String(_) => todo!(),
            Val::List(_vals) => todo!(),
            Val::Record(_items) => todo!(),
            Val::Tuple(_vals) => todo!(),
            Val::Variant(_, _val) => todo!(),
            Val::Enum(_) => todo!(),
            Val::Option(_val) => todo!(),
            Val::Result(_val) => todo!(),
            Val::Flags(_items) => todo!(),
            Val::Resource(_resource_any) => todo!(),
        }
    }
}
