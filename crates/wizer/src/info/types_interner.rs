use std::{
    borrow::Cow,
    collections::{BTreeMap, HashMap},
    convert::TryFrom,
    rc::Rc,
};

/// A de-duplicated set of type definitions.
///
/// We insert new entries via hash consing, to de-duplicate entries.
///
/// This is shared across all modules in a module linking bundle.
///
/// We assign and track a unique index for each type we insert. These end up
/// being the indices of each type in the root Wasm module. All nested modules
/// refer to these types, and pull them into their nested types index space, via
/// outer type aliases.
#[derive(Default)]
pub struct TypesInterner<'a> {
    /// The interned types.
    types: Vec<Rc<Type<'a>>>,

    /// An map from a type to its index in `self.types`.
    type_to_index: HashMap<Rc<Type<'a>>, u32>,
}

/// An interned Wasm type definition.
#[derive(PartialEq, Eq, Hash)]
pub enum Type<'a> {
    Func(wasmparser::FuncType),
    Instance(InstanceType<'a>),
    Module(ModuleType<'a>),
}

impl Type<'_> {
    pub fn is_instance(&self) -> bool {
        matches!(self, Type::Instance(_))
    }

    pub fn is_func(&self) -> bool {
        matches!(self, Type::Func(_))
    }
}

/// An interned Wasm instance type.
#[derive(PartialEq, Eq, Hash)]
pub struct InstanceType<'a> {
    pub exports: BTreeMap<Cow<'a, str>, EntityType>,
}

/// An interned type for some kind of Wasm entity.
#[derive(PartialEq, Eq, Hash)]
pub enum EntityType {
    Function(TypeId),
    Table(wasmparser::TableType),
    Memory(wasmparser::MemoryType),
    Global(wasmparser::GlobalType),
    Module(TypeId),
    Instance(TypeId),
}

/// An interned Wasm module type.
#[derive(PartialEq, Eq, Hash)]
pub struct ModuleType<'a> {
    pub imports: BTreeMap<(Cow<'a, str>, Option<Cow<'a, str>>), EntityType>,
    pub exports: BTreeMap<Cow<'a, str>, EntityType>,
}

/// An id of a type in a `TypesInterner` type set.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TypeId {
    index: u32,
}

impl TypeId {
    /// Get the index of this type inside its `TypesInterner`.
    ///
    /// This index is *not* the same as the index within a particular module's
    /// index space, it is within the `TypesInterner`'s index space (except for
    /// the eventual umbrella module, whose types index space matches the
    /// interner's index space).
    pub fn index(self) -> u32 {
        self.index
    }
}

impl<'a> TypesInterner<'a> {
    /// Iterate over the types defined in this type set and their index.
    pub fn iter<'b>(&'b self) -> impl Iterator<Item = (u32, &'b Type<'a>)> + 'b {
        assert!((self.types.len() as u64) < (u32::MAX as u64));
        self.types
            .iter()
            .enumerate()
            .map(|(idx, ty)| (u32::try_from(idx).unwrap(), &**ty))
    }

    /// Get a type by id.
    pub fn get(&self, id: TypeId) -> &Type<'a> {
        &*self.types[usize::try_from(id.index).unwrap()]
    }

    /// Intern a `wasmparser` type into this type set and get its id.
    ///
    /// The provided `types_space` must be a slice of the defining module's
    /// types index space.
    ///
    /// If the type has already been inserted and assigned an id before, then
    /// that entry and its id are reused.
    pub fn insert_wasmparser(
        &mut self,
        ty: wasmparser::TypeDef<'a>,
        types_space: &[TypeId],
    ) -> TypeId {
        match ty {
            wasmparser::TypeDef::Func(func_ty) => self.insert(Type::Func(func_ty)),
            wasmparser::TypeDef::Instance(inst_ty) => {
                self.insert_wasmparser_instance_type(inst_ty, types_space)
            }
            wasmparser::TypeDef::Module(module_ty) => {
                self.insert_wasmparser_module_type(module_ty, types_space)
            }
        }
    }

    /// Insert a new type into this type set and get its id.
    ///
    /// If the type has already been inserted and assigned an id before, then
    /// that entry and its id are reused.
    pub fn insert(&mut self, ty: Type<'a>) -> TypeId {
        if let Some(index) = self.type_to_index.get(&ty).copied() {
            return TypeId { index };
        }

        let index = u32::try_from(self.types.len()).unwrap();
        let ty = Rc::new(ty);
        self.type_to_index.insert(ty.clone(), index);
        self.types.push(ty);
        TypeId { index }
    }

    /// Convert a `wasmparser::ImportSectionEntryType` into an interned
    /// `EntityType`.
    ///
    /// The provided `types_space` must be a slice of the defining module's
    /// types index space.
    pub fn entity_type(
        &self,
        ty: wasmparser::ImportSectionEntryType,
        types_space: &[TypeId],
    ) -> EntityType {
        match ty {
            wasmparser::ImportSectionEntryType::Function(idx) => {
                EntityType::Function(types_space[usize::try_from(idx).unwrap()])
            }
            wasmparser::ImportSectionEntryType::Table(ty) => EntityType::Table(ty),
            wasmparser::ImportSectionEntryType::Memory(ty) => EntityType::Memory(ty),
            wasmparser::ImportSectionEntryType::Global(ty) => EntityType::Global(ty),
            wasmparser::ImportSectionEntryType::Module(idx) => {
                EntityType::Module(types_space[usize::try_from(idx).unwrap()])
            }
            wasmparser::ImportSectionEntryType::Instance(idx) => {
                EntityType::Instance(types_space[usize::try_from(idx).unwrap()])
            }
            wasmparser::ImportSectionEntryType::Tag(_) => unreachable!(),
        }
    }

    fn insert_wasmparser_instance_type(
        &mut self,
        inst_ty: wasmparser::InstanceType<'a>,
        types_space: &[TypeId],
    ) -> TypeId {
        self.insert(Type::Instance(InstanceType {
            exports: inst_ty
                .exports
                .iter()
                .map(|exp| (exp.name.into(), self.entity_type(exp.ty, types_space)))
                .collect(),
        }))
    }

    fn insert_wasmparser_module_type(
        &mut self,
        module_ty: wasmparser::ModuleType<'a>,
        types_space: &[TypeId],
    ) -> TypeId {
        self.insert(Type::Module(ModuleType {
            imports: module_ty
                .imports
                .iter()
                .map(|imp| {
                    (
                        (imp.module.into(), imp.field.map(Cow::from)),
                        self.entity_type(imp.ty, types_space),
                    )
                })
                .collect(),
            exports: module_ty
                .exports
                .iter()
                .map(|exp| (exp.name.into(), self.entity_type(exp.ty, types_space)))
                .collect(),
        }))
    }
}
