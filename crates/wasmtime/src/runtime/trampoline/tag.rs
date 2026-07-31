use crate::TagType;
use crate::prelude::*;
use crate::runtime::vm::{Imports, ModuleRuntimeInfo, OnDemandInstanceAllocator};
use crate::store::{AllocateInstanceKind, InstanceId, StoreOpaque};
use alloc::sync::Arc;
use wasmtime_environ::EngineOrModuleTypeIndex;
use wasmtime_environ::Tag;
use wasmtime_environ::{EntityIndex, Module, TypeTrace};

pub fn create_tag(store: &mut StoreOpaque, ty: &TagType) -> Result<InstanceId> {
    let mut module = Module::new();
    let func_ty = ty.ty().clone().into_registered_type();

    debug_assert!(
        func_ty.is_canonicalized_for_runtime_usage(),
        "should be canonicalized for runtime usage: {func_ty:?}",
    );

    let tag_id = module.tags.push(Tag {
        signature: EngineOrModuleTypeIndex::Engine(func_ty.index()),
    });

    module
        .exports
        .insert(String::new(), EntityIndex::Tag(tag_id));

    let imports = Imports::default();

    // The tag's signature type is referred to by engine-level type index from
    // the dummy module's `Tag`, so its `RegisteredType` must be handed to the
    // instance's runtime info to keep that index rooted in the engine's type
    // registry for as long as the instance (and thus the store) is alive.
    let runtime_info = ModuleRuntimeInfo::bare_with_registered_type(
        Arc::new(module),
        store.engine(),
        Some(func_ty),
    )?;

    unsafe {
        let allocator =
            OnDemandInstanceAllocator::new(store.engine().config().mem_creator.clone(), 0, false);
        store.allocate_instance(
            AllocateInstanceKind::Dummy {
                allocator: &allocator,
            },
            &runtime_info,
            imports,
        )
    }
}
