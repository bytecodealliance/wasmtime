//! Static analysis of "thread transparency" for fused adapters.
//!
//! Every fused adapter brackets its callee with `enter-sync-call`/
//! `exit-sync-call` calls, which preserve the caller's task state and create
//! the callee's task state (`current_thread`, context slots, etc...). However,
//! we call an adapter that statically cannot call anything that accesses this
//! task state *thread-transparent*, and we can entirely avoid the task state
//! save/creation/restore for these adapters.
//!
//! The analysis is performed at the granularity of component instances and
//! their `canon lower`s: if an instance doesn't `canon lower` anything that
//! reads or writes that task state, then it cannot access that task state and
//! adapters calling into it are transparent.

use crate::component::translate::inline::{
    ComponentFuncDef, ComponentInstanceDef, ComponentItemDef, InlinerFrame,
};
use crate::component::translate::*;
use cranelift_entity::EntitySet;

/// Results of the analysis, built by the inliner and queried by
/// `ThreadTransparency::adapter_is_transparent` when adapter modules are generated.
#[derive(Default)]
pub struct ThreadTransparency {
    /// The instances known to be transparent; an absent instance is implicitly
    /// opaque.
    instances: EntitySet<RuntimeComponentInstanceIndex>,
}

impl ThreadTransparency {
    /// Registers the root component instance as `instance`.
    ///
    /// The root starts out transparent: its host imports only disqualify it at
    /// the point where they are actually `canon lower`ed, which
    /// `process_initializer` takes care of.
    pub(super) fn push_root_instance(&mut self, instance: RuntimeComponentInstanceIndex) {
        self.instances.insert(instance);
    }

    /// Registers as `instance` a component instance being instantiated with `args`.
    ///
    /// Such an instance is only transparent if all of its instantiation
    /// arguments are; anything else it does that's disqualifying is noticed as
    /// its own initializers are processed.
    pub(super) fn push_instance(
        &mut self,
        instance: RuntimeComponentInstanceIndex,
        args: &HashMap<&str, ComponentItemDef<'_>>,
    ) {
        if args.values().all(|item| self.item_def_is_transparent(item)) {
            self.instances.insert(instance);
        }
    }

    /// Records the effect of `frame`'s component instance processing `init`.
    pub(super) fn process_initializer(
        &mut self,
        types: &ComponentTypesBuilder,
        frame: &InlinerFrame<'_>,
        init: &LocalInitializer<'_>,
    ) {
        if self.initializer_is_opaque(types, frame, init) {
            self.instances.remove(frame.instance);
        }
    }

    /// Returns whether `adapter` can omit its `{enter,exit}-sync-call` window.
    pub fn adapter_is_transparent(&self, types: &ComponentTypesBuilder, adapter: &Adapter) -> bool {
        Self::signature_is_transparent(types, adapter)
            && self.instances.contains(adapter.lift_options.instance)
    }

    fn signature_is_transparent(types: &ComponentTypesBuilder, adapter: &Adapter) -> bool {
        // Async adapters really do need task state.
        if adapter.lift_options.async_
            || adapter.lower_options.async_
            || types[adapter.lift_ty].async_
            || types[adapter.lower_ty].async_
        {
            return false;
        }

        // Transferring handles across instances unconditionally requires task
        // state for now.
        if types.func_contains_any_handle(adapter.lift_ty)
            || types.func_contains_any_handle(adapter.lower_ty)
        {
            return false;
        }

        true
    }

    fn initializer_is_opaque(
        &self,
        types: &ComponentTypesBuilder,
        frame: &InlinerFrame<'_>,
        init: &LocalInitializer<'_>,
    ) -> bool {
        use LocalInitializer::*;
        // NB: This `match` is deliberately exhaustive so that a new canon must
        // be classified here rather than silently defaulting to transparent.
        match init {
            Lower { func, .. } => {
                let def = &frame.component_funcs[*func];

                // At least as opaque as the function being made callable.
                !self.func_def_is_transparent(def)
                    // Calling an `async`-typed lift from a sync-typed call
                    // needs the callee's thread state: the callee may block,
                    // in which case the scheduler has to find the sync-typed
                    // call in progress on this instance.
                    || matches!(def, ComponentFuncDef::Lifted { ty, .. } if types[*ty].async_)
            }

            // These make no intrinsic callable from this instance's core Wasm.
            Import(..)
            | IntrinsicsImport
            | Lift(..)
            | ModuleStatic(..)
            | ModuleInstantiate(..)
            | ModuleSynthetic(..)
            | ComponentStatic(..)
            | ComponentSynthetic(..)
            | AliasExportFunc(..)
            | AliasExportTable(..)
            | AliasExportGlobal(..)
            | AliasExportMemory(..)
            | AliasExportTag(..)
            | AliasComponentExport(..)
            | AliasModule(..)
            | AliasComponent(..)
            | Resource(..)
            | Export(..) => false,

            // `ComponentInstantiate` also does not make any intrinsic directly
            // callable from this instance's core Wasm: it is reachable only
            // through a separate adapter, which may or may not itself be
            // transparent, but it doesn't affect this instance's transparency.
            ComponentInstantiate(..) => false,

            // Each of these `canon` intrinsics forces the `VMDeferredThread`,
            // mutates backpressure, or touches the current context slots.
            ResourceNew(..)
            | ResourceRep(..)
            | ResourceDrop(..)
            | BackpressureInc { .. }
            | BackpressureDec { .. }
            | TaskReturn { .. }
            | TaskCancel { .. }
            | WaitableSetNew { .. }
            | WaitableSetWait { .. }
            | WaitableSetPoll { .. }
            | WaitableSetDrop { .. }
            | WaitableJoin { .. }
            | SubtaskDrop { .. }
            | SubtaskCancel { .. }
            | StreamNew { .. }
            | StreamRead { .. }
            | StreamWrite { .. }
            | StreamCancelRead { .. }
            | StreamCancelWrite { .. }
            | StreamDropReadable { .. }
            | StreamDropWritable { .. }
            | FutureNew { .. }
            | FutureRead { .. }
            | FutureWrite { .. }
            | FutureCancelRead { .. }
            | FutureCancelWrite { .. }
            | FutureDropReadable { .. }
            | FutureDropWritable { .. }
            | ErrorContextNew { .. }
            | ErrorContextDebugMessage { .. }
            | ErrorContextDrop { .. }
            | ContextGet { .. }
            | ContextSet { .. }
            | ThreadIndex { .. }
            | ThreadNewIndirect { .. }
            | ThreadResumeLater { .. }
            | ThreadSuspend { .. }
            | ThreadYield { .. }
            | ThreadSuspendThenResume { .. }
            | ThreadYieldThenResume { .. }
            | ThreadSuspendThenPromote { .. }
            | ThreadYieldThenPromote { .. } => true,
        }
    }

    fn item_def_is_transparent(&self, def: &ComponentItemDef<'_>) -> bool {
        match def {
            ComponentItemDef::Func(func) => self.func_def_is_transparent(func),
            ComponentItemDef::Instance(instance) => self.instance_def_is_transparent(instance),
            ComponentItemDef::Module(_)
            | ComponentItemDef::Type(_)
            | ComponentItemDef::Component(_) => true,
        }
    }

    fn func_def_is_transparent(&self, def: &ComponentFuncDef<'_>) -> bool {
        match def {
            // Goes through an adapter, which saves/restores its own state if
            // needed, but doesn't affect this adapter.
            ComponentFuncDef::Lifted { .. } => true,

            // A host import can do anything at all.
            ComponentFuncDef::Import(_) => false,

            // Transparent except `context.{get,set}`.
            ComponentFuncDef::UnsafeIntrinsic(intrinsic) => !matches!(
                intrinsic,
                UnsafeIntrinsic::ContextGetI32_0
                    | UnsafeIntrinsic::ContextSetI32_0
                    | UnsafeIntrinsic::ContextGetI32_1
                    | UnsafeIntrinsic::ContextSetI32_1
            ),
        }
    }

    fn instance_def_is_transparent(&self, def: &ComponentInstanceDef<'_>) -> bool {
        match def {
            // A host import can do anything at all.
            ComponentInstanceDef::Import(..) => false,

            ComponentInstanceDef::Intrinsics => true,

            ComponentInstanceDef::Items(items, _) => items
                .values()
                .all(|(def, _)| self.item_def_is_transparent(def)),
        }
    }
}
