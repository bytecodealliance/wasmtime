use crate::runtime::types::TagType;
use crate::{
    store::{StoreData, StoreOpaque, Stored},
    AsContext,
};

/// A WebAssembly `tag`.
#[derive(Copy, Clone, Debug)]
#[repr(transparent)] // here for the C API
pub struct Tag(pub(super) Stored<crate::runtime::vm::ExportTag>);

impl Tag {
    pub(crate) unsafe fn from_wasmtime_tag(
        mut wasmtime_export: crate::runtime::vm::ExportTag,
        store: &mut StoreOpaque,
    ) -> Self {
        use wasmtime_environ::TypeTrace;
        wasmtime_export
            .tag
            .canonicalize_for_runtime_usage(&mut |module_index| {
                crate::runtime::vm::Instance::from_vmctx(wasmtime_export.vmctx, |instance| {
                    instance.engine_type_index(module_index)
                })
            });

        Tag(store.store_data_mut().insert(wasmtime_export))
    }

    /// Returns the underlying type of this `tag`.
    ///
    /// # Panics
    ///
    /// Panics if `store` does not own this tag.
    pub fn ty(&self, store: impl AsContext) -> TagType {
        self._ty(store.as_context().0)
    }

    pub(crate) fn _ty(&self, store: &StoreOpaque) -> TagType {
        let ty = &store[self.0].tag;
        TagType::from_wasmtime_tag(store.engine(), &ty)
    }

    pub(crate) fn wasmtime_ty<'a>(&self, data: &'a StoreData) -> &'a wasmtime_environ::Tag {
        &data[self.0].tag
    }

    pub(crate) fn vmimport(&self, store: &StoreOpaque) -> crate::runtime::vm::VMTagImport {
        let export = &store[self.0];
        crate::runtime::vm::VMTagImport {
            from: export.definition.into(),
            vmctx: export.vmctx.into(),
        }
    }

    /// Determines whether this tag is reference equal to the other
    /// given tag in the given store.
    ///
    /// # Panics
    ///
    /// Panics if either tag do not belong to the given `store`.
    pub fn eq(a: &Tag, b: &Tag, store: impl AsContext) -> bool {
        let store = store.as_context().0;
        let a = &store[a.0];
        let b = &store[b.0];
        a.definition.eq(&b.definition)
    }
}
