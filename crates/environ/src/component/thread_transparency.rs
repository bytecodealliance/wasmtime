//! Static analysis of "thread transparency" for fused adapters.
//!
//! Every fused sync adapter normally brackets its callee with
//! `enter-sync-call`/`exit-sync-call`. That pair saves the caller's
//! component-model thread state (`current_thread`, the context slots, the "may
//! block" flag, etc...) and installs fresh state for the callee, restoring the
//! caller's state on the way back out.
//!
//! However, that bracketing is only necessary if the callee can observe that
//! thread state. If it cannot, the callee may simply run on top of whatever
//! thread state the caller left in place, and the adapter can skip the
//! save/restore entirely. We call such an adapter "thread transparent".
//!
//! Core Wasm state cannot be shared between components, so the only way core
//! Wasm can touch component-model thread state is by calling a `canon lower`ed
//! function or a canonical built-in. All of those are declared by a particular
//! component instance and are only reachable from core Wasm belonging to that
//! same component instance.
//!
//! Therefore this analysis works per component instance: an adapter whose
//! callee instance declares nothing capable of touching thread state is
//! transparent.
//!
//! Note that (lack of) transparency is not transitive across component
//! instances because a call into an opaque instance will itself save/restore
//! the thread state, even if none of its transparent callers needed to.

use crate::component::dfg::{
    AdapterId, CanonicalOptionsDataModel, ComponentDfg, CoreDef, Export, Instance, SideEffect,
    Trampoline,
};
use crate::component::{
    ComponentTypesBuilder, DataModel, RuntimeComponentInstanceIndex, UnsafeIntrinsic,
};
use cranelift_entity::EntitySet;

/// Something a component instance makes callable by its core Wasm instances.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum CoreCallable {
    /// A `canon lower` of a function imported from the host.
    ///
    /// The host can do anything at all, including reading and writing the
    /// current thread's context slots.
    HostImport,

    /// A `canon lower` of a guest function that was `canon lift`ed elsewhere.
    Adapter {
        /// Whether the function on the lifting side uses the `async` ABI.
        async_lift: bool,
    },

    /// A `canon context.get` or `canon context.set`.
    ContextAccess,

    /// Any other canonical built-in that reads or writes thread state:
    /// `resource.*`, `backpressure.*`, `task.*`, `waitable*.*`, `subtask.*`,
    /// `stream.*`, `future.*`, `error-context.*`, or `thread.*`.
    ThreadStateBuiltin,

    /// Something that cannot touch thread state at all: a plain core function,
    /// a string transcoder, a non-context Wasmtime intrinsic, and so on.
    Inert,
}

impl CoreCallable {
    /// Can calling this reach code that reads or writes component-model thread
    /// state?
    fn may_touch_thread_state(self) -> bool {
        match self {
            // The host is unconstrained.
            CoreCallable::HostImport => true,

            // A sync-lifted callee is reached through its own fused adapter,
            // which saves and restores whatever it needs; nothing about that
            // call is visible in this instance's thread state.
            //
            // An async-lifted callee is different: it may block, and if it
            // does the scheduler has to find the sync-typed call in progress
            // on this instance, which means this instance needs real thread
            // state of its own.
            CoreCallable::Adapter { async_lift } => async_lift,

            CoreCallable::ContextAccess | CoreCallable::ThreadStateBuiltin => true,

            CoreCallable::Inert => false,
        }
    }
}

/// Everything about one fused adapter that bears on whether it can skip its
/// `{enter,exit}-sync-call` window.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct AdapterFacts {
    /// The component instance whose core Wasm this adapter calls into, i.e.
    /// the instance that did the `canon lift`.
    callee: RuntimeComponentInstanceIndex,

    /// Whether either side of this adapter is `async` in either its canonical
    /// options or function signature.
    any_async: bool,

    /// Whether either function signature mentions a handle.
    any_handle: bool,
}

/// A builder for a `ThreadTransparency`.
///
/// Accumulates the observations that ultimately produce the
/// `ThreadTransparency` analysis.
#[derive(Default)]
struct ThreadTransparencyBuilder {
    /// The component instances observed to be able to touch thread state.
    opaque: EntitySet<RuntimeComponentInstanceIndex>,
}

impl ThreadTransparencyBuilder {
    /// Record that `instance` makes `callable` available to its core Wasm.
    fn observe(&mut self, instance: RuntimeComponentInstanceIndex, callable: CoreCallable) {
        if callable.may_touch_thread_state() {
            self.opaque.insert(instance);
        }
    }

    /// Finish observing and produce the queryable analysis results.
    fn finish(self) -> ThreadTransparency {
        ThreadTransparency {
            opaque: self.opaque,
        }
    }
}

/// A completed thread-transparency analysis, ready to be queried.
struct ThreadTransparency {
    /// The component instances that can touch thread state.
    opaque: EntitySet<RuntimeComponentInstanceIndex>,
}

impl ThreadTransparency {
    /// May the adapter described by `facts` omit its `{enter,exit}-sync-call`
    /// window?
    fn adapter_is_transparent(&self, facts: AdapterFacts) -> bool {
        // Async adapters genuinely need thread state of their own.
        if facts.any_async {
            return false;
        }

        // Transferring handles across an instance boundary unconditionally
        // requires thread state for now.
        if facts.any_handle {
            return false;
        }

        !self.opaque.contains(facts.callee)
    }
}

/// Run the thread-transparency analysis over `dfg`, returning the set of
/// adapters that may skip their `{enter,exit}-sync-call` window.
pub fn transparent_adapters(
    dfg: &ComponentDfg,
    types: &ComponentTypesBuilder,
) -> EntitySet<AdapterId> {
    let mut builder = ThreadTransparencyBuilder::default();

    // Observe canonical built-ins.
    for (_, (_, trampoline)) in dfg.trampolines.iter() {
        let (instance, callable) = trampoline_callable(dfg, trampoline);
        builder.observe(instance, callable);
    }

    // Observe fused adapters.
    for (_, adapter) in dfg.adapters.iter() {
        let async_lift = types[adapter.lift_ty].async_;
        builder.observe(
            adapter.lower_options.instance,
            CoreCallable::Adapter { async_lift },
        );
    }

    // Observe `CoreDef`s.
    for_each_core_def(dfg, |instance, def| {
        builder.observe(instance, core_def_callable(def));
    });

    // Finish the analysis and query its results to determine the set of
    // adapters that are transparent.
    let analysis = builder.finish();
    let mut transparent = EntitySet::new();
    for (id, adapter) in dfg.adapters.iter() {
        if analysis.adapter_is_transparent(facts(types, adapter)) {
            transparent.insert(id);
        }
    }
    transparent
}

fn facts(types: &ComponentTypesBuilder, adapter: &super::Adapter) -> AdapterFacts {
    AdapterFacts {
        callee: adapter.lift_options.instance,
        any_async: adapter.lift_options.async_
            || adapter.lower_options.async_
            || types[adapter.lift_ty].async_
            || types[adapter.lower_ty].async_,
        any_handle: types.func_contains_any_handle(adapter.lift_ty)
            || types.func_contains_any_handle(adapter.lower_ty),
    }
}

/// Classify a `CoreDef` that some component instance references.
fn core_def_callable(def: &CoreDef) -> CoreCallable {
    // NB: deliberately exhaustive so that new variants must be classified here.
    match def {
        CoreDef::UnsafeIntrinsic(_, intrinsic) => match intrinsic {
            UnsafeIntrinsic::ContextGetI32_0
            | UnsafeIntrinsic::ContextSetI32_0
            | UnsafeIntrinsic::ContextGetI32_1
            | UnsafeIntrinsic::ContextSetI32_1 => CoreCallable::ContextAccess,

            // Unsafe intrinsics can't access thread state.
            UnsafeIntrinsic::StoreDataAddress
            | UnsafeIntrinsic::U8NativeLoad
            | UnsafeIntrinsic::U8NativeStore
            | UnsafeIntrinsic::U16NativeLoad
            | UnsafeIntrinsic::U16NativeStore
            | UnsafeIntrinsic::U32NativeLoad
            | UnsafeIntrinsic::U32NativeStore
            | UnsafeIntrinsic::U64NativeLoad
            | UnsafeIntrinsic::U64NativeStore
            | UnsafeIntrinsic::U8CheckedNativeLoad
            | UnsafeIntrinsic::U8CheckedNativeStore
            | UnsafeIntrinsic::U16CheckedNativeLoad
            | UnsafeIntrinsic::U16CheckedNativeStore
            | UnsafeIntrinsic::U32CheckedNativeLoad
            | UnsafeIntrinsic::U32CheckedNativeStore
            | UnsafeIntrinsic::U64CheckedNativeLoad
            | UnsafeIntrinsic::U64CheckedNativeStore => CoreCallable::Inert,
        },

        // Trampolines and adapters are observed directly in
        // `transparent_adapters`, so we don't need to worry about them here.
        CoreDef::Trampoline(_) | CoreDef::Adapter(_) => CoreCallable::Inert,

        // Plain core Wasm; can't introduce any state-access capability that
        // isn't already there.
        CoreDef::Export(_) | CoreDef::InstanceFlags(_) => CoreCallable::Inert,
    }
}

/// Classify a trampoline, and identify the component instance that declared it.
fn trampoline_callable(
    dfg: &ComponentDfg,
    trampoline: &Trampoline,
) -> (RuntimeComponentInstanceIndex, CoreCallable) {
    use Trampoline::*;

    // NB: deliberately exhaustive so that new variants must be classified here.
    match trampoline {
        LowerImport { options, .. } => (dfg.options[*options].instance, CoreCallable::HostImport),

        ResourceNew { instance, .. }
        | ResourceRep { instance, .. }
        | ResourceDrop { instance, .. }
        | BackpressureInc { instance }
        | BackpressureDec { instance }
        | TaskReturn { instance, .. }
        | TaskCancel { instance }
        | WaitableSetNew { instance }
        | WaitableSetWait { instance, .. }
        | WaitableSetPoll { instance, .. }
        | WaitableSetDrop { instance }
        | WaitableJoin { instance }
        | SubtaskDrop { instance }
        | SubtaskCancel { instance, .. }
        | StreamNew { instance, .. }
        | StreamRead { instance, .. }
        | StreamWrite { instance, .. }
        | StreamCancelRead { instance, .. }
        | StreamCancelWrite { instance, .. }
        | StreamDropReadable { instance, .. }
        | StreamDropWritable { instance, .. }
        | FutureNew { instance, .. }
        | FutureRead { instance, .. }
        | FutureWrite { instance, .. }
        | FutureCancelRead { instance, .. }
        | FutureCancelWrite { instance, .. }
        | FutureDropReadable { instance, .. }
        | FutureDropWritable { instance, .. }
        | ErrorContextNew { instance, .. }
        | ErrorContextDebugMessage { instance, .. }
        | ErrorContextDrop { instance, .. }
        | ThreadIndex { instance }
        | ThreadNewIndirect { instance, .. }
        | ThreadResumeLater { instance }
        | ThreadSuspend { instance, .. }
        | ThreadYield { instance, .. }
        | ThreadSuspendThenResume { instance, .. }
        | ThreadYieldThenResume { instance, .. }
        | ThreadSuspendThenPromote { instance, .. }
        | ThreadYieldThenPromote { instance, .. } => (*instance, CoreCallable::ThreadStateBuiltin),

        // These trampolines are only ever created in `translate::adapt`, which
        // happens strictly after this analysis has run. Therefore none of them
        // can exist yet.
        Transcoder { .. }
        | ResourceTransferOwn
        | ResourceTransferBorrow
        | PrepareCall { .. }
        | SyncStartCall { .. }
        | AsyncStartCall { .. }
        | FutureTransfer
        | StreamTransfer
        | ErrorContextTransfer
        | Trap(_)
        | EnterSyncCall
        | ExitSyncCall => {
            unreachable!("these trampolines do not exist yet")
        }
    }
}

/// Visit every `CoreDef` in `dfg` together with the component instance that
/// references it.
fn for_each_core_def(
    dfg: &ComponentDfg,
    mut f: impl FnMut(RuntimeComponentInstanceIndex, &CoreDef),
) {
    for effect in dfg.side_effects.iter() {
        let SideEffect::Instance(id, instance) = effect else {
            continue;
        };
        match &dfg.instances[*id] {
            Instance::Static(_, args) => {
                for def in args.iter() {
                    f(*instance, def);
                }
            }
            Instance::Import(_, args) => {
                for (_, defs) in args {
                    for (_, def) in defs {
                        f(*instance, def);
                    }
                }
            }
        }
    }

    for (_, adapter) in dfg.adapters.iter() {
        f(adapter.lift_options.instance, &adapter.func);
        for options in [&adapter.lift_options, &adapter.lower_options] {
            for def in options
                .callback
                .iter()
                .chain(options.post_return.iter())
                .chain(match &options.data_model {
                    DataModel::LinearMemory { realloc, .. } => realloc.iter(),
                    DataModel::Gc {} => None.iter(),
                })
            {
                f(options.instance, def);
            }
        }
    }

    for (_, options) in dfg.options.iter() {
        if let Some(callback) = options.callback {
            f(options.instance, &dfg.callbacks[callback]);
        }
        if let Some(post_return) = options.post_return {
            f(options.instance, &dfg.post_returns[post_return]);
        }
        if let CanonicalOptionsDataModel::LinearMemory {
            realloc: Some(realloc),
            ..
        } = &options.data_model
        {
            f(options.instance, &dfg.reallocs[*realloc]);
        }
    }

    for (_, resource) in dfg.resources.iter() {
        if let Some(dtor) = &resource.dtor {
            f(resource.instance, dtor);
        }
    }

    for (_, (export, _)) in dfg.exports.iter() {
        for_each_export_core_def(dfg, export, &mut f);
    }
}

fn for_each_export_core_def(
    dfg: &ComponentDfg,
    export: &Export,
    f: &mut impl FnMut(RuntimeComponentInstanceIndex, &CoreDef),
) {
    match export {
        Export::LiftedFunction { func, options, .. } => {
            f(dfg.options[*options].instance, func);
        }
        Export::Instance { exports, .. } => {
            for (_, (export, _)) in exports.iter() {
                for_each_export_core_def(dfg, export, f);
            }
        }
        Export::ModuleStatic { .. } | Export::ModuleImport { .. } | Export::Type(_) => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn instance(i: u32) -> RuntimeComponentInstanceIndex {
        RuntimeComponentInstanceIndex::from_u32(i)
    }

    /// Run the build phase over `observations`, given as `(instance, callable)`
    /// pairs, and finish it into queryable results.
    fn analyze(observations: &[(u32, CoreCallable)]) -> ThreadTransparency {
        let mut builder = ThreadTransparencyBuilder::default();
        for (i, callable) in observations {
            builder.observe(instance(*i), *callable);
        }
        builder.finish()
    }

    /// An adapter calling into instance `callee` with a plain, handle-free,
    /// sync signature.
    fn facts(callee: u32) -> AdapterFacts {
        AdapterFacts {
            callee: instance(callee),
            any_async: false,
            any_handle: false,
        }
    }

    #[test]
    fn clean_instance_is_transparent() {
        let t = analyze(&[]);
        assert!(t.adapter_is_transparent(facts(0)));
    }

    #[test]
    fn context_access_makes_instance_opaque() {
        let t = analyze(&[(0, CoreCallable::ContextAccess)]);
        assert!(!t.adapter_is_transparent(facts(0)));
    }

    #[test]
    fn thread_state_builtin_makes_instance_opaque() {
        let t = analyze(&[(0, CoreCallable::ThreadStateBuiltin)]);
        assert!(!t.adapter_is_transparent(facts(0)));
    }

    #[test]
    fn host_import_lowering_makes_instance_opaque() {
        let t = analyze(&[(0, CoreCallable::HostImport)]);
        assert!(!t.adapter_is_transparent(facts(0)));
    }

    #[test]
    fn inert_callable_leaves_instance_transparent() {
        let t = analyze(&[(0, CoreCallable::Inert), (0, CoreCallable::Inert)]);
        assert!(t.adapter_is_transparent(facts(0)));
    }

    /// Lowering a sync-lifted guest function is just another adapter, and that
    /// adapter does its own save/restore, so it does not taint the instance
    /// doing the lowering.
    #[test]
    fn sync_adapter_lowering_leaves_instance_transparent() {
        let t = analyze(&[(0, CoreCallable::Adapter { async_lift: false })]);
        assert!(t.adapter_is_transparent(facts(0)));
    }

    /// Lowering an async-lifted guest function does taint the lowering
    /// instance: the callee may block, and then the scheduler needs to find
    /// this instance's sync call in progress.
    #[test]
    fn async_lift_makes_the_lowering_instance_opaque() {
        let t = analyze(&[(0, CoreCallable::Adapter { async_lift: true })]);
        assert!(!t.adapter_is_transparent(facts(0)));
    }

    /// Only the callee side is judged: an opaque caller calling into a clean
    /// callee still gets a transparent adapter.
    #[test]
    fn only_the_callee_side_is_judged() {
        let t = analyze(&[(0, CoreCallable::ContextAccess)]);
        assert!(t.adapter_is_transparent(facts(1)));
    }

    /// Opacity does not propagate outward: in a chain `outer -> mid -> inner`
    /// where only `inner` is opaque, the `outer -> mid` adapter is still
    /// transparent because the `mid -> inner` call has its own adapter.
    #[test]
    fn opacity_does_not_propagate_outward() {
        let t = analyze(&[(2, CoreCallable::ContextAccess)]);
        assert!(t.adapter_is_transparent(facts(1)), "outer -> mid");
        assert!(!t.adapter_is_transparent(facts(2)), "mid -> inner");
    }

    /// Nor inward: an opaque `outer` calling a clean `mid` calling a clean
    /// `inner` leaves both adapters transparent.
    #[test]
    fn opacity_does_not_propagate_inward() {
        let t = analyze(&[(0, CoreCallable::ContextAccess)]);
        assert!(t.adapter_is_transparent(facts(1)), "outer -> mid");
        assert!(t.adapter_is_transparent(facts(2)), "mid -> inner");
    }

    /// An unrelated sibling instance being opaque taints nobody else.
    #[test]
    fn siblings_are_judged_independently() {
        let t = analyze(&[(0, CoreCallable::ContextAccess)]);
        assert!(!t.adapter_is_transparent(facts(0)));
        assert!(t.adapter_is_transparent(facts(1)));
    }

    #[test]
    fn async_signature_is_never_transparent() {
        let t = analyze(&[]);
        assert!(!t.adapter_is_transparent(AdapterFacts {
            any_async: true,
            ..facts(0)
        }));
    }

    #[test]
    fn handle_in_signature_is_never_transparent() {
        let t = analyze(&[]);
        assert!(!t.adapter_is_transparent(AdapterFacts {
            any_handle: true,
            ..facts(0)
        }));
    }

    /// The build phase is monotone, so observations may arrive in any order and
    /// a clean observation can never undo an opaque one.
    #[test]
    fn observations_are_monotone_and_order_independent() {
        let dirty_first = analyze(&[
            (0, CoreCallable::ContextAccess),
            (0, CoreCallable::Inert),
            (0, CoreCallable::Adapter { async_lift: false }),
        ]);
        let dirty_last = analyze(&[
            (0, CoreCallable::Adapter { async_lift: false }),
            (0, CoreCallable::Inert),
            (0, CoreCallable::ContextAccess),
        ]);
        assert!(!dirty_first.adapter_is_transparent(facts(0)));
        assert!(!dirty_last.adapter_is_transparent(facts(0)));
    }
}
