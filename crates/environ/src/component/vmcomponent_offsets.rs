//! Offsets of the fields within a `VMComponentContext`.
//!
//! The layout itself is not defined here: it is defined once, alongside
//! `VMContext`'s, in `for_each_vmctx_type!`. Everything in this module is either
//! generated from that definition or is an offset that is not simply the offset
//! of one of the layout's fields.

use crate::GetPtrSize;
use crate::PtrSize;
use crate::component::*;

/// Equivalent of `VMCONTEXT_MAGIC` except for components.
///
/// This is stored at the start of all `VMComponentContext` structures and
/// double-checked on `VMComponentContext::from_opaque`.
pub const VMCOMPONENT_MAGIC: u32 = u32::from_le_bytes(*b"comp");

/// Runtime offsets within a `VMComponentContext` for a specific component.
#[derive(Debug, Clone, Copy)]
pub struct VMComponentOffsets<P> {
    /// The host pointer size
    pub ptr: P,

    /// The number of lowered functions this component will be creating.
    pub num_lowerings: u32,
    /// The number of memories which are recorded in this component for options.
    pub num_runtime_memories: u32,
    /// The number of tables which are recorded in this component for options.
    pub num_runtime_tables: u32,
    /// The number of reallocs which are recorded in this component for options.
    pub num_runtime_reallocs: u32,
    /// The number of callbacks which are recorded in this component for options.
    pub num_runtime_callbacks: u32,
    /// The number of post-returns which are recorded in this component for options.
    pub num_runtime_post_returns: u32,
    /// Number of component instances internally in the component (always at
    /// least 1).
    pub num_runtime_component_instances: u32,
    /// Number of cranelift-compiled trampolines required for this component.
    pub num_trampolines: u32,
    /// Number of `VMFuncRef`s for unsafe intrinsics within this component's
    /// context.
    pub num_unsafe_intrinsics: u32,
    /// Number of resources within a component which need destructors stored.
    pub num_resources: u32,

    // Precalculated offsets of the dynamically-positioned fields, one per entry
    // in `VMComponentContext`'s `dynamic` section in `for_each_vmctx_type!`,
    // plus this `VMComponentContext`'s total size. These are all computed by the
    // generated `compute_field_offsets` and read by the generated accessors of
    // the same names.
    may_leave: u32,
    task_may_block: u32,
    trampoline_func_refs: u32,
    intrinsic_func_refs: u32,
    lowerings: u32,
    memories: u32,
    tables: u32,
    reallocs: u32,
    callbacks: u32,
    post_returns: u32,
    resource_destructors: u32,
    size: u32,
}

impl<P: PtrSize> GetPtrSize for VMComponentOffsets<P> {
    type Ptr = P;

    #[inline]
    fn get_ptr_size(&self) -> &Self::Ptr {
        &self.ptr
    }
}

impl_vmctx_array_index! {
    RuntimeComponentInstanceIndex,
    TrampolineIndex,
    LoweredIndex,
    RuntimeMemoryIndex,
    RuntimeTableIndex,
    RuntimeReallocIndex,
    RuntimeCallbackIndex,
    RuntimePostReturnIndex,
    ResourceIndex,
}

impl crate::VmctxArrayIndex for UnsafeIntrinsic {
    #[inline]
    fn vmctx_array_index(self) -> u32 {
        self.index()
    }
}

/// Generate the offsets of `VMComponentContext`'s dynamically-positioned fields,
/// ignoring every other vmctx type.
macro_rules! define_vmcomponent_offsets_dynamic_offsets {
    (@one VMComponentContext $snake:ident { $($dyn:tt)* }) => {
        /// Offsets of the dynamically-positioned fields of `VMComponentContext`.
        impl<P: PtrSize> VMComponentOffsets<P> {
            define_vmctx_dynamic_offsets!(@accessors (self) [ $($dyn)* ]);

            define_vmctx_dynamic_offsets!(@compute_fn (self, next) $snake [ $($dyn)* ]);

            /// Return the size of the `VMComponentContext` allocation.
            #[inline]
            pub fn size_of_vmctx(&self) -> u32 {
                self.size
            }
        }
    };
    (@one $other:ident $snake:ident { $($dyn:tt)* }) => {};

    ( $(
        {
            $Name:ident $snake:ident
            static { $($stat:tt)* }
            dynamic { $($dyn:tt)* }
        }
    )* ) => {
        $( define_vmcomponent_offsets_dynamic_offsets!(@one $Name $snake { $($dyn)* }); )*
    };
}

for_each_vmctx_type!(define_vmcomponent_offsets_dynamic_offsets);

impl<P: PtrSize> VMComponentOffsets<P> {
    /// Creates a new set of offsets for the `component` specified configured
    /// additionally for the `ptr` size specified.
    pub fn new(ptr: P, component: &Component) -> Self {
        // This is required by the implementation of
        // `VMComponentContext::from_opaque`. If this value changes then this
        // location needs to be updated.
        assert_eq!(ptr.vmcomponent().magic(), 0);

        let mut ret = Self {
            ptr,
            num_lowerings: component.num_lowerings,
            num_runtime_memories: component.num_runtime_memories,
            num_runtime_tables: component.num_runtime_tables,
            num_runtime_reallocs: component.num_runtime_reallocs,
            num_runtime_callbacks: component.num_runtime_callbacks,
            num_runtime_post_returns: component.num_runtime_post_returns,
            num_runtime_component_instances: component.num_runtime_component_instances,
            num_trampolines: component.trampolines.len().try_into().unwrap(),
            num_unsafe_intrinsics: if let Some(i) = component
                .unsafe_intrinsics
                .iter()
                .rposition(|x| x.is_some())
            {
                // Note: We do not currently have an indirection between "the
                // `i`th unsafe intrinsic in the vmctx" and
                // `UnsafeIntrinsic::from_u32(i)`, so therefore if we are
                // compiling in *any* intrinsics, we need to include space for
                // all of them up to the max `i` that is used.
                //
                // We _could_ introduce such an indirection via a map in
                // `Component` like `PrimaryMap<UnsafeIntrinsicIndex,
                // UnsafeIntrinsic>`, and that would allow us to densely pack
                // intrinsics in the vmctx. However we do not do that today
                // because there are very few unsafe intrinsics, and we do not
                // see that changing anytime soon, so we aren't wasting much
                // space.
                u32::try_from(i + 1).unwrap()
            } else {
                0
            },
            num_resources: component.num_resources,
            may_leave: 0,
            task_may_block: 0,
            trampoline_func_refs: 0,
            intrinsic_func_refs: 0,
            lowerings: 0,
            memories: 0,
            tables: 0,
            reallocs: 0,
            callbacks: 0,
            post_returns: 0,
            resource_destructors: 0,
            size: 0,
        };

        ret.compute_field_offsets();

        ret
    }

    /// The size, in bytes, of the host pointer.
    #[inline]
    pub fn pointer_size(&self) -> u8 {
        self.ptr.size()
    }

    /// The offset of the `callee` for the `index` specified.
    #[inline]
    pub fn lowering_callee(&self, index: LoweredIndex) -> u32 {
        self.lowerings().at(index) + self.lowering_callee_offset()
    }

    /// The offset of the `data` for the `index` specified.
    #[inline]
    pub fn lowering_data(&self, index: LoweredIndex) -> u32 {
        self.lowerings().at(index) + self.lowering_data_offset()
    }

    /// The size of the `VMLowering` type
    #[inline]
    pub fn lowering_size(&self) -> u8 {
        2 * self.ptr.size()
    }

    /// The offset of the `callee` field within the `VMLowering` type.
    #[inline]
    pub fn lowering_callee_offset(&self) -> u32 {
        0
    }

    /// The offset of the `data` field within the `VMLowering` type.
    #[inline]
    pub fn lowering_data_offset(&self) -> u32 {
        u32::from(self.ptr.size())
    }
}
