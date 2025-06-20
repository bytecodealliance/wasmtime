use crate::runtime::types::TagType;
use crate::{
    AsContext,
    store::{StoreInstanceId, StoreOpaque},
};
use wasmtime_environ::{DefinedTagIndex, VMSharedTypeIndex};

/// A WebAssembly `tag`.
#[derive(Copy, Clone, Debug)]
#[repr(C)] // here for the C API in the future
pub struct Tag {
    instance: StoreInstanceId,
    index: DefinedTagIndex,
}

impl Tag {
    pub(crate) unsafe fn from_wasmtime_tag(
        wasmtime_export: crate::runtime::vm::ExportTag,
        store: &StoreOpaque,
    ) -> Self {
        debug_assert!(
            wasmtime_export.tag.signature.unwrap_engine_type_index()
                != VMSharedTypeIndex::default()
        );
        Tag {
            instance: store.vmctx_id(wasmtime_export.vmctx),
            index: wasmtime_export.index,
        }
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
        TagType::from_wasmtime_tag(store.engine(), self.wasmtime_ty(store))
    }

    pub(crate) fn wasmtime_ty<'a>(&self, store: &'a StoreOpaque) -> &'a wasmtime_environ::Tag {
        let module = store[self.instance].env_module();
        let index = module.tag_index(self.index);
        &module.tags[index]
    }

    pub(crate) fn vmimport(&self, store: &StoreOpaque) -> crate::runtime::vm::VMTagImport {
        let instance = &store[self.instance];
        crate::runtime::vm::VMTagImport {
            from: instance.tag_ptr(self.index).into(),
            vmctx: instance.vmctx().into(),
            index: self.index,
        }
    }

    pub(crate) fn comes_from_same_store(&self, store: &StoreOpaque) -> bool {
        store.id() == self.instance.store_id()
    }

    /// Determines whether this tag is reference equal to the other
    /// given tag in the given store.
    ///
    /// # Panics
    ///
    /// Panics if either tag do not belong to the given `store`.
    pub fn eq(a: &Tag, b: &Tag, store: impl AsContext) -> bool {
        // make sure both tags belong to the store
        let store = store.as_context();
        let _ = &store[a.instance];
        let _ = &store[b.instance];

        // then compare to see if they have the same definition
        a.instance == b.instance && a.index == b.index
    }
}
