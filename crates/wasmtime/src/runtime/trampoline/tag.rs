use crate::ExnType;
use crate::TagType;
use crate::prelude::*;
use crate::runtime::vm::{self, Imports, ModuleRuntimeInfo, OnDemandInstanceAllocator};
use crate::store::{AllocateInstanceKind, InstanceId, StoreOpaque};
use alloc::sync::Arc;
use wasmtime_environ::EngineOrModuleTypeIndex;
use wasmtime_environ::StaticModuleIndex;
use wasmtime_environ::Tag;
use wasmtime_environ::{EntityIndex, Module, TypeTrace};

pub fn create_tag(store: &mut StoreOpaque, ty: &TagType) -> Result<InstanceId> {
    let mut module = Module::new(StaticModuleIndex::from_u32(0));
    let func_ty = ty.ty().clone().into_registered_type();
    let exn_ty = ExnType::from_tag_type(ty)?.registered_type().clone();

    debug_assert!(
        func_ty.is_canonicalized_for_runtime_usage(),
        "should be canonicalized for runtime usage: {func_ty:?}",
    );
    debug_assert!(
        exn_ty.is_canonicalized_for_runtime_usage(),
        "should be canonicalized for runtime usage: {exn_ty:?}",
    );

    let tag_id = module.tags.push(Tag {
        signature: EngineOrModuleTypeIndex::Engine(func_ty.index()),
        exception: EngineOrModuleTypeIndex::Engine(exn_ty.index()),
    })?;

    let name = module.strings.insert("")?;
    module.exports.insert(name, EntityIndex::Tag(tag_id))?;

    let imports = Imports::default();

    // Both the tag's signature type and its exception type are referred to by
    // engine-level type index from the dummy module's `Tag`, so both
    // `RegisteredType`s must be handed to the instance's runtime info to keep
    // those indices rooted in the engine's type registry for as long as the
    // instance (and thus the store) is alive.
    let runtime_info = ModuleRuntimeInfo::bare_with_registered_types(
        try_new::<Arc<_>>(module)?,
        store.engine(),
        [func_ty, exn_ty],
    )?;

    unsafe {
        let allocator =
            OnDemandInstanceAllocator::new(store.engine().config().mem_creator.clone(), 0, false);

        // Note that `assert_ready` should be valid here because this module
        // doesn't allocate tables or memories meaning it shouldn't need a
        // resource limiter so `None` is passed. As a result no `await` points
        // should ever be hit.
        vm::assert_ready(store.allocate_instance(
            None,
            AllocateInstanceKind::Dummy {
                allocator: &allocator,
            },
            &runtime_info,
            imports,
        ))
    }
}
