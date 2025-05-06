use wasmtime::component::Val;

use crate::wasm_name_t;

#[repr(C)]
pub struct wasmtime_component_vallist_t {
    ptr: *mut wasmtime_component_val_t,
    len: usize,
}

impl From<&wasmtime_component_vallist_t> for Vec<Val> {
    fn from(value: &wasmtime_component_vallist_t) -> Self {
        let x = unsafe { std::slice::from_raw_parts(value.ptr, value.len) };
        x.iter().map(Val::from).collect()
    }
}

impl From<&[Val]> for wasmtime_component_vallist_t {
    fn from(value: &[Val]) -> Self {
        let a = value
            .iter()
            .map(wasmtime_component_val_t::from)
            .collect::<Box<[_]>>();

        let a = Box::into_raw(a);

        Self {
            ptr: a.cast(),
            len: a.len(),
        }
    }
}

impl Drop for wasmtime_component_vallist_t {
    fn drop(&mut self) {
        unsafe {
            let slice = std::slice::from_raw_parts_mut(self.ptr, self.len);
            let _ = Box::from_raw(slice);
        }
    }
}

#[repr(C)]
pub struct wasmtime_component_valrecord_entry_t {
    name: wasm_name_t,
    val: wasmtime_component_val_t,
}

impl From<&wasmtime_component_valrecord_entry_t> for (String, Val) {
    fn from(value: &wasmtime_component_valrecord_entry_t) -> Self {
        (
            String::from_utf8(value.name.clone().take()).unwrap(),
            Val::from(&value.val),
        )
    }
}

impl From<&(String, Val)> for wasmtime_component_valrecord_entry_t {
    fn from((name, val): &(String, Val)) -> Self {
        Self {
            name: wasm_name_t::from_name(name.clone()),
            val: wasmtime_component_val_t::from(val),
        }
    }
}

#[repr(C)]
pub struct wasmtime_component_valrecord_t {
    ptr: *mut wasmtime_component_valrecord_entry_t,
    len: usize,
}

impl From<&wasmtime_component_valrecord_t> for Vec<(String, Val)> {
    fn from(value: &wasmtime_component_valrecord_t) -> Self {
        let value = unsafe { std::slice::from_raw_parts(value.ptr, value.len) };
        value.iter().map(|x| x.into()).collect()
    }
}

impl From<&[(String, Val)]> for wasmtime_component_valrecord_t {
    fn from(value: &[(String, Val)]) -> Self {
        let a = value
            .iter()
            .map(wasmtime_component_valrecord_entry_t::from)
            .collect::<Box<[_]>>();

        let a = Box::into_raw(a);

        Self {
            ptr: a.cast(),
            len: a.len(),
        }
    }
}

impl Drop for wasmtime_component_valrecord_t {
    fn drop(&mut self) {
        unsafe {
            let slice = std::slice::from_raw_parts_mut(self.ptr, self.len);
            let _ = Box::from_raw(slice);
        }
    }
}

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
    Char(char),
    String(wasm_name_t),
    List(wasmtime_component_vallist_t),
    Record(wasmtime_component_valrecord_t),
}

impl From<&wasmtime_component_val_t> for Val {
    fn from(value: &wasmtime_component_val_t) -> Self {
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
            wasmtime_component_val_t::Char(x) => Val::Char(*x),
            wasmtime_component_val_t::String(x) => {
                Val::String(String::from_utf8(x.clone().take()).unwrap())
            }
            wasmtime_component_val_t::List(x) => Val::List(x.into()),
            wasmtime_component_val_t::Record(x) => Val::Record(x.into()),
        }
    }
}

impl From<&Val> for wasmtime_component_val_t {
    fn from(value: &Val) -> Self {
        match value {
            Val::Bool(x) => wasmtime_component_val_t::Bool(*x),
            Val::S8(x) => wasmtime_component_val_t::S8(*x),
            Val::U8(x) => wasmtime_component_val_t::U8(*x),
            Val::S16(x) => wasmtime_component_val_t::S16(*x),
            Val::U16(x) => wasmtime_component_val_t::U16(*x),
            Val::S32(x) => wasmtime_component_val_t::S32(*x),
            Val::U32(x) => wasmtime_component_val_t::U32(*x),
            Val::S64(x) => wasmtime_component_val_t::S64(*x),
            Val::U64(x) => wasmtime_component_val_t::U64(*x),
            Val::Float32(x) => wasmtime_component_val_t::F32(*x),
            Val::Float64(x) => wasmtime_component_val_t::F64(*x),
            Val::Char(x) => wasmtime_component_val_t::Char(*x),
            Val::String(x) => wasmtime_component_val_t::String(wasm_name_t::from_name(x.clone())),
            Val::List(x) => wasmtime_component_val_t::List(x.as_slice().into()),
            Val::Record(x) => wasmtime_component_val_t::Record(x.as_slice().into()),
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_component_val_delete(value: *mut wasmtime_component_val_t) {
    unsafe {
        std::ptr::drop_in_place(value);
    }
}
