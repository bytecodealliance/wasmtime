//! This module generates test cases for the Wasmtime component model function APIs,
//! e.g. `wasmtime::component::func::Func` and `TypedFunc`.
//!
//! Each case includes a list of arbitrary interface types to use as parameters, plus another one to use as a
//! result, and a component which exports a function and imports a function.  The exported function forwards its
//! parameters to the imported one and forwards the result back to the caller.  This serves to exercise Wasmtime's
//! lifting and lowering code and verify the values remain intact during both processes.

use arbitrary::{Arbitrary, Unstructured};
use indexmap::IndexSet;
use proc_macro2::{Ident, TokenStream};
use quote::{ToTokens, format_ident, quote};
use std::borrow::Cow;
use std::fmt::{self, Debug, Write};
use std::hash::{Hash, Hasher};
use std::iter;
use std::ops::Deref;
use wasmtime_component_util::{DiscriminantSize, FlagsSize, REALLOC_AND_FREE};

const MAX_FLAT_PARAMS: usize = 16;
const MAX_FLAT_ASYNC_PARAMS: usize = 4;
const MAX_FLAT_RESULTS: usize = 1;

/// The name of the imported host function which the generated component will call
pub const IMPORT_FUNCTION: &str = "echo-import";

/// The name of the exported guest function which the host should call
pub const EXPORT_FUNCTION: &str = "echo-export";

/// Wasmtime allows up to 100 type depth so limit this to just under that.
pub const MAX_TYPE_DEPTH: u32 = 99;

macro_rules! uwriteln {
    ($($arg:tt)*) => {
        writeln!($($arg)*).unwrap()
    };
}

macro_rules! uwrite {
    ($($arg:tt)*) => {
        write!($($arg)*).unwrap()
    };
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
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
        generate: impl Fn(&mut Unstructured<'a>, &mut u32) -> arbitrary::Result<T>,
    ) -> arbitrary::Result<Self> {
        let mut ret = Vec::new();
        input.arbitrary_loop(Some(L), Some(H), |input| {
            if *fuel > 0 {
                *fuel = *fuel - 1;
                ret.push(generate(input, fuel)?);
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
#[expect(missing_docs, reason = "self-describing")]
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
    Map(Box<Type>, Box<Type>),

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
        let max = if depth == 0 || *fuel == 0 { 12 } else { 21 };
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
            21 => Type::Map(
                Box::new(Type::generate_hashable_key(u, fuel)?),
                Box::new(Type::generate(u, depth - 1, fuel)?),
            ),
            // ^-- if you add something here update the `depth != 0` case above
            _ => unreachable!(),
        })
    }

    /// Generate a type that can be used as a HashMap key (implements Hash + Eq).
    /// This excludes floats and complex types that might contain floats.
    fn generate_hashable_key(u: &mut Unstructured<'_>, fuel: &mut u32) -> arbitrary::Result<Type> {
        *fuel = fuel.saturating_sub(1);
        // Only generate types that implement Hash and Eq:
        // - No Float32/Float64 (NaN comparison issues)
        // - No complex types (Record, Tuple, Variant, etc.) as they might contain floats
        // - String is allowed as it implements Hash + Eq
        Ok(match u.int_in_range(0..=11)? {
            0 => Type::Bool,
            1 => Type::S8,
            2 => Type::U8,
            3 => Type::S16,
            4 => Type::U16,
            5 => Type::S32,
            6 => Type::U32,
            7 => Type::S64,
            8 => Type::U64,
            9 => Type::Char,
            10 => Type::String,
            11 => {
                let amt = u.int_in_range(1..=(*fuel).max(1).min(257))?;
                *fuel -= amt;
                Type::Enum(amt)
            }
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

    /// Generates text format wasm into `s` to store a value of this type, in
    /// its flat representation stored in the `locals` provided, to the local
    /// named `ptr` at the `offset` provided.
    ///
    /// This will register helper functions necessary in `helpers`. The
    /// `locals` iterator will be advanced for all locals consumed by this
    /// store operation.
    fn store_flat<'a>(
        &'a self,
        s: &mut String,
        ptr: &str,
        offset: u32,
        locals: &mut dyn Iterator<Item = FlatSource>,
        helpers: &mut IndexSet<Helper<'a>>,
    ) {
        enum Kind {
            Primitive(&'static str),
            PointerPair,
            Helper,
        }
        let kind = match self {
            Type::Bool | Type::S8 | Type::U8 => Kind::Primitive("i32.store8"),
            Type::S16 | Type::U16 => Kind::Primitive("i32.store16"),
            Type::S32 | Type::U32 | Type::Char => Kind::Primitive("i32.store"),
            Type::S64 | Type::U64 => Kind::Primitive("i64.store"),
            Type::Float32 => Kind::Primitive("f32.store"),
            Type::Float64 => Kind::Primitive("f64.store"),
            Type::String | Type::List(_) | Type::Map(_, _) => Kind::PointerPair,
            Type::Enum(n) if *n <= (1 << 8) => Kind::Primitive("i32.store8"),
            Type::Enum(n) if *n <= (1 << 16) => Kind::Primitive("i32.store16"),
            Type::Enum(_) => Kind::Primitive("i32.store"),
            Type::Flags(n) if *n <= 8 => Kind::Primitive("i32.store8"),
            Type::Flags(n) if *n <= 16 => Kind::Primitive("i32.store16"),
            Type::Flags(n) if *n <= 32 => Kind::Primitive("i32.store"),
            Type::Flags(_) => unreachable!(),
            Type::Record(_)
            | Type::Tuple(_)
            | Type::Variant(_)
            | Type::Option(_)
            | Type::Result { .. } => Kind::Helper,
        };

        match kind {
            Kind::Primitive(op) => uwriteln!(
                s,
                "({op} offset={offset} (local.get {ptr}) {})",
                locals.next().unwrap()
            ),
            Kind::PointerPair => {
                let abi_ptr = locals.next().unwrap();
                let abi_len = locals.next().unwrap();
                uwriteln!(s, "(i32.store offset={offset} (local.get {ptr}) {abi_ptr})",);
                let offset = offset + 4;
                uwriteln!(s, "(i32.store offset={offset} (local.get {ptr}) {abi_len})",);
            }
            Kind::Helper => {
                let (index, _) = helpers.insert_full(Helper(self));
                uwriteln!(s, "(i32.add (local.get {ptr}) (i32.const {offset}))");
                for _ in 0..self.lowered().len() {
                    let i = locals.next().unwrap();
                    uwriteln!(s, "{i}");
                }
                uwriteln!(s, "call $store_helper_{index}");
            }
        }
    }

    /// Generates a text-format wasm function which takes a pointer and this
    /// type's flat representation as arguments and then stores this value in
    /// the first argument.
    ///
    /// This is used to store records/variants to cut down on the size of final
    /// functions and make codegen here a bit easier.
    fn store_flat_helper<'a>(
        &'a self,
        s: &mut String,
        i: usize,
        helpers: &mut IndexSet<Helper<'a>>,
    ) {
        uwrite!(s, "(func $store_helper_{i} (param i32)");
        let lowered = self.lowered();
        for ty in &lowered {
            uwrite!(s, " (param {ty})");
        }
        s.push_str("\n");
        let locals = (0..lowered.len() as u32).map(|i| i + 1).collect::<Vec<_>>();
        let record = |s: &mut String, helpers: &mut IndexSet<Helper<'a>>, types: &'a [Type]| {
            let mut locals = locals.iter().cloned().map(FlatSource::Local);
            for (offset, ty) in record_field_offsets(types) {
                ty.store_flat(s, "0", offset, &mut locals, helpers);
            }
            assert!(locals.next().is_none());
        };
        let variant = |s: &mut String,
                       helpers: &mut IndexSet<Helper<'a>>,
                       types: &[Option<&'a Type>]| {
            let (size, offset) = variant_memory_info(types.iter().cloned());
            // One extra block for out-of-bounds discriminants.
            for _ in 0..types.len() + 1 {
                s.push_str("block\n");
            }

            // Store the discriminant in memory, then branch on it to figure
            // out which case we're in.
            let store = match size {
                DiscriminantSize::Size1 => "i32.store8",
                DiscriminantSize::Size2 => "i32.store16",
                DiscriminantSize::Size4 => "i32.store",
            };
            uwriteln!(s, "({store} (local.get 0) (local.get 1))");
            s.push_str("local.get 1\n");
            s.push_str("br_table");
            for i in 0..types.len() + 1 {
                uwrite!(s, " {i}");
            }
            s.push_str("\nend\n");

            // Store each payload individually while converting locals from
            // their source types to the precise type necessary for this
            // variant.
            for ty in types {
                if let Some(ty) = ty {
                    let ty_lowered = ty.lowered();
                    let mut locals = locals[1..].iter().zip(&lowered[1..]).zip(&ty_lowered).map(
                        |((i, from), to)| FlatSource::LocalConvert {
                            local: *i,
                            from: *from,
                            to: *to,
                        },
                    );
                    ty.store_flat(s, "0", offset, &mut locals, helpers);
                }
                s.push_str("return\n");
                s.push_str("end\n");
            }

            // Catch-all result which is for out-of-bounds discriminants.
            s.push_str("unreachable\n");
        };
        match self {
            Type::Bool
            | Type::S8
            | Type::U8
            | Type::S16
            | Type::U16
            | Type::S32
            | Type::U32
            | Type::Char
            | Type::S64
            | Type::U64
            | Type::Float32
            | Type::Float64
            | Type::String
            | Type::List(_)
            | Type::Map(_, _)
            | Type::Flags(_)
            | Type::Enum(_) => unreachable!(),

            Type::Record(r) => record(s, helpers, r),
            Type::Tuple(t) => record(s, helpers, t),
            Type::Variant(v) => variant(
                s,
                helpers,
                &v.iter().map(|t| t.as_ref()).collect::<Vec<_>>(),
            ),
            Type::Option(o) => variant(s, helpers, &[None, Some(&**o)]),
            Type::Result { ok, err } => variant(s, helpers, &[ok.as_deref(), err.as_deref()]),
        };
        s.push_str(")\n");
    }

    /// Same as `store_flat`, except loads the flat values from `ptr+offset`.
    ///
    /// Results are placed directly on the wasm stack.
    fn load_flat<'a>(
        &'a self,
        s: &mut String,
        ptr: &str,
        offset: u32,
        helpers: &mut IndexSet<Helper<'a>>,
    ) {
        enum Kind {
            Primitive(&'static str),
            PointerPair,
            Helper,
        }
        let kind = match self {
            Type::Bool | Type::U8 => Kind::Primitive("i32.load8_u"),
            Type::S8 => Kind::Primitive("i32.load8_s"),
            Type::U16 => Kind::Primitive("i32.load16_u"),
            Type::S16 => Kind::Primitive("i32.load16_s"),
            Type::U32 | Type::S32 | Type::Char => Kind::Primitive("i32.load"),
            Type::U64 | Type::S64 => Kind::Primitive("i64.load"),
            Type::Float32 => Kind::Primitive("f32.load"),
            Type::Float64 => Kind::Primitive("f64.load"),
            Type::String | Type::List(_) | Type::Map(_, _) => Kind::PointerPair,
            Type::Enum(n) if *n <= (1 << 8) => Kind::Primitive("i32.load8_u"),
            Type::Enum(n) if *n <= (1 << 16) => Kind::Primitive("i32.load16_u"),
            Type::Enum(_) => Kind::Primitive("i32.load"),
            Type::Flags(n) if *n <= 8 => Kind::Primitive("i32.load8_u"),
            Type::Flags(n) if *n <= 16 => Kind::Primitive("i32.load16_u"),
            Type::Flags(n) if *n <= 32 => Kind::Primitive("i32.load"),
            Type::Flags(_) => unreachable!(),

            Type::Record(_)
            | Type::Tuple(_)
            | Type::Variant(_)
            | Type::Option(_)
            | Type::Result { .. } => Kind::Helper,
        };
        match kind {
            Kind::Primitive(op) => uwriteln!(s, "({op} offset={offset} (local.get {ptr}))"),
            Kind::PointerPair => {
                uwriteln!(s, "(i32.load offset={offset} (local.get {ptr}))",);
                let offset = offset + 4;
                uwriteln!(s, "(i32.load offset={offset} (local.get {ptr}))",);
            }
            Kind::Helper => {
                let (index, _) = helpers.insert_full(Helper(self));
                uwriteln!(s, "(i32.add (local.get {ptr}) (i32.const {offset}))");
                uwriteln!(s, "call $load_helper_{index}");
            }
        }
    }

    /// Same as `store_flat_helper` but for loading the flat representation.
    fn load_flat_helper<'a>(
        &'a self,
        s: &mut String,
        i: usize,
        helpers: &mut IndexSet<Helper<'a>>,
    ) {
        uwrite!(s, "(func $load_helper_{i} (param i32)");
        let lowered = self.lowered();
        for ty in &lowered {
            uwrite!(s, " (result {ty})");
        }
        s.push_str("\n");
        let record = |s: &mut String, helpers: &mut IndexSet<Helper<'a>>, types: &'a [Type]| {
            for (offset, ty) in record_field_offsets(types) {
                ty.load_flat(s, "0", offset, helpers);
            }
        };
        let variant = |s: &mut String,
                       helpers: &mut IndexSet<Helper<'a>>,
                       types: &[Option<&'a Type>]| {
            let (size, offset) = variant_memory_info(types.iter().cloned());

            // Destination locals where the flat representation will be stored.
            // These are automatically zero which handles unused fields too.
            for (i, ty) in lowered.iter().enumerate() {
                uwriteln!(s, " (local $r{i} {ty})");
            }

            // Return block each case jumps to after setting all locals.
            s.push_str("block $r\n");

            // One extra block for "out of bounds discriminant".
            for _ in 0..types.len() + 1 {
                s.push_str("block\n");
            }

            // Load the discriminant and branch on it, storing it in
            // `$r0` as well which is the first flat local representation.
            let load = match size {
                DiscriminantSize::Size1 => "i32.load8_u",
                DiscriminantSize::Size2 => "i32.load16",
                DiscriminantSize::Size4 => "i32.load",
            };
            uwriteln!(s, "({load} (local.get 0))");
            s.push_str("local.tee $r0\n");
            s.push_str("br_table");
            for i in 0..types.len() + 1 {
                uwrite!(s, " {i}");
            }
            s.push_str("\nend\n");

            // For each payload, which is in its own block, load payloads from
            // memory as necessary and convert them into the final locals.
            for ty in types {
                if let Some(ty) = ty {
                    let ty_lowered = ty.lowered();
                    ty.load_flat(s, "0", offset, helpers);
                    for (i, (from, to)) in ty_lowered.iter().zip(&lowered[1..]).enumerate().rev() {
                        let i = i + 1;
                        match (from, to) {
                            (CoreType::F32, CoreType::I32) => {
                                s.push_str("i32.reinterpret_f32\n");
                            }
                            (CoreType::I32, CoreType::I64) => {
                                s.push_str("i64.extend_i32_u\n");
                            }
                            (CoreType::F32, CoreType::I64) => {
                                s.push_str("i32.reinterpret_f32\n");
                                s.push_str("i64.extend_i32_u\n");
                            }
                            (CoreType::F64, CoreType::I64) => {
                                s.push_str("i64.reinterpret_f64\n");
                            }
                            (a, b) if a == b => {}
                            _ => unimplemented!("convert {from:?} to {to:?}"),
                        }
                        uwriteln!(s, "local.set $r{i}");
                    }
                }
                s.push_str("br $r\n");
                s.push_str("end\n");
            }

            // The catch-all block for out-of-bounds discriminants.
            s.push_str("unreachable\n");
            s.push_str("end\n");
            for i in 0..lowered.len() {
                uwriteln!(s, " local.get $r{i}");
            }
        };

        match self {
            Type::Bool
            | Type::S8
            | Type::U8
            | Type::S16
            | Type::U16
            | Type::S32
            | Type::U32
            | Type::Char
            | Type::S64
            | Type::U64
            | Type::Float32
            | Type::Float64
            | Type::String
            | Type::List(_)
            | Type::Map(_, _)
            | Type::Flags(_)
            | Type::Enum(_) => unreachable!(),

            Type::Record(r) => record(s, helpers, r),
            Type::Tuple(t) => record(s, helpers, t),
            Type::Variant(v) => variant(
                s,
                helpers,
                &v.iter().map(|t| t.as_ref()).collect::<Vec<_>>(),
            ),
            Type::Option(o) => variant(s, helpers, &[None, Some(&**o)]),
            Type::Result { ok, err } => variant(s, helpers, &[ok.as_deref(), err.as_deref()]),
        };
        s.push_str(")\n");
    }
}

#[derive(Clone)]
enum FlatSource {
    Local(u32),
    LocalConvert {
        local: u32,
        from: CoreType,
        to: CoreType,
    },
}

impl fmt::Display for FlatSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FlatSource::Local(i) => write!(f, "(local.get {i})"),
            FlatSource::LocalConvert { local, from, to } => {
                match (from, to) {
                    (a, b) if a == b => write!(f, "(local.get {local})"),
                    (CoreType::I32, CoreType::F32) => {
                        write!(f, "(f32.reinterpret_i32 (local.get {local}))")
                    }
                    (CoreType::I64, CoreType::I32) => {
                        write!(f, "(i32.wrap_i64 (local.get {local}))")
                    }
                    (CoreType::I64, CoreType::F64) => {
                        write!(f, "(f64.reinterpret_i64 (local.get {local}))")
                    }
                    (CoreType::I64, CoreType::F32) => {
                        write!(
                            f,
                            "(f32.reinterpret_i32 (i32.wrap_i64 (local.get {local})))"
                        )
                    }
                    _ => unimplemented!("convert {from:?} to {to:?}"),
                }
                // ..
            }
        }
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
            Type::String | Type::List(_) | Type::Map(_, _) => {
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

            Type::String | Type::List(_) | Type::Map(_, _) => SizeAndAlignment {
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

fn record_field_offsets<'a>(
    types: impl IntoIterator<Item = &'a Type>,
) -> impl Iterator<Item = (u32, &'a Type)> {
    let mut offset = 0;
    types.into_iter().map(move |ty| {
        let SizeAndAlignment { size, alignment } = ty.size_and_alignment();
        let ret = align_to(offset, alignment);
        offset = ret + size;
        (ret as u32, ty)
    })
}

fn record_size_and_alignment<'a>(types: impl IntoIterator<Item = &'a Type>) -> SizeAndAlignment {
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

fn variant_memory_info<'a>(
    types: impl ExactSizeIterator<Item = Option<&'a Type>>,
) -> (DiscriminantSize, u32) {
    let discriminant_size = DiscriminantSize::from_count(types.len()).unwrap();
    let mut alignment = u32::from(discriminant_size);
    for ty in types {
        if let Some(ty) = ty {
            let size_and_alignment = ty.size_and_alignment();
            alignment = alignment.max(size_and_alignment.alignment);
        }
    }

    (
        discriminant_size,
        align_to(usize::from(discriminant_size), alignment) as u32,
    )
}

/// Generates the internals of a core wasm module which imports a single
/// component function `IMPORT_FUNCTION` and exports a single component
/// function `EXPORT_FUNCTION`.
///
/// The component function takes `params` as arguments and optionally returns
/// `result`. The `lift_abi` and `lower_abi` fields indicate the ABI in-use for
/// this operation.
fn make_import_and_export(
    params: &[&Type],
    result: Option<&Type>,
    lift_abi: LiftAbi,
    lower_abi: LowerAbi,
) -> String {
    let params_lowered = params
        .iter()
        .flat_map(|ty| ty.lowered())
        .collect::<Box<[_]>>();
    let result_lowered = result.map(|t| t.lowered()).unwrap_or(Vec::new());

    let mut wat = String::new();

    enum Location {
        Flat,
        Indirect(u32),
    }

    // Generate the core wasm type corresponding to the imported function being
    // lowered with `lower_abi`.
    wat.push_str(&format!("(type $import (func"));
    let max_import_params = match lower_abi {
        LowerAbi::Sync => MAX_FLAT_PARAMS,
        LowerAbi::Async => MAX_FLAT_ASYNC_PARAMS,
    };
    let (import_params_loc, nparams) = push_params(&mut wat, &params_lowered, max_import_params);
    let import_results_loc = match lower_abi {
        LowerAbi::Sync => {
            push_result_or_retptr(&mut wat, &result_lowered, nparams, MAX_FLAT_RESULTS)
        }
        LowerAbi::Async => {
            let loc = if result.is_none() {
                Location::Flat
            } else {
                wat.push_str(" (param i32)"); // result pointer
                Location::Indirect(nparams)
            };
            wat.push_str(" (result i32)"); // status code
            loc
        }
    };
    wat.push_str("))\n");

    // Generate the import function.
    wat.push_str(&format!(
        r#"(import "host" "{IMPORT_FUNCTION}" (func $host (type $import)))"#
    ));

    // Do the same as above for the exported function's type which is lifted
    // with `lift_abi`.
    //
    // Note that `export_results_loc` being `None` means that `task.return` is
    // used to communicate results.
    wat.push_str(&format!("(type $export (func"));
    let (export_params_loc, _nparams) = push_params(&mut wat, &params_lowered, MAX_FLAT_PARAMS);
    let export_results_loc = match lift_abi {
        LiftAbi::Sync => Some(push_group(&mut wat, "result", &result_lowered, MAX_FLAT_RESULTS).0),
        LiftAbi::AsyncCallback => {
            wat.push_str(" (result i32)"); // status code
            None
        }
        LiftAbi::AsyncStackful => None,
    };
    wat.push_str("))\n");

    // If the export is async, generate `task.return` as an import as well
    // which is necesary to communicate the results.
    if export_results_loc.is_none() {
        wat.push_str(&format!("(type $task.return (func"));
        push_params(&mut wat, &result_lowered, MAX_FLAT_PARAMS);
        wat.push_str("))\n");
        wat.push_str(&format!(
            r#"(import "" "task.return" (func $task.return (type $task.return)))"#
        ));
    }

    wat.push_str(&format!(
        r#"
(func (export "{EXPORT_FUNCTION}") (type $export)
    (local $retptr i32)
    (local $argptr i32)
        "#
    ));
    let mut store_helpers = IndexSet::new();
    let mut load_helpers = IndexSet::new();

    match (export_params_loc, import_params_loc) {
        // flat => flat is just moving locals around
        (Location::Flat, Location::Flat) => {
            for (index, _) in params_lowered.iter().enumerate() {
                uwrite!(wat, "local.get {index}\n");
            }
        }

        // indirect => indirect is just moving locals around
        (Location::Indirect(i), Location::Indirect(j)) => {
            assert_eq!(j, 0);
            uwrite!(wat, "local.get {i}\n");
        }

        // flat => indirect means that all parameters are stored in memory as
        // if it was a record of all the parameters.
        (Location::Flat, Location::Indirect(_)) => {
            let SizeAndAlignment { size, alignment } =
                record_size_and_alignment(params.iter().cloned());
            wat.push_str(&format!(
                r#"
                    (local.set $argptr
                        (call $realloc
                            (i32.const 0)
                            (i32.const 0)
                            (i32.const {alignment})
                            (i32.const {size})))
                    local.get $argptr
                "#
            ));
            let mut locals = (0..params_lowered.len() as u32).map(FlatSource::Local);
            for (offset, ty) in record_field_offsets(params.iter().cloned()) {
                ty.store_flat(&mut wat, "$argptr", offset, &mut locals, &mut store_helpers);
            }
            assert!(locals.next().is_none());
        }

        (Location::Indirect(_), Location::Flat) => unreachable!(),
    }

    // Pass a return-pointer if necessary.
    match import_results_loc {
        Location::Flat => {}
        Location::Indirect(_) => {
            let SizeAndAlignment { size, alignment } = result.unwrap().size_and_alignment();

            wat.push_str(&format!(
                r#"
                    (local.set $retptr
                        (call $realloc
                            (i32.const 0)
                            (i32.const 0)
                            (i32.const {alignment})
                            (i32.const {size})))
                    local.get $retptr
                "#
            ));
        }
    }

    wat.push_str("call $host\n");

    // Assert the lowered call is ready if an async code was returned.
    //
    // TODO: handle when the import isn't ready yet
    if let LowerAbi::Async = lower_abi {
        wat.push_str("i32.const 2\n");
        wat.push_str("i32.ne\n");
        wat.push_str("if unreachable end\n");
    }

    // TODO: conditionally inject a yield here

    match (import_results_loc, export_results_loc) {
        // flat => flat results involves nothing, the results are already on
        // the stack.
        (Location::Flat, Some(Location::Flat)) => {}

        // indirect => indirect results requires returning the `$retptr` the
        // host call filled in.
        (Location::Indirect(_), Some(Location::Indirect(_))) => {
            wat.push_str("local.get $retptr\n");
        }

        // indirect => flat requires loading the result from the return pointer
        (Location::Indirect(_), Some(Location::Flat)) => {
            result
                .unwrap()
                .load_flat(&mut wat, "$retptr", 0, &mut load_helpers);
        }

        // flat => task.return is easy, the results are already there so just
        // call the function.
        (Location::Flat, None) => {
            wat.push_str("call $task.return\n");
        }

        // indirect => task.return needs to forward `$retptr` if the results
        // are indirect, or otherwise it must be loaded from memory to a flat
        // representation.
        (Location::Indirect(_), None) => {
            if result_lowered.len() <= MAX_FLAT_PARAMS {
                result
                    .unwrap()
                    .load_flat(&mut wat, "$retptr", 0, &mut load_helpers);
            } else {
                wat.push_str("local.get $retptr\n");
            }
            wat.push_str("call $task.return\n");
        }

        (Location::Flat, Some(Location::Indirect(_))) => unreachable!(),
    }

    if let LiftAbi::AsyncCallback = lift_abi {
        wat.push_str("i32.const 0\n"); // completed status code
    }

    wat.push_str(")\n");

    // Generate a `callback` function for the callback ABI.
    //
    // TODO: fill this in
    if let LiftAbi::AsyncCallback = lift_abi {
        wat.push_str(
            r#"
(func (export "callback") (param i32 i32 i32) (result i32) unreachable)
            "#,
        );
    }

    // Fill out all store/load helpers that were needed during generation
    // above. This is a fix-point-loop since each helper may end up requiring
    // more helpers.
    let mut i = 0;
    while i < store_helpers.len() {
        let ty = store_helpers[i].0;
        ty.store_flat_helper(&mut wat, i, &mut store_helpers);
        i += 1;
    }
    i = 0;
    while i < load_helpers.len() {
        let ty = load_helpers[i].0;
        ty.load_flat_helper(&mut wat, i, &mut load_helpers);
        i += 1;
    }

    return wat;

    fn push_params(wat: &mut String, params: &[CoreType], max_flat: usize) -> (Location, u32) {
        push_group(wat, "param", params, max_flat)
    }

    fn push_group(
        wat: &mut String,
        name: &str,
        params: &[CoreType],
        max_flat: usize,
    ) -> (Location, u32) {
        let mut nparams = 0;
        let loc = if params.is_empty() {
            // nothing to emit...
            Location::Flat
        } else if params.len() <= max_flat {
            wat.push_str(&format!(" ({name}"));
            for ty in params {
                wat.push_str(&format!(" {ty}"));
                nparams += 1;
            }
            wat.push_str(")");
            Location::Flat
        } else {
            wat.push_str(&format!(" ({name} i32)"));
            nparams += 1;
            Location::Indirect(0)
        };
        (loc, nparams)
    }

    fn push_result_or_retptr(
        wat: &mut String,
        results: &[CoreType],
        nparams: u32,
        max_flat: usize,
    ) -> Location {
        if results.is_empty() {
            // nothing to emit...
            Location::Flat
        } else if results.len() <= max_flat {
            wat.push_str(" (result");
            for ty in results {
                wat.push_str(&format!(" {ty}"));
            }
            wat.push_str(")");
            Location::Flat
        } else {
            wat.push_str(" (param i32)");
            Location::Indirect(nparams)
        }
    }
}

struct Helper<'a>(&'a Type);

impl Hash for Helper<'_> {
    fn hash<H: Hasher>(&self, h: &mut H) {
        std::ptr::hash(self.0, h);
    }
}

impl PartialEq for Helper<'_> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.0, other.0)
    }
}

impl Eq for Helper<'_> {}

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
        Type::Map(key_ty, value_ty) => {
            let key_ty = rust_type(key_ty, name_counter, declarations);
            let value_ty = rust_type(value_ty, name_counter, declarations);
            quote!(std::collections::HashMap<#key_ty, #value_ty>)
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
            let repr = match DiscriminantSize::from_count(*count as usize).unwrap() {
                DiscriminantSize::Size1 => quote!(u8),
                DiscriminantSize::Size2 => quote!(u16),
                DiscriminantSize::Size4 => quote!(u32),
            };

            declarations.extend(quote! {
                #[derive(ComponentType, Lift, Lower, PartialEq, Eq, Hash, Debug, Copy, Clone, Arbitrary)]
                #[component(enum)]
                #[repr(#repr)]
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
            | Type::Map(_, _)
            | Type::Record(_)
            | Type::Tuple(_)
            | Type::Variant(_)
            | Type::Enum(_)
            | Type::Option(_)
            | Type::Result { .. }
            | Type::Flags(_) => {
                let idx = self.next;
                self.next += 1;
                uwrite!(dst, "$t{idx}");
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
            Type::Map(key_ty, value_ty) => {
                decl.push_str("(map ");
                self.write_ref(key_ty, &mut decl);
                decl.push_str(" ");
                self.write_ref(value_ty, &mut decl);
                decl.push_str(")");
            }
            Type::Record(types) => {
                decl.push_str("(record");
                for (index, ty) in types.iter().enumerate() {
                    uwrite!(decl, r#" (field "f{index}" "#);
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
                    uwrite!(decl, r#" (case "C{index}""#);
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
                    uwrite!(decl, r#" "E{index}""#);
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
                    uwrite!(decl, r#" "F{index}""#);
                }
                decl.push_str(")");
            }
        }
        decl.push_str(")\n");
        uwriteln!(decl, "(import \"t{idx}\" (type $t{idx} (eq $t{idx}')))");
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
    /// Implementation of the "caller" component, which invokes the `callee`
    /// composed component.
    pub caller_module: Cow<'static, str>,
    /// Implementation of the "callee" component, which invokes the host.
    pub callee_module: Cow<'static, str>,
    /// Options used for caller/calle ABI/etc.
    pub options: TestCaseOptions,
}

impl Declarations {
    /// Generate a complete WAT file based on the specified fragments.
    pub fn make_component(&self) -> Box<str> {
        let Self {
            types,
            type_instantiation_args,
            params,
            results,
            caller_module,
            callee_module,
            options,
        } = self;
        let mk_component = |name: &str,
                            module: &str,
                            import_async: bool,
                            export_async: bool,
                            encoding: StringEncoding,
                            lift_abi: LiftAbi,
                            lower_abi: LowerAbi| {
            let import_async = if import_async { "async" } else { "" };
            let export_async = if export_async { "async" } else { "" };
            let lower_async_option = match lower_abi {
                LowerAbi::Sync => "",
                LowerAbi::Async => "async",
            };
            let lift_async_option = match lift_abi {
                LiftAbi::Sync => "",
                LiftAbi::AsyncStackful => "async",
                LiftAbi::AsyncCallback => "async (callback (func $i \"callback\"))",
            };

            let mut intrinsic_defs = String::new();
            let mut intrinsic_imports = String::new();

            match lift_abi {
                LiftAbi::Sync => {}
                LiftAbi::AsyncCallback | LiftAbi::AsyncStackful => {
                    intrinsic_defs.push_str(&format!(
                        r#"
(core func $task.return (canon task.return {results}
    (memory $libc "memory") string-encoding={encoding}))
                        "#,
                    ));
                    intrinsic_imports.push_str(
                        r#"
(with "" (instance (export "task.return" (func $task.return))))
                        "#,
                    );
                }
            }

            format!(
                r#"
(component ${name}
    {types}
    (type $import_sig (func {import_async} {params} {results}))
    (type $export_sig (func {export_async} {params} {results}))
    (import "{IMPORT_FUNCTION}" (func $f (type $import_sig)))

    (core instance $libc (instantiate $libc))

    (core func $f_lower (canon lower
        (func $f)
        (memory $libc "memory")
        (realloc (func $libc "realloc"))
        string-encoding={encoding}
        {lower_async_option}
    ))

    {intrinsic_defs}

    (core module $m
        (memory (import "libc" "memory") 1)
        (func $realloc (import "libc" "realloc") (param i32 i32 i32 i32) (result i32))

        {module}
    )

    (core instance $i (instantiate $m
        (with "libc" (instance $libc))
        (with "host" (instance (export "{IMPORT_FUNCTION}" (func $f_lower))))
        {intrinsic_imports}
    ))

    (func (export "{EXPORT_FUNCTION}") (type $export_sig)
        (canon lift
            (core func $i "{EXPORT_FUNCTION}")
            (memory $libc "memory")
            (realloc (func $libc "realloc"))
            string-encoding={encoding}
            {lift_async_option}
        )
    )
)
            "#
            )
        };

        let c1 = mk_component(
            "callee",
            &callee_module,
            options.host_async,
            options.guest_callee_async,
            options.callee_encoding,
            options.callee_lift_abi,
            options.callee_lower_abi,
        );
        let c2 = mk_component(
            "caller",
            &caller_module,
            options.guest_callee_async,
            options.guest_caller_async,
            options.caller_encoding,
            options.caller_lift_abi,
            options.caller_lower_abi,
        );
        let host_async = if options.host_async { "async" } else { "" };

        format!(
            r#"
            (component
                (core module $libc
                    (memory (export "memory") 1)
                    {REALLOC_AND_FREE}
                )


                {types}

                (type $host_sig (func {host_async} {params} {results}))
                (import "{IMPORT_FUNCTION}" (func $f (type $host_sig)))

                {c1}
                {c2}
                (instance $c1 (instantiate $callee
                    {type_instantiation_args}
                    (with "{IMPORT_FUNCTION}" (func $f))
                ))
                (instance $c2 (instantiate $caller
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
    pub result: Option<&'a Type>,
    /// ABI options to use for this test case.
    pub options: TestCaseOptions,
}

/// Collection of options which configure how the caller/callee/etc ABIs are
/// all configured.
#[derive(Debug, Arbitrary, Copy, Clone)]
pub struct TestCaseOptions {
    /// Whether or not the guest caller component (the entrypoint) is using an
    /// `async` function type.
    pub guest_caller_async: bool,
    /// Whether or not the guest callee component (what the entrypoint calls)
    /// is using an `async` function type.
    pub guest_callee_async: bool,
    /// Whether or not the host is using an async function type (what the
    /// guest callee calls).
    pub host_async: bool,
    /// The string encoding of the caller component.
    pub caller_encoding: StringEncoding,
    /// The string encoding of the callee component.
    pub callee_encoding: StringEncoding,
    /// The ABI that the caller component is using to lift its export (the main
    /// entrypoint).
    pub caller_lift_abi: LiftAbi,
    /// The ABI that the callee component is using to lift its export (called
    /// by the caller).
    pub callee_lift_abi: LiftAbi,
    /// The ABI that the caller component is using to lower its import (the
    /// callee's export).
    pub caller_lower_abi: LowerAbi,
    /// The ABI that the callee component is using to lower its import (the
    /// host function).
    pub callee_lower_abi: LowerAbi,
}

#[derive(Debug, Arbitrary, Copy, Clone)]
pub enum LiftAbi {
    Sync,
    AsyncStackful,
    AsyncCallback,
}

#[derive(Debug, Arbitrary, Copy, Clone)]
pub enum LowerAbi {
    Sync,
    Async,
}

impl<'a> TestCase<'a> {
    pub fn generate(types: &'a [Type], u: &mut Unstructured<'_>) -> arbitrary::Result<Self> {
        let max_params = if types.len() > 0 { 5 } else { 0 };
        let params = (0..u.int_in_range(0..=max_params)?)
            .map(|_| u.choose(&types))
            .collect::<arbitrary::Result<Vec<_>>>()?;
        let result = if types.len() > 0 && u.arbitrary()? {
            Some(u.choose(&types)?)
        } else {
            None
        };

        let mut options = u.arbitrary::<TestCaseOptions>()?;

        // Sync tasks cannot call async functions via a sync lower, nor can they
        // block in other ways (e.g. by calling `waitable-set.wait`, returning
        // `CALLBACK_CODE_WAIT`, etc.) prior to returning.  Therefore,
        // async-ness cascades to the callers:
        if options.host_async {
            options.guest_callee_async = true;
        }
        if options.guest_callee_async {
            options.guest_caller_async = true;
        }

        Ok(Self {
            params,
            result,
            options,
        })
    }

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
        if let Some(ty) = self.result {
            results.push_str(&format!(" (result "));
            builder.write_ref(ty, &mut results);
            results.push_str(")");
        }

        let caller_module = make_import_and_export(
            &self.params,
            self.result,
            self.options.caller_lift_abi,
            self.options.caller_lower_abi,
        );
        let callee_module = make_import_and_export(
            &self.params,
            self.result,
            self.options.callee_lift_abi,
            self.options.callee_lower_abi,
        );

        let mut type_decls = Vec::new();
        let mut type_instantiation_args = String::new();
        while let Some((idx, ty)) = builder.worklist.pop() {
            type_decls.push(builder.write_decl(idx, ty));
            uwriteln!(type_instantiation_args, "(with \"t{idx}\" (type $t{idx}))");
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
            caller_module: caller_module.into(),
            callee_module: callee_module.into(),
            options: self.options,
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

impl ToTokens for TestCaseOptions {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let TestCaseOptions {
            guest_caller_async,
            guest_callee_async,
            host_async,
            caller_encoding,
            callee_encoding,
            caller_lift_abi,
            callee_lift_abi,
            caller_lower_abi,
            callee_lower_abi,
        } = self;
        tokens.extend(quote!(wasmtime_test_util::component_fuzz::TestCaseOptions {
            guest_caller_async: #guest_caller_async,
            guest_callee_async: #guest_callee_async,
            host_async: #host_async,
            caller_encoding: #caller_encoding,
            callee_encoding: #callee_encoding,
            caller_lift_abi: #caller_lift_abi,
            callee_lift_abi: #callee_lift_abi,
            caller_lower_abi: #caller_lower_abi,
            callee_lower_abi: #callee_lower_abi,
        }));
    }
}

impl ToTokens for LowerAbi {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let me = match self {
            LowerAbi::Sync => quote!(Sync),
            LowerAbi::Async => quote!(Async),
        };
        tokens.extend(quote!(wasmtime_test_util::component_fuzz::LowerAbi::#me));
    }
}

impl ToTokens for LiftAbi {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let me = match self {
            LiftAbi::Sync => quote!(Sync),
            LiftAbi::AsyncCallback => quote!(AsyncCallback),
            LiftAbi::AsyncStackful => quote!(AsyncStackful),
        };
        tokens.extend(quote!(wasmtime_test_util::component_fuzz::LiftAbi::#me));
    }
}

impl ToTokens for StringEncoding {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let me = match self {
            StringEncoding::Utf8 => quote!(Utf8),
            StringEncoding::Utf16 => quote!(Utf16),
            StringEncoding::Latin1OrUtf16 => quote!(Latin1OrUtf16),
        };
        tokens.extend(quote!(wasmtime_test_util::component_fuzz::StringEncoding::#me));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arbtest() {
        arbtest::arbtest(|u| {
            let mut fuel = 100;
            let types = (0..5)
                .map(|_| Type::generate(u, 3, &mut fuel))
                .collect::<arbitrary::Result<Vec<_>>>()?;
            let case = TestCase::generate(&types, u)?;
            let decls = case.declarations();
            let component = decls.make_component();
            let wasm = wat::parse_str(&component).unwrap_or_else(|e| {
                panic!("failed to parse generated component as wat: {e}\n\n{component}");
            });
            wasmparser::Validator::new_with_features(wasmparser::WasmFeatures::all())
                .validate_all(&wasm)
                .unwrap_or_else(|e| {
                    let mut wat = String::new();
                    let mut dst = wasmprinter::PrintFmtWrite(&mut wat);
                    let to_print = if wasmprinter::Config::new()
                        .print_offsets(true)
                        .print_operand_stack(true)
                        .print(&wasm, &mut dst)
                        .is_ok()
                    {
                        &wat[..]
                    } else {
                        &component[..]
                    };
                    panic!("generated component is not valid wasm: {e}\n\n{to_print}");
                });
            Ok(())
        })
        .budget_ms(1_000)
        // .seed(0x3c9050d4000000e9)
        ;
    }
}
