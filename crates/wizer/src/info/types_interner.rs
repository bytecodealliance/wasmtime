use std::{collections::HashMap, convert::TryFrom, rc::Rc};

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
pub struct TypesInterner {
    /// The interned types.
    types: Vec<Rc<Type>>,

    /// An map from a type to its index in `self.types`.
    type_to_index: HashMap<Rc<Type>, u32>,
}

/// An interned Wasm type definition.
#[derive(PartialEq, Eq, Hash)]
pub enum Type {
    Func(wasmparser::FuncType),
}

impl Type {
    pub fn is_func(&self) -> bool {
        matches!(self, Type::Func(_))
    }
}

/// An interned type for some kind of Wasm entity.
#[derive(PartialEq, Eq, Hash)]
pub enum EntityType {
    Function(TypeId),
    Table(wasmparser::TableType),
    Memory(wasmparser::MemoryType),
    Global(wasmparser::GlobalType),
}

/// An id of a type in a `TypesInterner` type set.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TypeId {
    index: u32,
}

impl TypesInterner {
    /// Get a type by id.
    pub fn get(&self, id: TypeId) -> &Type {
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
        ty: wasmparser::CompositeType,
        _types_space: &[TypeId],
    ) -> TypeId {
        match ty {
            wasmparser::CompositeType::Func(func_ty) => self.insert(Type::Func(func_ty)),
            wasmparser::CompositeType::Array(_) => todo!(),
            wasmparser::CompositeType::Struct(_) => todo!(),
        }
    }

    /// Insert a new type into this type set and get its id.
    ///
    /// If the type has already been inserted and assigned an id before, then
    /// that entry and its id are reused.
    pub fn insert(&mut self, ty: Type) -> TypeId {
        if let Some(index) = self.type_to_index.get(&ty).copied() {
            return TypeId { index };
        }

        let index = u32::try_from(self.types.len()).unwrap();
        let ty = Rc::new(ty);
        self.type_to_index.insert(ty.clone(), index);
        self.types.push(ty);
        TypeId { index }
    }

    /// Convert a `wasmparser::EntityType` into an interned
    /// `EntityType`.
    ///
    /// The provided `types_space` must be a slice of the defining module's
    /// types index space.
    pub fn entity_type(&self, ty: wasmparser::TypeRef, types_space: &[TypeId]) -> EntityType {
        match ty {
            wasmparser::TypeRef::Func(idx) => {
                EntityType::Function(types_space[usize::try_from(idx).unwrap()])
            }
            wasmparser::TypeRef::Table(ty) => EntityType::Table(ty),
            wasmparser::TypeRef::Memory(ty) => EntityType::Memory(ty),
            wasmparser::TypeRef::Global(ty) => EntityType::Global(ty),
            wasmparser::TypeRef::Tag(_) => unreachable!(),
        }
    }
}
