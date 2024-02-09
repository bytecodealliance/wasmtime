use crate::component::func::{bad_type_info, desc, LiftContext, LowerContext};
use crate::component::instance::RuntimeImport;
use crate::component::linker::ResourceImportIndex;
use crate::component::matching::InstanceType;
use crate::component::{ComponentType, InstancePre, Lift, Lower};
use crate::store::{StoreId, StoreOpaque};
use crate::{AsContextMut, StoreContextMut, Trap};
use anyhow::{bail, ensure, Context, Result};
use std::any::TypeId;
use std::fmt;
use std::marker;
use std::mem::MaybeUninit;
use std::sync::atomic::{AtomicU32, Ordering::Relaxed};
use wasmtime_environ::component::{
    CanonicalAbiInfo, DefinedResourceIndex, InterfaceType, TypeResourceTableIndex,
};
use wasmtime_runtime::component::{ComponentInstance, InstanceFlags, ResourceTables};
use wasmtime_runtime::{SendSyncPtr, VMFuncRef, ValRaw};

/// Representation of a resource type in the component model.
///
/// Resources are currently always represented as 32-bit integers but they have
/// unique types across instantiations and the host. For example instantiating
/// the same component twice means that defined resource types in the component
/// will all be different. Values of this type can be compared to see if
/// resources have the same type.
///
/// Resource types can also be defined on the host in addition to guests. On the
/// host resource types are tied to a `T`, an arbitrary Rust type. Two host
/// resource types are the same if they point to the same `T`.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct ResourceType {
    kind: ResourceTypeKind,
}

impl ResourceType {
    /// Creates a new host resource type corresponding to `T`.
    ///
    /// Note that `T` is a mostly a phantom type parameter here. It does not
    /// need to reflect the actual storage of the resource `T`. For example this
    /// is valid:
    ///
    /// ```rust
    /// use wasmtime::component::ResourceType;
    ///
    /// struct Foo;
    ///
    /// let ty = ResourceType::host::<Foo>();
    /// ```
    ///
    /// A resource type of type `ResourceType::host::<T>()` will match the type
    /// of the value produced by `Resource::<T>::new_{own,borrow}`.
    pub fn host<T: 'static>() -> ResourceType {
        ResourceType {
            kind: ResourceTypeKind::Host(TypeId::of::<T>()),
        }
    }

    pub(crate) fn guest(
        store: StoreId,
        instance: &ComponentInstance,
        id: DefinedResourceIndex,
    ) -> ResourceType {
        ResourceType {
            kind: ResourceTypeKind::Guest {
                store,
                instance: instance as *const _ as usize,
                id,
            },
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum ResourceTypeKind {
    Host(TypeId),
    Guest {
        store: StoreId,
        // For now this is the `*mut ComponentInstance` pointer within the store
        // that this guest corresponds to. It's used to distinguish different
        // instantiations of the same component within the store.
        instance: usize,
        id: DefinedResourceIndex,
    },
}

/// A host-defined resource in the component model.
///
/// This type can be thought of as roughly a newtype wrapper around `u32` for
/// use as a resource with the component model. The main guarantee that the
/// component model provides is that the `u32` is not forgeable by guests and
/// there are guaranteed semantics about when a `u32` may be in use by the guest
/// and when it's guaranteed no longer needed. This means that it is safe for
/// embedders to consider the internal `u32` representation "trusted" and use it
/// for things like table indices with infallible accessors that panic on
/// out-of-bounds. This should only panic for embedder bugs, not because of any
/// possible behavior in the guest.
///
/// A `Resource<T>` value dynamically represents both an `(own $t)` in the
/// component model as well as a `(borrow $t)`. It can be inspected via
/// [`Resource::owned`] to test whether it is an owned handle. An owned handle
/// which is not actively borrowed can be destroyed at any time as it's
/// guaranteed that the guest does not have access to it. A borrowed handle, on
/// the other hand, may be accessed by the guest so it's not necessarily
/// guaranteed to be able to be destroyed.
///
/// Note that the "own" and "borrow" here refer to the component model, not
/// Rust. The semantics of Rust ownership and borrowing are slightly different
/// than the component model's (but spiritually the same) in that more dynamic
/// tracking is employed as part of the component model. This means that it's
/// possible to get runtime errors when using a `Resource<T>`. For example it is
/// an error to call [`Resource::new_borrow`] and pass that to a component
/// function expecting `(own $t)` and this is not statically disallowed.
///
/// The [`Resource`] type implements both the [`Lift`] and [`Lower`] trait to be
/// used with typed functions in the component model or as part of aggregate
/// structures and datatypes.
///
/// # Destruction of a resource
///
/// Resources in the component model are optionally defined with a destructor,
/// but this host resource type does not specify a destructor. It is left up to
/// the embedder to be able to determine how best to a destroy a resource when
/// it is owned.
///
/// Note, though, that while [`Resource`] itself does not specify destructors
/// it's still possible to do so via the [`Linker::resource`] definition. When a
/// resource type is defined for a guest component a destructor can be specified
/// which can be used to hook into resource destruction triggered by the guest.
///
/// This means that there are two ways that resource destruction is handled:
///
/// * Host resources destroyed by the guest can hook into the
///   [`Linker::resource`] destructor closure to handle resource destruction.
///   This could, for example, remove table entries.
///
/// * Host resources owned by the host itself have no automatic means of
///   destruction. The host can make its own determination that its own resource
///   is not lent out to the guest and at that time choose to destroy or
///   deallocate it.
///
/// # Dynamic behavior of a resource
///
/// A host-defined [`Resource`] does not necessarily represent a static value.
/// Its internals may change throughout its usage to track the state associated
/// with the resource. The internal 32-bit host-defined representation never
/// changes, however.
///
/// For example if there's a component model function of the form:
///
/// ```wasm
/// (func (param "a" (borrow $t)) (param "b" (own $t)))
/// ```
///
/// Then that can be extracted in Rust with:
///
/// ```rust,ignore
/// let func = instance.get_typed_func::<(&Resource<T>, &Resource<T>), ()>(&mut store, "name")?;
/// ```
///
/// Here the exact same resource can be provided as both arguments but that is
/// not valid to do so because the same resource cannot be actively borrowed and
/// passed by-value as the second parameter at the same time. The internal state
/// in `Resource<T>` will track this information and provide a dynamic runtime
/// error in this situation.
///
/// Mostly it's important to be aware that there is dynamic state associated
/// with a [`Resource<T>`] to provide errors in situations that cannot be
/// statically ruled out.
///
/// # Borrows and host responsibilities
///
/// Borrows to resources in the component model are guaranteed to be transient
/// such that if a borrow is passed as part of a function call then when the
/// function returns it's guaranteed that the guest no longer has access to the
/// resource. This guarantee, however, must be manually upheld by the host when
/// it receives its own borrow.
///
/// As mentioned above the [`Resource<T>`] type can represent a borrowed value
/// in addition to an owned value. This means a guest can provide the host with
/// a borrow, such as an argument to an imported function:
///
/// ```rust,ignore
/// linker.root().func_wrap("name", |_cx, (r,): (Resource<MyType>,)| {
///     assert!(!r.owned());
///     // .. here `r` is a borrowed value provided by the guest and the host
///     // shouldn't continue to access it beyond the scope of this call
/// })?;
/// ```
///
/// In this situation the host should take care to not attempt to persist the
/// resource beyond the scope of the call. It's the host's resource so it
/// technically can do what it wants with it but nothing is statically
/// preventing `r` to stay pinned to the lifetime of the closure invocation.
/// It's considered a mistake that the host performed if `r` is persisted too
/// long and accessed at the wrong time.
///
/// [`Linker::resource`]: crate::component::LinkerInstance::resource
pub struct Resource<T> {
    /// The host-defined 32-bit representation of this resource.
    rep: u32,

    /// Dear rust please consider `T` used even though it's not actually used.
    _marker: marker::PhantomData<fn() -> T>,

    /// Internal dynamic state tracking for this resource. This can be one of
    /// four different states:
    ///
    /// * `BORROW` / `u32::MAX` - this indicates that this is a borrowed
    ///   resource. The `rep` doesn't live in the host table and this `Resource`
    ///   instance is transiently available. It's the host's responsibility to
    ///   discard this resource when the borrow duration has finished.
    ///
    /// * `NOT_IN_TABLE` / `u32::MAX - 1` - this indicates that this is an owned
    ///   resource not present in any store's table. This resource is not lent
    ///   out. It can be passed as an `(own $t)` directly into a guest's table
    ///   or it can be passed as a borrow to a guest which will insert it into
    ///   a host store's table for dynamic borrow tracking.
    ///
    /// * `TAKEN` / `u32::MAX - 2` - while the `rep` is available the resource
    ///   has been dynamically moved into a guest and cannot be moved in again.
    ///   This is used for example to prevent the same resource from being
    ///   passed twice to a guest.
    ///
    /// * All other values - any other value indicates that the value is an
    ///   index into a store's table of host resources. It's guaranteed that the
    ///   table entry represents a host resource and the resource may have
    ///   borrow tracking associated with it.
    ///
    /// Note that this is an `AtomicU32` but it's not intended to actually be
    /// used in conjunction with threads as generally a `Store<T>` lives on one
    /// thread at a time. The `AtomicU32` here is used to ensure that this type
    /// is `Send + Sync` when captured as a reference to make async programming
    /// more ergonomic.
    state: AtomicU32,
}

// See comments on `state` above for info about these values.
const BORROW: u32 = u32::MAX;
const NOT_IN_TABLE: u32 = u32::MAX - 1;
const TAKEN: u32 = u32::MAX - 2;

/// TODO
pub struct HostResourceTables<'a> {
    tables: ResourceTables<'a>,
    host_resource_types: &'a mut Vec<ResourceType>,
}

struct UnusedHostTableSlot;

impl<'a> HostResourceTables<'a> {
    pub fn new_host(store: &'a mut StoreOpaque) -> HostResourceTables<'_> {
        let (calls, host_table, host_resource_types) = store.component_resource_state();
        HostResourceTables::from_parts(
            ResourceTables {
                host_table: Some(host_table),
                calls,
                tables: None,
            },
            host_resource_types,
        )
    }

    pub fn from_parts(
        tables: ResourceTables<'a>,
        host_resource_types: &'a mut Vec<ResourceType>,
    ) -> Self {
        HostResourceTables {
            tables,
            host_resource_types,
        }
    }

    /// Lifts an `own` resource that resides in the host's tables at the `idx`
    /// specified into its `rep`.
    ///
    /// This method additionally takes an `expected` type which the resource is
    /// expected to have. All host resources are stored into a single table so
    /// this is used to perform a runtime check to ensure that the resource
    /// still has the same type as when it was originally inserted.
    ///
    /// # Errors
    ///
    /// Returns an error if `idx` doesn't point to a valid owned resource, if
    /// `idx` can't be lifted as an `own` (e.g. it has active borrows), or if
    /// the resource at `idx` does not have the type `expected`.
    pub fn host_resource_lift_own(&mut self, idx: u32, expected: ResourceType) -> Result<u32> {
        self.validate_host_type(idx, expected)?;
        self.tables.resource_lift_own(None, idx)
    }

    /// See [`HostResourceTables::host_resource_lift_own`].
    pub fn host_resource_lift_borrow(&mut self, idx: u32, expected: ResourceType) -> Result<u32> {
        self.validate_host_type(idx, expected)?;
        self.tables.resource_lift_borrow(None, idx)
    }

    /// Lowers an `own` resource to be owned by the host.
    ///
    /// This returns a new index into the host's set of resource tables which
    /// will point to the `rep` specified as well as recording that it has the
    /// `ty` specified. The returned index is suitable for conversion into
    /// either [`Resource`] or [`ResourceAny`].
    pub fn host_resource_lower_own(&mut self, rep: u32, ty: ResourceType) -> u32 {
        let idx = self.tables.resource_lower_own(None, rep);
        self.register_host_type(idx, ty);
        idx
    }

    /// See [`HostResourceTables::host_resource_lower_own`].
    pub fn host_resource_lower_borrow(&mut self, rep: u32, ty: ResourceType) -> u32 {
        let idx = self.tables.resource_lower_borrow(None, rep);
        self.register_host_type(idx, ty);
        idx
    }

    /// Validates that the host resource at `idx` has the `expected` type.
    ///
    /// If `idx` is out-of-bounds or not actively being used then this method
    /// does not return an error. That is deferred to retun an error via the
    /// lift/drop operation corresponding to this method to return a more
    /// precise error.
    fn validate_host_type(&mut self, idx: u32, expected: ResourceType) -> Result<()> {
        let actual = usize::try_from(idx)
            .ok()
            .and_then(|i| self.host_resource_types.get(i).copied());

        // If `idx` is out-of-bounds, or if the slot is known as being
        // not-in-use (e.g. dropped by the host) then skip returning an error.
        // In such a situation the operation that this is guarding will return a
        // more precise error, such as a lift operation.
        if let Some(actual) = actual {
            if actual != expected && actual != ResourceType::host::<UnusedHostTableSlot>() {
                bail!("host-owned resource is being used with the wrong type");
            }
        }
        Ok(())
    }

    fn register_host_type(&mut self, idx: u32, ty: ResourceType) {
        let idx = idx as usize;
        match self.host_resource_types.get_mut(idx) {
            Some(slot) => *slot = ty,
            None => {
                assert_eq!(idx, self.host_resource_types.len());
                self.host_resource_types.push(ty);
            }
        }
    }

    /// Drops a host-owned resource from host tables.
    ///
    /// This method will attempt to interpret `idx` as pointing to either a
    /// `borrow` or `own` resource with the `expected` type specified. This
    /// method will then return the underlying `rep` if it points to an `own`
    /// resource which can then be further processed for destruction.
    ///
    /// # Errors
    ///
    /// Returns an error if `idx` doesn't point to a valid resource, points to
    /// an `own` with active borrows, or if it doesn't have the type `expected`
    /// in the host tables.
    pub fn host_resource_drop(&mut self, idx: u32, expected: ResourceType) -> Result<Option<u32>> {
        self.validate_host_type(idx, expected)?;
        let ret = self.tables.resource_drop(None, idx);
        if ret.is_ok() {
            self.host_resource_types[idx as usize] = ResourceType::host::<UnusedHostTableSlot>();
        }
        ret
    }

    /// Lowers an `own` resource into the guest, converting the `rep` specified
    /// into a guest-local index.
    ///
    /// The `ty` provided is which table to put this into.
    pub fn guest_resource_lower_own(&mut self, rep: u32, ty: TypeResourceTableIndex) -> u32 {
        self.tables.resource_lower_own(Some(ty), rep)
    }

    /// Lowers a `borrow` resource into the guest, converting the `rep`
    /// specified into a guest-local index.
    ///
    /// The `ty` provided is which table to put this into.
    ///
    /// Note that this cannot be used in isolation because lowering a borrow
    /// into a guest has a special case where `rep` is returned directly if `ty`
    /// belongs to the component being lowered into. That property must be
    /// handled by the caller of this function.
    pub fn guest_resource_lower_borrow(&mut self, rep: u32, ty: TypeResourceTableIndex) -> u32 {
        self.tables.resource_lower_borrow(Some(ty), rep)
    }

    /// Lifts an `own` resource from the `idx` specified from the table `ty`.
    ///
    /// This will lookup the appropriate table in the guest and return the `rep`
    /// corresponding to `idx` if it's valid.
    pub fn guest_resource_lift_own(&mut self, idx: u32, ty: TypeResourceTableIndex) -> Result<u32> {
        self.tables.resource_lift_own(Some(ty), idx)
    }

    /// Lifts a `borrow` resource from the `idx` specified from the table `ty`.
    ///
    /// This will lookup the appropriate table in the guest and return the `rep`
    /// corresponding to `idx` if it's valid.
    pub fn guest_resource_lift_borrow(
        &mut self,
        idx: u32,
        ty: TypeResourceTableIndex,
    ) -> Result<u32> {
        self.tables.resource_lift_borrow(Some(ty), idx)
    }

    /// Begins a call into the component instance, starting recording of
    /// metadata related to resource borrowing.
    #[inline]
    pub fn enter_call(&mut self) {
        self.tables.enter_call()
    }

    /// Completes a call into the component instance, validating that it's ok to
    /// complete by ensuring the are no remaining active borrows.
    #[inline]
    pub fn exit_call(&mut self) -> Result<()> {
        self.tables.exit_call()
    }
}

// fn host_resource_tables(store: &mut StoreOpaque) -> ResourceTables<'_> {
//     let (calls, host_table, _) = store.component_resource_state();
//     ResourceTables {
//         calls,
//         host_table: Some(host_table),
//         tables: None,
//     }
// }

impl<T> Resource<T>
where
    T: 'static,
{
    /// Creates a new owned resource with the `rep` specified.
    ///
    /// The returned value is suitable for passing to a guest as either a
    /// `(borrow $t)` or `(own $t)`.
    pub fn new_own(rep: u32) -> Resource<T> {
        Resource {
            state: AtomicU32::new(NOT_IN_TABLE),
            rep,
            _marker: marker::PhantomData,
        }
    }

    /// Creates a new borrowed resource which isn't actually rooted in any
    /// ownership.
    ///
    /// This can be used to pass to a guest as a borrowed resource and the
    /// embedder will know that the `rep` won't be in use by the guest
    /// afterwards. Exactly how the lifetime of `rep` works is up to the
    /// embedder.
    pub fn new_borrow(rep: u32) -> Resource<T> {
        Resource {
            state: AtomicU32::new(BORROW),
            rep,
            _marker: marker::PhantomData,
        }
    }

    /// Returns the underlying 32-bit representation used to originally create
    /// this resource.
    pub fn rep(&self) -> u32 {
        self.rep
    }

    /// Returns whether this is an owned resource or not.
    ///
    /// Owned resources can be safely destroyed by the embedder at any time, and
    /// borrowed resources have an owner somewhere else on the stack so can only
    /// be accessed, not destroyed.
    pub fn owned(&self) -> bool {
        match self.state.load(Relaxed) {
            BORROW => false,
            _ => true,
        }
    }

    fn lower_to_index<U>(&self, cx: &mut LowerContext<'_, U>, ty: InterfaceType) -> Result<u32> {
        let rty = ResourceType::host::<T>();
        match ty {
            InterfaceType::Own(t) => {
                let rep = match self.state.load(Relaxed) {
                    // If this is a borrow resource then this is a dynamic
                    // error on behalf of the embedder.
                    BORROW => {
                        bail!("cannot lower a `borrow` resource into an `own`")
                    }

                    // If this resource does not yet live in a table then we're
                    // dynamically transferring ownership to wasm. Record that
                    // it's no longer present and then pass through the
                    // representation.
                    NOT_IN_TABLE => {
                        let prev = self.state.swap(TAKEN, Relaxed);
                        assert_eq!(prev, NOT_IN_TABLE);
                        self.rep
                    }

                    // This resource has already been moved into wasm so this is
                    // a dynamic error on behalf of the embedder.
                    TAKEN => bail!("host resource already consumed"),

                    // If this resource lives in a host table then try to take
                    // it out of the table, which may fail, and on success we
                    // can move the rep into the guest table.
                    idx => cx.host_resource_lift_own(idx, rty)?,
                };
                Ok(cx.guest_resource_lower_own(t, rep))
            }
            InterfaceType::Borrow(t) => {
                let rep = match self.state.load(Relaxed) {
                    // If this is already a borrowed resource, nothing else to
                    // do and the rep is passed through.
                    BORROW => self.rep,

                    // If this resource is already gone, that's a dynamic error
                    // for the embedder.
                    TAKEN => bail!("host resource already consumed"),

                    // If this resource is not currently in a table then it
                    // needs to move into a table to participate in state
                    // related to borrow tracking. Execute the
                    // `host_resource_lower_own` operation here and update our
                    // state.
                    //
                    // Afterwards this is the same as the `idx` case below.
                    NOT_IN_TABLE => {
                        let idx = cx.host_resource_lower_own(self.rep, rty);
                        let prev = self.state.swap(idx, Relaxed);
                        assert_eq!(prev, NOT_IN_TABLE);
                        cx.host_resource_lift_borrow(idx, rty)?
                    }

                    // If this resource lives in a table then it needs to come
                    // out of the table with borrow-tracking employed.
                    idx => cx.host_resource_lift_borrow(idx, ResourceType::host::<T>())?,
                };
                Ok(cx.guest_resource_lower_borrow(t, rep))
            }
            _ => bad_type_info(),
        }
    }

    fn lift_from_index(cx: &mut LiftContext<'_>, ty: InterfaceType, index: u32) -> Result<Self> {
        let (state, rep) = match ty {
            // Ownership is being transferred from a guest to the host, so move
            // it from the guest table into a new `Resource`. Note that this
            // isn't immediately inserted into the host table and that's left
            // for the future if it's necessary to take a borrow from this owned
            // resource.
            InterfaceType::Own(t) => {
                debug_assert!(cx.resource_type(t) == ResourceType::host::<T>());
                let (rep, dtor, flags) = cx.guest_resource_lift_own(t, index)?;
                assert!(dtor.is_some());
                assert!(flags.is_none());
                (AtomicU32::new(NOT_IN_TABLE), rep)
            }

            // The borrow here is lifted from the guest, but note the lack of
            // `host_resource_lower_borrow` as it's intentional. Lowering
            // a borrow has a special case in the canonical ABI where if the
            // receiving module is the owner of the resource then it directly
            // receives the `rep` and no other dynamic tracking is employed.
            // This effectively mirrors that even though the canonical ABI
            // isn't really all that applicable in host context here.
            InterfaceType::Borrow(t) => {
                debug_assert!(cx.resource_type(t) == ResourceType::host::<T>());
                let rep = cx.guest_resource_lift_borrow(t, index)?;
                (AtomicU32::new(BORROW), rep)
            }
            _ => bad_type_info(),
        };
        Ok(Resource {
            state,
            rep,
            _marker: marker::PhantomData,
        })
    }

    /// Attempts to convert a [`ResourceAny`] into [`Resource`].
    ///
    /// This method will check that `resource` has type
    /// `ResourceType::host::<T>()` and then convert it into a typed version of
    /// the resource.
    ///
    /// # Errors
    ///
    /// This function will return an error if `resource` does not have type
    /// `ResourceType::host::<T>()`. This function may also return an error if
    /// `resource` is no longer valid, for example it was previously converted.
    ///
    /// # Panics
    ///
    /// This function will panic if `resource` does not belong to the `store`
    /// specified.
    pub fn try_from_resource_any(
        resource: ResourceAny,
        mut store: impl AsContextMut,
    ) -> Result<Self> {
        let store = store.as_context_mut();
        let store_id = store.0.id();
        let mut tables = HostResourceTables::new_host(store.0);
        let ResourceAny { idx, ty, own_state } = resource;
        ensure!(ty == ResourceType::host::<T>(), "resource type mismatch");
        let (state, rep) = if let Some(OwnState { store, dtor, flags }) = own_state {
            assert_eq!(store_id, store, "wrong store used to convert resource");
            assert!(dtor.is_some(), "destructor must be set");
            assert!(flags.is_none(), "flags must not be set");
            let rep = tables.host_resource_lift_own(idx, ty)?;
            (AtomicU32::new(NOT_IN_TABLE), rep)
        } else {
            let rep = tables.host_resource_lift_borrow(idx, ty)?;
            (AtomicU32::new(BORROW), rep)
        };
        Ok(Resource {
            state,
            rep,
            _marker: marker::PhantomData,
        })
    }

    /// See [`ResourceAny::try_from_resource`]
    pub fn try_into_resource_any<U>(
        self,
        store: impl AsContextMut,
        instance: &InstancePre<U>,
        idx: ResourceImportIndex,
    ) -> Result<ResourceAny> {
        ResourceAny::try_from_resource(self, store, instance, idx)
    }
}

unsafe impl<T: 'static> ComponentType for Resource<T> {
    const ABI: CanonicalAbiInfo = CanonicalAbiInfo::SCALAR4;

    type Lower = <u32 as ComponentType>::Lower;

    fn typecheck(ty: &InterfaceType, types: &InstanceType<'_>) -> Result<()> {
        let resource = match ty {
            InterfaceType::Own(t) | InterfaceType::Borrow(t) => *t,
            other => bail!("expected `own` or `borrow`, found `{}`", desc(other)),
        };
        match types.resource_type(resource).kind {
            ResourceTypeKind::Host(id) if TypeId::of::<T>() == id => {}
            _ => bail!("resource type mismatch"),
        }

        Ok(())
    }
}

unsafe impl<T: 'static> Lower for Resource<T> {
    fn lower<U>(
        &self,
        cx: &mut LowerContext<'_, U>,
        ty: InterfaceType,
        dst: &mut MaybeUninit<Self::Lower>,
    ) -> Result<()> {
        self.lower_to_index(cx, ty)?
            .lower(cx, InterfaceType::U32, dst)
    }

    fn store<U>(
        &self,
        cx: &mut LowerContext<'_, U>,
        ty: InterfaceType,
        offset: usize,
    ) -> Result<()> {
        self.lower_to_index(cx, ty)?
            .store(cx, InterfaceType::U32, offset)
    }
}

unsafe impl<T: 'static> Lift for Resource<T> {
    fn lift(cx: &mut LiftContext<'_>, ty: InterfaceType, src: &Self::Lower) -> Result<Self> {
        let index = u32::lift(cx, InterfaceType::U32, src)?;
        Resource::lift_from_index(cx, ty, index)
    }

    fn load(cx: &mut LiftContext<'_>, ty: InterfaceType, bytes: &[u8]) -> Result<Self> {
        let index = u32::load(cx, InterfaceType::U32, bytes)?;
        Resource::lift_from_index(cx, ty, index)
    }
}

impl<T> fmt::Debug for Resource<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let state = match self.state.load(Relaxed) {
            BORROW => "borrow",
            NOT_IN_TABLE => "own (not in table)",
            TAKEN => "taken",
            _ => "own",
        };
        f.debug_struct("Resource")
            .field("rep", &self.rep)
            .field("state", &state)
            .finish()
    }
}

/// Representation of a resource in the component model, either a guest-defined
/// or a host-defined resource.
///
/// This type is similar to [`Resource`] except that it can be used to represent
/// any resource, either host or guest. This type cannot be directly constructed
/// and is only available if the guest returns it to the host (e.g. a function
/// returning a guest-defined resource) or by a conversion from [`Resource`] via
/// [`ResourceAny::try_from_resource`].
/// This type also does not carry a static type parameter `T` for example and
/// does not have as much information about its type.
/// This means that it's possible to get runtime type-errors when
/// using this type because it cannot statically prevent mismatching resource
/// types.
///
/// Like [`Resource`] this type represents either an `own` or a `borrow`
/// resource internally. Unlike [`Resource`], however, a [`ResourceAny`] must
/// always be explicitly destroyed with the [`ResourceAny::resource_drop`]
/// method. This will update internal dynamic state tracking and invoke the
/// WebAssembly-defined destructor for a resource, if any.
///
/// Note that it is required to call `resource_drop` for all instances of
/// [`ResourceAny`]: even borrows. Both borrows and own handles have state
/// associated with them that must be discarded by the time they're done being
/// used.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct ResourceAny {
    idx: u32,
    ty: ResourceType,
    own_state: Option<OwnState>,
}

#[derive(Copy, Clone)]
struct OwnState {
    store: StoreId,
    flags: Option<InstanceFlags>,
    dtor: Option<SendSyncPtr<VMFuncRef>>,
}

impl ResourceAny {
    /// Attempts to convert an imported [`Resource`] into [`ResourceAny`].
    ///
    /// * `resource` is the resource to convert.
    /// * `store` is the store to place the returned resource into.
    /// * `instance_pre` is the instance from where `idx` below was derived.
    /// * `idx` is the [`ResourceImportIndex`] returned by [`Linker::resource`].
    ///
    /// [`Linker::resource`]: crate::component::LinkerInstance::resource
    ///
    /// # Errors
    ///
    /// This method will return an error if `idx` isn't valid for
    /// `instance_pre` or if `resource` is not of the correct type for the
    /// `idx` import.
    pub fn try_from_resource<T: 'static, U>(
        resource: Resource<T>,
        mut store: impl AsContextMut,
        instance_pre: &InstancePre<U>,
        idx: ResourceImportIndex,
    ) -> Result<Self> {
        let Resource { rep, state, .. } = resource;
        let store = store.as_context_mut();
        let import = instance_pre
            .resource_import(idx)
            .context("import not found")?;
        let RuntimeImport::Resource {
            ty, dtor_funcref, ..
        } = import
        else {
            bail!("import is not a resource")
        };
        ensure!(*ty == ResourceType::host::<T>(), "resource type mismatch");

        let mut tables = HostResourceTables::new_host(store.0);
        let (idx, own_state) = match state.load(Relaxed) {
            BORROW => (tables.host_resource_lower_borrow(rep, *ty), None),
            NOT_IN_TABLE => {
                let idx = tables.host_resource_lower_own(rep, *ty);
                (
                    idx,
                    Some(OwnState {
                        dtor: Some(dtor_funcref.into()),
                        flags: None,
                        store: store.0.id(),
                    }),
                )
            }
            TAKEN => bail!("host resource already consumed"),
            idx => (
                idx,
                Some(OwnState {
                    dtor: Some(dtor_funcref.into()),
                    flags: None,
                    store: store.0.id(),
                }),
            ),
        };
        Ok(Self {
            idx,
            ty: *ty,
            own_state,
        })
    }

    /// See [`Resource::try_from_resource_any`]
    pub fn try_into_resource<T: 'static>(self, store: impl AsContextMut) -> Result<Resource<T>> {
        Resource::try_from_resource_any(self, store)
    }

    /// Returns the corresponding type associated with this resource, either a
    /// host-defined type or a guest-defined type.
    ///
    /// This can be compared against [`ResourceType::host`] for example to see
    /// if it's a host-resource or against a type extracted with
    /// [`Instance::get_resource`] to see if it's a guest-defined resource.
    ///
    /// [`Instance::get_resource`]: crate::component::Instance::get_resource
    pub fn ty(&self) -> ResourceType {
        self.ty
    }

    /// Returns whether this is an owned resource, and if not it's a borrowed
    /// resource.
    pub fn owned(&self) -> bool {
        self.own_state.is_some()
    }

    /// Destroy this resource and release any state associated with it.
    ///
    /// This is required to be called (or the async version) for all instances
    /// of [`ResourceAny`] to ensure that state associated with this resource is
    /// properly cleaned up. For owned resources this may execute the
    /// guest-defined destructor if applicable (or the host-defined destructor
    /// if one was specified).
    pub fn resource_drop(self, mut store: impl AsContextMut) -> Result<()> {
        let mut store = store.as_context_mut();
        assert!(
            !store.0.async_support(),
            "must use `resource_drop_async` when async support is enabled on the config"
        );
        self.resource_drop_impl(&mut store.as_context_mut())
    }

    /// Same as [`ResourceAny::resource_drop`] except for use with async stores
    /// to execute the destructor asynchronously.
    #[cfg(feature = "async")]
    #[cfg_attr(nightlydoc, doc(cfg(feature = "async")))]
    pub async fn resource_drop_async<T>(self, mut store: impl AsContextMut<Data = T>) -> Result<()>
    where
        T: Send,
    {
        let mut store = store.as_context_mut();
        assert!(
            store.0.async_support(),
            "cannot use `resource_drop_async` without enabling async support in the config"
        );
        store
            .on_fiber(|store| self.resource_drop_impl(store))
            .await?
    }

    fn resource_drop_impl<T>(self, store: &mut StoreContextMut<'_, T>) -> Result<()> {
        // Attempt to remove `self.idx` from the host table in `store`.
        //
        // This could fail if the index is invalid or if this is removing an
        // `Own` entry which is currently being borrowed.
        let rep = HostResourceTables::new_host(store.0).host_resource_drop(self.idx, self.ty)?;

        let (rep, state) = match (rep, &self.own_state) {
            (Some(rep), Some(state)) => (rep, state),

            // A `borrow` was removed from the table and no further
            // destruction, e.g. the destructor, is required so we're done.
            (None, None) => return Ok(()),

            _ => unreachable!(),
        };

        // Double-check that accessing the raw pointers on `state` are safe due
        // to the presence of `store` above.
        assert_eq!(
            store.0.id(),
            state.store,
            "wrong store used to destroy resource"
        );

        // Implement the reentrance check required by the canonical ABI. Note
        // that this happens whether or not a destructor is present.
        //
        // Note that this should be safe because the raw pointer access in
        // `flags` is valid due to `store` being the owner of the flags and
        // flags are never destroyed within the store.
        if let Some(flags) = state.flags {
            unsafe {
                if !flags.may_enter() {
                    bail!(Trap::CannotEnterComponent);
                }
            }
        }

        let dtor = match state.dtor {
            Some(dtor) => dtor.as_non_null(),
            None => return Ok(()),
        };
        let mut args = [ValRaw::u32(rep)];

        // This should be safe because `dtor` has been checked to belong to the
        // `store` provided which means it's valid and still alive. Additionally
        // destructors have al been previously type-checked and are guaranteed
        // to take one i32 argument and return no results, so the parameters
        // here should be configured correctly.
        unsafe { crate::Func::call_unchecked_raw(store, dtor, args.as_mut_ptr(), args.len()) }
    }

    fn lower_to_index<U>(&self, cx: &mut LowerContext<'_, U>, ty: InterfaceType) -> Result<u32> {
        match ty {
            InterfaceType::Own(t) => {
                if cx.resource_type(t) != self.ty {
                    bail!("mismatched resource types");
                }
                let rep = cx.host_resource_lift_own(self.idx, self.ty)?;
                Ok(cx.guest_resource_lower_own(t, rep))
            }
            InterfaceType::Borrow(t) => {
                if cx.resource_type(t) != self.ty {
                    bail!("mismatched resource types");
                }
                let rep = cx.host_resource_lift_borrow(self.idx, self.ty)?;
                Ok(cx.guest_resource_lower_borrow(t, rep))
            }
            _ => bad_type_info(),
        }
    }

    fn lift_from_index(cx: &mut LiftContext<'_>, ty: InterfaceType, index: u32) -> Result<Self> {
        match ty {
            InterfaceType::Own(t) => {
                let ty = cx.resource_type(t);
                let (rep, dtor, flags) = cx.guest_resource_lift_own(t, index)?;
                let idx = cx.host_resource_lower_own(rep, ty);
                Ok(ResourceAny {
                    idx,
                    ty,
                    own_state: Some(OwnState {
                        dtor: dtor.map(SendSyncPtr::new),
                        flags,
                        store: cx.store_id(),
                    }),
                })
            }
            InterfaceType::Borrow(t) => {
                let ty = cx.resource_type(t);
                let rep = cx.guest_resource_lift_borrow(t, index)?;
                let idx = cx.host_resource_lower_borrow(rep, ty);
                Ok(ResourceAny {
                    idx,
                    ty,
                    own_state: None,
                })
            }
            _ => bad_type_info(),
        }
    }
}

unsafe impl ComponentType for ResourceAny {
    const ABI: CanonicalAbiInfo = CanonicalAbiInfo::SCALAR4;

    type Lower = <u32 as ComponentType>::Lower;

    fn typecheck(ty: &InterfaceType, _types: &InstanceType<'_>) -> Result<()> {
        match ty {
            InterfaceType::Own(_) | InterfaceType::Borrow(_) => Ok(()),
            other => bail!("expected `own` or `borrow`, found `{}`", desc(other)),
        }
    }
}

unsafe impl Lower for ResourceAny {
    fn lower<T>(
        &self,
        cx: &mut LowerContext<'_, T>,
        ty: InterfaceType,
        dst: &mut MaybeUninit<Self::Lower>,
    ) -> Result<()> {
        self.lower_to_index(cx, ty)?
            .lower(cx, InterfaceType::U32, dst)
    }

    fn store<T>(
        &self,
        cx: &mut LowerContext<'_, T>,
        ty: InterfaceType,
        offset: usize,
    ) -> Result<()> {
        self.lower_to_index(cx, ty)?
            .store(cx, InterfaceType::U32, offset)
    }
}

unsafe impl Lift for ResourceAny {
    fn lift(cx: &mut LiftContext<'_>, ty: InterfaceType, src: &Self::Lower) -> Result<Self> {
        let index = u32::lift(cx, InterfaceType::U32, src)?;
        ResourceAny::lift_from_index(cx, ty, index)
    }

    fn load(cx: &mut LiftContext<'_>, ty: InterfaceType, bytes: &[u8]) -> Result<Self> {
        let index = u32::load(cx, InterfaceType::U32, bytes)?;
        ResourceAny::lift_from_index(cx, ty, index)
    }
}

impl fmt::Debug for OwnState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OwnState")
            .field("store", &self.store)
            .finish()
    }
}

// This is a loose definition for `Val` primarily so it doesn't need to be
// strictly 100% correct, and equality of resources is a bit iffy anyway, so
// ignore equality here and only factor in the indices and other metadata in
// `ResourceAny`.
impl PartialEq for OwnState {
    fn eq(&self, _other: &OwnState) -> bool {
        true
    }
}

impl Eq for OwnState {}
