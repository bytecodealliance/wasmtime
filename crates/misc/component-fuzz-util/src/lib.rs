//! This module generates test cases for the Wasmtime component model function APIs,
//! e.g. `wasmtime::component::func::Func` and `TypedFunc`.
//!
//! Each case includes a list of arbitrary interface types to use as parameters, plus another one to use as a
//! result, and a component which exports a function and imports a function.  The exported function forwards its
//! parameters to the imported one and forwards the result back to the caller.  This serves to exercise Wasmtime's
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

/// The name of the imported host function which the generated component will call
pub const IMPORT_FUNCTION: &str = "echo-import";

/// The name of the exported guest function which the host should call
pub const EXPORT_FUNCTION: &str = "echo-export";

/// Wasmtime allows up to 100 type depth so limit this to just under that.
pub const MAX_TYPE_DEPTH: u32 = 99;

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

/// Wraps a `Box<[T]>` and provides an `Arbitrary` implementation that always generates slices of length less than
/// or equal to the longest tuple for which Wasmtime generates a `ComponentType` impl
#[derive(Debug, Clone)]
pub struct VecInRange<T, const L: u32, const H: u32>(Vec<T>);

impl<T, const L: u32, const H: u32> VecInRange<T, L, H> {
    fn new<'a>(
        input: &mut Unstructured<'a>,
        fuel: &mut u32,
        gen: impl Fn(&mut Unstructured<'a>, &mut u32) -> arbitrary::Result<T>,
    ) -> arbitrary::Result<Self> {
        let mut ret = Vec::new();
        input.arbitrary_loop(Some(L), Some(H), |input| {
            if *fuel > 0 {
                *fuel = *fuel - 1;
                ret.push(gen(input, fuel)?);
                Ok(std::ops::ControlFlow::Continue(()))
            } else {
                Ok(std::ops::ControlFlow::Break(()))
            }
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
#[derive(Debug, Clone)]
pub enum Type {
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
    Record(VecInRange<Type, 1, 200>),

    // Tuples can only have up to 16 type parameters in wasmtime right now for
    // the static API, but the standard library only supports `Debug` up to 11
    // elements, so compromise at an even 10.
    Tuple(VecInRange<Type, 1, 10>),

    // Like records, allow a good number of variants, but variants require at
    // least one case.
    Variant(VecInRange<Option<Type>, 1, 200>),
    Enum(u32),

    Option(Box<Type>),
    Result {
        ok: Option<Box<Type>>,
        err: Option<Box<Type>>,
    },

    Flags(u32),
}

impl Type {
    pub fn generate(
        u: &mut Unstructured<'_>,
        depth: u32,
        fuel: &mut u32,
    ) -> arbitrary::Result<Type> {
        *fuel = fuel.saturating_sub(1);
        let max = if depth == 0 || *fuel == 0 { 12 } else { 20 };
        Ok(match u.int_in_range(0..=max)? {
            0 => Type::Bool,
            1 => Type::S8,
            2 => Type::U8,
            3 => Type::S16,
            4 => Type::U16,
            5 => Type::S32,
            6 => Type::U32,
            7 => Type::S64,
            8 => Type::U64,
            9 => Type::Float32,
            10 => Type::Float64,
            11 => Type::Char,
            12 => Type::String,
            // ^-- if you add something here update the `depth == 0` case above
            13 => Type::List(Box::new(Type::generate(u, depth - 1, fuel)?)),
            14 => Type::Record(Type::generate_list(u, depth - 1, fuel)?),
            15 => Type::Tuple(Type::generate_list(u, depth - 1, fuel)?),
            16 => Type::Variant(VecInRange::new(u, fuel, |u, fuel| {
                Type::generate_opt(u, depth - 1, fuel)
            })?),
            17 => {
                let amt = u.int_in_range(1..=(*fuel).max(1).min(257))?;
                *fuel -= amt;
                Type::Enum(amt)
            }
            18 => Type::Option(Box::new(Type::generate(u, depth - 1, fuel)?)),
            19 => Type::Result {
                ok: Type::generate_opt(u, depth - 1, fuel)?.map(Box::new),
                err: Type::generate_opt(u, depth - 1, fuel)?.map(Box::new),
            },
            20 => {
                let amt = u.int_in_range(1..=(*fuel).min(32))?;
                *fuel -= amt;
                Type::Flags(amt)
            }
            // ^-- if you add something here update the `depth != 0` case above
            _ => unreachable!(),
        })
    }

    fn generate_opt(
        u: &mut Unstructured<'_>,
        depth: u32,
        fuel: &mut u32,
    ) -> arbitrary::Result<Option<Type>> {
        Ok(if u.arbitrary()? {
            Some(Type::generate(u, depth, fuel)?)
        } else {
            None
        })
    }

    fn generate_list<const L: u32, const H: u32>(
        u: &mut Unstructured<'_>,
        depth: u32,
        fuel: &mut u32,
    ) -> arbitrary::Result<VecInRange<Type, L, H>> {
        VecInRange::new(u, fuel, |u, fuel| Type::generate(u, depth, fuel))
    }
}

fn lower_record<'a>(types: impl Iterator<Item = &'a Type>, vec: &mut Vec<CoreType>) {
    for ty in types {
        ty.lower(vec);
    }
}

fn lower_variant<'a>(types: impl Iterator<Item = Option<&'a Type>>, vec: &mut Vec<CoreType>) {
    vec.push(CoreType::I32);
    let offset = vec.len();
    for ty in types {
        let ty = match ty {
            Some(ty) => ty,
            None => continue,
        };
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
        FlagsSize::Size4Plus(n) => n.into(),
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
            Type::Variant(types) => lower_variant(types.0.iter().map(|t| t.as_ref()), vec),
            Type::Option(ty) => lower_variant([None, Some(&**ty)].into_iter(), vec),
            Type::Result { ok, err } => {
                lower_variant([ok.as_deref(), err.as_deref()].into_iter(), vec)
            }
            Type::Flags(count) => vec.extend(
                iter::repeat(CoreType::I32).take(u32_count_from_flag_count(*count as usize)),
            ),
        }
    }

    fn size_and_alignment(&self) -> SizeAndAlignment {
        match self {
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

            Type::Variant(types) => variant_size_and_alignment(types.0.iter().map(|t| t.as_ref())),

            Type::Enum(count) => variant_size_and_alignment((0..*count).map(|_| None)),

            Type::Option(ty) => variant_size_and_alignment([None, Some(&**ty)].into_iter()),

            Type::Result { ok, err } => {
                variant_size_and_alignment([ok.as_deref(), err.as_deref()].into_iter())
            }

            Type::Flags(count) => match FlagsSize::from_count(*count as usize) {
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
                    size: usize::from(n) * 4,
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
    types: impl ExactSizeIterator<Item = Option<&'a Type>>,
) -> SizeAndAlignment {
    let discriminant_size = DiscriminantSize::from_count(types.len()).unwrap();
    let mut alignment = u32::from(discriminant_size);
    let mut size = 0;
    for ty in types {
        if let Some(ty) = ty {
            let size_and_alignment = ty.size_and_alignment();
            alignment = alignment.max(size_and_alignment.alignment);
            size = size.max(size_and_alignment.size);
        }
    }

    SizeAndAlignment {
        size: align_to(
            align_to(usize::from(discriminant_size), alignment) + size,
            alignment,
        ),
        alignment,
    }
}

fn make_import_and_export(params: &[&Type], results: &[&Type]) -> String {
    let params_lowered = params
        .iter()
        .flat_map(|ty| ty.lowered())
        .collect::<Box<[_]>>();
    let results_lowered = results
        .iter()
        .flat_map(|ty| ty.lowered())
        .collect::<Box<[_]>>();

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

    if results_lowered.len() <= MAX_FLAT_RESULTS {
        let mut core_results = String::new();
        for result in results_lowered.iter() {
            write!(&mut core_results, " {result}").unwrap();
        }

        let maybe_core_results = if results_lowered.is_empty() {
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
        let SizeAndAlignment { size, alignment } =
            Type::Record(VecInRange(results.iter().map(|t| (*t).clone()).collect()))
                .size_and_alignment();

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
        Type::Variant(types) => {
            let cases = types
                .0
                .iter()
                .enumerate()
                .map(|(index, ty)| {
                    let name = format_ident!("C{index}");
                    let ty = match ty {
                        Some(ty) => {
                            let ty = rust_type(ty, name_counter, declarations);
                            quote!((#ty))
                        }
                        None => quote!(),
                    };
                    quote!(#name #ty,)
                })
                .collect::<TokenStream>();

            let name = make_rust_name(name_counter);
            declarations.extend(quote! {
                #[derive(ComponentType, Lift, Lower, PartialEq, Debug, Clone, Arbitrary)]
                #[component(variant)]
                enum #name {
                    #cases
                }
            });

            quote!(#name)
        }
        Type::Enum(count) => {
            let cases = (0..*count)
                .map(|index| {
                    let name = format_ident!("E{index}");
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
        Type::Result { ok, err } => {
            let ok = match ok {
                Some(ok) => rust_type(ok, name_counter, declarations),
                None => quote!(()),
            };
            let err = match err {
                Some(err) => rust_type(err, name_counter, declarations),
                None => quote!(()),
            };
            quote!(Result<#ok, #err>)
        }
        Type::Flags(count) => {
            let type_name = make_rust_name(name_counter);

            let mut flags = TokenStream::new();
            let mut names = TokenStream::new();

            for index in 0..*count {
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

                impl<'a> arbitrary::Arbitrary<'a> for #type_name {
                    fn arbitrary(input: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
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

#[derive(Default)]
struct TypesBuilder<'a> {
    next: u32,
    worklist: Vec<(u32, &'a Type)>,
}

impl<'a> TypesBuilder<'a> {
    fn write_ref(&mut self, ty: &'a Type, dst: &mut String) {
        match ty {
            // Primitive types can be referenced directly
            Type::Bool => dst.push_str("bool"),
            Type::S8 => dst.push_str("s8"),
            Type::U8 => dst.push_str("u8"),
            Type::S16 => dst.push_str("s16"),
            Type::U16 => dst.push_str("u16"),
            Type::S32 => dst.push_str("s32"),
            Type::U32 => dst.push_str("u32"),
            Type::S64 => dst.push_str("s64"),
            Type::U64 => dst.push_str("u64"),
            Type::Float32 => dst.push_str("float32"),
            Type::Float64 => dst.push_str("float64"),
            Type::Char => dst.push_str("char"),
            Type::String => dst.push_str("string"),

            // Otherwise emit a reference to the type and remember to generate
            // the corresponding type alias later.
            Type::List(_)
            | Type::Record(_)
            | Type::Tuple(_)
            | Type::Variant(_)
            | Type::Enum(_)
            | Type::Option(_)
            | Type::Result { .. }
            | Type::Flags(_) => {
                let idx = self.next;
                self.next += 1;
                write!(dst, "$t{idx}").unwrap();
                self.worklist.push((idx, ty));
            }
        }
    }

    fn write_decl(&mut self, idx: u32, ty: &'a Type) -> String {
        let mut decl = format!("(type $t{idx}' ");
        match ty {
            Type::Bool
            | Type::S8
            | Type::U8
            | Type::S16
            | Type::U16
            | Type::S32
            | Type::U32
            | Type::S64
            | Type::U64
            | Type::Float32
            | Type::Float64
            | Type::Char
            | Type::String => unreachable!(),

            Type::List(ty) => {
                decl.push_str("(list ");
                self.write_ref(ty, &mut decl);
                decl.push_str(")");
            }
            Type::Record(types) => {
                decl.push_str("(record");
                for (index, ty) in types.iter().enumerate() {
                    write!(decl, r#" (field "f{index}" "#).unwrap();
                    self.write_ref(ty, &mut decl);
                    decl.push_str(")");
                }
                decl.push_str(")");
            }
            Type::Tuple(types) => {
                decl.push_str("(tuple");
                for ty in types.iter() {
                    decl.push_str(" ");
                    self.write_ref(ty, &mut decl);
                }
                decl.push_str(")");
            }
            Type::Variant(types) => {
                decl.push_str("(variant");
                for (index, ty) in types.iter().enumerate() {
                    write!(decl, r#" (case "C{index}""#).unwrap();
                    if let Some(ty) = ty {
                        decl.push_str(" ");
                        self.write_ref(ty, &mut decl);
                    }
                    decl.push_str(")");
                }
                decl.push_str(")");
            }
            Type::Enum(count) => {
                decl.push_str("(enum");
                for index in 0..*count {
                    write!(decl, r#" "E{index}""#).unwrap();
                }
                decl.push_str(")");
            }
            Type::Option(ty) => {
                decl.push_str("(option ");
                self.write_ref(ty, &mut decl);
                decl.push_str(")");
            }
            Type::Result { ok, err } => {
                decl.push_str("(result");
                if let Some(ok) = ok {
                    decl.push_str(" ");
                    self.write_ref(ok, &mut decl);
                }
                if let Some(err) = err {
                    decl.push_str(" (error ");
                    self.write_ref(err, &mut decl);
                    decl.push_str(")");
                }
                decl.push_str(")");
            }
            Type::Flags(count) => {
                decl.push_str("(flags");
                for index in 0..*count {
                    write!(decl, r#" "F{index}""#).unwrap();
                }
                decl.push_str(")");
            }
        }
        decl.push_str(")\n");
        writeln!(decl, "(import \"t{idx}\" (type $t{idx} (eq $t{idx}')))").unwrap();
        decl
    }
}

/// Represents custom fragments of a WAT file which may be used to create a component for exercising [`TestCase`]s
#[derive(Debug)]
pub struct Declarations {
    /// Type declarations (if any) referenced by `params` and/or `result`
    pub types: Cow<'static, str>,
    /// Types to thread through when instantiating sub-components.
    pub type_instantiation_args: Cow<'static, str>,
    /// Parameter declarations used for the imported and exported functions
    pub params: Cow<'static, str>,
    /// Result declaration used for the imported and exported functions
    pub results: Cow<'static, str>,
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
            type_instantiation_args,
            params,
            results,
            import_and_export,
            encoding1,
            encoding2,
        } = self;
        let mk_component = |name: &str, encoding: StringEncoding| {
            format!(
                r#"
                (component ${name}
                    {types}
                    (type $sig (func {params} {results}))
                    (import "{IMPORT_FUNCTION}" (func $f (type $sig)))

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

                    (func (export "{EXPORT_FUNCTION}") (type $sig)
                        (canon lift
                            (core func $i "{EXPORT_FUNCTION}")
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

                (type $sig (func {params} {results}))
                (import "{IMPORT_FUNCTION}" (func $f (type $sig)))

                {c1}
                {c2}
                (instance $c1 (instantiate $c1
                    {type_instantiation_args}
                    (with "{IMPORT_FUNCTION}" (func $f))
                ))
                (instance $c2 (instantiate $c2
                    {type_instantiation_args}
                    (with "{IMPORT_FUNCTION}" (func $c1 "{EXPORT_FUNCTION}"))
                ))
                (export "{EXPORT_FUNCTION}" (func $c2 "{EXPORT_FUNCTION}"))
            )"#,
        )
        .into()
    }
}

/// Represents a test case for calling a component function
#[derive(Debug)]
pub struct TestCase<'a> {
    /// The types of parameters to pass to the function
    pub params: Vec<&'a Type>,
    /// The result types of the function
    pub results: Vec<&'a Type>,
    /// String encoding to use from host-to-component.
    pub encoding1: StringEncoding,
    /// String encoding to use from component-to-host.
    pub encoding2: StringEncoding,
}

impl TestCase<'_> {
    /// Generate a `Declarations` for this `TestCase` which may be used to build a component to execute the case.
    pub fn declarations(&self) -> Declarations {
        let mut builder = TypesBuilder::default();

        let mut params = String::new();
        for (i, ty) in self.params.iter().enumerate() {
            params.push_str(&format!(" (param \"p{i}\" "));
            builder.write_ref(ty, &mut params);
            params.push_str(")");
        }

        let mut results = String::new();
        for (i, ty) in self.results.iter().enumerate() {
            results.push_str(&format!(" (result \"r{i}\" "));
            builder.write_ref(ty, &mut results);
            results.push_str(")");
        }

        let import_and_export = make_import_and_export(&self.params, &self.results);

        let mut type_decls = Vec::new();
        let mut type_instantiation_args = String::new();
        while let Some((idx, ty)) = builder.worklist.pop() {
            type_decls.push(builder.write_decl(idx, ty));
            writeln!(type_instantiation_args, "(with \"t{idx}\" (type $t{idx}))").unwrap();
        }

        // Note that types are printed here in reverse order since they were
        // pushed onto `type_decls` as they were referenced meaning the last one
        // is the "base" one.
        let mut types = String::new();
        for decl in type_decls.into_iter().rev() {
            types.push_str(&decl);
            types.push_str("\n");
        }

        Declarations {
            types: types.into(),
            type_instantiation_args: type_instantiation_args.into(),
            params: params.into(),
            results: results.into(),
            import_and_export: import_and_export.into(),
            encoding1: self.encoding1,
            encoding2: self.encoding2,
        }
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
