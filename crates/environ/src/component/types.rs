use crate::component::{Export, MAX_FLAT_PARAMS, MAX_FLAT_RESULTS};
use crate::{
    CompiledModuleInfo, EntityType, Module, ModuleTypes, ModuleTypesBuilder, PrimaryMap,
    TypeConvert, WasmHeapType, WasmValType,
};
use anyhow::{bail, Result};
use cranelift_entity::EntityRef;
use indexmap::{IndexMap, IndexSet};
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::ops::Index;
use wasmparser::names::KebabString;
use wasmparser::{types, Validator};
use wasmtime_component_util::{DiscriminantSize, FlagsSize};
use wasmtime_types::ModuleInternedTypeIndex;

pub use wasmtime_types::StaticModuleIndex;

mod resources;
pub use resources::ResourcesBuilder;

/// Maximum nesting depth of a type allowed in Wasmtime.
///
/// This constant isn't chosen via any scientific means and its main purpose is
/// to enable most of Wasmtime to handle types via recursion without worrying
/// about stack overflow.
///
/// Some more information about this can be found in #4814
const MAX_TYPE_DEPTH: u32 = 100;

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
        #[repr(transparent)]
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

    /// Index within a component's component type index space.
    pub struct ComponentTypeIndex(u32);

    /// Index within a component's module index space.
    pub struct ModuleIndex(u32);

    /// Index within a component's component index space.
    pub struct ComponentIndex(u32);

    /// Index within a component's module instance index space.
    pub struct ModuleInstanceIndex(u32);

    /// Index within a component's component instance index space.
    pub struct ComponentInstanceIndex(u32);

    /// Index within a component's component function index space.
    pub struct ComponentFuncIndex(u32);

    // ========================================================================
    // These indices are used to lookup type information within a `TypeTables`
    // structure. These represent generally deduplicated type information across
    // an entire component and are a form of RTTI in a sense.

    /// Index pointing to a component's type (exports/imports with
    /// component-model types)
    pub struct TypeComponentIndex(u32);

    /// Index pointing to a component instance's type (exports with
    /// component-model types, no imports)
    pub struct TypeComponentInstanceIndex(u32);

    /// Index pointing to a core wasm module's type (exports/imports with
    /// core wasm types)
    pub struct TypeModuleIndex(u32);

    /// Index pointing to a component model function type with arguments/result
    /// as interface types.
    pub struct TypeFuncIndex(u32);

    /// Index pointing to a record type in the component model (aka a struct).
    pub struct TypeRecordIndex(u32);
    /// Index pointing to a variant type in the component model (aka an enum).
    pub struct TypeVariantIndex(u32);
    /// Index pointing to a tuple type in the component model.
    pub struct TypeTupleIndex(u32);
    /// Index pointing to a flags type in the component model.
    pub struct TypeFlagsIndex(u32);
    /// Index pointing to an enum type in the component model.
    pub struct TypeEnumIndex(u32);
    /// Index pointing to an option type in the component model (aka a
    /// `Option<T, E>`)
    pub struct TypeOptionIndex(u32);
    /// Index pointing to an result type in the component model (aka a
    /// `Result<T, E>`)
    pub struct TypeResultIndex(u32);
    /// Index pointing to a list type in the component model.
    pub struct TypeListIndex(u32);

    /// Index pointing to a resource table within a component.
    ///
    /// This is a Wasmtime-specific type index which isn't part of the component
    /// model per-se (or at least not the binary format). This index represents
    /// a pointer to a table of runtime information tracking state for resources
    /// within a component. Tables are generated per-resource-per-component
    /// meaning that if the exact same resource is imported into 4 subcomponents
    /// then that's 5 tables: one for the defining component and one for each
    /// subcomponent.
    ///
    /// All resource-related intrinsics operate on table-local indices which
    /// indicate which table the intrinsic is modifying. Each resource table has
    /// an origin resource type (defined by `ResourceIndex`) along with a
    /// component instance that it's recorded for.
    pub struct TypeResourceTableIndex(u32);

    /// Index pointing to a resource within a component.
    ///
    /// This index space covers all unique resource type definitions. For
    /// example all unique imports come first and then all locally-defined
    /// resources come next. Note that this does not count the number of runtime
    /// tables required to track resources (that's `TypeResourceTableIndex`
    /// instead). Instead this is a count of the number of unique
    /// `(type (resource (rep ..)))` declarations within a component, plus
    /// imports.
    ///
    /// This is then used for correlating various information such as
    /// destructors, origin information, etc.
    pub struct ResourceIndex(u32);

    /// Index pointing to a local resource defined within a component.
    ///
    /// This is similar to `FooIndex` and `DefinedFooIndex` for core wasm and
    /// the idea here is that this is guaranteed to be a wasm-defined resource
    /// which is connected to a component instance for example.
    pub struct DefinedResourceIndex(u32);

    // ========================================================================
    // Index types used to identify modules and components during compilation.

    /// Index into a "closed over variables" list for components used to
    /// implement outer aliases. For more information on this see the
    /// documentation for the `LexicalScope` structure.
    pub struct ModuleUpvarIndex(u32);

    /// Same as `ModuleUpvarIndex` but for components.
    pub struct ComponentUpvarIndex(u32);

    /// Same as `StaticModuleIndex` but for components.
    pub struct StaticComponentIndex(u32);

    // ========================================================================
    // These indices are actually used at runtime when managing a component at
    // this time.

    /// Index that represents a core wasm instance created at runtime.
    ///
    /// This is used to keep track of when instances are created and is able to
    /// refer back to previously created instances for exports and such.
    pub struct RuntimeInstanceIndex(u32);

    /// Same as `RuntimeInstanceIndex` but tracks component instances instead.
    pub struct RuntimeComponentInstanceIndex(u32);

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

    /// Same as `RuntimeMemoryIndex` except for the `post-return` function.
    pub struct RuntimePostReturnIndex(u32);

    /// Index for all trampolines that are compiled in Cranelift for a
    /// component.
    ///
    /// This is used to point to various bits of metadata within a compiled
    /// component and is stored in the final compilation artifact. This does not
    /// have a direct corresponance to any wasm definition.
    pub struct TrampolineIndex(u32);
}

// Reexport for convenience some core-wasm indices which are also used in the
// component model, typically for when aliasing exports of core wasm modules.
pub use crate::{FuncIndex, GlobalIndex, MemoryIndex, TableIndex};

/// Equivalent of `EntityIndex` but for the component model instead of core
/// wasm.
#[derive(Debug, Clone, Copy)]
#[allow(missing_docs)]
pub enum ComponentItem {
    Func(ComponentFuncIndex),
    Module(ModuleIndex),
    Component(ComponentIndex),
    ComponentInstance(ComponentInstanceIndex),
    Type(types::ComponentAnyTypeId),
}

/// Runtime information about the type information contained within a component.
///
/// One of these is created per top-level component which describes all of the
/// types contained within the top-level component itself. Each sub-component
/// will have a pointer to this value as well.
#[derive(Default, Serialize, Deserialize)]
pub struct ComponentTypes {
    modules: PrimaryMap<TypeModuleIndex, TypeModule>,
    components: PrimaryMap<TypeComponentIndex, TypeComponent>,
    component_instances: PrimaryMap<TypeComponentInstanceIndex, TypeComponentInstance>,
    functions: PrimaryMap<TypeFuncIndex, TypeFunc>,
    lists: PrimaryMap<TypeListIndex, TypeList>,
    records: PrimaryMap<TypeRecordIndex, TypeRecord>,
    variants: PrimaryMap<TypeVariantIndex, TypeVariant>,
    tuples: PrimaryMap<TypeTupleIndex, TypeTuple>,
    enums: PrimaryMap<TypeEnumIndex, TypeEnum>,
    flags: PrimaryMap<TypeFlagsIndex, TypeFlags>,
    options: PrimaryMap<TypeOptionIndex, TypeOption>,
    results: PrimaryMap<TypeResultIndex, TypeResult>,
    resource_tables: PrimaryMap<TypeResourceTableIndex, TypeResourceTable>,

    module_types: Option<ModuleTypes>,
}

impl ComponentTypes {
    /// Returns the core wasm module types known within this component.
    pub fn module_types(&self) -> &ModuleTypes {
        self.module_types.as_ref().unwrap()
    }

    /// Returns the canonical ABI information about the specified type.
    pub fn canonical_abi(&self, ty: &InterfaceType) -> &CanonicalAbiInfo {
        match ty {
            InterfaceType::U8 | InterfaceType::S8 | InterfaceType::Bool => {
                &CanonicalAbiInfo::SCALAR1
            }

            InterfaceType::U16 | InterfaceType::S16 => &CanonicalAbiInfo::SCALAR2,

            InterfaceType::U32
            | InterfaceType::S32
            | InterfaceType::Float32
            | InterfaceType::Char
            | InterfaceType::Own(_)
            | InterfaceType::Borrow(_) => &CanonicalAbiInfo::SCALAR4,

            InterfaceType::U64 | InterfaceType::S64 | InterfaceType::Float64 => {
                &CanonicalAbiInfo::SCALAR8
            }

            InterfaceType::String | InterfaceType::List(_) => &CanonicalAbiInfo::POINTER_PAIR,

            InterfaceType::Record(i) => &self[*i].abi,
            InterfaceType::Variant(i) => &self[*i].abi,
            InterfaceType::Tuple(i) => &self[*i].abi,
            InterfaceType::Flags(i) => &self[*i].abi,
            InterfaceType::Enum(i) => &self[*i].abi,
            InterfaceType::Option(i) => &self[*i].abi,
            InterfaceType::Result(i) => &self[*i].abi,
        }
    }
}

macro_rules! impl_index {
    ($(impl Index<$ty:ident> for ComponentTypes { $output:ident => $field:ident })*) => ($(
        impl std::ops::Index<$ty> for ComponentTypes {
            type Output = $output;
            #[inline]
            fn index(&self, idx: $ty) -> &$output {
                &self.$field[idx]
            }
        }

        impl std::ops::Index<$ty> for ComponentTypesBuilder {
            type Output = $output;
            #[inline]
            fn index(&self, idx: $ty) -> &$output {
                &self.component_types[idx]
            }
        }
    )*)
}

impl_index! {
    impl Index<TypeModuleIndex> for ComponentTypes { TypeModule => modules }
    impl Index<TypeComponentIndex> for ComponentTypes { TypeComponent => components }
    impl Index<TypeComponentInstanceIndex> for ComponentTypes { TypeComponentInstance => component_instances }
    impl Index<TypeFuncIndex> for ComponentTypes { TypeFunc => functions }
    impl Index<TypeRecordIndex> for ComponentTypes { TypeRecord => records }
    impl Index<TypeVariantIndex> for ComponentTypes { TypeVariant => variants }
    impl Index<TypeTupleIndex> for ComponentTypes { TypeTuple => tuples }
    impl Index<TypeEnumIndex> for ComponentTypes { TypeEnum => enums }
    impl Index<TypeFlagsIndex> for ComponentTypes { TypeFlags => flags }
    impl Index<TypeOptionIndex> for ComponentTypes { TypeOption => options }
    impl Index<TypeResultIndex> for ComponentTypes { TypeResult => results }
    impl Index<TypeListIndex> for ComponentTypes { TypeList => lists }
    impl Index<TypeResourceTableIndex> for ComponentTypes { TypeResourceTable => resource_tables }
}

// Additionally forward anything that can index `ModuleTypes` to `ModuleTypes`
// (aka `SignatureIndex`)
impl<T> Index<T> for ComponentTypes
where
    ModuleTypes: Index<T>,
{
    type Output = <ModuleTypes as Index<T>>::Output;
    fn index(&self, idx: T) -> &Self::Output {
        self.module_types.as_ref().unwrap().index(idx)
    }
}

impl<T> Index<T> for ComponentTypesBuilder
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
pub struct ComponentTypesBuilder {
    functions: HashMap<TypeFunc, TypeFuncIndex>,
    lists: HashMap<TypeList, TypeListIndex>,
    records: HashMap<TypeRecord, TypeRecordIndex>,
    variants: HashMap<TypeVariant, TypeVariantIndex>,
    tuples: HashMap<TypeTuple, TypeTupleIndex>,
    enums: HashMap<TypeEnum, TypeEnumIndex>,
    flags: HashMap<TypeFlags, TypeFlagsIndex>,
    options: HashMap<TypeOption, TypeOptionIndex>,
    results: HashMap<TypeResult, TypeResultIndex>,

    component_types: ComponentTypes,
    module_types: ModuleTypesBuilder,

    // Cache of what the "flat" representation of all types are which is only
    // used at compile-time and not used at runtime, hence the location here
    // as opposed to `ComponentTypes`.
    type_info: TypeInformationCache,

    resources: ResourcesBuilder,
}

macro_rules! intern_and_fill_flat_types {
    ($me:ident, $name:ident, $val:ident) => {{
        if let Some(idx) = $me.$name.get(&$val) {
            return *idx;
        }
        let idx = $me.component_types.$name.push($val.clone());
        let mut info = TypeInformation::new();
        info.$name($me, &$val);
        let idx2 = $me.type_info.$name.push(info);
        assert_eq!(idx, idx2);
        $me.$name.insert($val, idx);
        return idx;
    }};
}

impl ComponentTypesBuilder {
    /// Construct a new `ComponentTypesBuilder` for use with the given validator.
    pub fn new(validator: &Validator) -> Self {
        Self {
            module_types: ModuleTypesBuilder::new(validator),

            functions: HashMap::default(),
            lists: HashMap::default(),
            records: HashMap::default(),
            variants: HashMap::default(),
            tuples: HashMap::default(),
            enums: HashMap::default(),
            flags: HashMap::default(),
            options: HashMap::default(),
            results: HashMap::default(),
            component_types: ComponentTypes::default(),
            type_info: TypeInformationCache::default(),
            resources: ResourcesBuilder::default(),
        }
    }

    fn export_type_def(
        &mut self,
        static_modules: &PrimaryMap<StaticModuleIndex, CompiledModuleInfo>,
        ty: &Export,
    ) -> TypeDef {
        match ty {
            Export::LiftedFunction { ty, .. } => TypeDef::ComponentFunc(*ty),
            Export::ModuleStatic(idx) => {
                let mut module_ty = TypeModule::default();
                let module = &static_modules[*idx].module;
                for (namespace, name, ty) in module.imports() {
                    module_ty
                        .imports
                        .insert((namespace.to_string(), name.to_string()), ty);
                }
                for (name, ty) in module.exports.iter() {
                    module_ty
                        .exports
                        .insert(name.to_string(), module.type_of(*ty));
                }
                TypeDef::Module(self.component_types.modules.push(module_ty))
            }
            Export::ModuleImport { ty, .. } => TypeDef::Module(*ty),
            Export::Instance { ty: Some(ty), .. } => TypeDef::ComponentInstance(*ty),
            Export::Instance { exports, .. } => {
                let mut instance_ty = TypeComponentInstance::default();
                for (name, ty) in exports {
                    instance_ty
                        .exports
                        .insert(name.to_string(), self.export_type_def(static_modules, ty));
                }
                TypeDef::ComponentInstance(
                    self.component_types.component_instances.push(instance_ty),
                )
            }
            Export::Type(ty) => *ty,
        }
    }

    /// Finishes this list of component types and returns the finished
    /// structure and the [`TypeComponentIndex`] corresponding to top-level component
    /// with `imports` and `exports` specified.
    pub fn finish<'a>(
        mut self,
        static_modules: &PrimaryMap<StaticModuleIndex, CompiledModuleInfo>,
        imports: impl IntoIterator<Item = (String, TypeDef)>,
        exports: impl IntoIterator<Item = (String, &'a Export)>,
    ) -> (ComponentTypes, TypeComponentIndex) {
        let mut component_ty = TypeComponent::default();
        for (name, ty) in imports {
            component_ty.imports.insert(name, ty);
        }
        for (name, ty) in exports {
            component_ty
                .exports
                .insert(name, self.export_type_def(static_modules, ty));
        }
        let ty = self.component_types.components.push(component_ty);

        self.component_types.module_types = Some(self.module_types.finish());
        (self.component_types, ty)
    }

    /// Smaller helper method to find a `ModuleInternedTypeIndex` which
    /// corresponds to the `resource.drop` intrinsic in components, namely a
    /// core wasm function type which takes one `i32` argument and has no
    /// results.
    ///
    /// This is a bit of a hack right now as ideally this find operation
    /// wouldn't be needed and instead the `ModuleInternedTypeIndex` itself
    /// would be threaded through appropriately, but that's left for a future
    /// refactoring. Try not to lean too hard on this method though.
    pub fn find_resource_drop_signature(&self) -> Option<ModuleInternedTypeIndex> {
        self.module_types
            .wasm_types()
            .find(|(_, ty)| match &ty.composite_type {
                wasmtime_types::WasmCompositeType::Func(sig) => {
                    sig.params().len() == 1
                        && sig.returns().len() == 0
                        && sig.params()[0] == WasmValType::I32
                }
            })
            .map(|(i, _)| i)
    }

    /// Returns the underlying builder used to build up core wasm module types.
    ///
    /// Note that this is shared across all modules found within a component to
    /// improve the wins from deduplicating function signatures.
    pub fn module_types_builder(&self) -> &ModuleTypesBuilder {
        &self.module_types
    }

    /// Same as `module_types_builder`, but `mut`.
    pub fn module_types_builder_mut(&mut self) -> &mut ModuleTypesBuilder {
        &mut self.module_types
    }

    /// Returns the number of resource tables allocated so far, or the maximum
    /// `TypeResourceTableIndex`.
    pub fn num_resource_tables(&self) -> usize {
        self.component_types.resource_tables.len()
    }

    /// Returns a mutable reference to the underlying `ResourcesBuilder`.
    pub fn resources_mut(&mut self) -> &mut ResourcesBuilder {
        &mut self.resources
    }

    /// Work around the borrow checker to borrow two sub-fields simultaneously
    /// externally.
    pub fn resources_mut_and_types(&mut self) -> (&mut ResourcesBuilder, &ComponentTypes) {
        (&mut self.resources, &self.component_types)
    }

    /// Converts a wasmparser `ComponentFuncType` into Wasmtime's type
    /// representation.
    pub fn convert_component_func_type(
        &mut self,
        types: types::TypesRef<'_>,
        id: types::ComponentFuncTypeId,
    ) -> Result<TypeFuncIndex> {
        assert_eq!(types.id(), self.module_types.validator_id());
        let ty = &types[id];
        let params = ty
            .params
            .iter()
            .map(|(_name, ty)| self.valtype(types, ty))
            .collect::<Result<_>>()?;
        let results = ty
            .results
            .iter()
            .map(|(_name, ty)| self.valtype(types, ty))
            .collect::<Result<_>>()?;
        let ty = TypeFunc {
            params: self.new_tuple_type(params),
            results: self.new_tuple_type(results),
        };
        Ok(self.add_func_type(ty))
    }

    /// Converts a wasmparser `ComponentEntityType` into Wasmtime's type
    /// representation.
    pub fn convert_component_entity_type(
        &mut self,
        types: types::TypesRef<'_>,
        ty: types::ComponentEntityType,
    ) -> Result<TypeDef> {
        assert_eq!(types.id(), self.module_types.validator_id());
        Ok(match ty {
            types::ComponentEntityType::Module(id) => {
                TypeDef::Module(self.convert_module(types, id)?)
            }
            types::ComponentEntityType::Component(id) => {
                TypeDef::Component(self.convert_component(types, id)?)
            }
            types::ComponentEntityType::Instance(id) => {
                TypeDef::ComponentInstance(self.convert_instance(types, id)?)
            }
            types::ComponentEntityType::Func(id) => {
                TypeDef::ComponentFunc(self.convert_component_func_type(types, id)?)
            }
            types::ComponentEntityType::Type { created, .. } => match created {
                types::ComponentAnyTypeId::Defined(id) => {
                    TypeDef::Interface(self.defined_type(types, id)?)
                }
                types::ComponentAnyTypeId::Resource(id) => {
                    TypeDef::Resource(self.resource_id(id.resource()))
                }
                _ => bail!("unsupported type export"),
            },
            types::ComponentEntityType::Value(_) => bail!("values not supported"),
        })
    }

    /// Converts a wasmparser `Type` into Wasmtime's type representation.
    pub fn convert_type(
        &mut self,
        types: types::TypesRef<'_>,
        id: types::ComponentAnyTypeId,
    ) -> Result<TypeDef> {
        assert_eq!(types.id(), self.module_types.validator_id());
        Ok(match id {
            types::ComponentAnyTypeId::Defined(id) => {
                TypeDef::Interface(self.defined_type(types, id)?)
            }
            types::ComponentAnyTypeId::Component(id) => {
                TypeDef::Component(self.convert_component(types, id)?)
            }
            types::ComponentAnyTypeId::Instance(id) => {
                TypeDef::ComponentInstance(self.convert_instance(types, id)?)
            }
            types::ComponentAnyTypeId::Func(id) => {
                TypeDef::ComponentFunc(self.convert_component_func_type(types, id)?)
            }
            types::ComponentAnyTypeId::Resource(id) => {
                TypeDef::Resource(self.resource_id(id.resource()))
            }
        })
    }

    fn convert_component(
        &mut self,
        types: types::TypesRef<'_>,
        id: types::ComponentTypeId,
    ) -> Result<TypeComponentIndex> {
        assert_eq!(types.id(), self.module_types.validator_id());
        let ty = &types[id];
        let mut result = TypeComponent::default();
        for (name, ty) in ty.imports.iter() {
            result.imports.insert(
                name.clone(),
                self.convert_component_entity_type(types, *ty)?,
            );
        }
        for (name, ty) in ty.exports.iter() {
            result.exports.insert(
                name.clone(),
                self.convert_component_entity_type(types, *ty)?,
            );
        }
        Ok(self.component_types.components.push(result))
    }

    pub(crate) fn convert_instance(
        &mut self,
        types: types::TypesRef<'_>,
        id: types::ComponentInstanceTypeId,
    ) -> Result<TypeComponentInstanceIndex> {
        assert_eq!(types.id(), self.module_types.validator_id());
        let ty = &types[id];
        let mut result = TypeComponentInstance::default();
        for (name, ty) in ty.exports.iter() {
            result.exports.insert(
                name.clone(),
                self.convert_component_entity_type(types, *ty)?,
            );
        }
        Ok(self.component_types.component_instances.push(result))
    }

    fn convert_module(
        &mut self,
        types: types::TypesRef<'_>,
        id: types::ComponentCoreModuleTypeId,
    ) -> Result<TypeModuleIndex> {
        assert_eq!(types.id(), self.module_types.validator_id());
        let ty = &types[id];
        let mut result = TypeModule::default();
        for ((module, field), ty) in ty.imports.iter() {
            result.imports.insert(
                (module.clone(), field.clone()),
                self.entity_type(types, ty)?,
            );
        }
        for (name, ty) in ty.exports.iter() {
            result
                .exports
                .insert(name.clone(), self.entity_type(types, ty)?);
        }
        Ok(self.component_types.modules.push(result))
    }

    fn entity_type(
        &mut self,
        types: types::TypesRef<'_>,
        ty: &types::EntityType,
    ) -> Result<EntityType> {
        assert_eq!(types.id(), self.module_types.validator_id());
        Ok(match ty {
            types::EntityType::Func(id) => EntityType::Function({
                let module = Module::default();
                self.module_types_builder_mut()
                    .intern_type(&module, types, *id)?
                    .into()
            }),
            types::EntityType::Table(ty) => EntityType::Table(self.convert_table_type(ty)),
            types::EntityType::Memory(ty) => EntityType::Memory(ty.clone().into()),
            types::EntityType::Global(ty) => EntityType::Global(self.convert_global_type(ty)),
            types::EntityType::Tag(_) => bail!("exceptions proposal not implemented"),
        })
    }

    fn defined_type(
        &mut self,
        types: types::TypesRef<'_>,
        id: types::ComponentDefinedTypeId,
    ) -> Result<InterfaceType> {
        assert_eq!(types.id(), self.module_types.validator_id());
        let ret = match &types[id] {
            types::ComponentDefinedType::Primitive(ty) => ty.into(),
            types::ComponentDefinedType::Record(e) => {
                InterfaceType::Record(self.record_type(types, e)?)
            }
            types::ComponentDefinedType::Variant(e) => {
                InterfaceType::Variant(self.variant_type(types, e)?)
            }
            types::ComponentDefinedType::List(e) => InterfaceType::List(self.list_type(types, e)?),
            types::ComponentDefinedType::Tuple(e) => {
                InterfaceType::Tuple(self.tuple_type(types, e)?)
            }
            types::ComponentDefinedType::Flags(e) => InterfaceType::Flags(self.flags_type(e)),
            types::ComponentDefinedType::Enum(e) => InterfaceType::Enum(self.enum_type(e)),
            types::ComponentDefinedType::Option(e) => {
                InterfaceType::Option(self.option_type(types, e)?)
            }
            types::ComponentDefinedType::Result { ok, err } => {
                InterfaceType::Result(self.result_type(types, ok, err)?)
            }
            types::ComponentDefinedType::Own(r) => {
                InterfaceType::Own(self.resource_id(r.resource()))
            }
            types::ComponentDefinedType::Borrow(r) => {
                InterfaceType::Borrow(self.resource_id(r.resource()))
            }
        };
        let info = self.type_information(&ret);
        if info.depth > MAX_TYPE_DEPTH {
            bail!("type nesting is too deep");
        }
        Ok(ret)
    }

    fn valtype(
        &mut self,
        types: types::TypesRef<'_>,
        ty: &types::ComponentValType,
    ) -> Result<InterfaceType> {
        assert_eq!(types.id(), self.module_types.validator_id());
        match ty {
            types::ComponentValType::Primitive(p) => Ok(p.into()),
            types::ComponentValType::Type(id) => self.defined_type(types, *id),
        }
    }

    fn record_type(
        &mut self,
        types: types::TypesRef<'_>,
        ty: &types::RecordType,
    ) -> Result<TypeRecordIndex> {
        assert_eq!(types.id(), self.module_types.validator_id());
        let fields = ty
            .fields
            .iter()
            .map(|(name, ty)| {
                Ok(RecordField {
                    name: name.to_string(),
                    ty: self.valtype(types, ty)?,
                })
            })
            .collect::<Result<Box<[_]>>>()?;
        let abi = CanonicalAbiInfo::record(
            fields
                .iter()
                .map(|field| self.component_types.canonical_abi(&field.ty)),
        );
        Ok(self.add_record_type(TypeRecord { fields, abi }))
    }

    fn variant_type(
        &mut self,
        types: types::TypesRef<'_>,
        ty: &types::VariantType,
    ) -> Result<TypeVariantIndex> {
        assert_eq!(types.id(), self.module_types.validator_id());
        let cases = ty
            .cases
            .iter()
            .map(|(name, case)| {
                // FIXME: need to implement `refines`, not sure what that
                // is at this time.
                if case.refines.is_some() {
                    bail!("refines is not supported at this time");
                }
                Ok((
                    name.to_string(),
                    match &case.ty.as_ref() {
                        Some(ty) => Some(self.valtype(types, ty)?),
                        None => None,
                    },
                ))
            })
            .collect::<Result<IndexMap<_, _>>>()?;
        let (info, abi) = VariantInfo::new(
            cases
                .iter()
                .map(|(_, c)| c.as_ref().map(|ty| self.component_types.canonical_abi(ty))),
        );
        Ok(self.add_variant_type(TypeVariant { cases, abi, info }))
    }

    fn tuple_type(
        &mut self,
        types: types::TypesRef<'_>,
        ty: &types::TupleType,
    ) -> Result<TypeTupleIndex> {
        assert_eq!(types.id(), self.module_types.validator_id());
        let types = ty
            .types
            .iter()
            .map(|ty| self.valtype(types, ty))
            .collect::<Result<Box<[_]>>>()?;
        Ok(self.new_tuple_type(types))
    }

    fn new_tuple_type(&mut self, types: Box<[InterfaceType]>) -> TypeTupleIndex {
        let abi = CanonicalAbiInfo::record(
            types
                .iter()
                .map(|ty| self.component_types.canonical_abi(ty)),
        );
        self.add_tuple_type(TypeTuple { types, abi })
    }

    fn flags_type(&mut self, flags: &wasmparser::map::IndexSet<KebabString>) -> TypeFlagsIndex {
        let flags = TypeFlags {
            names: flags.iter().map(|s| s.to_string()).collect(),
            abi: CanonicalAbiInfo::flags(flags.len()),
        };
        self.add_flags_type(flags)
    }

    fn enum_type(&mut self, variants: &wasmparser::map::IndexSet<KebabString>) -> TypeEnumIndex {
        let names = variants
            .iter()
            .map(|s| s.to_string())
            .collect::<IndexSet<_>>();
        let (info, abi) = VariantInfo::new(names.iter().map(|_| None));
        self.add_enum_type(TypeEnum { names, abi, info })
    }

    fn option_type(
        &mut self,
        types: types::TypesRef<'_>,
        ty: &types::ComponentValType,
    ) -> Result<TypeOptionIndex> {
        assert_eq!(types.id(), self.module_types.validator_id());
        let ty = self.valtype(types, ty)?;
        let (info, abi) = VariantInfo::new([None, Some(self.component_types.canonical_abi(&ty))]);
        Ok(self.add_option_type(TypeOption { ty, abi, info }))
    }

    fn result_type(
        &mut self,
        types: types::TypesRef<'_>,
        ok: &Option<types::ComponentValType>,
        err: &Option<types::ComponentValType>,
    ) -> Result<TypeResultIndex> {
        assert_eq!(types.id(), self.module_types.validator_id());
        let ok = match ok {
            Some(ty) => Some(self.valtype(types, ty)?),
            None => None,
        };
        let err = match err {
            Some(ty) => Some(self.valtype(types, ty)?),
            None => None,
        };
        let (info, abi) = VariantInfo::new([
            ok.as_ref().map(|t| self.component_types.canonical_abi(t)),
            err.as_ref().map(|t| self.component_types.canonical_abi(t)),
        ]);
        Ok(self.add_result_type(TypeResult { ok, err, abi, info }))
    }

    fn list_type(
        &mut self,
        types: types::TypesRef<'_>,
        ty: &types::ComponentValType,
    ) -> Result<TypeListIndex> {
        assert_eq!(types.id(), self.module_types.validator_id());
        let element = self.valtype(types, ty)?;
        Ok(self.add_list_type(TypeList { element }))
    }

    /// Converts a wasmparser `id`, which must point to a resource, to its
    /// corresponding `TypeResourceTableIndex`.
    pub fn resource_id(&mut self, id: types::ResourceId) -> TypeResourceTableIndex {
        self.resources.convert(id, &mut self.component_types)
    }

    /// Interns a new function type within this type information.
    pub fn add_func_type(&mut self, ty: TypeFunc) -> TypeFuncIndex {
        intern(&mut self.functions, &mut self.component_types.functions, ty)
    }

    /// Interns a new record type within this type information.
    pub fn add_record_type(&mut self, ty: TypeRecord) -> TypeRecordIndex {
        intern_and_fill_flat_types!(self, records, ty)
    }

    /// Interns a new flags type within this type information.
    pub fn add_flags_type(&mut self, ty: TypeFlags) -> TypeFlagsIndex {
        intern_and_fill_flat_types!(self, flags, ty)
    }

    /// Interns a new tuple type within this type information.
    pub fn add_tuple_type(&mut self, ty: TypeTuple) -> TypeTupleIndex {
        intern_and_fill_flat_types!(self, tuples, ty)
    }

    /// Interns a new variant type within this type information.
    pub fn add_variant_type(&mut self, ty: TypeVariant) -> TypeVariantIndex {
        intern_and_fill_flat_types!(self, variants, ty)
    }

    /// Interns a new enum type within this type information.
    pub fn add_enum_type(&mut self, ty: TypeEnum) -> TypeEnumIndex {
        intern_and_fill_flat_types!(self, enums, ty)
    }

    /// Interns a new option type within this type information.
    pub fn add_option_type(&mut self, ty: TypeOption) -> TypeOptionIndex {
        intern_and_fill_flat_types!(self, options, ty)
    }

    /// Interns a new result type within this type information.
    pub fn add_result_type(&mut self, ty: TypeResult) -> TypeResultIndex {
        intern_and_fill_flat_types!(self, results, ty)
    }

    /// Interns a new type within this type information.
    pub fn add_list_type(&mut self, ty: TypeList) -> TypeListIndex {
        intern_and_fill_flat_types!(self, lists, ty)
    }

    /// Returns the canonical ABI information about the specified type.
    pub fn canonical_abi(&self, ty: &InterfaceType) -> &CanonicalAbiInfo {
        self.component_types.canonical_abi(ty)
    }

    /// Returns the "flat types" for the given interface type used in the
    /// canonical ABI.
    ///
    /// Returns `None` if the type is too large to be represented via flat types
    /// in the canonical abi.
    pub fn flat_types(&self, ty: &InterfaceType) -> Option<FlatTypes<'_>> {
        self.type_information(ty).flat.as_flat_types()
    }

    /// Returns whether the type specified contains any borrowed resources
    /// within it.
    pub fn ty_contains_borrow_resource(&self, ty: &InterfaceType) -> bool {
        self.type_information(ty).has_borrow
    }

    fn type_information(&self, ty: &InterfaceType) -> &TypeInformation {
        match ty {
            InterfaceType::U8
            | InterfaceType::S8
            | InterfaceType::Bool
            | InterfaceType::U16
            | InterfaceType::S16
            | InterfaceType::U32
            | InterfaceType::S32
            | InterfaceType::Char
            | InterfaceType::Own(_) => {
                static INFO: TypeInformation = TypeInformation::primitive(FlatType::I32);
                &INFO
            }
            InterfaceType::Borrow(_) => {
                static INFO: TypeInformation = {
                    let mut info = TypeInformation::primitive(FlatType::I32);
                    info.has_borrow = true;
                    info
                };
                &INFO
            }
            InterfaceType::U64 | InterfaceType::S64 => {
                static INFO: TypeInformation = TypeInformation::primitive(FlatType::I64);
                &INFO
            }
            InterfaceType::Float32 => {
                static INFO: TypeInformation = TypeInformation::primitive(FlatType::F32);
                &INFO
            }
            InterfaceType::Float64 => {
                static INFO: TypeInformation = TypeInformation::primitive(FlatType::F64);
                &INFO
            }
            InterfaceType::String => {
                static INFO: TypeInformation = TypeInformation::string();
                &INFO
            }

            InterfaceType::List(i) => &self.type_info.lists[*i],
            InterfaceType::Record(i) => &self.type_info.records[*i],
            InterfaceType::Variant(i) => &self.type_info.variants[*i],
            InterfaceType::Tuple(i) => &self.type_info.tuples[*i],
            InterfaceType::Flags(i) => &self.type_info.flags[*i],
            InterfaceType::Enum(i) => &self.type_info.enums[*i],
            InterfaceType::Option(i) => &self.type_info.options[*i],
            InterfaceType::Result(i) => &self.type_info.results[*i],
        }
    }
}

impl TypeConvert for ComponentTypesBuilder {
    fn lookup_heap_type(&self, _index: wasmparser::UnpackedIndex) -> WasmHeapType {
        panic!("heap types are not supported yet")
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
    /// A component and its type.
    Component(TypeComponentIndex),
    /// An instance of a component.
    ComponentInstance(TypeComponentInstanceIndex),
    /// A component function, not to be confused with a core wasm function.
    ComponentFunc(TypeFuncIndex),
    /// An type in an interface.
    Interface(InterfaceType),
    /// A core wasm module and its type.
    Module(TypeModuleIndex),
    /// A core wasm function using only core wasm types.
    CoreFunc(ModuleInternedTypeIndex),
    /// A resource type which operates on the specified resource table.
    ///
    /// Note that different resource tables may point to the same underlying
    /// actual resource type, but that's a private detail.
    Resource(TypeResourceTableIndex),
}

impl TypeDef {
    /// A human readable description of what kind of type definition this is.
    pub fn desc(&self) -> &str {
        match self {
            TypeDef::Component(_) => "component",
            TypeDef::ComponentInstance(_) => "instance",
            TypeDef::ComponentFunc(_) => "function",
            TypeDef::Interface(_) => "type",
            TypeDef::Module(_) => "core module",
            TypeDef::CoreFunc(_) => "core function",
            TypeDef::Resource(_) => "resource",
        }
    }
}

// NB: Note that maps below are stored as an `IndexMap` now but the order
// typically does not matter. As a minor implementation detail we want the
// serialization of this type to always be deterministic and using `IndexMap`
// gets us that over using a `HashMap` for example.

/// The type of a module in the component model.
///
/// Note that this is not to be confused with `TypeComponent` below. This is
/// intended only for core wasm modules, not for components.
#[derive(Serialize, Deserialize, Default)]
pub struct TypeModule {
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
pub struct TypeComponent {
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
pub struct TypeComponentInstance {
    /// The list of exports that this component has along with their types.
    pub exports: IndexMap<String, TypeDef>,
}

/// A component function type in the component model.
#[derive(Serialize, Deserialize, Clone, Hash, Eq, PartialEq, Debug)]
pub struct TypeFunc {
    /// Parameters to the function represented as a tuple.
    pub params: TypeTupleIndex,
    /// Results of the function represented as a tuple.
    pub results: TypeTupleIndex,
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
    Record(TypeRecordIndex),
    Variant(TypeVariantIndex),
    List(TypeListIndex),
    Tuple(TypeTupleIndex),
    Flags(TypeFlagsIndex),
    Enum(TypeEnumIndex),
    Option(TypeOptionIndex),
    Result(TypeResultIndex),
    Own(TypeResourceTableIndex),
    Borrow(TypeResourceTableIndex),
}

impl From<&wasmparser::PrimitiveValType> for InterfaceType {
    fn from(ty: &wasmparser::PrimitiveValType) -> InterfaceType {
        match ty {
            wasmparser::PrimitiveValType::Bool => InterfaceType::Bool,
            wasmparser::PrimitiveValType::S8 => InterfaceType::S8,
            wasmparser::PrimitiveValType::U8 => InterfaceType::U8,
            wasmparser::PrimitiveValType::S16 => InterfaceType::S16,
            wasmparser::PrimitiveValType::U16 => InterfaceType::U16,
            wasmparser::PrimitiveValType::S32 => InterfaceType::S32,
            wasmparser::PrimitiveValType::U32 => InterfaceType::U32,
            wasmparser::PrimitiveValType::S64 => InterfaceType::S64,
            wasmparser::PrimitiveValType::U64 => InterfaceType::U64,
            wasmparser::PrimitiveValType::F32 => InterfaceType::Float32,
            wasmparser::PrimitiveValType::F64 => InterfaceType::Float64,
            wasmparser::PrimitiveValType::Char => InterfaceType::Char,
            wasmparser::PrimitiveValType::String => InterfaceType::String,
        }
    }
}

/// Bye information about a type in the canonical ABI, with metadata for both
/// memory32 and memory64-based types.
#[derive(Serialize, Deserialize, Clone, Hash, Eq, PartialEq, Debug)]
pub struct CanonicalAbiInfo {
    /// The byte-size of this type in a 32-bit memory.
    pub size32: u32,
    /// The byte-alignment of this type in a 32-bit memory.
    pub align32: u32,
    /// The byte-size of this type in a 64-bit memory.
    pub size64: u32,
    /// The byte-alignment of this type in a 64-bit memory.
    pub align64: u32,
    /// The number of types it takes to represents this type in the "flat"
    /// representation of the canonical abi where everything is passed as
    /// immediate arguments or results.
    ///
    /// If this is `None` then this type is not representable in the flat ABI
    /// because it is too large.
    pub flat_count: Option<u8>,
}

impl Default for CanonicalAbiInfo {
    fn default() -> CanonicalAbiInfo {
        CanonicalAbiInfo {
            size32: 0,
            align32: 1,
            size64: 0,
            align64: 1,
            flat_count: Some(0),
        }
    }
}

const fn align_to(a: u32, b: u32) -> u32 {
    assert!(b.is_power_of_two());
    (a + (b - 1)) & !(b - 1)
}

const fn max(a: u32, b: u32) -> u32 {
    if a > b {
        a
    } else {
        b
    }
}

impl CanonicalAbiInfo {
    /// ABI information for zero-sized types.
    const ZERO: CanonicalAbiInfo = CanonicalAbiInfo {
        size32: 0,
        align32: 1,
        size64: 0,
        align64: 1,
        flat_count: Some(0),
    };

    /// ABI information for one-byte scalars.
    pub const SCALAR1: CanonicalAbiInfo = CanonicalAbiInfo::scalar(1);
    /// ABI information for two-byte scalars.
    pub const SCALAR2: CanonicalAbiInfo = CanonicalAbiInfo::scalar(2);
    /// ABI information for four-byte scalars.
    pub const SCALAR4: CanonicalAbiInfo = CanonicalAbiInfo::scalar(4);
    /// ABI information for eight-byte scalars.
    pub const SCALAR8: CanonicalAbiInfo = CanonicalAbiInfo::scalar(8);

    const fn scalar(size: u32) -> CanonicalAbiInfo {
        CanonicalAbiInfo {
            size32: size,
            align32: size,
            size64: size,
            align64: size,
            flat_count: Some(1),
        }
    }

    /// ABI information for lists/strings which are "pointer pairs"
    pub const POINTER_PAIR: CanonicalAbiInfo = CanonicalAbiInfo {
        size32: 8,
        align32: 4,
        size64: 16,
        align64: 8,
        flat_count: Some(2),
    };

    /// Returns the abi for a record represented by the specified fields.
    pub fn record<'a>(fields: impl Iterator<Item = &'a CanonicalAbiInfo>) -> CanonicalAbiInfo {
        // NB: this is basically a duplicate copy of
        // `CanonicalAbiInfo::record_static` and the two should be kept in sync.

        let mut ret = CanonicalAbiInfo::default();
        for field in fields {
            ret.size32 = align_to(ret.size32, field.align32) + field.size32;
            ret.align32 = ret.align32.max(field.align32);
            ret.size64 = align_to(ret.size64, field.align64) + field.size64;
            ret.align64 = ret.align64.max(field.align64);
            ret.flat_count = add_flat(ret.flat_count, field.flat_count);
        }
        ret.size32 = align_to(ret.size32, ret.align32);
        ret.size64 = align_to(ret.size64, ret.align64);
        return ret;
    }

    /// Same as `CanonicalAbiInfo::record` but in a `const`-friendly context.
    pub const fn record_static(fields: &[CanonicalAbiInfo]) -> CanonicalAbiInfo {
        // NB: this is basically a duplicate copy of `CanonicalAbiInfo::record`
        // and the two should be kept in sync.

        let mut ret = CanonicalAbiInfo::ZERO;
        let mut i = 0;
        while i < fields.len() {
            let field = &fields[i];
            ret.size32 = align_to(ret.size32, field.align32) + field.size32;
            ret.align32 = max(ret.align32, field.align32);
            ret.size64 = align_to(ret.size64, field.align64) + field.size64;
            ret.align64 = max(ret.align64, field.align64);
            ret.flat_count = add_flat(ret.flat_count, field.flat_count);
            i += 1;
        }
        ret.size32 = align_to(ret.size32, ret.align32);
        ret.size64 = align_to(ret.size64, ret.align64);
        return ret;
    }

    /// Returns the delta from the current value of `offset` to align properly
    /// and read the next record field of type `abi` for 32-bit memories.
    pub fn next_field32(&self, offset: &mut u32) -> u32 {
        *offset = align_to(*offset, self.align32) + self.size32;
        *offset - self.size32
    }

    /// Same as `next_field32`, but bumps a usize pointer
    pub fn next_field32_size(&self, offset: &mut usize) -> usize {
        let cur = u32::try_from(*offset).unwrap();
        let cur = align_to(cur, self.align32) + self.size32;
        *offset = usize::try_from(cur).unwrap();
        usize::try_from(cur - self.size32).unwrap()
    }

    /// Returns the delta from the current value of `offset` to align properly
    /// and read the next record field of type `abi` for 64-bit memories.
    pub fn next_field64(&self, offset: &mut u32) -> u32 {
        *offset = align_to(*offset, self.align64) + self.size64;
        *offset - self.size64
    }

    /// Same as `next_field64`, but bumps a usize pointer
    pub fn next_field64_size(&self, offset: &mut usize) -> usize {
        let cur = u32::try_from(*offset).unwrap();
        let cur = align_to(cur, self.align64) + self.size64;
        *offset = usize::try_from(cur).unwrap();
        usize::try_from(cur - self.size64).unwrap()
    }

    /// Returns ABI information for a structure which contains `count` flags.
    pub const fn flags(count: usize) -> CanonicalAbiInfo {
        let (size, align, flat_count) = match FlagsSize::from_count(count) {
            FlagsSize::Size0 => (0, 1, 0),
            FlagsSize::Size1 => (1, 1, 1),
            FlagsSize::Size2 => (2, 2, 1),
            FlagsSize::Size4Plus(n) => ((n as u32) * 4, 4, n),
        };
        CanonicalAbiInfo {
            size32: size,
            align32: align,
            size64: size,
            align64: align,
            flat_count: Some(flat_count),
        }
    }

    fn variant<'a, I>(cases: I) -> CanonicalAbiInfo
    where
        I: IntoIterator<Item = Option<&'a CanonicalAbiInfo>>,
        I::IntoIter: ExactSizeIterator,
    {
        // NB: this is basically a duplicate definition of
        // `CanonicalAbiInfo::variant_static`, these should be kept in sync.

        let cases = cases.into_iter();
        let discrim_size = u32::from(DiscriminantSize::from_count(cases.len()).unwrap());
        let mut max_size32 = 0;
        let mut max_align32 = discrim_size;
        let mut max_size64 = 0;
        let mut max_align64 = discrim_size;
        let mut max_case_count = Some(0);
        for case in cases {
            if let Some(case) = case {
                max_size32 = max_size32.max(case.size32);
                max_align32 = max_align32.max(case.align32);
                max_size64 = max_size64.max(case.size64);
                max_align64 = max_align64.max(case.align64);
                max_case_count = max_flat(max_case_count, case.flat_count);
            }
        }
        CanonicalAbiInfo {
            size32: align_to(
                align_to(discrim_size, max_align32) + max_size32,
                max_align32,
            ),
            align32: max_align32,
            size64: align_to(
                align_to(discrim_size, max_align64) + max_size64,
                max_align64,
            ),
            align64: max_align64,
            flat_count: add_flat(max_case_count, Some(1)),
        }
    }

    /// Same as `CanonicalAbiInfo::variant` but `const`-safe
    pub const fn variant_static(cases: &[Option<CanonicalAbiInfo>]) -> CanonicalAbiInfo {
        // NB: this is basically a duplicate definition of
        // `CanonicalAbiInfo::variant`, these should be kept in sync.

        let discrim_size = match DiscriminantSize::from_count(cases.len()) {
            Some(size) => size.byte_size(),
            None => unreachable!(),
        };
        let mut max_size32 = 0;
        let mut max_align32 = discrim_size;
        let mut max_size64 = 0;
        let mut max_align64 = discrim_size;
        let mut max_case_count = Some(0);
        let mut i = 0;
        while i < cases.len() {
            let case = &cases[i];
            if let Some(case) = case {
                max_size32 = max(max_size32, case.size32);
                max_align32 = max(max_align32, case.align32);
                max_size64 = max(max_size64, case.size64);
                max_align64 = max(max_align64, case.align64);
                max_case_count = max_flat(max_case_count, case.flat_count);
            }
            i += 1;
        }
        CanonicalAbiInfo {
            size32: align_to(
                align_to(discrim_size, max_align32) + max_size32,
                max_align32,
            ),
            align32: max_align32,
            size64: align_to(
                align_to(discrim_size, max_align64) + max_size64,
                max_align64,
            ),
            align64: max_align64,
            flat_count: add_flat(max_case_count, Some(1)),
        }
    }

    /// Returns the flat count of this ABI information so long as the count
    /// doesn't exceed the `max` specified.
    pub fn flat_count(&self, max: usize) -> Option<usize> {
        let flat = usize::from(self.flat_count?);
        if flat > max {
            None
        } else {
            Some(flat)
        }
    }
}

/// ABI information about the representation of a variant.
#[derive(Serialize, Deserialize, Clone, Hash, Eq, PartialEq, Debug)]
pub struct VariantInfo {
    /// The size of the discriminant used.
    #[serde(with = "serde_discrim_size")]
    pub size: DiscriminantSize,
    /// The offset of the payload from the start of the variant in 32-bit
    /// memories.
    pub payload_offset32: u32,
    /// The offset of the payload from the start of the variant in 64-bit
    /// memories.
    pub payload_offset64: u32,
}

impl VariantInfo {
    /// Returns the abi information for a variant represented by the specified
    /// cases.
    pub fn new<'a, I>(cases: I) -> (VariantInfo, CanonicalAbiInfo)
    where
        I: IntoIterator<Item = Option<&'a CanonicalAbiInfo>>,
        I::IntoIter: ExactSizeIterator,
    {
        let cases = cases.into_iter();
        let size = DiscriminantSize::from_count(cases.len()).unwrap();
        let abi = CanonicalAbiInfo::variant(cases);
        (
            VariantInfo {
                size,
                payload_offset32: align_to(u32::from(size), abi.align32),
                payload_offset64: align_to(u32::from(size), abi.align64),
            },
            abi,
        )
    }
    /// TODO
    pub const fn new_static(cases: &[Option<CanonicalAbiInfo>]) -> VariantInfo {
        let size = match DiscriminantSize::from_count(cases.len()) {
            Some(size) => size,
            None => unreachable!(),
        };
        let abi = CanonicalAbiInfo::variant_static(cases);
        VariantInfo {
            size,
            payload_offset32: align_to(size.byte_size(), abi.align32),
            payload_offset64: align_to(size.byte_size(), abi.align64),
        }
    }
}

mod serde_discrim_size {
    use super::DiscriminantSize;
    use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(disc: &DiscriminantSize, ser: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        u32::from(*disc).serialize(ser)
    }

    pub fn deserialize<'de, D>(deser: D) -> Result<DiscriminantSize, D::Error>
    where
        D: Deserializer<'de>,
    {
        match u32::deserialize(deser)? {
            1 => Ok(DiscriminantSize::Size1),
            2 => Ok(DiscriminantSize::Size2),
            4 => Ok(DiscriminantSize::Size4),
            _ => Err(D::Error::custom("invalid discriminant size")),
        }
    }
}

/// Shape of a "record" type in interface types.
///
/// This is equivalent to a `struct` in Rust.
#[derive(Serialize, Deserialize, Clone, Hash, Eq, PartialEq, Debug)]
pub struct TypeRecord {
    /// The fields that are contained within this struct type.
    pub fields: Box<[RecordField]>,
    /// Byte information about this type in the canonical ABI.
    pub abi: CanonicalAbiInfo,
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
#[derive(Serialize, Deserialize, Clone, Eq, PartialEq, Debug)]
pub struct TypeVariant {
    /// The list of cases that this variant can take.
    pub cases: IndexMap<String, Option<InterfaceType>>,
    /// Byte information about this type in the canonical ABI.
    pub abi: CanonicalAbiInfo,
    /// Byte information about this variant type.
    pub info: VariantInfo,
}

impl Hash for TypeVariant {
    fn hash<H: Hasher>(&self, h: &mut H) {
        let TypeVariant { cases, abi, info } = self;
        cases.as_slice().hash(h);
        abi.hash(h);
        info.hash(h);
    }
}

/// Shape of a "tuple" type in interface types.
///
/// This is largely the same as a tuple in Rust, basically a record with
/// unnamed fields.
#[derive(Serialize, Deserialize, Clone, Hash, Eq, PartialEq, Debug)]
pub struct TypeTuple {
    /// The types that are contained within this tuple.
    pub types: Box<[InterfaceType]>,
    /// Byte information about this type in the canonical ABI.
    pub abi: CanonicalAbiInfo,
}

/// Shape of a "flags" type in interface types.
///
/// This can be thought of as a record-of-bools, although the representation is
/// more efficient as bitflags.
#[derive(Serialize, Deserialize, Clone, Eq, PartialEq, Debug)]
pub struct TypeFlags {
    /// The names of all flags, all of which are unique.
    pub names: IndexSet<String>,
    /// Byte information about this type in the canonical ABI.
    pub abi: CanonicalAbiInfo,
}

impl Hash for TypeFlags {
    fn hash<H: Hasher>(&self, h: &mut H) {
        let TypeFlags { names, abi } = self;
        names.as_slice().hash(h);
        abi.hash(h);
    }
}

/// Shape of an "enum" type in interface types, not to be confused with a Rust
/// `enum` type.
///
/// In interface types enums are simply a bag of names, and can be seen as a
/// variant where all payloads are `Unit`.
#[derive(Serialize, Deserialize, Clone, Eq, PartialEq, Debug)]
pub struct TypeEnum {
    /// The names of this enum, all of which are unique.
    pub names: IndexSet<String>,
    /// Byte information about this type in the canonical ABI.
    pub abi: CanonicalAbiInfo,
    /// Byte information about this variant type.
    pub info: VariantInfo,
}

impl Hash for TypeEnum {
    fn hash<H: Hasher>(&self, h: &mut H) {
        let TypeEnum { names, abi, info } = self;
        names.as_slice().hash(h);
        abi.hash(h);
        info.hash(h);
    }
}

/// Shape of an "option" interface type.
#[derive(Serialize, Deserialize, Clone, Hash, Eq, PartialEq, Debug)]
pub struct TypeOption {
    /// The `T` in `Result<T, E>`
    pub ty: InterfaceType,
    /// Byte information about this type in the canonical ABI.
    pub abi: CanonicalAbiInfo,
    /// Byte information about this variant type.
    pub info: VariantInfo,
}

/// Shape of a "result" interface type.
#[derive(Serialize, Deserialize, Clone, Hash, Eq, PartialEq, Debug)]
pub struct TypeResult {
    /// The `T` in `Result<T, E>`
    pub ok: Option<InterfaceType>,
    /// The `E` in `Result<T, E>`
    pub err: Option<InterfaceType>,
    /// Byte information about this type in the canonical ABI.
    pub abi: CanonicalAbiInfo,
    /// Byte information about this variant type.
    pub info: VariantInfo,
}

/// Metadata about a resource table added to a component.
#[derive(Serialize, Deserialize, Clone, Hash, Eq, PartialEq, Debug)]
pub struct TypeResourceTable {
    /// The original resource that this table contains.
    ///
    /// This is used when destroying resources within this table since this
    /// original definition will know how to execute destructors.
    pub ty: ResourceIndex,

    /// The component instance that contains this resource table.
    pub instance: RuntimeComponentInstanceIndex,
}

/// Shape of a "list" interface type.
#[derive(Serialize, Deserialize, Clone, Hash, Eq, PartialEq, Debug)]
pub struct TypeList {
    /// The element type of the list.
    pub element: InterfaceType,
}

const MAX_FLAT_TYPES: usize = if MAX_FLAT_PARAMS > MAX_FLAT_RESULTS {
    MAX_FLAT_PARAMS
} else {
    MAX_FLAT_RESULTS
};

const fn add_flat(a: Option<u8>, b: Option<u8>) -> Option<u8> {
    const MAX: u8 = MAX_FLAT_TYPES as u8;
    let sum = match (a, b) {
        (Some(a), Some(b)) => match a.checked_add(b) {
            Some(c) => c,
            None => return None,
        },
        _ => return None,
    };
    if sum > MAX {
        None
    } else {
        Some(sum)
    }
}

const fn max_flat(a: Option<u8>, b: Option<u8>) -> Option<u8> {
    match (a, b) {
        (Some(a), Some(b)) => {
            if a > b {
                Some(a)
            } else {
                Some(b)
            }
        }
        _ => None,
    }
}

/// Flat representation of a type in just core wasm types.
pub struct FlatTypes<'a> {
    /// The flat representation of this type in 32-bit memories.
    pub memory32: &'a [FlatType],
    /// The flat representation of this type in 64-bit memories.
    pub memory64: &'a [FlatType],
}

#[allow(missing_docs)]
impl FlatTypes<'_> {
    /// Returns the number of flat types used to represent this type.
    ///
    /// Note that this length is the same regardless to the size of memory.
    pub fn len(&self) -> usize {
        assert_eq!(self.memory32.len(), self.memory64.len());
        self.memory32.len()
    }
}

// Note that this is intentionally duplicated here to keep the size to 1 byte
// irregardless to changes in the core wasm type system since this will only
// ever use integers/floats for the forseeable future.
#[derive(PartialEq, Eq, Copy, Clone)]
#[allow(missing_docs)]
pub enum FlatType {
    I32,
    I64,
    F32,
    F64,
}

struct FlatTypesStorage {
    // This could be represented as `Vec<FlatType>` but on 64-bit architectures
    // that's 24 bytes. Otherwise `FlatType` is 1 byte large and
    // `MAX_FLAT_TYPES` is 16, so it should ideally be more space-efficient to
    // use a flat array instead of a heap-based vector.
    memory32: [FlatType; MAX_FLAT_TYPES],
    memory64: [FlatType; MAX_FLAT_TYPES],

    // Tracks the number of flat types pushed into this storage. If this is
    // `MAX_FLAT_TYPES + 1` then this storage represents an un-reprsentable
    // type in flat types.
    len: u8,
}

impl FlatTypesStorage {
    const fn new() -> FlatTypesStorage {
        FlatTypesStorage {
            memory32: [FlatType::I32; MAX_FLAT_TYPES],
            memory64: [FlatType::I32; MAX_FLAT_TYPES],
            len: 0,
        }
    }

    fn as_flat_types(&self) -> Option<FlatTypes<'_>> {
        let len = usize::from(self.len);
        if len > MAX_FLAT_TYPES {
            assert_eq!(len, MAX_FLAT_TYPES + 1);
            None
        } else {
            Some(FlatTypes {
                memory32: &self.memory32[..len],
                memory64: &self.memory64[..len],
            })
        }
    }

    /// Pushes a new flat type into this list using `t32` for 32-bit memories
    /// and `t64` for 64-bit memories.
    ///
    /// Returns whether the type was actually pushed or whether this list of
    /// flat types just exceeded the maximum meaning that it is now
    /// unrepresentable with a flat list of types.
    fn push(&mut self, t32: FlatType, t64: FlatType) -> bool {
        let len = usize::from(self.len);
        if len < MAX_FLAT_TYPES {
            self.memory32[len] = t32;
            self.memory64[len] = t64;
            self.len += 1;
            true
        } else {
            // If this was the first one to go over then flag the length as
            // being incompatible with a flat representation.
            if len == MAX_FLAT_TYPES {
                self.len += 1;
            }
            false
        }
    }
}

impl FlatType {
    fn join(&mut self, other: FlatType) {
        if *self == other {
            return;
        }
        *self = match (*self, other) {
            (FlatType::I32, FlatType::F32) | (FlatType::F32, FlatType::I32) => FlatType::I32,
            _ => FlatType::I64,
        };
    }
}

#[derive(Default)]
struct TypeInformationCache {
    records: PrimaryMap<TypeRecordIndex, TypeInformation>,
    variants: PrimaryMap<TypeVariantIndex, TypeInformation>,
    tuples: PrimaryMap<TypeTupleIndex, TypeInformation>,
    enums: PrimaryMap<TypeEnumIndex, TypeInformation>,
    flags: PrimaryMap<TypeFlagsIndex, TypeInformation>,
    options: PrimaryMap<TypeOptionIndex, TypeInformation>,
    results: PrimaryMap<TypeResultIndex, TypeInformation>,
    lists: PrimaryMap<TypeListIndex, TypeInformation>,
}

struct TypeInformation {
    depth: u32,
    flat: FlatTypesStorage,
    has_borrow: bool,
}

impl TypeInformation {
    const fn new() -> TypeInformation {
        TypeInformation {
            depth: 0,
            flat: FlatTypesStorage::new(),
            has_borrow: false,
        }
    }

    const fn primitive(flat: FlatType) -> TypeInformation {
        let mut info = TypeInformation::new();
        info.depth = 1;
        info.flat.memory32[0] = flat;
        info.flat.memory64[0] = flat;
        info.flat.len = 1;
        info
    }

    const fn string() -> TypeInformation {
        let mut info = TypeInformation::new();
        info.depth = 1;
        info.flat.memory32[0] = FlatType::I32;
        info.flat.memory32[1] = FlatType::I32;
        info.flat.memory64[0] = FlatType::I64;
        info.flat.memory64[1] = FlatType::I64;
        info.flat.len = 2;
        info
    }

    /// Builds up all flat types internally using the specified representation
    /// for all of the component fields of the record.
    fn build_record<'a>(&mut self, types: impl Iterator<Item = &'a TypeInformation>) {
        self.depth = 1;
        for info in types {
            self.depth = self.depth.max(1 + info.depth);
            self.has_borrow = self.has_borrow || info.has_borrow;
            match info.flat.as_flat_types() {
                Some(types) => {
                    for (t32, t64) in types.memory32.iter().zip(types.memory64) {
                        if !self.flat.push(*t32, *t64) {
                            break;
                        }
                    }
                }
                None => {
                    self.flat.len = u8::try_from(MAX_FLAT_TYPES + 1).unwrap();
                }
            }
        }
    }

    /// Builds up the flat types used to represent a `variant` which notably
    /// handles "join"ing types together so each case is representable as a
    /// single flat list of types.
    ///
    /// The iterator item is:
    ///
    /// * `None` - no payload for this case
    /// * `Some(None)` - this case has a payload but can't be represented with
    ///   flat types
    /// * `Some(Some(types))` - this case has a payload and is represented with
    ///   the types specified in the flat representation.
    fn build_variant<'a, I>(&mut self, cases: I)
    where
        I: IntoIterator<Item = Option<&'a TypeInformation>>,
    {
        let cases = cases.into_iter();
        self.flat.push(FlatType::I32, FlatType::I32);
        self.depth = 1;

        for info in cases {
            let info = match info {
                Some(info) => info,
                // If this case doesn't have a payload then it doesn't change
                // the depth/flat representation
                None => continue,
            };
            self.depth = self.depth.max(1 + info.depth);
            self.has_borrow = self.has_borrow || info.has_borrow;

            // If this variant is already unrepresentable in a flat
            // representation then this can be skipped.
            if usize::from(self.flat.len) > MAX_FLAT_TYPES {
                continue;
            }

            let types = match info.flat.as_flat_types() {
                Some(types) => types,
                // If this case isn't representable with a flat list of types
                // then this variant also isn't representable.
                None => {
                    self.flat.len = u8::try_from(MAX_FLAT_TYPES + 1).unwrap();
                    continue;
                }
            };
            // If the case used all of the flat types then the discriminant
            // added for this variant means that this variant is no longer
            // representable.
            if types.memory32.len() >= MAX_FLAT_TYPES {
                self.flat.len = u8::try_from(MAX_FLAT_TYPES + 1).unwrap();
                continue;
            }
            let dst = self
                .flat
                .memory32
                .iter_mut()
                .zip(&mut self.flat.memory64)
                .skip(1);
            for (i, ((t32, t64), (dst32, dst64))) in types
                .memory32
                .iter()
                .zip(types.memory64)
                .zip(dst)
                .enumerate()
            {
                if i + 1 < usize::from(self.flat.len) {
                    // If this index hs already been set by some previous case
                    // then the types are joined together.
                    dst32.join(*t32);
                    dst64.join(*t64);
                } else {
                    // Otherwise if this is the first time that the
                    // representation has gotten this large then the destination
                    // is simply whatever the type is. The length is also
                    // increased here to indicate this.
                    self.flat.len += 1;
                    *dst32 = *t32;
                    *dst64 = *t64;
                }
            }
        }
    }

    fn records(&mut self, types: &ComponentTypesBuilder, ty: &TypeRecord) {
        self.build_record(ty.fields.iter().map(|f| types.type_information(&f.ty)));
    }

    fn tuples(&mut self, types: &ComponentTypesBuilder, ty: &TypeTuple) {
        self.build_record(ty.types.iter().map(|t| types.type_information(t)));
    }

    fn enums(&mut self, _types: &ComponentTypesBuilder, _ty: &TypeEnum) {
        self.depth = 1;
        self.flat.push(FlatType::I32, FlatType::I32);
    }

    fn flags(&mut self, _types: &ComponentTypesBuilder, ty: &TypeFlags) {
        self.depth = 1;
        match FlagsSize::from_count(ty.names.len()) {
            FlagsSize::Size0 => {}
            FlagsSize::Size1 | FlagsSize::Size2 => {
                self.flat.push(FlatType::I32, FlatType::I32);
            }
            FlagsSize::Size4Plus(n) => {
                for _ in 0..n {
                    self.flat.push(FlatType::I32, FlatType::I32);
                }
            }
        }
    }

    fn variants(&mut self, types: &ComponentTypesBuilder, ty: &TypeVariant) {
        self.build_variant(
            ty.cases
                .iter()
                .map(|(_, c)| c.as_ref().map(|ty| types.type_information(ty))),
        )
    }

    fn results(&mut self, types: &ComponentTypesBuilder, ty: &TypeResult) {
        self.build_variant([
            ty.ok.as_ref().map(|ty| types.type_information(ty)),
            ty.err.as_ref().map(|ty| types.type_information(ty)),
        ])
    }

    fn options(&mut self, types: &ComponentTypesBuilder, ty: &TypeOption) {
        self.build_variant([None, Some(types.type_information(&ty.ty))]);
    }

    fn lists(&mut self, types: &ComponentTypesBuilder, ty: &TypeList) {
        *self = TypeInformation::string();
        let info = types.type_information(&ty.element);
        self.depth += info.depth;
        self.has_borrow = info.has_borrow;
    }
}
