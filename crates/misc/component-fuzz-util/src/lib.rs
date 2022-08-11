//! This module generates test cases for the Wasmtime component model function APIs,
//! e.g. `wasmtime::component::func::Func` and `TypedFunc`.
//!
//! Each case includes a list of arbitrary interface types to use as parameters, plus another one to use as a
//! result, and a component which exports a function and imports a function.  The exported function forwards its
//! parameters to the imported one and forwards the result back to the caller.  This serves to excercise Wasmtime's
//! lifting and lowering code and verify the values remain intact during both processes.

use arbitrary::{Arbitrary, Unstructured};
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote, ToTokens};
use std::borrow::Cow;
use std::fmt::{self, Debug, Write};
use std::iter;
use std::ops::Deref;
use wasmtime_component_util::{DiscriminantSize, FlagsSize, REALLOC_AND_FREE};

const MAX_FLAT_PARAMS: usize = 16;
const MAX_FLAT_RESULTS: usize = 1;
const MAX_ARITY: usize = 5;

/// The name of the imported host function which the generated component will call
pub const IMPORT_FUNCTION: &str = "echo";

/// The name of the exported guest function which the host should call
pub const EXPORT_FUNCTION: &str = "echo";

#[derive(Copy, Clone, PartialEq, Eq)]
enum CoreType {
    I32,
    I64,
    F32,
    F64,
}

impl CoreType {
    /// This is the `join` operation specified in [the canonical
    /// ABI](https://github.com/WebAssembly/component-model/blob/main/design/mvp/CanonicalABI.md#flattening) for
    /// variant types.
    fn join(self, other: Self) -> Self {
        match (self, other) {
            _ if self == other => self,
            (Self::I32, Self::F32) | (Self::F32, Self::I32) => Self::I32,
            _ => Self::I64,
        }
    }
}

impl fmt::Display for CoreType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::I32 => f.write_str("i32"),
            Self::I64 => f.write_str("i64"),
            Self::F32 => f.write_str("f32"),
            Self::F64 => f.write_str("f64"),
        }
    }
}

#[derive(Debug)]
pub struct UsizeInRange<const L: usize, const H: usize>(usize);

impl<const L: usize, const H: usize> UsizeInRange<L, H> {
    pub fn as_usize(&self) -> usize {
        self.0
    }
}

impl<'a, const L: usize, const H: usize> Arbitrary<'a> for UsizeInRange<L, H> {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        Ok(UsizeInRange(u.int_in_range(L..=H)?))
    }
}

/// Wraps a `Box<[T]>` and provides an `Arbitrary` implementation that always generates slices of length less than
/// or equal to the longest tuple for which Wasmtime generates a `ComponentType` impl
#[derive(Debug)]
pub struct VecInRange<T, const L: u32, const H: u32>(Vec<T>);

impl<'a, T: Arbitrary<'a>, const L: u32, const H: u32> Arbitrary<'a> for VecInRange<T, L, H> {
    fn arbitrary(input: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        let mut ret = Vec::new();
        input.arbitrary_loop(Some(L), Some(H), |input| {
            ret.push(input.arbitrary()?);
            Ok(std::ops::ControlFlow::Continue(()))
        })?;
        Ok(Self(ret))
    }
}

impl<T, const L: u32, const H: u32> Deref for VecInRange<T, L, H> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        self.0.deref()
    }
}

/// Represents a component model interface type
#[allow(missing_docs)]
#[derive(Arbitrary, Debug)]
pub enum Type {
    Unit,
    Bool,
    S8,
    U8,
    S16,
    U16,
    S32,
    U32,
    S64,
    U64,
    Float32,
    Float64,
    Char,
    String,
    List(Box<Type>),

    // Give records the ability to generate a generous amount of fields but
    // don't let the fuzzer go too wild since `wasmparser`'s validator currently
    // has hard limits in the 1000-ish range on the number of fields a record
    // may contain.
    Record(VecInRange<Type, 0, 200>),

    // Tuples can only have up to 16 type parameters in wasmtime right now for
    // the static API, but the standard library only supports `Debug` up to 11
    // elements, so compromise at an even 10.
    Tuple(VecInRange<Type, 0, 10>),

    // Like records, allow a good number of variants, but variants require at
    // least one case.
    Variant(VecInRange<Type, 1, 200>),
    Enum(UsizeInRange<1, 257>),
    Union(VecInRange<Type, 1, 200>),

    Option(Box<Type>),
    Expected { ok: Box<Type>, err: Box<Type> },

    // Generate 0 flags all the way up to 65 flags which exercises the 0 to
    // 3 x u32 cases.
    Flags(UsizeInRange<0, 65>),
}

fn lower_record<'a>(types: impl Iterator<Item = &'a Type>, vec: &mut Vec<CoreType>) {
    for ty in types {
        ty.lower(vec);
    }
}

fn lower_variant<'a>(types: impl Iterator<Item = &'a Type>, vec: &mut Vec<CoreType>) {
    vec.push(CoreType::I32);
    let offset = vec.len();
    for ty in types {
        for (index, ty) in ty.lowered().iter().enumerate() {
            let index = offset + index;
            if index < vec.len() {
                vec[index] = vec[index].join(*ty);
            } else {
                vec.push(*ty)
            }
        }
    }
}

fn u32_count_from_flag_count(count: usize) -> usize {
    match FlagsSize::from_count(count) {
        FlagsSize::Size0 => 0,
        FlagsSize::Size1 | FlagsSize::Size2 => 1,
        FlagsSize::Size4Plus(n) => n,
    }
}

struct SizeAndAlignment {
    size: usize,
    alignment: u32,
}

impl Type {
    fn lowered(&self) -> Vec<CoreType> {
        let mut vec = Vec::new();
        self.lower(&mut vec);
        vec
    }

    fn lower(&self, vec: &mut Vec<CoreType>) {
        match self {
            Type::Unit => (),
            Type::Bool
            | Type::U8
            | Type::S8
            | Type::S16
            | Type::U16
            | Type::S32
            | Type::U32
            | Type::Char
            | Type::Enum(_) => vec.push(CoreType::I32),
            Type::S64 | Type::U64 => vec.push(CoreType::I64),
            Type::Float32 => vec.push(CoreType::F32),
            Type::Float64 => vec.push(CoreType::F64),
            Type::String | Type::List(_) => {
                vec.push(CoreType::I32);
                vec.push(CoreType::I32);
            }
            Type::Record(types) => lower_record(types.iter(), vec),
            Type::Tuple(types) => lower_record(types.0.iter(), vec),
            Type::Variant(types) | Type::Union(types) => lower_variant(types.0.iter(), vec),
            Type::Option(ty) => lower_variant([&Type::Unit, ty].into_iter(), vec),
            Type::Expected { ok, err } => lower_variant([ok.deref(), err].into_iter(), vec),
            Type::Flags(count) => {
                vec.extend(iter::repeat(CoreType::I32).take(u32_count_from_flag_count(count.0)))
            }
        }
    }

    fn size_and_alignment(&self) -> SizeAndAlignment {
        match self {
            Type::Unit => SizeAndAlignment {
                size: 0,
                alignment: 1,
            },

            Type::Bool | Type::S8 | Type::U8 => SizeAndAlignment {
                size: 1,
                alignment: 1,
            },

            Type::S16 | Type::U16 => SizeAndAlignment {
                size: 2,
                alignment: 2,
            },

            Type::S32 | Type::U32 | Type::Char | Type::Float32 => SizeAndAlignment {
                size: 4,
                alignment: 4,
            },

            Type::S64 | Type::U64 | Type::Float64 => SizeAndAlignment {
                size: 8,
                alignment: 8,
            },

            Type::String | Type::List(_) => SizeAndAlignment {
                size: 8,
                alignment: 4,
            },

            Type::Record(types) => record_size_and_alignment(types.iter()),

            Type::Tuple(types) => record_size_and_alignment(types.0.iter()),

            Type::Variant(types) | Type::Union(types) => variant_size_and_alignment(types.0.iter()),

            Type::Enum(count) => variant_size_and_alignment((0..count.0).map(|_| &Type::Unit)),

            Type::Option(ty) => variant_size_and_alignment([&Type::Unit, ty].into_iter()),

            Type::Expected { ok, err } => variant_size_and_alignment([ok.deref(), err].into_iter()),

            Type::Flags(count) => match FlagsSize::from_count(count.0) {
                FlagsSize::Size0 => SizeAndAlignment {
                    size: 0,
                    alignment: 1,
                },
                FlagsSize::Size1 => SizeAndAlignment {
                    size: 1,
                    alignment: 1,
                },
                FlagsSize::Size2 => SizeAndAlignment {
                    size: 2,
                    alignment: 2,
                },
                FlagsSize::Size4Plus(n) => SizeAndAlignment {
                    size: n * 4,
                    alignment: 4,
                },
            },
        }
    }
}

fn align_to(a: usize, align: u32) -> usize {
    let align = align as usize;
    (a + (align - 1)) & !(align - 1)
}

fn record_size_and_alignment<'a>(types: impl Iterator<Item = &'a Type>) -> SizeAndAlignment {
    let mut offset = 0;
    let mut align = 1;
    for ty in types {
        let SizeAndAlignment { size, alignment } = ty.size_and_alignment();
        offset = align_to(offset, alignment) + size;
        align = align.max(alignment);
    }

    SizeAndAlignment {
        size: align_to(offset, align),
        alignment: align,
    }
}

fn variant_size_and_alignment<'a>(
    types: impl ExactSizeIterator<Item = &'a Type>,
) -> SizeAndAlignment {
    let discriminant_size = DiscriminantSize::from_count(types.len()).unwrap();
    let mut alignment = u32::from(discriminant_size);
    let mut size = 0;
    for ty in types {
        let size_and_alignment = ty.size_and_alignment();
        alignment = alignment.max(size_and_alignment.alignment);
        size = size.max(size_and_alignment.size);
    }

    SizeAndAlignment {
        size: align_to(
            align_to(usize::from(discriminant_size), alignment) + size,
            alignment,
        ),
        alignment,
    }
}

fn make_import_and_export(params: &[Type], result: &Type) -> String {
    let params_lowered = params
        .iter()
        .flat_map(|ty| ty.lowered())
        .collect::<Box<[_]>>();
    let result_lowered = result.lowered();

    let mut core_params = String::new();
    let mut gets = String::new();

    if params_lowered.len() <= MAX_FLAT_PARAMS {
        for (index, param) in params_lowered.iter().enumerate() {
            write!(&mut core_params, " {param}").unwrap();
            write!(&mut gets, "local.get {index} ").unwrap();
        }
    } else {
        write!(&mut core_params, " i32").unwrap();
        write!(&mut gets, "local.get 0 ").unwrap();
    }

    let maybe_core_params = if params_lowered.is_empty() {
        String::new()
    } else {
        format!("(param{core_params})")
    };

    if result_lowered.len() <= MAX_FLAT_RESULTS {
        let mut core_results = String::new();
        for result in result_lowered.iter() {
            write!(&mut core_results, " {result}").unwrap();
        }

        let maybe_core_results = if result_lowered.is_empty() {
            String::new()
        } else {
            format!("(result{core_results})")
        };

        format!(
            r#"
            (func $f (import "host" "{IMPORT_FUNCTION}") {maybe_core_params} {maybe_core_results})

            (func (export "{EXPORT_FUNCTION}") {maybe_core_params} {maybe_core_results}
                {gets}

                call $f
            )"#
        )
    } else {
        let SizeAndAlignment { size, alignment } = result.size_and_alignment();

        format!(
            r#"
            (func $f (import "host" "{IMPORT_FUNCTION}") (param{core_params} i32))

            (func (export "{EXPORT_FUNCTION}") {maybe_core_params} (result i32)
                (local $base i32)
                (local.set $base
                    (call $realloc
                        (i32.const 0)
                        (i32.const 0)
                        (i32.const {alignment})
                        (i32.const {size})))
                {gets}
                local.get $base

                call $f

                local.get $base
            )"#
        )
    }
}

fn make_rust_name(name_counter: &mut u32) -> Ident {
    let name = format_ident!("Foo{name_counter}");
    *name_counter += 1;
    name
}

/// Generate a [`TokenStream`] containing the rust type name for a type.
///
/// The `name_counter` parameter is used to generate names for each recursively visited type.  The `declarations`
/// parameter is used to accumulate declarations for each recursively visited type.
pub fn rust_type(ty: &Type, name_counter: &mut u32, declarations: &mut TokenStream) -> TokenStream {
    match ty {
        Type::Unit => quote!(()),
        Type::Bool => quote!(bool),
        Type::S8 => quote!(i8),
        Type::U8 => quote!(u8),
        Type::S16 => quote!(i16),
        Type::U16 => quote!(u16),
        Type::S32 => quote!(i32),
        Type::U32 => quote!(u32),
        Type::S64 => quote!(i64),
        Type::U64 => quote!(u64),
        Type::Float32 => quote!(Float32),
        Type::Float64 => quote!(Float64),
        Type::Char => quote!(char),
        Type::String => quote!(Box<str>),
        Type::List(ty) => {
            let ty = rust_type(ty, name_counter, declarations);
            quote!(Vec<#ty>)
        }
        Type::Record(types) => {
            let fields = types
                .iter()
                .enumerate()
                .map(|(index, ty)| {
                    let name = format_ident!("f{index}");
                    let ty = rust_type(ty, name_counter, declarations);
                    quote!(#name: #ty,)
                })
                .collect::<TokenStream>();

            let name = make_rust_name(name_counter);

            declarations.extend(quote! {
                #[derive(ComponentType, Lift, Lower, PartialEq, Debug, Clone, Arbitrary)]
                #[component(record)]
                struct #name {
                    #fields
                }
            });

            quote!(#name)
        }
        Type::Tuple(types) => {
            let fields = types
                .0
                .iter()
                .map(|ty| {
                    let ty = rust_type(ty, name_counter, declarations);
                    quote!(#ty,)
                })
                .collect::<TokenStream>();

            quote!((#fields))
        }
        Type::Variant(types) | Type::Union(types) => {
            let cases = types
                .0
                .iter()
                .enumerate()
                .map(|(index, ty)| {
                    let name = format_ident!("C{index}");
                    let ty = rust_type(ty, name_counter, declarations);
                    quote!(#name(#ty),)
                })
                .collect::<TokenStream>();

            let name = make_rust_name(name_counter);

            let which = if let Type::Variant(_) = ty {
                quote!(variant)
            } else {
                quote!(union)
            };

            declarations.extend(quote! {
                #[derive(ComponentType, Lift, Lower, PartialEq, Debug, Clone, Arbitrary)]
                #[component(#which)]
                enum #name {
                    #cases
                }
            });

            quote!(#name)
        }
        Type::Enum(count) => {
            let cases = (0..count.0)
                .map(|index| {
                    let name = format_ident!("C{index}");
                    quote!(#name,)
                })
                .collect::<TokenStream>();

            let name = make_rust_name(name_counter);

            declarations.extend(quote! {
                #[derive(ComponentType, Lift, Lower, PartialEq, Debug, Copy, Clone, Arbitrary)]
                #[component(enum)]
                enum #name {
                    #cases
                }
            });

            quote!(#name)
        }
        Type::Option(ty) => {
            let ty = rust_type(ty, name_counter, declarations);
            quote!(Option<#ty>)
        }
        Type::Expected { ok, err } => {
            let ok = rust_type(ok, name_counter, declarations);
            let err = rust_type(err, name_counter, declarations);
            quote!(Result<#ok, #err>)
        }
        Type::Flags(count) => {
            let type_name = make_rust_name(name_counter);

            let mut flags = TokenStream::new();
            let mut names = TokenStream::new();

            for index in 0..count.0 {
                let name = format_ident!("F{index}");
                flags.extend(quote!(const #name;));
                names.extend(quote!(#type_name::#name,))
            }

            declarations.extend(quote! {
                wasmtime::component::flags! {
                    #type_name {
                        #flags
                    }
                }

                impl<'a> Arbitrary<'a> for #type_name {
                    fn arbitrary(input: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
                        let mut flags = #type_name::default();
                        for flag in [#names] {
                            if input.arbitrary()? {
                                flags |= flag;
                            }
                        }
                        Ok(flags)
                    }
                }
            });

            quote!(#type_name)
        }
    }
}

fn make_component_name(name_counter: &mut u32) -> String {
    let name = format!("$Foo{name_counter}");
    *name_counter += 1;
    name
}

fn write_component_type(
    ty: &Type,
    f: &mut String,
    name_counter: &mut u32,
    declarations: &mut String,
) {
    match ty {
        Type::Unit => f.push_str("unit"),
        Type::Bool => f.push_str("bool"),
        Type::S8 => f.push_str("s8"),
        Type::U8 => f.push_str("u8"),
        Type::S16 => f.push_str("s16"),
        Type::U16 => f.push_str("u16"),
        Type::S32 => f.push_str("s32"),
        Type::U32 => f.push_str("u32"),
        Type::S64 => f.push_str("s64"),
        Type::U64 => f.push_str("u64"),
        Type::Float32 => f.push_str("float32"),
        Type::Float64 => f.push_str("float64"),
        Type::Char => f.push_str("char"),
        Type::String => f.push_str("string"),
        Type::List(ty) => {
            let mut case = String::new();
            write_component_type(ty, &mut case, name_counter, declarations);
            let name = make_component_name(name_counter);
            write!(declarations, "(type {name} (list {case}))").unwrap();
            f.push_str(&name);
        }
        Type::Record(types) => {
            let mut fields = String::new();
            for (index, ty) in types.iter().enumerate() {
                write!(fields, r#" (field "f{index}" "#).unwrap();
                write_component_type(ty, &mut fields, name_counter, declarations);
                fields.push_str(")");
            }
            let name = make_component_name(name_counter);
            write!(declarations, "(type {name} (record{fields}))").unwrap();
            f.push_str(&name);
        }
        Type::Tuple(types) => {
            let mut fields = String::new();
            for ty in types.0.iter() {
                fields.push_str(" ");
                write_component_type(ty, &mut fields, name_counter, declarations);
            }
            let name = make_component_name(name_counter);
            write!(declarations, "(type {name} (tuple{fields}))").unwrap();
            f.push_str(&name);
        }
        Type::Variant(types) => {
            let mut cases = String::new();
            for (index, ty) in types.0.iter().enumerate() {
                write!(cases, r#" (case "C{index}" "#).unwrap();
                write_component_type(ty, &mut cases, name_counter, declarations);
                cases.push_str(")");
            }
            let name = make_component_name(name_counter);
            write!(declarations, "(type {name} (variant{cases}))").unwrap();
            f.push_str(&name);
        }
        Type::Enum(count) => {
            f.push_str("(enum");
            for index in 0..count.0 {
                write!(f, r#" "C{index}""#).unwrap();
            }
            f.push_str(")");
        }
        Type::Union(types) => {
            let mut cases = String::new();
            for ty in types.0.iter() {
                cases.push_str(" ");
                write_component_type(ty, &mut cases, name_counter, declarations);
            }
            let name = make_component_name(name_counter);
            write!(declarations, "(type {name} (union{cases}))").unwrap();
            f.push_str(&name);
        }
        Type::Option(ty) => {
            let mut case = String::new();
            write_component_type(ty, &mut case, name_counter, declarations);
            let name = make_component_name(name_counter);
            write!(declarations, "(type {name} (option {case}))").unwrap();
            f.push_str(&name);
        }
        Type::Expected { ok, err } => {
            let mut cases = String::new();
            write_component_type(ok, &mut cases, name_counter, declarations);
            cases.push_str(" ");
            write_component_type(err, &mut cases, name_counter, declarations);
            let name = make_component_name(name_counter);
            write!(declarations, "(type {name} (expected {cases}))").unwrap();
            f.push_str(&name);
        }
        Type::Flags(count) => {
            f.push_str("(flags");
            for index in 0..count.0 {
                write!(f, r#" "F{index}""#).unwrap();
            }
            f.push_str(")");
        }
    }
}

/// Represents custom fragments of a WAT file which may be used to create a component for exercising [`TestCase`]s
#[derive(Debug)]
pub struct Declarations {
    /// Type declarations (if any) referenced by `params` and/or `result`
    pub types: Cow<'static, str>,
    /// Parameter declarations used for the imported and exported functions
    pub params: Cow<'static, str>,
    /// Result declaration used for the imported and exported functions
    pub result: Cow<'static, str>,
    /// A WAT fragment representing the core function import and export to use for testing
    pub import_and_export: Cow<'static, str>,
    /// String encoding to use for host -> component
    pub encoding1: StringEncoding,
    /// String encoding to use for component -> host
    pub encoding2: StringEncoding,
}

impl Declarations {
    /// Generate a complete WAT file based on the specified fragments.
    pub fn make_component(&self) -> Box<str> {
        let Self {
            types,
            params,
            result,
            import_and_export,
            encoding1,
            encoding2,
        } = self;
        let mk_component = |name: &str, encoding: StringEncoding| {
            format!(
                r#"
                (component ${name}
                    (import "echo" (func $f (type $sig)))

                    (core instance $libc (instantiate $libc))

                    (core func $f_lower (canon lower
                        (func $f)
                        (memory $libc "memory")
                        (realloc (func $libc "realloc"))
                        string-encoding={encoding}
                    ))

                    (core instance $i (instantiate $m
                        (with "libc" (instance $libc))
                        (with "host" (instance (export "{IMPORT_FUNCTION}" (func $f_lower))))
                    ))

                    (func (export "echo") (type $sig)
                        (canon lift
                            (core func $i "echo")
                            (memory $libc "memory")
                            (realloc (func $libc "realloc"))
                            string-encoding={encoding}
                        )
                    )
                )
            "#
            )
        };

        let c1 = mk_component("c1", *encoding2);
        let c2 = mk_component("c2", *encoding1);

        format!(
            r#"
            (component
                (core module $libc
                    (memory (export "memory") 1)
                    {REALLOC_AND_FREE}
                )

                (core module $m
                    (memory (import "libc" "memory") 1)
                    (func $realloc (import "libc" "realloc") (param i32 i32 i32 i32) (result i32))

                    {import_and_export}
                )

                {types}

                (type $sig (func {params} {result}))
                (import "{IMPORT_FUNCTION}" (func $f (type $sig)))

                {c1}
                {c2}
                (instance $c1 (instantiate $c1 (with "echo" (func $f))))
                (instance $c2 (instantiate $c2 (with "echo" (func $c1 "echo"))))
                (export "echo" (func $c2 "echo"))
            )"#,
        )
        .into()
    }
}

/// Represents a test case for calling a component function
#[derive(Debug)]
pub struct TestCase {
    /// The types of parameters to pass to the function
    pub params: Box<[Type]>,
    /// The type of the result to be returned by the function
    pub result: Type,
    /// String encoding to use from host-to-component.
    pub encoding1: StringEncoding,
    /// String encoding to use from component-to-host.
    pub encoding2: StringEncoding,
}

impl TestCase {
    /// Generate a `Declarations` for this `TestCase` which may be used to build a component to execute the case.
    pub fn declarations(&self) -> Declarations {
        let mut types = String::new();
        let name_counter = &mut 0;

        let params = self
            .params
            .iter()
            .map(|ty| {
                let mut tmp = String::new();
                write_component_type(ty, &mut tmp, name_counter, &mut types);
                format!("(param {tmp})")
            })
            .collect::<Box<[_]>>()
            .join(" ")
            .into();

        let result = {
            let mut tmp = String::new();
            write_component_type(&self.result, &mut tmp, name_counter, &mut types);
            format!("(result {tmp})")
        }
        .into();

        let import_and_export = make_import_and_export(&self.params, &self.result);

        Declarations {
            types: types.into(),
            params,
            result,
            import_and_export: import_and_export.into(),
            encoding1: self.encoding1,
            encoding2: self.encoding2,
        }
    }
}

impl<'a> Arbitrary<'a> for TestCase {
    /// Generate an arbitrary [`TestCase`].
    fn arbitrary(input: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        Ok(Self {
            params: input
                .arbitrary_iter()?
                .take(MAX_ARITY)
                .collect::<arbitrary::Result<Box<[_]>>>()?,
            result: input.arbitrary()?,
            encoding1: input.arbitrary()?,
            encoding2: input.arbitrary()?,
        })
    }
}

#[derive(Copy, Clone, Debug, Arbitrary)]
pub enum StringEncoding {
    Utf8,
    Utf16,
    Latin1OrUtf16,
}

impl fmt::Display for StringEncoding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StringEncoding::Utf8 => fmt::Display::fmt(&"utf8", f),
            StringEncoding::Utf16 => fmt::Display::fmt(&"utf16", f),
            StringEncoding::Latin1OrUtf16 => fmt::Display::fmt(&"latin1+utf16", f),
        }
    }
}

impl ToTokens for StringEncoding {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let me = match self {
            StringEncoding::Utf8 => quote!(Utf8),
            StringEncoding::Utf16 => quote!(Utf16),
            StringEncoding::Latin1OrUtf16 => quote!(Latin1OrUtf16),
        };
        tokens.extend(quote!(component_fuzz_util::StringEncoding::#me));
    }
}
