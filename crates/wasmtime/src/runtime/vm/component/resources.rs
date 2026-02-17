//! Implementation of the canonical-ABI related intrinsics for resources in the
//! component model.
//!
//! This module contains all the relevant gory details of the
//! component model related to lifting and lowering resources. For example
//! intrinsics like `resource.new` will bottom out in calling this file, and
//! this is where resource tables are actually defined and modified.
//!
//! The main types in this file are:
//!
//! * `ResourceTables` - the "here's everything" context which is required to
//!   perform canonical ABI operations.
//!
//! * `CallContext` - per-task information about active calls and borrows
//!   and runtime state tracking that to ensure that everything is handled
//!   correctly.
//!
//! Individual operations are exposed through methods on `ResourceTables` for
//! lifting/lowering/etc. This does mean though that some other fiddly bits
//! about ABI details can be found in lifting/lowering throughout Wasmtime,
//! namely in the `Resource<T>` and `ResourceAny` types.

use super::{HandleTable, InstanceState, RemovedResource};
use crate::component::store::ComponentTaskState;
use crate::prelude::*;
use core::error::Error;
use core::fmt;
use core::mem;
use wasmtime_environ::PrimaryMap;
use wasmtime_environ::component::{
    ComponentTypes, RuntimeComponentInstanceIndex, TypeResourceTableIndex,
};

/// Contextual state necessary to perform resource-related operations.
///
/// This state a bit odd since it has a few optional bits, but the idea is that
/// whenever this is constructed the bits required to perform operations are
/// always `Some`. For example:
///
/// * During lifting and lowering both `guest_table` and `host_table` are
///   `Some`.
/// * During wasm's own intrinsics only `guest_table` is `Some`.
/// * During embedder-invoked resource destruction calls only `host_table` is
///   `Some`.
///
/// This is all packaged up into one state though to make it easier to operate
/// on and to centralize handling of the state related to resources due to how
/// critical it is for correctness.
pub struct ResourceTables<'a> {
    /// Runtime state for all resources defined in a component.
    ///
    /// This is required whenever a `TypeResourceTableIndex`, for example, is
    /// provided as it's the lookup where that happens. Not present during
    /// embedder-originating operations though such as
    /// `ResourceAny::resource_drop` which won't consult this table as it's
    /// only operating over the host table.
    pub guest: Option<(
        &'a mut PrimaryMap<RuntimeComponentInstanceIndex, InstanceState>,
        &'a ComponentTypes,
    )>,

    /// Runtime state for resources currently owned by the host.
    ///
    /// This is the single table used by the host stored within `Store<T>`. Host
    /// resources will point into this and effectively have the same semantics
    /// as-if they're in-component resources. The major distinction though is
    /// that this is a heterogeneous table instead of only containing a single
    /// type.
    pub host_table: &'a mut HandleTable,

    /// Task information about calls actively in use to track information such
    /// as borrow counts.
    pub task_state: &'a mut ComponentTaskState,
}

/// Typed representation of a "rep" for a resource.
///
/// All resources in the component model are stored in a single heterogeneous
/// table so this type is used to disambiguate what everything is. This is the
/// representation of a resource stored at-rest in memory.
#[derive(Debug)]
pub enum TypedResource {
    /// A resource defined by the host.
    ///
    /// The meaning of the 32-bit integer here is up to the embedder, it
    /// otherwise is not used within the runtime here.
    Host(u32),

    /// A resource defined within a component.
    Component {
        /// This is an integer supplied by the component itself when this
        /// resource was created. Typically this is a pointer into linear
        /// memory for a component.
        rep: u32,

        /// The type of this component resource.
        ///
        /// This is used, within the context of a component, to keep track of
        /// what the type of `rep` is. This is then used when getting/removing
        /// from the table to ensure that the guest does indeed have the right
        /// permission to access this slot.
        ty: TypeResourceTableIndex,
    },
}

impl TypedResource {
    pub(super) fn rep(&self, access_ty: &TypedResourceIndex) -> Result<u32> {
        match (self, access_ty) {
            (Self::Host(rep), TypedResourceIndex::Host(_)) => Ok(*rep),
            (Self::Host(_), expected) => bail!(ResourceTypeMismatch {
                expected: *expected,
                found: "host resource",
            }),
            (Self::Component { rep, ty }, TypedResourceIndex::Component { ty: expected, .. }) => {
                if ty == expected {
                    Ok(*rep)
                } else {
                    bail!(ResourceTypeMismatch {
                        expected: *access_ty,
                        found: "a different guest-defined resource",
                    })
                }
            }
            (Self::Component { .. }, expected) => bail!(ResourceTypeMismatch {
                expected: *expected,
                found: "guest-defined resource",
            }),
        }
    }
}

/// An index used to access a resource.
///
/// This reflects how index operations are always accompanied not only with a
/// 32-bit index but additionally with a type. For example a `resource.drop`
/// intrinsic in a guest takes only a 32-bit integer argument, but it
/// inherently is used to only drop one type of resource which is additionally
/// ascribed here.
#[derive(Debug, Copy, Clone)]
pub enum TypedResourceIndex {
    /// A host resource at the given index is being accessed.
    Host(u32),

    /// A guest resource is being accessed.
    Component {
        /// The index supplied by the guest being accessed.
        index: u32,

        /// The fully-specific type of this resource.
        ty: TypeResourceTableIndex,
    },
}

impl TypedResourceIndex {
    pub(super) fn raw_index(&self) -> u32 {
        match self {
            Self::Host(index) | Self::Component { index, .. } => *index,
        }
    }

    fn desc(&self) -> &'static str {
        match self {
            Self::Host(_) => "host resource",
            Self::Component { .. } => "guest-defined resource",
        }
    }
}

/// State related to borrows for a specific call.
#[derive(Default)]
pub struct CallContext {
    lenders: Vec<TypedResourceIndex>,
    borrow_count: u32,
}

impl ResourceTables<'_> {
    fn table_for_resource(&mut self, resource: &TypedResource) -> &mut HandleTable {
        match resource {
            TypedResource::Host(_) => self.host_table,
            TypedResource::Component { ty, .. } => {
                let (states, types) = self.guest.as_mut().unwrap();
                states[types[*ty].unwrap_concrete_instance()].handle_table()
            }
        }
    }

    fn table_for_index(&mut self, index: &TypedResourceIndex) -> &mut HandleTable {
        match index {
            TypedResourceIndex::Host(_) => self.host_table,
            TypedResourceIndex::Component { ty, .. } => {
                let (states, types) = self.guest.as_mut().unwrap();
                states[types[*ty].unwrap_concrete_instance()].handle_table()
            }
        }
    }

    /// Implementation of the `resource.new` canonical intrinsic.
    ///
    /// Note that this is the same as `resource_lower_own`.
    pub fn resource_new(&mut self, resource: TypedResource) -> Result<u32> {
        self.table_for_resource(&resource)
            .resource_own_insert(resource)
    }

    /// Implementation of the `resource.rep` canonical intrinsic.
    ///
    /// This one's one of the simpler ones: "just get the rep please"
    pub fn resource_rep(&mut self, index: TypedResourceIndex) -> Result<u32> {
        self.table_for_index(&index).resource_rep(index)
    }

    /// Implementation of the `resource.drop` canonical intrinsic minus the
    /// actual invocation of the destructor.
    ///
    /// This will drop the handle at the `index` specified, removing it from
    /// the specified table. This operation can fail if:
    ///
    /// * The index is invalid.
    /// * The index points to an `own` resource which has active borrows.
    /// * The index's type is mismatched with the entry in the table's type.
    ///
    /// Otherwise this will return `Some(rep)` if the destructor for `rep` needs
    /// to run. If `None` is returned then that means a `borrow` handle was
    /// removed and no destructor is necessary.
    pub fn resource_drop(&mut self, index: TypedResourceIndex) -> Result<Option<u32>> {
        match self.table_for_index(&index).remove_resource(index)? {
            RemovedResource::Own { rep } => Ok(Some(rep)),
            RemovedResource::Borrow { scope } => {
                self.task_state.call_context(scope).borrow_count -= 1;
                Ok(None)
            }
        }
    }

    /// Inserts a new "own" handle into the specified table.
    ///
    /// This will insert the specified representation into the specified type
    /// table.
    ///
    /// Note that this operation is infallible, and additionally that this is
    /// the same as `resource_new` implementation-wise.
    ///
    /// This is an implementation of the canonical ABI `lower_own` function.
    pub fn resource_lower_own(&mut self, resource: TypedResource) -> Result<u32> {
        self.table_for_resource(&resource)
            .resource_own_insert(resource)
    }

    /// Attempts to remove an "own" handle from the specified table and its
    /// index.
    ///
    /// This operation will fail if `index` is invalid, if it's a `borrow`
    /// handle, if the own handle has currently been "lent" as a borrow, or if
    /// `index` has a different type in the table than the index.
    ///
    /// This is an implementation of the canonical ABI `lift_own` function.
    pub fn resource_lift_own(&mut self, index: TypedResourceIndex) -> Result<u32> {
        match self.table_for_index(&index).remove_resource(index)? {
            RemovedResource::Own { rep } => Ok(rep),
            RemovedResource::Borrow { .. } => bail!("cannot lift own resource from a borrow"),
        }
    }

    /// Extracts the underlying resource representation by lifting a "borrow"
    /// from the tables.
    ///
    /// This primarily employs dynamic tracking when a borrow is created from an
    /// "own" handle to ensure that the "own" handle isn't dropped while the
    /// borrow is active and additionally that when the current call scope
    /// returns the lend operation is undone.
    ///
    /// This is an implementation of the canonical ABI `lift_borrow` function.
    pub fn resource_lift_borrow(&mut self, index: TypedResourceIndex) -> Result<u32> {
        let (rep, is_own) = self.table_for_index(&index).resource_lend(index)?;
        if is_own {
            let scope = self.task_state.current_call_context_scope_id();
            self.task_state.call_context(scope).lenders.push(index);
        }
        Ok(rep)
    }

    /// Records a new `borrow` resource with the given representation within the
    /// current call scope.
    ///
    /// This requires that a call scope is active. Additionally the number of
    /// active borrows in the latest scope will be increased and must be
    /// decreased through a future use of `resource_drop` before the current
    /// call scope exits.
    ///
    /// This some of the implementation of the canonical ABI `lower_borrow`
    /// function. The other half of this implementation is located on
    /// `VMComponentContext` which handles the special case of avoiding borrow
    /// tracking entirely.
    pub fn resource_lower_borrow(&mut self, resource: TypedResource) -> Result<u32> {
        let scope = self.task_state.current_call_context_scope_id();
        let cx = self.task_state.call_context(scope);
        cx.borrow_count = cx.borrow_count.checked_add(1).unwrap();
        self.table_for_resource(&resource)
            .resource_borrow_insert(resource, scope)
    }

    /// Validates that the current scope can be exited.
    ///
    /// This will ensure that this context's active borrows have all been
    /// dropped. This will then commit the lend decrements back to the owned
    /// resources that were originally passed in.
    #[inline]
    pub fn validate_scope_exit(&mut self) -> Result<()> {
        let current = self.task_state.current_call_context_scope_id();
        let cx = self.task_state.call_context(current);
        if cx.borrow_count > 0 {
            bail!("borrow handles still remain at the end of the call")
        }
        for lender in mem::take(&mut cx.lenders) {
            // Note the panics here which should never get triggered in theory
            // due to the dynamic tracking of borrows and such employed for
            // resources.
            self.table_for_index(&lender)
                .resource_undo_lend(lender)
                .unwrap();
        }
        Ok(())
    }
}

#[derive(Debug)]
struct ResourceTypeMismatch {
    expected: TypedResourceIndex,
    found: &'static str,
}

impl fmt::Display for ResourceTypeMismatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "handle index {} used with the wrong type, \
             expected {} but found {}",
            self.expected.raw_index(),
            self.expected.desc(),
            self.found,
        )
    }
}

impl Error for ResourceTypeMismatch {}
