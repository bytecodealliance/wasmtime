//! Module comprising of component model wasm events
use super::*;
#[allow(unused_imports)]
use crate::component::ComponentType;
use wasmtime_environ::component::{InterfaceType, TypeTuple};

/// A call event from a Wasm component into the host
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostFuncEntryEvent {
    /// Raw values passed across the call entry boundary
    args: RRFuncArgVals,

    /// Optional param/return types (required to support replay validation).
    ///
    /// Note: This relies on the invariant that [InterfaceType] will always be
    /// deterministic. Currently, the type indices into various [ComponentTypes]
    /// maintain this, allowing for quick type-checking.
    types: Option<TypeTuple>,
}
impl HostFuncEntryEvent {
    // Record
    pub fn new(args: &[MaybeUninit<ValRaw>], types: Option<&TypeTuple>) -> Self {
        Self {
            args: func_argvals_from_raw_slice(args),
            types: types.cloned(),
        }
    }
    // Replay
    pub fn validate(&self, expect_types: &TypeTuple) -> Result<(), ReplayError> {
        replay_args_typecheck(self.types.as_ref(), expect_types)
    }
}

/// A return event after a host call for a Wasm component
///
/// Matches 1:1 with [`HostFuncEntryEvent`]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostFuncReturnEvent {
    /// Lowered values passed across the call return boundary
    args: RRFuncArgVals,
    /// Optional param/return types (required to support replay validation).
    ///
    /// Note: This relies on the invariant that [InterfaceType] will always be
    /// deterministic. Currently, the type indices into various [ComponentTypes]
    /// maintain this, allowing for quick type-checking.
    types: Option<TypeTuple>,
}
impl HostFuncReturnEvent {
    // Record
    pub fn new(args: &[ValRaw], types: Option<&TypeTuple>) -> Self {
        Self {
            args: func_argvals_from_raw_slice(args),
            types: types.cloned(),
        }
    }
    // Replay
    pub fn validate(&self, expect_types: &TypeTuple) -> Result<(), ReplayError> {
        replay_args_typecheck(self.types.as_ref(), expect_types)
    }

    /// Consume the caller event and encode it back into the slice with an optional
    /// typechecking validation of the event.
    pub fn move_into_slice(
        self,
        args: &mut [ValRaw],
        expect_types: Option<&TypeTuple>,
    ) -> Result<(), ReplayError> {
        if let Some(e) = expect_types {
            self.validate(e)?;
        }
        func_argvals_into_raw_slice(self.args, args);
        Ok(())
    }
}

macro_rules! generic_new_result_events {
    (
        $(
            $(#[doc = $doc:literal])*
            $event:ident => ($ok_ty:ty,$err_variant:path)
        ),*
    ) => (
        $(
            $(#[doc= $doc])*
            #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
            pub struct $event {
                ret: Result<$ok_ty, EventError>,
            }
            impl $event {
                pub fn new(ret: &Result<$ok_ty>) -> Self {
                    Self {
                        ret: ret.as_ref().map(|t| *t).map_err(|e| $err_variant(e.to_string()))
                    }
                }
            }
        )*
    );
}

macro_rules! generic_new_events {
    (
        $(
            $(#[doc = $doc:literal])*
            $struct:ident {
                $(
                    $field:ident : $field_ty:ty
                ),*
            }
        ),*
    ) => (
        $(
            #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
            pub struct $struct {
                $(
                    $field: $field_ty,
                )*
            }
        )*
        $(
            impl $struct {
                #[allow(dead_code)]
                pub fn new($($field: $field_ty),*) -> Self {
                    Self {
                        $($field),*
                    }
                }
            }
        )*
    );
}

generic_new_result_events! {
    /// Return from a reallocation call (needed only for validation)
    ReallocReturnEvent => (usize, EventError::ReallocError),
    /// Return from a type lowering invocation
    LowerReturnEvent => ((), EventError::LowerError),
    /// Return from store invocations during type lowering
    LowerStoreReturnEvent => ((), EventError::LowerStoreError)
}

generic_new_events! {
    /// A reallocation call event in the Component Model canonical ABI
    ///
    /// Usually performed during lowering of complex [`ComponentType`]s to Wasm
    ReallocEntryEvent {
        old_addr: usize,
        old_size: usize,
        old_align: u32,
        new_size: usize
    },

    LowerEntryEvent {
        ty: InterfaceType
    },

    LowerStoreEntryEvent {
        ty: InterfaceType,
        offset: usize
    },

    /// A mutable borrow of a slice of Wasm linear memory by the host
    ///
    /// This is the fundamental interface used during lowering of a [`ComponentType`].
    /// Note that this currently signifies a single mutable operation at the smallest granularity
    /// on a given linear memory slice. These can be optimized and coalesced into
    /// larger granularity operations in the future at either the recording or the replay level.
    MemorySliceBorrowEvent {
        offset: usize,
        size: usize
    }
}
