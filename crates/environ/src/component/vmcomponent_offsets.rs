// Currently the `VMComponentContext` allocation by field looks like this:
//
// struct VMComponentContext {
//      magic: u32,
//      may_enter: u8,
//      may_leave: u8,
//      store: *mut dyn Store,
//      lowering_anyfuncs: [VMCallerCheckedAnyfunc; component.num_lowerings],
//      lowerings: [VMLowering; component.num_lowerings],
//      memories: [*mut VMMemoryDefinition; component.num_memories],
//      reallocs: [*mut VMCallerCheckedAnyfunc; component.num_reallocs],
// }

use crate::component::{Component, LoweredIndex, RuntimeMemoryIndex, RuntimeReallocIndex};
use crate::PtrSize;

/// Equivalent of `VMCONTEXT_MAGIC` except for components.
///
/// This is stored at the start of all `VMComponentContext` structures adn
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
    /// The number of reallocs which are recorded in this component for options.
    pub num_runtime_reallocs: u32,

    // precalculated offsets of various member fields
    magic: u32,
    may_enter: u32,
    may_leave: u32,
    store: u32,
    lowering_anyfuncs: u32,
    lowerings: u32,
    memories: u32,
    reallocs: u32,
    size: u32,
}

#[inline]
fn align(offset: u32, align: u32) -> u32 {
    assert!(align.is_power_of_two());
    (offset + (align - 1)) & !(align - 1)
}

impl<P: PtrSize> VMComponentOffsets<P> {
    /// Creates a new set of offsets for the `component` specified configured
    /// additionally for the `ptr` size specified.
    pub fn new(ptr: P, component: &Component) -> Self {
        let mut ret = Self {
            ptr,
            num_lowerings: component.num_lowerings.try_into().unwrap(),
            num_runtime_memories: component.num_runtime_memories.try_into().unwrap(),
            num_runtime_reallocs: component.num_runtime_reallocs.try_into().unwrap(),
            magic: 0,
            may_enter: 0,
            may_leave: 0,
            store: 0,
            lowering_anyfuncs: 0,
            lowerings: 0,
            memories: 0,
            reallocs: 0,
            size: 0,
        };

        // Convenience functions for checked addition and multiplication.
        // As side effect this reduces binary size by using only a single
        // `#[track_caller]` location for each function instead of one for
        // each individual invocation.
        #[inline]
        fn cmul(count: u32, size: u8) -> u32 {
            count.checked_mul(u32::from(size)).unwrap()
        }

        let mut next_field_offset = 0;

        macro_rules! fields {
            (size($field:ident) = $size:expr, $($rest:tt)*) => {
                ret.$field = next_field_offset;
                next_field_offset = next_field_offset.checked_add(u32::from($size)).unwrap();
                fields!($($rest)*);
            };
            (align($align:expr), $($rest:tt)*) => {
                next_field_offset = align(next_field_offset, $align);
                fields!($($rest)*);
            };
            () => {};
        }

        fields! {
            size(magic) = 4u32,
            size(may_enter) = 1u32,
            size(may_leave) = 1u32,
            align(u32::from(ret.ptr.size())),
            size(store) = cmul(2, ret.ptr.size()),
            size(lowering_anyfuncs) = cmul(ret.num_lowerings, ret.ptr.size_of_vmcaller_checked_anyfunc()),
            size(lowerings) = cmul(ret.num_lowerings, ret.ptr.size() * 2),
            size(memories) = cmul(ret.num_runtime_memories, ret.ptr.size()),
            size(reallocs) = cmul(ret.num_runtime_reallocs, ret.ptr.size()),
        }

        ret.size = next_field_offset;

        // This is required by the implementation of
        // `VMComponentContext::from_opaque`. If this value changes then this
        // location needs to be updated.
        assert_eq!(ret.magic, 0);

        return ret;
    }

    /// The size, in bytes, of the host pointer.
    #[inline]
    pub fn pointer_size(&self) -> u8 {
        self.ptr.size()
    }

    /// The offset of the `magic` field.
    #[inline]
    pub fn magic(&self) -> u32 {
        self.magic
    }

    /// The offset of the `may_leave` field.
    #[inline]
    pub fn may_leave(&self) -> u32 {
        self.may_leave
    }

    /// The offset of the `may_enter` field.
    #[inline]
    pub fn may_enter(&self) -> u32 {
        self.may_enter
    }

    /// The offset of the `store` field.
    #[inline]
    pub fn store(&self) -> u32 {
        self.store
    }

    /// The offset of the `lowering_anyfuncs` field.
    #[inline]
    pub fn lowering_anyfuncs(&self) -> u32 {
        self.lowering_anyfuncs
    }

    /// The offset of `VMCallerCheckedAnyfunc` for the `index` specified.
    #[inline]
    pub fn lowering_anyfunc(&self, index: LoweredIndex) -> u32 {
        assert!(index.as_u32() < self.num_lowerings);
        self.lowering_anyfuncs()
            + index.as_u32() * u32::from(self.ptr.size_of_vmcaller_checked_anyfunc())
    }

    /// The offset of the `lowerings` field.
    #[inline]
    pub fn lowerings(&self) -> u32 {
        self.lowerings
    }

    /// The offset of the `VMLowering` for the `index` specified.
    #[inline]
    pub fn lowering(&self, index: LoweredIndex) -> u32 {
        assert!(index.as_u32() < self.num_lowerings);
        self.lowerings() + index.as_u32() * u32::from(2 * self.ptr.size())
    }

    /// The offset of the `callee` for the `index` specified.
    #[inline]
    pub fn lowering_callee(&self, index: LoweredIndex) -> u32 {
        self.lowering(index) + self.lowering_callee_offset()
    }

    /// The offset of the `data` for the `index` specified.
    #[inline]
    pub fn lowering_data(&self, index: LoweredIndex) -> u32 {
        self.lowering(index) + self.lowering_data_offset()
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

    /// The offset of the base of the `runtime_memories` field
    #[inline]
    pub fn runtime_memories(&self) -> u32 {
        self.memories
    }

    /// The offset of the `*mut VMMemoryDefinition` for the runtime index
    /// provided.
    #[inline]
    pub fn runtime_memory(&self, index: RuntimeMemoryIndex) -> u32 {
        assert!(index.as_u32() < self.num_runtime_memories);
        self.runtime_memories() + index.as_u32() * u32::from(self.ptr.size())
    }

    /// The offset of the base of the `runtime_reallocs` field
    #[inline]
    pub fn runtime_reallocs(&self) -> u32 {
        self.reallocs
    }

    /// The offset of the `*mut VMCallerCheckedAnyfunc` for the runtime index
    /// provided.
    #[inline]
    pub fn runtime_realloc(&self, index: RuntimeReallocIndex) -> u32 {
        assert!(index.as_u32() < self.num_runtime_reallocs);
        self.runtime_reallocs() + index.as_u32() * u32::from(self.ptr.size())
    }

    /// Return the size of the `VMComponentContext` allocation.
    #[inline]
    pub fn size_of_vmctx(&self) -> u32 {
        self.size
    }
}
