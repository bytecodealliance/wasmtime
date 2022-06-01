use crate::{
    EntityType, Global, GlobalInit, ModuleTypes, ModuleTypesBuilder, PrimaryMap, SignatureIndex,
};
use anyhow::{bail, Result};
use cranelift_entity::EntityRef;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::Hash;
use std::ops::Index;

macro_rules! indices {
    ($(
        $(#[$a:meta])*
        pub struct $name:ident(u32);
    )*) => ($(
        $(#[$a])*
        #[derive(
            Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug,
            Serialize, Deserialize,
        )]
        pub struct $name(u32);
        cranelift_entity::entity_impl!($name);
    )*);
}

indices! {
    // ========================================================================
    // These indices are used during compile time only when we're translating a
    // component at this time. The actual indices are not persisted beyond the
    // compile phase to when we're actually working with the component at
    // runtime.

    /// Index within a component's module index space.
    pub struct ModuleIndex(u32);

    /// Index within a component's component index space.
    pub struct ComponentIndex(u32);

    /// Index within a component's instance index space.
    pub struct InstanceIndex(u32);

    // ========================================================================
    // These indices are used to lookup type information within a `TypeTables`
    // structure. These represent generally deduplicated type information across
    // an entire component and are a form of RTTI in a sense.

    /// Index pointing to a component's type (exports/imports with
    /// component-model types)
    pub struct ComponentTypeIndex(u32);

    /// Index pointing to a component instance's type (exports with
    /// component-model types, no imports)
    pub struct ComponentInstanceTypeIndex(u32);

    /// Index pointing to a core wasm module's type (exports/imports with
    /// core wasm types)
    pub struct ModuleTypeIndex(u32);

    /// Index pointing to a component model function type with arguments/result
    /// as interface types.
    pub struct FuncTypeIndex(u32);

    /// Index pointing to an interface type, used for recursive types such as
    /// `List<T>`.
    pub struct InterfaceTypeIndex(u32);

    /// Index pointing to a record type in the component model (aka a struct).
    pub struct RecordTypeIndex(u32);
    /// Index pointing to a variant type in the component model (aka an enum).
    pub struct VariantTypeIndex(u32);
    /// Index pointing to a tuple type in the component model.
    pub struct TupleTypeIndex(u32);
    /// Index pointing to a flags type in the component model.
    pub struct FlagsTypeIndex(u32);
    /// Index pointing to an enum type in the component model.
    pub struct EnumTypeIndex(u32);
    /// Index pointing to a union type in the component model.
    pub struct UnionTypeIndex(u32);
    /// Index pointing to an expected type in the component model (aka a
    /// `Result<T, E>`)
    pub struct ExpectedTypeIndex(u32);

    // ========================================================================
    // These indices are actually used at runtime when managing a component at
    // this time.

    /// Index that represents a core wasm instance created at runtime.
    ///
    /// This is used to keep track of when instances are created and is able to
    /// refer back to previously created instances for exports and such.
    pub struct RuntimeInstanceIndex(u32);

    /// Index that represents a closed-over-module for a component.
    ///
    /// Components which embed modules or otherwise refer to module (such as
    /// through `alias` annotations) pull in items in to the list of closed over
    /// modules, and this index indexes, at runtime, which of the upvars is
    /// referenced.
    pub struct ModuleUpvarIndex(u32);

    /// Used to index imports into a `Component`
    ///
    /// This does not correspond to anything in the binary format for the
    /// component model.
    pub struct ImportIndex(u32);

    /// Index that represents a leaf item imported into a component where a
    /// "leaf" means "not an instance".
    ///
    /// This does not correspond to anything in the binary format for the
    /// component model.
    pub struct RuntimeImportIndex(u32);

    /// Index that represents a lowered host function and is used to represent
    /// host function lowerings with options and such.
    ///
    /// This does not correspond to anything in the binary format for the
    /// component model.
    pub struct LoweredIndex(u32);

    /// Index representing a linear memory extracted from a wasm instance
    /// which is stored in a `VMComponentContext`. This is used to deduplicate
    /// references to the same linear memory where it's only stored once in a
    /// `VMComponentContext`.
    ///
    /// This does not correspond to anything in the binary format for the
    /// component model.
    pub struct RuntimeMemoryIndex(u32);

    /// Same as `RuntimeMemoryIndex` except for the `realloc` function.
    pub struct RuntimeReallocIndex(u32);
}

// Reexport for convenience some core-wasm indices which are also used in the
// component model, typically for when aliasing exports of core wasm modules.
pub use crate::{FuncIndex, GlobalIndex, MemoryIndex, TableIndex, TypeIndex};

/// Equivalent of `EntityIndex` but for the component model instead of core
/// wasm.
#[derive(Debug, Clone, Copy)]
#[allow(missing_docs)]
pub enum ComponentItem {
    Func(FuncIndex),
    Module(ModuleIndex),
    Instance(InstanceIndex),
    Component(ComponentIndex),
}

/// Runtime information about the type information contained within a component.
///
/// One of these is created per top-level component which describes all of the
/// types contained within the top-level component itself. Each sub-component
/// will have a pointer to this value as well.
#[derive(Default, Serialize, Deserialize)]
pub struct ComponentTypes {
    modules: PrimaryMap<ModuleTypeIndex, ModuleType>,
    components: PrimaryMap<ComponentTypeIndex, ComponentType>,
    component_instances: PrimaryMap<ComponentInstanceTypeIndex, ComponentInstanceType>,
    functions: PrimaryMap<FuncTypeIndex, FuncType>,
    interface_types: PrimaryMap<InterfaceTypeIndex, InterfaceType>,
    records: PrimaryMap<RecordTypeIndex, RecordType>,
    variants: PrimaryMap<VariantTypeIndex, VariantType>,
    tuples: PrimaryMap<TupleTypeIndex, TupleType>,
    enums: PrimaryMap<EnumTypeIndex, EnumType>,
    flags: PrimaryMap<FlagsTypeIndex, FlagsType>,
    unions: PrimaryMap<UnionTypeIndex, UnionType>,
    expecteds: PrimaryMap<ExpectedTypeIndex, ExpectedType>,

    module_types: ModuleTypes,
}

impl ComponentTypes {
    /// Returns the core wasm module types known within this component.
    pub fn module_types(&self) -> &ModuleTypes {
        &self.module_types
    }
}

macro_rules! impl_index {
    ($(impl Index<$ty:ident> for ComponentTypes { $output:ident => $field:ident })*) => ($(
        impl std::ops::Index<$ty> for ComponentTypes {
            type Output = $output;
            fn index(&self, idx: $ty) -> &$output {
                &self.$field[idx]
            }
        }
    )*)
}

impl_index! {
    impl Index<ModuleTypeIndex> for ComponentTypes { ModuleType => modules }
    impl Index<ComponentTypeIndex> for ComponentTypes { ComponentType => components }
    impl Index<ComponentInstanceTypeIndex> for ComponentTypes { ComponentInstanceType => component_instances }
    impl Index<FuncTypeIndex> for ComponentTypes { FuncType => functions }
    impl Index<InterfaceTypeIndex> for ComponentTypes { InterfaceType => interface_types }
    impl Index<RecordTypeIndex> for ComponentTypes { RecordType => records }
    impl Index<VariantTypeIndex> for ComponentTypes { VariantType => variants }
    impl Index<TupleTypeIndex> for ComponentTypes { TupleType => tuples }
    impl Index<EnumTypeIndex> for ComponentTypes { EnumType => enums }
    impl Index<FlagsTypeIndex> for ComponentTypes { FlagsType => flags }
    impl Index<UnionTypeIndex> for ComponentTypes { UnionType => unions }
    impl Index<ExpectedTypeIndex> for ComponentTypes { ExpectedType => expecteds }
}

// Additionally forward anything that can index `ModuleTypes` to `ModuleTypes`
// (aka `SignatureIndex`)
impl<T> Index<T> for ComponentTypes
where
    ModuleTypes: Index<T>,
{
    type Output = <ModuleTypes as Index<T>>::Output;
    fn index(&self, idx: T) -> &Self::Output {
        self.module_types.index(idx)
    }
}

/// Structured used to build a [`ComponentTypes`] during translation.
///
/// This contains tables to intern any component types found as well as
/// managing building up core wasm [`ModuleTypes`] as well.
#[derive(Default)]
pub struct ComponentTypesBuilder {
    type_scopes: Vec<PrimaryMap<TypeIndex, TypeDef>>,
    functions: HashMap<FuncType, FuncTypeIndex>,
    interface_types: HashMap<InterfaceType, InterfaceTypeIndex>,
    records: HashMap<RecordType, RecordTypeIndex>,
    variants: HashMap<VariantType, VariantTypeIndex>,
    tuples: HashMap<TupleType, TupleTypeIndex>,
    enums: HashMap<EnumType, EnumTypeIndex>,
    flags: HashMap<FlagsType, FlagsTypeIndex>,
    unions: HashMap<UnionType, UnionTypeIndex>,
    expecteds: HashMap<ExpectedType, ExpectedTypeIndex>,

    component_types: ComponentTypes,
    module_types: ModuleTypesBuilder,
}

impl ComponentTypesBuilder {
    /// Finishes this list of component types and returns the finished
    /// structure.
    pub fn finish(mut self) -> ComponentTypes {
        self.component_types.module_types = self.module_types.finish();
        self.component_types
    }

    /// Returns the underlying builder used to build up core wasm module types.
    ///
    /// Note that this is shared across all modules found within a component to
    /// improve the wins from deduplicating function signatures.
    pub fn module_types_builder(&mut self) -> &mut ModuleTypesBuilder {
        &mut self.module_types
    }

    /// Pushes a new scope when entering a new index space for types in the
    /// component model.
    ///
    /// This happens when a component is recursed into or a module/instance
    /// type is recursed into.
    pub fn push_component_types_scope(&mut self) {
        self.type_scopes.push(PrimaryMap::new());
    }

    /// Adds a new `TypeDef` definition within the current component types
    /// scope.
    ///
    /// Returns the `TypeIndex` associated with the type being pushed..
    ///
    /// # Panics
    ///
    /// Requires that `push_component_types_scope` was called previously.
    pub fn push_component_typedef(&mut self, ty: TypeDef) -> TypeIndex {
        self.type_scopes.last_mut().unwrap().push(ty)
    }

    /// Looks up an "outer" type in this builder to handle outer aliases.
    ///
    /// The `count` parameter and `ty` are taken from the binary format itself,
    /// and the `TypeDef` returned is what the outer type refers to.
    ///
    /// # Panics
    ///
    /// Assumes that `count` and `ty` are valid.
    pub fn component_outer_type(&self, count: u32, ty: TypeIndex) -> TypeDef {
        // Reverse the index and 0 means the "current scope"
        let idx = self.type_scopes.len() - (count as usize) - 1;
        self.type_scopes[idx][ty]
    }

    /// Pops a scope pushed by `push_component_types_scope`.
    pub fn pop_component_types_scope(&mut self) {
        self.type_scopes.pop().unwrap();
    }

    /// Translates a wasmparser `ComponentTypeDef` into a Wasmtime `TypeDef`,
    /// interning types along the way.
    pub fn component_type_def(&mut self, ty: &wasmparser::ComponentTypeDef<'_>) -> Result<TypeDef> {
        Ok(match ty {
            wasmparser::ComponentTypeDef::Module(ty) => TypeDef::Module(self.module_type(ty)?),
            wasmparser::ComponentTypeDef::Component(ty) => {
                TypeDef::Component(self.component_type(ty)?)
            }
            wasmparser::ComponentTypeDef::Instance(ty) => {
                TypeDef::ComponentInstance(self.component_instance_type(ty)?)
            }
            wasmparser::ComponentTypeDef::Function(ty) => TypeDef::Func(self.func_type(ty)),
            wasmparser::ComponentTypeDef::Value(_ty) => unimplemented!("value types"),
            wasmparser::ComponentTypeDef::Interface(ty) => {
                TypeDef::Interface(self.interface_type(ty))
            }
        })
    }

    fn module_type(&mut self, ty: &[wasmparser::ModuleType<'_>]) -> Result<ModuleTypeIndex> {
        let mut result = ModuleType::default();
        let mut functypes: PrimaryMap<TypeIndex, SignatureIndex> = PrimaryMap::default();

        for item in ty {
            match item {
                wasmparser::ModuleType::Type(wasmparser::TypeDef::Func(f)) => {
                    functypes.push(self.module_types.wasm_func_type(f.clone().try_into()?));
                }
                wasmparser::ModuleType::Export { name, ty } => {
                    let prev = result
                        .exports
                        .insert(name.to_string(), type_ref(ty, &functypes)?);
                    assert!(prev.is_none());
                }
                wasmparser::ModuleType::Import(import) => {
                    let prev = result.imports.insert(
                        (import.module.to_string(), import.name.to_string()),
                        type_ref(&import.ty, &functypes)?,
                    );
                    assert!(prev.is_none());
                }
            }
        }

        return Ok(self.component_types.modules.push(result));

        fn type_ref(
            ty: &wasmparser::TypeRef,
            functypes: &PrimaryMap<TypeIndex, SignatureIndex>,
        ) -> Result<EntityType> {
            Ok(match ty {
                wasmparser::TypeRef::Func(idx) => {
                    EntityType::Function(functypes[TypeIndex::from_u32(*idx)])
                }
                wasmparser::TypeRef::Table(ty) => EntityType::Table(ty.clone().try_into()?),
                wasmparser::TypeRef::Memory(ty) => EntityType::Memory(ty.clone().into()),
                wasmparser::TypeRef::Global(ty) => {
                    EntityType::Global(Global::new(ty.clone(), GlobalInit::Import)?)
                }
                wasmparser::TypeRef::Tag(_) => bail!("exceptions proposal not implemented"),
            })
        }
    }

    fn component_type(
        &mut self,
        ty: &[wasmparser::ComponentType<'_>],
    ) -> Result<ComponentTypeIndex> {
        let mut result = ComponentType::default();
        self.push_component_types_scope();

        for item in ty {
            match item {
                wasmparser::ComponentType::Type(ty) => {
                    let ty = self.component_type_def(ty)?;
                    self.push_component_typedef(ty);
                }
                wasmparser::ComponentType::OuterType { count, index } => {
                    let ty = self.component_outer_type(*count, TypeIndex::from_u32(*index));
                    self.push_component_typedef(ty);
                }
                wasmparser::ComponentType::Export { name, ty } => {
                    result.exports.insert(
                        name.to_string(),
                        self.component_outer_type(0, TypeIndex::from_u32(*ty)),
                    );
                }
                wasmparser::ComponentType::Import(import) => {
                    result.imports.insert(
                        import.name.to_string(),
                        self.component_outer_type(0, TypeIndex::from_u32(import.ty)),
                    );
                }
            }
        }

        self.pop_component_types_scope();

        Ok(self.component_types.components.push(result))
    }

    fn component_instance_type(
        &mut self,
        ty: &[wasmparser::InstanceType<'_>],
    ) -> Result<ComponentInstanceTypeIndex> {
        let mut result = ComponentInstanceType::default();
        self.push_component_types_scope();

        for item in ty {
            match item {
                wasmparser::InstanceType::Type(ty) => {
                    let ty = self.component_type_def(ty)?;
                    self.push_component_typedef(ty);
                }
                wasmparser::InstanceType::OuterType { count, index } => {
                    let ty = self.component_outer_type(*count, TypeIndex::from_u32(*index));
                    self.push_component_typedef(ty);
                }
                wasmparser::InstanceType::Export { name, ty } => {
                    result.exports.insert(
                        name.to_string(),
                        self.component_outer_type(0, TypeIndex::from_u32(*ty)),
                    );
                }
            }
        }

        self.pop_component_types_scope();

        Ok(self.component_types.component_instances.push(result))
    }

    fn func_type(&mut self, ty: &wasmparser::ComponentFuncType<'_>) -> FuncTypeIndex {
        let ty = FuncType {
            params: ty
                .params
                .iter()
                .map(|(name, ty)| (name.map(|s| s.to_string()), self.interface_type_ref(ty)))
                .collect(),
            result: self.interface_type_ref(&ty.result),
        };
        intern(&mut self.functions, &mut self.component_types.functions, ty)
    }

    fn interface_type(&mut self, ty: &wasmparser::InterfaceType<'_>) -> InterfaceType {
        match ty {
            wasmparser::InterfaceType::Primitive(ty) => ty.into(),
            wasmparser::InterfaceType::Record(e) => InterfaceType::Record(self.record_type(e)),
            wasmparser::InterfaceType::Variant(e) => InterfaceType::Variant(self.variant_type(e)),
            wasmparser::InterfaceType::List(e) => {
                let ty = self.interface_type_ref(e);
                InterfaceType::List(self.intern_interface_type(ty))
            }
            wasmparser::InterfaceType::Tuple(e) => InterfaceType::Tuple(self.tuple_type(e)),
            wasmparser::InterfaceType::Flags(e) => InterfaceType::Flags(self.flags_type(e)),
            wasmparser::InterfaceType::Enum(e) => InterfaceType::Enum(self.enum_type(e)),
            wasmparser::InterfaceType::Union(e) => InterfaceType::Union(self.union_type(e)),
            wasmparser::InterfaceType::Option(e) => {
                let ty = self.interface_type_ref(e);
                InterfaceType::Option(self.intern_interface_type(ty))
            }
            wasmparser::InterfaceType::Expected { ok, error } => {
                InterfaceType::Expected(self.expected_type(ok, error))
            }
        }
    }

    fn interface_type_ref(&mut self, ty: &wasmparser::InterfaceTypeRef) -> InterfaceType {
        match ty {
            wasmparser::InterfaceTypeRef::Primitive(p) => p.into(),
            wasmparser::InterfaceTypeRef::Type(idx) => {
                let idx = TypeIndex::from_u32(*idx);
                match self.component_outer_type(0, idx) {
                    TypeDef::Interface(ty) => ty,
                    // this should not be possible if the module validated
                    _ => unreachable!(),
                }
            }
        }
    }

    fn intern_interface_type(&mut self, ty: InterfaceType) -> InterfaceTypeIndex {
        intern(
            &mut self.interface_types,
            &mut self.component_types.interface_types,
            ty,
        )
    }

    fn record_type(&mut self, record: &[(&str, wasmparser::InterfaceTypeRef)]) -> RecordTypeIndex {
        let record = RecordType {
            fields: record
                .iter()
                .map(|(name, ty)| RecordField {
                    name: name.to_string(),
                    ty: self.interface_type_ref(ty),
                })
                .collect(),
        };
        intern(&mut self.records, &mut self.component_types.records, record)
    }

    fn variant_type(&mut self, cases: &[wasmparser::VariantCase<'_>]) -> VariantTypeIndex {
        let variant = VariantType {
            cases: cases
                .iter()
                .map(|case| {
                    // FIXME: need to implement `default_to`, not sure what that
                    // is at this time.
                    assert!(case.default_to.is_none());
                    VariantCase {
                        name: case.name.to_string(),
                        ty: self.interface_type_ref(&case.ty),
                    }
                })
                .collect(),
        };
        intern(
            &mut self.variants,
            &mut self.component_types.variants,
            variant,
        )
    }

    fn tuple_type(&mut self, types: &[wasmparser::InterfaceTypeRef]) -> TupleTypeIndex {
        let tuple = TupleType {
            types: types.iter().map(|ty| self.interface_type_ref(ty)).collect(),
        };
        intern(&mut self.tuples, &mut self.component_types.tuples, tuple)
    }

    fn flags_type(&mut self, flags: &[&str]) -> FlagsTypeIndex {
        let flags = FlagsType {
            names: flags.iter().map(|s| s.to_string()).collect(),
        };
        intern(&mut self.flags, &mut self.component_types.flags, flags)
    }

    fn enum_type(&mut self, variants: &[&str]) -> EnumTypeIndex {
        let e = EnumType {
            names: variants.iter().map(|s| s.to_string()).collect(),
        };
        intern(&mut self.enums, &mut self.component_types.enums, e)
    }

    fn union_type(&mut self, types: &[wasmparser::InterfaceTypeRef]) -> UnionTypeIndex {
        let union = UnionType {
            types: types.iter().map(|ty| self.interface_type_ref(ty)).collect(),
        };
        intern(&mut self.unions, &mut self.component_types.unions, union)
    }

    fn expected_type(
        &mut self,
        ok: &wasmparser::InterfaceTypeRef,
        err: &wasmparser::InterfaceTypeRef,
    ) -> ExpectedTypeIndex {
        let expected = ExpectedType {
            ok: self.interface_type_ref(ok),
            err: self.interface_type_ref(err),
        };
        intern(
            &mut self.expecteds,
            &mut self.component_types.expecteds,
            expected,
        )
    }
}

// Forward the indexing impl to the internal `TypeTables`
impl<T> Index<T> for ComponentTypesBuilder
where
    ComponentTypes: Index<T>,
{
    type Output = <ComponentTypes as Index<T>>::Output;

    fn index(&self, sig: T) -> &Self::Output {
        &self.component_types[sig]
    }
}

fn intern<T, U>(map: &mut HashMap<T, U>, list: &mut PrimaryMap<U, T>, item: T) -> U
where
    T: Hash + Clone + Eq,
    U: Copy + EntityRef,
{
    if let Some(idx) = map.get(&item) {
        return *idx;
    }
    let idx = list.push(item.clone());
    map.insert(item, idx);
    return idx;
}

/// Types of imports and exports in the component model.
///
/// These types are what's available for import and export in components. Note
/// that all indirect indices contained here are intended to be looked up
/// through a sibling `ComponentTypes` structure.
#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum TypeDef {
    /// A core wasm module and its type.
    Module(ModuleTypeIndex),
    /// A component and its type.
    Component(ComponentTypeIndex),
    /// An instance of a component.
    ComponentInstance(ComponentInstanceTypeIndex),
    /// A component function, not to be confused with a core wasm function.
    Func(FuncTypeIndex),
    /// An interface type.
    Interface(InterfaceType),
}

// NB: Note that maps below are stored as an `IndexMap` now but the order
// typically does not matter. As a minor implementation detail we want the
// serialization of this type to always be deterministic and using `IndexMap`
// gets us that over using a `HashMap` for example.

/// The type of a module in the component model.
///
/// Note that this is not to be confused with `ComponentType` below. This is
/// intended only for core wasm modules, not for components.
#[derive(Serialize, Deserialize, Default)]
pub struct ModuleType {
    /// The values that this module imports.
    ///
    /// Note that the value of this map is a core wasm `EntityType`, not a
    /// component model `TypeRef`. Additionally note that this reflects the
    /// two-level namespace of core WebAssembly, but unlike core wasm all import
    /// names are required to be unique to describe a module in the component
    /// model.
    pub imports: IndexMap<(String, String), EntityType>,

    /// The values that this module exports.
    ///
    /// Note that the value of this map is the core wasm `EntityType` to
    /// represent that core wasm items are being exported.
    pub exports: IndexMap<String, EntityType>,
}

/// The type of a component in the component model.
#[derive(Serialize, Deserialize, Default)]
pub struct ComponentType {
    /// The named values that this component imports.
    pub imports: IndexMap<String, TypeDef>,
    /// The named values that this component exports.
    pub exports: IndexMap<String, TypeDef>,
}

/// The type of a component instance in the component model, or an instantiated
/// component.
///
/// Component instances only have exports of types in the component model.
#[derive(Serialize, Deserialize, Default)]
pub struct ComponentInstanceType {
    /// The list of exports that this component has along with their types.
    pub exports: IndexMap<String, TypeDef>,
}

/// A component function type in the component model.
#[derive(Serialize, Deserialize, Clone, Hash, Eq, PartialEq, Debug)]
pub struct FuncType {
    /// The list of optionally named parameters for this function, and their
    /// types.
    pub params: Box<[(Option<String>, InterfaceType)]>,
    /// The return value of this function.
    pub result: InterfaceType,
}

/// All possible interface types that values can have.
///
/// This list represents an exhaustive listing of interface types and the
/// shapes that they can take. Note that this enum is considered an "index" of
/// forms where for non-primitive types a `ComponentTypes` structure is used to
/// lookup further information based on the index found here.
#[derive(Serialize, Deserialize, Copy, Clone, Hash, Eq, PartialEq, Debug)]
#[allow(missing_docs)]
pub enum InterfaceType {
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
    Record(RecordTypeIndex),
    Variant(VariantTypeIndex),
    List(InterfaceTypeIndex),
    Tuple(TupleTypeIndex),
    Flags(FlagsTypeIndex),
    Enum(EnumTypeIndex),
    Union(UnionTypeIndex),
    Option(InterfaceTypeIndex),
    Expected(ExpectedTypeIndex),
}

impl From<&wasmparser::PrimitiveInterfaceType> for InterfaceType {
    fn from(ty: &wasmparser::PrimitiveInterfaceType) -> InterfaceType {
        match ty {
            wasmparser::PrimitiveInterfaceType::Unit => InterfaceType::Unit,
            wasmparser::PrimitiveInterfaceType::Bool => InterfaceType::Bool,
            wasmparser::PrimitiveInterfaceType::S8 => InterfaceType::S8,
            wasmparser::PrimitiveInterfaceType::U8 => InterfaceType::U8,
            wasmparser::PrimitiveInterfaceType::S16 => InterfaceType::S16,
            wasmparser::PrimitiveInterfaceType::U16 => InterfaceType::U16,
            wasmparser::PrimitiveInterfaceType::S32 => InterfaceType::S32,
            wasmparser::PrimitiveInterfaceType::U32 => InterfaceType::U32,
            wasmparser::PrimitiveInterfaceType::S64 => InterfaceType::S64,
            wasmparser::PrimitiveInterfaceType::U64 => InterfaceType::U64,
            wasmparser::PrimitiveInterfaceType::Float32 => InterfaceType::Float32,
            wasmparser::PrimitiveInterfaceType::Float64 => InterfaceType::Float64,
            wasmparser::PrimitiveInterfaceType::Char => InterfaceType::Char,
            wasmparser::PrimitiveInterfaceType::String => InterfaceType::String,
        }
    }
}

/// Shape of a "record" type in interface types.
///
/// This is equivalent to a `struct` in Rust.
#[derive(Serialize, Deserialize, Clone, Hash, Eq, PartialEq, Debug)]
pub struct RecordType {
    /// The fields that are contained within this struct type.
    pub fields: Box<[RecordField]>,
}

/// One field within a record.
#[derive(Serialize, Deserialize, Clone, Hash, Eq, PartialEq, Debug)]
pub struct RecordField {
    /// The name of the field, unique amongst all fields in a record.
    pub name: String,
    /// The type that this field contains.
    pub ty: InterfaceType,
}

/// Shape of a "variant" type in interface types.
///
/// Variants are close to Rust `enum` declarations where a value is one of many
/// cases and each case has a unique name and an optional payload associated
/// with it.
#[derive(Serialize, Deserialize, Clone, Hash, Eq, PartialEq, Debug)]
pub struct VariantType {
    /// The list of cases that this variant can take.
    pub cases: Box<[VariantCase]>,
}

/// One case of a `variant` type which contains the name of the variant as well
/// as the payload.
#[derive(Serialize, Deserialize, Clone, Hash, Eq, PartialEq, Debug)]
pub struct VariantCase {
    /// Name of the variant, unique amongst all cases in a variant.
    pub name: String,
    /// Type associated with this payload, maybe `Unit`.
    pub ty: InterfaceType,
}

/// Shape of a "tuple" type in interface types.
///
/// This is largely the same as a tuple in Rust, basically a record with
/// unnamed fields.
#[derive(Serialize, Deserialize, Clone, Hash, Eq, PartialEq, Debug)]
pub struct TupleType {
    /// The types that are contained within this tuple.
    pub types: Box<[InterfaceType]>,
}

/// Shape of a "flags" type in interface types.
///
/// This can be thought of as a record-of-bools, although the representation is
/// more efficient as bitflags.
#[derive(Serialize, Deserialize, Clone, Hash, Eq, PartialEq, Debug)]
pub struct FlagsType {
    /// The names of all flags, all of which are unique.
    pub names: Box<[String]>,
}

/// Shape of an "enum" type in interface types, not to be confused with a Rust
/// `enum` type.
///
/// In interface types enums are simply a bag of names, and can be seen as a
/// variant where all payloads are `Unit`.
#[derive(Serialize, Deserialize, Clone, Hash, Eq, PartialEq, Debug)]
pub struct EnumType {
    /// The names of this enum, all of which are unique.
    pub names: Box<[String]>,
}

/// Shape of a "union" type in interface types.
///
/// Note that this can be viewed as a specialization of the `variant` interface
/// type where each type here has a name that's numbered. This is still a
/// tagged union.
#[derive(Serialize, Deserialize, Clone, Hash, Eq, PartialEq, Debug)]
pub struct UnionType {
    /// The list of types this is a union over.
    pub types: Box<[InterfaceType]>,
}

/// Shape of an "expected" interface type.
#[derive(Serialize, Deserialize, Clone, Hash, Eq, PartialEq, Debug)]
pub struct ExpectedType {
    /// The `T` in `Result<T, E>`
    pub ok: InterfaceType,
    /// The `E` in `Result<T, E>`
    pub err: InterfaceType,
}
