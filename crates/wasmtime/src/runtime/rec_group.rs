//! Embedder API for defining recursion groups of Wasm types.
//!
//! The one-off constructors `StructType::new`, `ArrayType::new`, and
//! `FuncType::new` cannot describe types that reference themselves or each
//! other, because the constructors require all referenced types to already
//! exist. [`RecGroupBuilder`] lifts that restriction: you *declare* a type to
//! get a [`PendingType`] handle, use that handle as a forward reference while
//! defining other types, and the whole group is validated and registered
//! together when [`RecGroupBuilder::build`] is called.
//!
//! Each declared type is defined via a builder (e.g.
//! [`RecGroupBuilder::define_struct`]) and committed by calling `build` on that
//! builder.
//!
//! Already-registered types (and abstract heap types) are used directly via the
//! normal [`FieldType`]/[`ValType`] APIs; the `forward_ref_*` builder methods
//! are only for references to other types being defined in the same group.
//!
//! The order in which types are declared is significant; see
//! [`RecGroupBuilder`'s docs][RecGroupBuilder#declaration-order-is-significant].
//!
//! ```
//! # use wasmtime::*;
//! # fn main() -> Result<()> {
//! let engine = Engine::default();
//!
//! // Two mutually-recursive struct types.
//! let mut builder = RecGroupBuilder::new(&engine);
//! let a = builder.declare_struct();
//! let b = builder.declare_struct();
//! builder.define_struct(a).forward_ref_field(b).nullable(true).build().build();
//! builder.define_struct(b).forward_ref_field(a).nullable(false).build().build();
//! let group = builder.build()?;
//!
//! let a: StructType = group.get_struct(a).unwrap();
//! let b: StructType = group.get_struct(b).unwrap();
//! assert!(a.field(0).unwrap().element_type().is_val_type());
//! # Ok(())
//! # }
//! ```

use crate::prelude::*;
use crate::type_registry::RegisteredType;
use crate::{
    ArrayType, Engine, FieldType, Finality, FuncType, HeapType, Mutability, StructType, ValType,
};
use core::sync::atomic::{AtomicUsize, Ordering::Relaxed};
use wasmtime_environ::{
    EngineOrModuleTypeIndex, EntityRef, ModuleInternedTypeIndex, WasmArrayType,
    WasmCompositeInnerType, WasmCompositeType, WasmFieldType, WasmFuncType, WasmHeapType,
    WasmRefType, WasmStorageType, WasmStructType, WasmSubType, WasmValType,
};

/// A process-global counter used to give each [`RecGroupBuilder`] a distinct id
/// so that handles from one builder cannot be accidentally used with another.
static NEXT_BUILDER_ID: AtomicUsize = AtomicUsize::new(0);

fn next_builder_id() -> usize {
    NEXT_BUILDER_ID.fetch_add(1, Relaxed)
}

/// The 0-based index of a member within the rec group being built, expressed as
/// the module-level type reference that `register_rec_group` expects for
/// intra-group references.
fn module_index(index: u32) -> EngineOrModuleTypeIndex {
    EngineOrModuleTypeIndex::Module(ModuleInternedTypeIndex::new(index as usize))
}

/// A handle to a type being defined in a [`RecGroupBuilder`].
///
/// Obtained from [`RecGroupBuilder::declare_struct`] and friends. It is used
/// both to define the type (via [`RecGroupBuilder::define_struct`] and friends)
/// and to forward-reference it from other types in the same group (via the
/// `forward_ref_*` builder methods).
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct PendingType {
    builder_id: usize,
    index: u32,
}

/// A builder for defining a recursion group of Wasm types, including types that
/// reference themselves or each other.
///
/// See the [module-level documentation](crate::RecGroupBuilder) for an overview
/// and examples.
///
/// # Declaration order is significant
///
/// The order in which types are *declared* (via
/// [`declare_struct`][Self::declare_struct] and friends) fixes their order
/// within the rec group, and that order is semantically visible: two rec groups
/// containing the same types in a different order are *distinct* types, as are
/// their corresponding members. For example, `$f != $f'` and `$s != $s'` in the
/// following, even though each group defines one `func` and one `struct`:
///
/// ```wat
/// (rec (type $f (func))
///      (type $s (struct)))
///
/// (rec (type $s' (struct))
///      (type $f' (func)))
/// ```
///
/// It is the order of the `declare_*` calls that determines this, not the order
/// in which the types are subsequently defined and built.
pub struct RecGroupBuilder {
    engine: Engine,
    builder_id: usize,
    /// The first error encountered while adding types to the builder (for
    /// example, referencing a type from a different engine). Surfaced by
    /// [`build`][Self::build]; the chaining builder methods stay infallible.
    error: Option<Error>,
    /// Each declared member, initialized to a default of its declared kind and
    /// mutated in place as it is defined.
    members: Vec<WasmSubType>,
}

impl RecGroupBuilder {
    /// Create a new, empty rec group builder associated with the given engine.
    pub fn new(engine: &Engine) -> Self {
        RecGroupBuilder {
            engine: engine.clone(),
            builder_id: next_builder_id(),
            error: None,
            members: Vec::new(),
        }
    }

    fn declare(&mut self, inner: WasmCompositeInnerType) -> PendingType {
        let index = u32::try_from(self.members.len()).expect("too many types in a rec group");
        self.members.push(WasmSubType {
            is_final: Finality::Final.is_final(),
            supertype: None,
            composite_type: WasmCompositeType {
                shared: false,
                inner,
            },
        });
        PendingType {
            builder_id: self.builder_id,
            index,
        }
    }

    /// Declare a new struct type in this rec group, returning a handle that can
    /// be used as a forward reference before the type is defined.
    ///
    /// The order of `declare_*` calls fixes the types' order within the rec
    /// group, which is semantically significant; see the
    /// [type documentation][Self#declaration-order-is-significant] for details.
    pub fn declare_struct(&mut self) -> PendingType {
        self.declare(WasmCompositeInnerType::Struct(WasmStructType {
            fields: Vec::new(),
        }))
    }

    /// Declare a new array type in this rec group, returning a handle that can
    /// be used as a forward reference before the type is defined.
    ///
    /// The order of `declare_*` calls fixes the types' order within the rec
    /// group, which is semantically significant; see the
    /// [type documentation][Self#declaration-order-is-significant] for details.
    pub fn declare_array(&mut self) -> PendingType {
        // An array requires an element type; use a placeholder here that the
        // caller is expected to overwrite via the array builder.
        self.declare(WasmCompositeInnerType::Array(WasmArrayType(
            WasmFieldType {
                element_type: WasmStorageType::Val(WasmValType::I32),
                mutable: false,
            },
        )))
    }

    /// Declare a new function type in this rec group, returning a handle that
    /// can be used as a forward reference before the type is defined.
    ///
    /// The order of `declare_*` calls fixes the types' order within the rec
    /// group, which is semantically significant; see the
    /// [type documentation][Self#declaration-order-is-significant] for details.
    pub fn declare_func(&mut self) -> PendingType {
        self.declare(WasmCompositeInnerType::Func(WasmFuncType::empty()))
    }

    #[track_caller]
    fn check_owns(&self, ty: PendingType) {
        assert_eq!(
            ty.builder_id, self.builder_id,
            "`PendingType` used with a different `RecGroupBuilder` than it came from"
        );
    }

    /// Record an error to be surfaced by [`build`][Self::build]. Only the first
    /// error is kept, so the chaining builder methods can stay infallible.
    fn record_error(&mut self, error: Error) {
        if self.error.is_none() {
            self.error = Some(error);
        }
    }

    fn check_engine(&mut self, same_engine: bool, what: &str) {
        if !same_engine {
            self.record_error(format_err!("{what} is associated with a different engine"));
        }
    }

    /// Begin defining the given handle as a struct type.
    ///
    /// The definition is committed to the group by calling
    /// [`build`][StructTypeBuilder::build].
    ///
    /// # Panics
    ///
    /// Panics if the handle was not declared via
    /// [`declare_struct`][Self::declare_struct].
    #[track_caller]
    pub fn define_struct(&mut self, ty: PendingType) -> StructTypeBuilder<'_> {
        self.check_owns(ty);
        assert!(
            matches!(
                self.members[ty.index as usize].composite_type.inner,
                WasmCompositeInnerType::Struct(_)
            ),
            "handle was not declared as a struct type"
        );
        StructTypeBuilder {
            rec: self,
            index: ty.index,
        }
    }

    /// Begin defining the given handle as an array type.
    ///
    /// The definition is committed to the group by calling
    /// [`build`][ArrayTypeBuilder::build].
    ///
    /// # Panics
    ///
    /// Panics if the handle was not declared via
    /// [`declare_array`][Self::declare_array].
    #[track_caller]
    pub fn define_array(&mut self, ty: PendingType) -> ArrayTypeBuilder<'_> {
        self.check_owns(ty);
        assert!(
            matches!(
                self.members[ty.index as usize].composite_type.inner,
                WasmCompositeInnerType::Array(_)
            ),
            "handle was not declared as an array type"
        );
        ArrayTypeBuilder {
            rec: self,
            index: ty.index,
        }
    }

    /// Begin defining the given handle as a function type.
    ///
    /// The definition is committed to the group by calling
    /// [`build`][FuncTypeBuilder::build].
    ///
    /// # Panics
    ///
    /// Panics if the handle was not declared via
    /// [`declare_func`][Self::declare_func].
    #[track_caller]
    pub fn define_func(&mut self, ty: PendingType) -> FuncTypeBuilder<'_> {
        self.check_owns(ty);
        assert!(
            matches!(
                self.members[ty.index as usize].composite_type.inner,
                WasmCompositeInnerType::Func(_)
            ),
            "handle was not declared as a function type"
        );
        FuncTypeBuilder {
            rec: self,
            index: ty.index,
        }
    }

    /// Finish building the rec group: validate all of its types, register them
    /// with the engine, and return the resulting [`RecGroup`].
    ///
    /// An empty group is allowed and produces an empty [`RecGroup`].
    ///
    /// Returns an error if any type references a type from a different engine,
    /// or if a struct exceeds the implementation's field-count limit.
    pub fn build(self) -> Result<RecGroup> {
        let RecGroupBuilder {
            engine,
            builder_id,
            error,
            members,
        } = self;

        if let Some(error) = error {
            return Err(error);
        }

        for (i, member) in members.iter().enumerate() {
            if let WasmCompositeInnerType::Struct(s) = &member.composite_type.inner {
                ensure!(
                    s.fields.len() <= StructType::MAX_FIELDS,
                    "attempted to define struct type {i} with {} fields, but that is more than \
                     the maximum supported number of fields ({})",
                    s.fields.len(),
                    StructType::MAX_FIELDS,
                );
            }
        }

        // Keep each member's declared supertype so we can structurally validate
        // it after registration, once forward references resolve to registered
        // types. (`register_rec_group_types` does not itself check subtyping.)
        let supertypes: Vec<_> = members.iter().map(|m| m.supertype).collect();

        let registered = engine.register_rec_group_types(members.into_iter())?;
        let group = RecGroup {
            builder_id,
            types: registered,
        };

        // On validation failure, `group` is dropped, which unregisters the types.
        for (i, supertype) in supertypes.into_iter().enumerate() {
            validate_supertype(&engine, &group, i, supertype)?;
        }

        Ok(group)
    }
}

/// Builder for a struct type within a [`RecGroupBuilder`].
///
/// Returned by [`RecGroupBuilder::define_struct`]. Call [`build`][Self::build]
/// to commit the type to the group.
pub struct StructTypeBuilder<'a> {
    rec: &'a mut RecGroupBuilder,
    index: u32,
}

impl<'a> StructTypeBuilder<'a> {
    fn struct_mut(&mut self) -> &mut WasmStructType {
        match &mut self.rec.members[self.index as usize].composite_type.inner {
            WasmCompositeInnerType::Struct(s) => s,
            _ => unreachable!("struct builder on a non-struct member"),
        }
    }

    /// Set this struct type's finality. Defaults to [`Finality::Final`].
    pub fn finality(&mut self, finality: Finality) -> &mut Self {
        self.rec.members[self.index as usize].is_final = finality.is_final();
        self
    }

    /// Set this struct type's supertype to an already-registered struct type.
    pub fn supertype(&mut self, supertype: StructType) -> &mut Self {
        let same = supertype.comes_from_same_engine(&self.rec.engine);
        self.rec.check_engine(same, "supertype");
        self.rec.members[self.index as usize].supertype =
            Some(EngineOrModuleTypeIndex::Engine(supertype.type_index()));
        self
    }

    /// Set this struct type's supertype to another struct being defined in the
    /// same rec group.
    #[track_caller]
    pub fn forward_supertype(&mut self, supertype: PendingType) -> &mut Self {
        self.rec.check_owns(supertype);
        self.rec.members[self.index as usize].supertype = Some(module_index(supertype.index));
        self
    }

    /// Append a field whose type is already known (a scalar, an abstract ref, or
    /// a reference to an already-registered type).
    pub fn field(&mut self, ty: FieldType) -> &mut Self {
        let same = ty.comes_from_same_engine(&self.rec.engine);
        self.rec.check_engine(same, "field type");
        let field = ty.to_wasm_field_type();
        self.struct_mut().fields.push(field);
        self
    }

    /// Append a field that is a reference to another type being defined in the
    /// same rec group.
    ///
    /// The field is added immediately (immutable and nullable by default) and the
    /// returned builder tweaks it in place, so nothing is lost if its
    /// [`build`][ForwardRefFieldBuilder::build] is never called.
    #[track_caller]
    pub fn forward_ref_field(&mut self, ty: PendingType) -> ForwardRefFieldBuilder<'_, 'a> {
        self.rec.check_owns(ty);
        let field = forward_field(&self.rec.members, ty.index, true, false);
        self.struct_mut().fields.push(field);
        let field_index = self.struct_mut().fields.len() - 1;
        ForwardRefFieldBuilder {
            parent: self,
            field_index,
        }
    }

    /// Commit this struct definition and return the [`RecGroupBuilder`] so that
    /// further types can be defined.
    ///
    /// Fields are written to the group as they are added, so this currently only
    /// serves to mark the end of the definition.
    pub fn build(&mut self) -> &mut RecGroupBuilder {
        &mut *self.rec
    }
}

/// Builder for a struct field that forward-references another type in the same
/// rec group. Created by [`StructTypeBuilder::forward_ref_field`].
///
/// The field is already part of the struct; this builder tweaks it in place.
/// Call [`build`][Self::build] to return to the struct builder.
pub struct ForwardRefFieldBuilder<'p, 'a> {
    parent: &'p mut StructTypeBuilder<'a>,
    field_index: usize,
}

impl<'p, 'a> ForwardRefFieldBuilder<'p, 'a> {
    /// Set the field's mutability. Defaults to [`Mutability::Const`].
    pub fn mutability(self, mutability: Mutability) -> Self {
        self.parent.struct_mut().fields[self.field_index].mutable = mutability.is_var();
        self
    }

    /// Set whether the reference is nullable. Defaults to `true`.
    pub fn nullable(self, is_nullable: bool) -> Self {
        if let WasmStorageType::Val(WasmValType::Ref(r)) =
            &mut self.parent.struct_mut().fields[self.field_index].element_type
        {
            r.nullable = is_nullable;
        }
        self
    }

    /// Return to the struct builder.
    ///
    /// The field is written as it is configured, so this only returns control to
    /// the struct builder and never loses anything.
    pub fn build(self) -> &'p mut StructTypeBuilder<'a> {
        self.parent
    }
}

/// Builder for an array type within a [`RecGroupBuilder`].
///
/// Returned by [`RecGroupBuilder::define_array`]. An array has exactly one
/// element type, which must be set via [`element`][Self::element] or
/// [`forward_ref_element`][Self::forward_ref_element]. Call
/// [`build`][Self::build] to commit the type to the group.
pub struct ArrayTypeBuilder<'a> {
    rec: &'a mut RecGroupBuilder,
    index: u32,
}

impl<'a> ArrayTypeBuilder<'a> {
    fn set_element(&mut self, field: WasmFieldType) {
        *self.element_mut() = field;
    }

    fn element_mut(&mut self) -> &mut WasmFieldType {
        match &mut self.rec.members[self.index as usize].composite_type.inner {
            WasmCompositeInnerType::Array(a) => &mut a.0,
            _ => unreachable!("array builder on a non-array member"),
        }
    }

    /// Set this array type's finality. Defaults to [`Finality::Final`].
    pub fn finality(&mut self, finality: Finality) -> &mut Self {
        self.rec.members[self.index as usize].is_final = finality.is_final();
        self
    }

    /// Set this array type's supertype to an already-registered array type.
    pub fn supertype(&mut self, supertype: ArrayType) -> &mut Self {
        let same = supertype.comes_from_same_engine(&self.rec.engine);
        self.rec.check_engine(same, "supertype");
        self.rec.members[self.index as usize].supertype =
            Some(EngineOrModuleTypeIndex::Engine(supertype.type_index()));
        self
    }

    /// Set this array type's supertype to another array being defined in the
    /// same rec group.
    #[track_caller]
    pub fn forward_supertype(&mut self, supertype: PendingType) -> &mut Self {
        self.rec.check_owns(supertype);
        self.rec.members[self.index as usize].supertype = Some(module_index(supertype.index));
        self
    }

    /// Set the array's element type to an already-known type.
    pub fn element(&mut self, ty: FieldType) -> &mut Self {
        let same = ty.comes_from_same_engine(&self.rec.engine);
        self.rec.check_engine(same, "element type");
        let field = ty.to_wasm_field_type();
        self.set_element(field);
        self
    }

    /// Set the array's element type to a reference to another type being defined
    /// in the same rec group.
    ///
    /// The element is set immediately (immutable and nullable by default) and the
    /// returned builder tweaks it in place, so nothing is lost if its
    /// [`build`][ForwardRefElementBuilder::build] is never called.
    #[track_caller]
    pub fn forward_ref_element(&mut self, ty: PendingType) -> ForwardRefElementBuilder<'_, 'a> {
        self.rec.check_owns(ty);
        let field = forward_field(&self.rec.members, ty.index, true, false);
        self.set_element(field);
        ForwardRefElementBuilder { parent: self }
    }

    /// Commit this array definition and return the [`RecGroupBuilder`] so that
    /// further types can be defined.
    ///
    /// The element type is written to the group as it is set, so this currently
    /// only serves to mark the end of the definition.
    pub fn build(&mut self) -> &mut RecGroupBuilder {
        &mut *self.rec
    }
}

/// Builder for an array element that forward-references another type in the same
/// rec group. Created by [`ArrayTypeBuilder::forward_ref_element`].
///
/// The element is already set on the array; this builder tweaks it in place.
/// Call [`build`][Self::build] to return to the array builder.
pub struct ForwardRefElementBuilder<'p, 'a> {
    parent: &'p mut ArrayTypeBuilder<'a>,
}

impl<'p, 'a> ForwardRefElementBuilder<'p, 'a> {
    /// Set the element's mutability. Defaults to [`Mutability::Const`].
    pub fn mutability(self, mutability: Mutability) -> Self {
        self.parent.element_mut().mutable = mutability.is_var();
        self
    }

    /// Set whether the reference is nullable. Defaults to `true`.
    pub fn nullable(self, is_nullable: bool) -> Self {
        if let WasmStorageType::Val(WasmValType::Ref(r)) =
            &mut self.parent.element_mut().element_type
        {
            r.nullable = is_nullable;
        }
        self
    }

    /// Return to the array builder.
    ///
    /// The element is written as it is configured, so this only returns control
    /// to the array builder and never loses anything.
    pub fn build(self) -> &'p mut ArrayTypeBuilder<'a> {
        self.parent
    }
}

/// Builder for a function type within a [`RecGroupBuilder`].
///
/// Returned by [`RecGroupBuilder::define_func`]. Call [`build`][Self::build]
/// to commit the type to the group.
pub struct FuncTypeBuilder<'a> {
    rec: &'a mut RecGroupBuilder,
    index: u32,
}

impl<'a> FuncTypeBuilder<'a> {
    fn func_mut(&mut self) -> &mut WasmFuncType {
        match &mut self.rec.members[self.index as usize].composite_type.inner {
            WasmCompositeInnerType::Func(f) => f,
            _ => unreachable!("func builder on a non-func member"),
        }
    }

    /// Set this function type's finality. Defaults to [`Finality::Final`].
    pub fn finality(&mut self, finality: Finality) -> &mut Self {
        self.rec.members[self.index as usize].is_final = finality.is_final();
        self
    }

    /// Set this function type's supertype to an already-registered function type.
    pub fn supertype(&mut self, supertype: FuncType) -> &mut Self {
        let same = supertype.comes_from_same_engine(&self.rec.engine);
        self.rec.check_engine(same, "supertype");
        self.rec.members[self.index as usize].supertype =
            Some(EngineOrModuleTypeIndex::Engine(supertype.type_index()));
        self
    }

    /// Set this function type's supertype to another function being defined in
    /// the same rec group.
    #[track_caller]
    pub fn forward_supertype(&mut self, supertype: PendingType) -> &mut Self {
        self.rec.check_owns(supertype);
        self.rec.members[self.index as usize].supertype = Some(module_index(supertype.index));
        self
    }

    /// Append a parameter whose type is already known.
    pub fn param(&mut self, ty: ValType) -> &mut Self {
        let same = ty.comes_from_same_engine(&self.rec.engine);
        self.rec.check_engine(same, "type");
        let val = ty.to_wasm_type();
        self.func_mut().push_param(val);
        self
    }

    /// Append a result whose type is already known.
    pub fn result(&mut self, ty: ValType) -> &mut Self {
        let same = ty.comes_from_same_engine(&self.rec.engine);
        self.rec.check_engine(same, "type");
        let val = ty.to_wasm_type();
        self.func_mut().push_result(val);
        self
    }

    /// Append a parameter that is a reference to another type being defined in
    /// the same rec group.
    ///
    /// The parameter is added immediately (nullable by default) and the returned
    /// builder tweaks it in place, so nothing is lost if its
    /// [`build`][ForwardRefFuncValBuilder::build] is never called.
    #[track_caller]
    pub fn forward_ref_param(&mut self, ty: PendingType) -> ForwardRefFuncValBuilder<'_, 'a> {
        self.rec.check_owns(ty);
        let val = WasmValType::Ref(WasmRefType {
            nullable: true,
            heap_type: forward_heap(&self.rec.members, ty.index),
        });
        let val_index = {
            let func = self.func_mut();
            func.push_param(val);
            func.params().len() - 1
        };
        ForwardRefFuncValBuilder {
            parent: self,
            val_index,
            is_result: false,
        }
    }

    /// Append a result that is a reference to another type being defined in the
    /// same rec group.
    ///
    /// The result is added immediately (nullable by default) and the returned
    /// builder tweaks it in place, so nothing is lost if its
    /// [`build`][ForwardRefFuncValBuilder::build] is never called.
    #[track_caller]
    pub fn forward_ref_result(&mut self, ty: PendingType) -> ForwardRefFuncValBuilder<'_, 'a> {
        self.rec.check_owns(ty);
        let val = WasmValType::Ref(WasmRefType {
            nullable: true,
            heap_type: forward_heap(&self.rec.members, ty.index),
        });
        let val_index = {
            let func = self.func_mut();
            func.push_result(val);
            func.results().len() - 1
        };
        ForwardRefFuncValBuilder {
            parent: self,
            val_index,
            is_result: true,
        }
    }

    /// Commit this function definition and return the [`RecGroupBuilder`] so
    /// that further types can be defined.
    ///
    /// Params and results are written to the group as they are added, so this
    /// currently only serves to mark the end of the definition.
    pub fn build(&mut self) -> &mut RecGroupBuilder {
        &mut *self.rec
    }
}

/// Builder for a function parameter or result that forward-references another
/// type in the same rec group. Created by
/// [`FuncTypeBuilder::forward_ref_param`] and
/// [`FuncTypeBuilder::forward_ref_result`].
///
/// The parameter/result is already part of the function; this builder tweaks it
/// in place. Call [`build`][Self::build] to return to the function builder.
pub struct ForwardRefFuncValBuilder<'p, 'a> {
    parent: &'p mut FuncTypeBuilder<'a>,
    val_index: usize,
    is_result: bool,
}

impl<'p, 'a> ForwardRefFuncValBuilder<'p, 'a> {
    /// Set whether the reference is nullable. Defaults to `true`.
    pub fn nullable(self, is_nullable: bool) -> Self {
        let func = self.parent.func_mut();
        let mut val = if self.is_result {
            func.results()[self.val_index]
        } else {
            func.params()[self.val_index]
        };
        if let WasmValType::Ref(r) = &mut val {
            r.nullable = is_nullable;
        }
        if self.is_result {
            func.set_result(self.val_index, val);
        } else {
            func.set_param(self.val_index, val);
        }
        self
    }

    /// Return to the function builder.
    ///
    /// The parameter/result is written as it is configured, so this only returns
    /// control to the function builder and never loses anything.
    pub fn build(self) -> &'p mut FuncTypeBuilder<'a> {
        self.parent
    }
}

/// A registered recursion group of Wasm types.
///
/// Produced by [`RecGroupBuilder::build`]. Use the getters
/// ([`get_struct`][Self::get_struct], [`get_array`][Self::get_array],
/// [`get_func`][Self::get_func]) to retrieve a member by its handle.
///
/// The group's types stay registered with the engine for as long as either this
/// `RecGroup` or any type retrieved from it is alive.
#[derive(Debug)]
pub struct RecGroup {
    builder_id: usize,
    types: Vec<RegisteredType>,
}

impl RecGroup {
    /// The number of types in this rec group.
    pub fn len(&self) -> usize {
        self.types.len()
    }

    /// Whether this rec group is empty, i.e. was built without declaring any
    /// types.
    pub fn is_empty(&self) -> bool {
        self.types.is_empty()
    }

    #[track_caller]
    fn registered(&self, ty: PendingType) -> &RegisteredType {
        assert_eq!(
            ty.builder_id, self.builder_id,
            "`PendingType` used with a different `RecGroup` than it came from"
        );
        &self.types[ty.index as usize]
    }

    fn struct_at(&self, index: usize) -> StructType {
        StructType::from_registered_type(self.types[index].clone())
    }
    fn array_at(&self, index: usize) -> ArrayType {
        ArrayType::from_registered_type(self.types[index].clone())
    }
    fn func_at(&self, index: usize) -> FuncType {
        FuncType::from_registered_type(self.types[index].clone())
    }

    /// Get the struct type for the given handle, or `None` if it was defined as
    /// a different kind of type.
    pub fn get_struct(&self, ty: PendingType) -> Option<StructType> {
        let rt = self.registered(ty);
        rt.is_struct()
            .then(|| StructType::from_registered_type(rt.clone()))
    }

    /// Get the array type for the given handle, or `None` if it was defined as a
    /// different kind of type.
    pub fn get_array(&self, ty: PendingType) -> Option<ArrayType> {
        let rt = self.registered(ty);
        rt.is_array()
            .then(|| ArrayType::from_registered_type(rt.clone()))
    }

    /// Get the function type for the given handle, or `None` if it was defined
    /// as a different kind of type.
    pub fn get_func(&self, ty: PendingType) -> Option<FuncType> {
        let rt = self.registered(ty);
        rt.is_func()
            .then(|| FuncType::from_registered_type(rt.clone()))
    }

    /// Iterate over all of the types in this rec group, in definition order,
    /// each as a concrete [`HeapType`].
    pub fn types(&self) -> impl ExactSizeIterator<Item = HeapType> + '_ {
        self.types.iter().map(|rt| {
            let rt = rt.clone();
            if rt.is_struct() {
                HeapType::ConcreteStruct(StructType::from_registered_type(rt))
            } else if rt.is_array() {
                HeapType::ConcreteArray(ArrayType::from_registered_type(rt))
            } else {
                debug_assert!(rt.is_func());
                HeapType::ConcreteFunc(FuncType::from_registered_type(rt))
            }
        })
    }
}

/// The `WasmHeapType` for a forward reference to the member at `target`,
/// choosing the concrete variant based on the target's declared kind.
fn forward_heap(members: &[WasmSubType], target: u32) -> WasmHeapType {
    let index = module_index(target);
    match members[target as usize].composite_type.inner {
        WasmCompositeInnerType::Struct(_) => WasmHeapType::ConcreteStruct(index),
        WasmCompositeInnerType::Array(_) => WasmHeapType::ConcreteArray(index),
        WasmCompositeInnerType::Func(_) => WasmHeapType::ConcreteFunc(index),
        WasmCompositeInnerType::Cont(_) | WasmCompositeInnerType::Exn(_) => {
            unreachable!("rec group builder never declares cont or exn types")
        }
    }
}

/// The `WasmFieldType` for a struct field or array element that forward-references
/// the member at `target`.
fn forward_field(
    members: &[WasmSubType],
    target: u32,
    nullable: bool,
    mutable: bool,
) -> WasmFieldType {
    WasmFieldType {
        element_type: WasmStorageType::Val(WasmValType::Ref(WasmRefType {
            nullable,
            heap_type: forward_heap(members, target),
        })),
        mutable,
    }
}

/// Validate that the member at `index` structurally matches its declared
/// `supertype` (if any), now that all forward references are registered.
fn validate_supertype(
    engine: &Engine,
    group: &RecGroup,
    index: usize,
    supertype: Option<EngineOrModuleTypeIndex>,
) -> Result<()> {
    let Some(supertype) = supertype else {
        return Ok(());
    };
    let rt = &group.types[index];
    if rt.is_struct() {
        let sub = group.struct_at(index);
        let sup = match supertype {
            EngineOrModuleTypeIndex::Module(t) => {
                let t = t.index();
                ensure!(
                    group.types[t].is_struct(),
                    "a struct type's supertype must be a struct type"
                );
                group.struct_at(t)
            }
            EngineOrModuleTypeIndex::Engine(idx) => StructType::from_shared_type_index(engine, idx),
            EngineOrModuleTypeIndex::RecGroup(_) => unreachable!(),
        };
        ensure!(
            sup.finality().is_non_final(),
            "cannot create a subtype of a final supertype"
        );
        ensure!(
            struct_fields_match(&sub, &sup),
            "struct fields must match their supertype's fields"
        );
    } else if rt.is_array() {
        let sub = group.array_at(index);
        let sup = match supertype {
            EngineOrModuleTypeIndex::Module(t) => {
                let t = t.index();
                ensure!(
                    group.types[t].is_array(),
                    "an array type's supertype must be an array type"
                );
                group.array_at(t)
            }
            EngineOrModuleTypeIndex::Engine(idx) => ArrayType::from_shared_type_index(engine, idx),
            EngineOrModuleTypeIndex::RecGroup(_) => unreachable!(),
        };
        ensure!(
            sup.finality().is_non_final(),
            "cannot create a subtype of a final supertype"
        );
        ensure!(
            sub.field_type().matches(&sup.field_type()),
            "array field type must match its supertype's field type"
        );
    } else {
        debug_assert!(rt.is_func());
        let sub = group.func_at(index);
        let sup = match supertype {
            EngineOrModuleTypeIndex::Module(t) => {
                let t = t.index();
                ensure!(
                    group.types[t].is_func(),
                    "a function type's supertype must be a function type"
                );
                group.func_at(t)
            }
            EngineOrModuleTypeIndex::Engine(idx) => FuncType::from_shared_type_index(engine, idx),
            EngineOrModuleTypeIndex::RecGroup(_) => unreachable!(),
        };
        ensure!(
            sup.finality().is_non_final(),
            "cannot create a subtype of a final supertype"
        );
        // `FuncType::matches` performs structural (not nominal) matching for
        // distinct types, which is exactly the subtype check we want.
        ensure!(sub.matches(&sup), "function type must match its supertype");
    }
    Ok(())
}

/// Does struct `sub` structurally match (i.e. subtype) struct `sup`? `sub` must
/// have at least as many fields as `sup`, and each of `sup`'s fields must be
/// matched by `sub`'s.
fn struct_fields_match(sub: &StructType, sup: &StructType) -> bool {
    sub.fields().len() >= sup.fields().len()
        && sub.fields().zip(sup.fields()).all(|(a, b)| a.matches(&b))
}
