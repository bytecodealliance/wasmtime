use crate::wasmtime_component_resource_type_t;
use std::mem::{ManuallyDrop, MaybeUninit};
use wasmtime::component::types::*;

#[derive(Clone, PartialEq)]
#[repr(C, u8)]
pub enum wasmtime_component_valtype_t {
    Bool,
    S8,
    S16,
    S32,
    S64,
    U8,
    U16,
    U32,
    U64,
    F32,
    F64,
    Char,
    String,
    List(Box<wasmtime_component_list_type_t>),
    Record(Box<wasmtime_component_record_type_t>),
    Tuple(Box<wasmtime_component_tuple_type_t>),
    Variant(Box<wasmtime_component_variant_type_t>),
    Enum(Box<wasmtime_component_enum_type_t>),
    Option(Box<wasmtime_component_option_type_t>),
    Result(Box<wasmtime_component_result_type_t>),
    Flags(Box<wasmtime_component_flags_type_t>),
    Own(Box<wasmtime_component_resource_type_t>),
    Borrow(Box<wasmtime_component_resource_type_t>),
    Future(Box<wasmtime_component_future_type_t>),
    Stream(Box<wasmtime_component_stream_type_t>),
    ErrorContext,
    FixedSizeList(Box<wasmtime_component_list_type_t>),
}

impl From<Type> for wasmtime_component_valtype_t {
    fn from(item: Type) -> Self {
        match item {
            Type::Bool => Self::Bool,
            Type::S8 => Self::S8,
            Type::S16 => Self::S16,
            Type::S32 => Self::S32,
            Type::S64 => Self::S64,
            Type::U8 => Self::U8,
            Type::U16 => Self::U16,
            Type::U32 => Self::U32,
            Type::U64 => Self::U64,
            Type::Float32 => Self::F32,
            Type::Float64 => Self::F64,
            Type::Char => Self::Char,
            Type::String => Self::String,
            Type::List(ty) => Self::List(Box::new(ty.into())),
            Type::Record(ty) => Self::Record(Box::new(ty.into())),
            Type::Tuple(ty) => Self::Tuple(Box::new(ty.into())),
            Type::Variant(ty) => Self::Variant(Box::new(ty.into())),
            Type::Enum(ty) => Self::Enum(Box::new(ty.into())),
            Type::Option(ty) => Self::Option(Box::new(ty.into())),
            Type::Result(ty) => Self::Result(Box::new(ty.into())),
            Type::Flags(ty) => Self::Flags(Box::new(ty.into())),
            Type::Own(ty) => Self::Own(Box::new(ty.into())),
            Type::Borrow(ty) => Self::Borrow(Box::new(ty.into())),
            Type::Future(ty) => Self::Future(Box::new(ty.into())),
            Type::Stream(ty) => Self::Stream(Box::new(ty.into())),
            Type::FixedSizeList(ty) => Self::FixedSizeList(Box::new(ty.into())),
            Type::ErrorContext => Self::ErrorContext,
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_component_valtype_clone(
    ty: &wasmtime_component_valtype_t,
    ret: &mut MaybeUninit<wasmtime_component_valtype_t>,
) {
    ret.write(ty.clone());
}

#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_component_valtype_equal(
    a: &wasmtime_component_valtype_t,
    b: &wasmtime_component_valtype_t,
) -> bool {
    a == b
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_component_valtype_delete(
    ret: &mut ManuallyDrop<wasmtime_component_valtype_t>,
) {
    unsafe { ManuallyDrop::drop(ret) };
}

type_wrapper! {
    #[derive(PartialEq)]
    pub struct wasmtime_component_list_type_t {
        pub(crate) ty: List,
    }

    clone: wasmtime_component_list_type_clone,
    delete: wasmtime_component_list_type_delete,
    equal: wasmtime_component_list_type_equal,
}

#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_component_list_type_element(
    ty: &wasmtime_component_list_type_t,
    type_ret: &mut MaybeUninit<wasmtime_component_valtype_t>,
) {
    type_ret.write(ty.ty.ty().into());
}

type_wrapper! {
    #[derive(PartialEq)]
    pub struct wasmtime_component_record_type_t {
        pub(crate) ty: Record,
    }
    clone: wasmtime_component_record_type_clone,
    delete: wasmtime_component_record_type_delete,
    equal: wasmtime_component_record_type_equal,
}

#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_component_record_type_field_count(
    ty: &wasmtime_component_record_type_t,
) -> usize {
    ty.ty.fields().len()
}

#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_component_record_type_field_nth(
    ty: &wasmtime_component_record_type_t,
    nth: usize,
    name_ret: &mut MaybeUninit<*const u8>,
    name_len_ret: &mut MaybeUninit<usize>,
    type_ret: &mut MaybeUninit<wasmtime_component_valtype_t>,
) -> bool {
    match ty.ty.fields().nth(nth) {
        Some(Field { name, ty }) => {
            let name: &str = name;
            name_ret.write(name.as_ptr());
            name_len_ret.write(name.len());
            type_ret.write(ty.into());
            true
        }
        None => false,
    }
}

type_wrapper! {
    #[derive(PartialEq)]
    pub struct wasmtime_component_tuple_type_t {
        pub(crate) ty: Tuple,
    }
    clone: wasmtime_component_tuple_type_clone,
    delete: wasmtime_component_tuple_type_delete,
    equal: wasmtime_component_tuple_type_equal,
}

#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_component_tuple_type_types_count(
    ty: &wasmtime_component_tuple_type_t,
) -> usize {
    ty.ty.types().len()
}

#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_component_tuple_type_types_nth(
    ty: &wasmtime_component_tuple_type_t,
    nth: usize,
    type_ret: &mut MaybeUninit<wasmtime_component_valtype_t>,
) -> bool {
    match ty.ty.types().nth(nth) {
        Some(item) => {
            type_ret.write(item.into());
            true
        }
        None => false,
    }
}

type_wrapper! {
    #[derive(PartialEq)]
    pub struct wasmtime_component_variant_type_t {
        pub(crate) ty: Variant,
    }
    clone: wasmtime_component_variant_type_clone,
    delete: wasmtime_component_variant_type_delete,
    equal: wasmtime_component_variant_type_equal,
}

#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_component_variant_type_case_count(
    ty: &wasmtime_component_variant_type_t,
) -> usize {
    ty.ty.cases().len()
}

#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_component_variant_type_case_nth(
    ty: &wasmtime_component_variant_type_t,
    nth: usize,
    name_ret: &mut MaybeUninit<*const u8>,
    name_len_ret: &mut MaybeUninit<usize>,
    has_payload_ret: &mut MaybeUninit<bool>,
    payload_ret: &mut MaybeUninit<wasmtime_component_valtype_t>,
) -> bool {
    match ty.ty.cases().nth(nth) {
        Some(Case { name, ty }) => {
            let name: &str = name;
            name_ret.write(name.as_ptr());
            name_len_ret.write(name.len());
            match ty {
                Some(payload) => {
                    has_payload_ret.write(true);
                    payload_ret.write(payload.into());
                }
                None => {
                    has_payload_ret.write(false);
                }
            }
            true
        }
        None => false,
    }
}

type_wrapper! {
    #[derive(PartialEq)]
    pub struct wasmtime_component_enum_type_t {
        pub(crate) ty: Enum,
    }
    clone: wasmtime_component_enum_type_clone,
    delete: wasmtime_component_enum_type_delete,
    equal: wasmtime_component_enum_type_equal,
}

#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_component_enum_type_names_count(
    ty: &wasmtime_component_enum_type_t,
) -> usize {
    ty.ty.names().len()
}

#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_component_enum_type_names_nth(
    ty: &wasmtime_component_enum_type_t,
    nth: usize,
    name_ret: &mut MaybeUninit<*const u8>,
    name_len_ret: &mut MaybeUninit<usize>,
) -> bool {
    match ty.ty.names().nth(nth) {
        Some(name) => {
            let name: &str = name;
            name_ret.write(name.as_ptr());
            name_len_ret.write(name.len());
            true
        }
        None => false,
    }
}

type_wrapper! {
    #[derive(PartialEq)]
    pub struct wasmtime_component_option_type_t {
        pub(crate) ty: OptionType,
    }
    clone: wasmtime_component_option_type_clone,
    delete: wasmtime_component_option_type_delete,
    equal: wasmtime_component_option_type_equal,
}

#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_component_option_type_ty(
    ty: &wasmtime_component_option_type_t,
    ret: &mut MaybeUninit<wasmtime_component_valtype_t>,
) {
    ret.write(ty.ty.ty().into());
}

type_wrapper! {
    #[derive(PartialEq)]
    pub struct wasmtime_component_result_type_t {
        pub(crate) ty: ResultType,
    }
    clone: wasmtime_component_result_type_clone,
    delete: wasmtime_component_result_type_delete,
    equal: wasmtime_component_result_type_equal,
}

#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_component_result_type_ok(
    ty: &wasmtime_component_result_type_t,
    ret: &mut MaybeUninit<wasmtime_component_valtype_t>,
) -> bool {
    match ty.ty.ok() {
        Some(ok) => {
            ret.write(ok.into());
            true
        }
        None => false,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_component_result_type_err(
    ty: &wasmtime_component_result_type_t,
    ret: &mut MaybeUninit<wasmtime_component_valtype_t>,
) -> bool {
    match ty.ty.err() {
        Some(err) => {
            ret.write(err.into());
            true
        }
        None => false,
    }
}

type_wrapper! {
    #[derive(PartialEq)]
    pub struct wasmtime_component_flags_type_t {
        pub(crate) ty: Flags,
    }
    clone: wasmtime_component_flags_type_clone,
    delete: wasmtime_component_flags_type_delete,
    equal: wasmtime_component_flags_type_equal,
}

#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_component_flags_type_names_count(
    ty: &wasmtime_component_flags_type_t,
) -> usize {
    ty.ty.names().len()
}

#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_component_flags_type_names_nth(
    ty: &wasmtime_component_flags_type_t,
    nth: usize,
    name_ret: &mut MaybeUninit<*const u8>,
    name_len_ret: &mut MaybeUninit<usize>,
) -> bool {
    match ty.ty.names().nth(nth) {
        Some(name) => {
            let name: &str = name;
            name_ret.write(name.as_ptr());
            name_len_ret.write(name.len());
            true
        }
        None => false,
    }
}

type_wrapper! {
    #[derive(PartialEq)]
    pub struct wasmtime_component_future_type_t {
        pub(crate) ty: FutureType,
    }
    clone: wasmtime_component_future_type_clone,
    delete: wasmtime_component_future_type_delete,
    equal: wasmtime_component_future_type_equal,
}

#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_component_future_type_ty(
    ty: &wasmtime_component_future_type_t,
    ret: &mut MaybeUninit<wasmtime_component_valtype_t>,
) -> bool {
    match ty.ty.ty() {
        Some(ty) => {
            ret.write(ty.into());
            true
        }
        None => false,
    }
}

type_wrapper! {
    #[derive(PartialEq)]
    pub struct wasmtime_component_stream_type_t {
        pub(crate) ty: StreamType,
    }
    clone: wasmtime_component_stream_type_clone,
    delete: wasmtime_component_stream_type_delete,
    equal: wasmtime_component_stream_type_equal,
}

#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_component_stream_type_ty(
    ty: &wasmtime_component_stream_type_t,
    ret: &mut MaybeUninit<wasmtime_component_valtype_t>,
) -> bool {
    match ty.ty.ty() {
        Some(ty) => {
            ret.write(ty.into());
            true
        }
        None => false,
    }
}
