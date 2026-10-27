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

use super::{HandleCounter, HandleTable, InstanceState, RemovedResource};
use crate::bail_bug;
#[cfg(feature = "component-model-async")]
use crate::bug;
use crate::component::store::ComponentTaskState;
use crate::prelude::*;
#[cfg(feature = "component-model-async")]
use crate::runtime::vm::VMResourceIndex;
use core::error::Error;
use core::fmt;
use core::mem;
use wasmtime_environ::component::{
    ComponentTypes, RuntimeComponentInstanceIndex, TypeResourceTableIndex,
};
use wasmtime_environ::prelude::TryPrimaryMap;

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
        &'a mut TryPrimaryMap<RuntimeComponentInstanceIndex, InstanceState>,
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

    /// The store-wide live-handle counter, charged by every insertion into and
    /// removal from any of the tables above.
    pub handle_counter: &'a mut HandleCounter,

    /// Task information about calls actively in use to track information such
    /// as borrow counts.
    pub task_state: &'a mut ComponentTaskState,

    /// The current scope, used to mutate borrows and lenders for the call.
    pub current_scope: Option<CurrentScope>,

    /// The store's `lazy_lenders` buffer and its occupied length.
    ///
    /// `None` when the caller did not opt into lazy lends, in which case
    /// `current_scope` is never `DeferredGuest`.
    #[cfg(feature = "component-model-async")]
    pub lazy_lenders: Option<(&'a mut [VMResourceIndex], &'a mut usize)>,
}

/// The resource-borrow scope associated with the current component call.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum CurrentScope {
    /// A scope which already has an ID in the component task state.
    Id(Scope),
    /// A host scope whose task has not yet been materialized.
    #[cfg_attr(not(feature = "component-model-async"), allow(dead_code))]
    DeferredHost,
    /// A guest scope whose `VMDeferredThread` has not been forced.
    #[cfg(feature = "component-model-async")]
    DeferredGuest,
}

/// Identifier for a component call's resource-borrow scope.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Scope {
    /// A non-concurrent scope or a concurrent guest-task scope.
    Id(u32),
    /// A concurrent host-task scope.
    HostId(u32),
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
    lenders: TryVec<TypedResourceIndex>,
    borrow_count: u32,
}

impl CallContext {
    #[cfg_attr(not(feature = "component-model-async"), allow(dead_code))]
    pub(crate) fn is_empty(&self) -> bool {
        self.lenders.is_empty() && self.borrow_count == 0
    }

    /// Reserves room so that the matching `push_lender` calls cannot fail.
    #[cfg(feature = "component-model-async")]
    pub(crate) fn reserve_lenders(&mut self, additional: usize) -> Result<()> {
        self.lenders.reserve(additional)?;
        Ok(())
    }

    /// Records `index` as lent out, in space reserved by `reserve_lenders`.
    #[cfg(feature = "component-model-async")]
    pub(crate) fn push_lender(&mut self, index: TypedResourceIndex) {
        debug_assert!(self.lenders.capacity() - self.lenders.len() > 0);
        self.lenders
            .push(index)
            .expect("space reserved by `reserve_lenders`");
    }
}

impl ResourceTables<'_> {
    fn table_for_resource(
        &mut self,
        resource: &TypedResource,
    ) -> (&mut HandleTable, &mut HandleCounter) {
        let table = match resource {
            TypedResource::Host(_) => &mut *self.host_table,
            TypedResource::Component { ty, .. } => {
                let (states, types) = self.guest.as_mut().unwrap();
                states[types[*ty].unwrap_concrete_instance()].handle_table()
            }
        };
        (table, &mut *self.handle_counter)
    }

    fn table_for_index(
        &mut self,
        index: &TypedResourceIndex,
    ) -> (&mut HandleTable, &mut HandleCounter) {
        let table = match index {
            TypedResourceIndex::Host(_) => &mut *self.host_table,
            TypedResourceIndex::Component { ty, .. } => {
                let (states, types) = self.guest.as_mut().unwrap();
                states[types[*ty].unwrap_concrete_instance()].handle_table()
            }
        };
        (table, &mut *self.handle_counter)
    }

    /// Implementation of the `resource.new` canonical intrinsic.
    ///
    /// Note that this is the same as `resource_lower_own`.
    pub fn resource_new(&mut self, resource: TypedResource) -> Result<u32> {
        let (table, counter) = self.table_for_resource(&resource);
        table.resource_own_insert(counter, resource)
    }

    /// Implementation of the `resource.rep` canonical intrinsic.
    ///
    /// This one's one of the simpler ones: "just get the rep please"
    pub fn resource_rep(&mut self, index: TypedResourceIndex) -> Result<u32> {
        self.table_for_index(&index).0.resource_rep(index)
    }

    /// Implementation of the `resource.drop` canonical intrinsic minus the
    /// actual invocation of the destructor.
    ///
    /// This will drop the handle at the `index` specified, removing it from
    /// the specified table. This operation can fail if:
    ///
    /// * The index is invalid.
    /// * The index points to a resource which has been lent out.
    /// * The index's type is mismatched with the entry in the table's type.
    ///
    /// Otherwise this will return `Some(rep)` if the destructor for `rep` needs
    /// to run. If `None` is returned then that means a `borrow` handle was
    /// removed and no destructor is necessary.
    pub fn resource_drop(&mut self, index: TypedResourceIndex) -> Result<Option<u32>> {
        let (table, counter) = self.table_for_index(&index);
        match table.remove_resource(counter, index)? {
            RemovedResource::Own { rep } => Ok(Some(rep)),
            RemovedResource::Borrow { scope } => {
                self.task_state.call_context(scope)?.borrow_count -= 1;
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
        let (table, counter) = self.table_for_resource(&resource);
        table.resource_own_insert(counter, resource)
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
        let (table, counter) = self.table_for_index(&index);
        match table.remove_resource(counter, index)? {
            RemovedResource::Own { rep } => Ok(rep),
            RemovedResource::Borrow { .. } => bail!("cannot lift own resource from a borrow"),
        }
    }

    fn materialize_current_scope(&mut self) -> Result<Scope> {
        let id = match self.current_scope {
            Some(CurrentScope::Id(id)) => return Ok(id),
            Some(CurrentScope::DeferredHost) => self.task_state.materialize_current_scope()?,
            // Forcing needs the whole store, so it cannot happen here. Callers
            // needing a real `Scope` force before building these tables, so
            // this scope never reaches them.
            #[cfg(feature = "component-model-async")]
            Some(CurrentScope::DeferredGuest) => {
                bail_bug!("deferred guest scope cannot be materialized here")
            }
            None => bail_bug!("no current scope"),
        };
        self.current_scope = Some(CurrentScope::Id(id));
        Ok(id)
    }

    /// Returns the current call's resource-borrow scope.
    ///
    /// Unlike [`Self::materialize_current_scope`], this does not materialize a
    /// deferred host task.
    fn current_scope(&mut self) -> Result<&mut CallContext> {
        let id = match self.current_scope {
            Some(CurrentScope::Id(id)) => id,
            Some(CurrentScope::DeferredHost) => {
                // With a deferred host task the `CallContext` for this task
                // could be stored in one of two locations. The primary location
                // is `task_state.deferred_host_call_context()` but that stops
                // being used once the host task is actually materialized. On
                // materialization this over here isn't updated necessarily, so
                // this'll only lazily pick up when the host task is actually
                // materialized. In such a situation that's detected here and
                // `self.current_scope` is mutated to match.
                //
                // NB: this should use `if let` when rustc is smarter about
                // borrows.
                if self.task_state.deferred_host_call_context().is_some() {
                    return Ok(self.task_state.deferred_host_call_context().unwrap());
                }
                let id = self.task_state.materialize_current_scope()?;
                self.current_scope = Some(CurrentScope::Id(id));
                id
            }
            // These lends live in the `lazy_lenders` buffer instead; see
            // `reserve_lender`. Wanting a real `CallContext` means the thread
            // must be forced first.
            #[cfg(feature = "component-model-async")]
            Some(CurrentScope::DeferredGuest) => {
                bail_bug!("deferred guest scope has no call context")
            }
            None => bail_bug!("no current scope"),
        };
        self.task_state.call_context(id)
    }

    /// Reserves room to record a lender, before the caller performs the lend,
    /// so that `push_lender` cannot fail afterwards.
    ///
    /// Returns `false` if the `lazy_lenders` buffer is full, in which case the
    /// caller must force the thread and retry against a `CallContext`.
    fn reserve_lender(&mut self) -> Result<bool> {
        #[cfg(feature = "component-model-async")]
        if let Some(CurrentScope::DeferredGuest) = self.current_scope {
            let (buf, len) = self
                .lazy_lenders
                .as_mut()
                .ok_or_else(|| bug!("deferred guest scope without a lazy lenders buffer"))?;
            return Ok(**len < buf.len());
        }
        self.current_scope()?.lenders.reserve(1)?;
        Ok(true)
    }

    /// Commits a lender reserved by `reserve_lender`.
    fn push_lender(&mut self, index: TypedResourceIndex) -> Result<()> {
        // If we have not forced the task yet, then avoid doing so now and just
        // push to the lazy-lenders buffer instead.
        #[cfg(feature = "component-model-async")]
        if let Some(CurrentScope::DeferredGuest) = self.current_scope {
            let (buf, len) = self
                .lazy_lenders
                .as_mut()
                .ok_or_else(|| bug!("deferred guest scope without a lazy lenders buffer"))?;

            if **len >= buf.len() {
                bail_bug!("should have always called `reserve_lender` first");
            }

            buf[**len] = VMResourceIndex::pack(index)?;
            **len += 1;
            return Ok(());
        }

        self.current_scope()?.lenders.push(index)?;
        Ok(())
    }

    /// Extracts the underlying resource representation by lifting a "borrow"
    /// from the tables.
    ///
    /// This primarily employs dynamic tracking when a borrow is created from
    /// either an "own" or a "borrow" handle to ensure that the lending handle
    /// isn't dropped while the borrow is active and additionally that when the
    /// current call scope returns the lend operation is undone.
    ///
    /// This is an implementation of the canonical ABI `lift_borrow` function.
    ///
    /// Callers must either force the deferred thread, if any, first or check
    /// for and handle `Err(LazyLendersFull)`; see its docs for details.
    pub fn resource_lift_borrow(&mut self, index: TypedResourceIndex) -> Result<u32> {
        // FIXME: the logic here applies to all resources, both own and borrow,
        // although tracking `borrow` here isn't always necessary for sync
        // tasks. Ideally the `lenders.push` would be skipped for sync tasks but
        // that requires a bit more plumbing and also appropriately tracking the
        // `lend_count` and such. For now this treats own/borrow the same way.
        //
        // Space for the lender is reserved first so that recording it can't
        // fail after the lend.
        if !self.reserve_lender()? {
            // Nothing lent yet, so the caller can force the deferred task and
            // retry.
            #[cfg(feature = "component-model-async")]
            return Err(LazyLendersFull.into());
            // `reserve_lender` can only return `false` (meaning the
            // lazy-lenders buffer is full) when the `component-model-async`
            // feature is enabled.
            #[cfg(not(feature = "component-model-async"))]
            bail_bug!("`reserve_lender` only fails for lazy lends");
        }
        let rep = self.table_for_index(&index).0.resource_lend(index)?;
        self.push_lender(index)?;
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
        let scope = self.materialize_current_scope()?;
        let cx = self.task_state.call_context(scope)?;
        cx.borrow_count = cx.borrow_count.checked_add(1).unwrap();
        let (table, counter) = self.table_for_resource(&resource);
        table.resource_borrow_insert(counter, resource, scope)
    }

    /// Undoes the lazy lends recorded above `watermark`, truncating the buffer
    /// back to it, for a deferred frame exiting without ever having been
    /// forced.
    ///
    /// Unlike `validate_scope_exit` this needs no borrow scope, which is what
    /// lets the frame exit without materializing a thread. There is no
    /// `borrow_count` to check: lowering a borrow requires a materialized
    /// scope, so such a frame has none.
    #[cfg(feature = "component-model-async")]
    pub fn undo_lazy_lends(&mut self, watermark: usize) -> Result<()> {
        use smallvec::{SmallVec, ToSmallVec};

        // Only an unforced deferred guest frame records into the buffer, so
        // any other scope here means the caller mismatched the recording and
        // undoing paths and would truncate entries belonging to someone else.
        if !matches!(self.current_scope, Some(CurrentScope::DeferredGuest)) {
            bail_bug!(
                "undoing lazy lends outside a deferred guest scope: {:?}",
                self.current_scope
            );
        }

        let (buf, len) = self
            .lazy_lenders
            .as_mut()
            .ok_or_else(|| bug!("no lazy lenders buffer to undo lends from"))?;
        if watermark > **len {
            bail_bug!(
                "lazy lenders watermark {watermark} is beyond the buffer length {}",
                **len
            );
        }

        let lenders: SmallVec<[_; 8]> = buf[watermark..**len].to_smallvec();
        **len = watermark;
        for packed in lenders {
            let lender = packed.unpack();
            self.table_for_index(&lender)
                .0
                .resource_undo_lend(lender)
                .unwrap();
        }
        Ok(())
    }

    /// Validates that the current scope can be exited.
    ///
    /// This will ensure that this context's active borrows have all been
    /// dropped. This will then commit the lend decrements back to the owned
    /// resources that were originally passed in.
    #[inline]
    pub fn validate_scope_exit(&mut self) -> Result<()> {
        let cx = self.current_scope()?;
        if cx.borrow_count > 0 {
            bail!("borrow handles still remain at the end of the call")
        }
        for lender in mem::take(&mut cx.lenders) {
            // Note the panics here which should never get triggered in theory
            // due to the dynamic tracking of borrows and such employed for
            // resources.
            self.table_for_index(&lender)
                .0
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

/// The `lazy_lenders` buffer had no room, so the caller must force the deferred
/// thread and retry against the newly-constructed thread's `CallContext`.
///
/// This is raised before anything is actually lent, so no side effects have
/// happened yet, and it should never reach a guest: always caught in host
/// functions instead.
#[cfg(feature = "component-model-async")]
#[derive(Debug)]
pub struct LazyLendersFull;

#[cfg(feature = "component-model-async")]
impl fmt::Display for LazyLendersFull {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("lazy lenders buffer is full")
    }
}

#[cfg(feature = "component-model-async")]
impl Error for LazyLendersFull {}
