use crate::component::concurrent::table::{TableDebug, TableId};
use crate::component::concurrent::{ConcurrentState, CurrentThread};
use crate::error::Result;
use crate::store::StoreOpaque;
use crate::{AsContextMut as _, Store, StoreContextMut};
use alloc::boxed::Box;
use alloc::vec::Vec;

struct TaskGroup {
    ref_count: usize,
}

impl TableDebug for TaskGroup {
    fn type_name() -> &'static str {
        "TaskGroup"
    }
}

/// Represents a "task group" containing the "root" task of a host->guest call,
/// plus any subtasks transitively created by that task.
///
/// See [TaskGroupHook] for details.
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct TaskGroupId(TableId<TaskGroup>);

/// Trait for being notified by the runtime of activity concerning a "task group".
///
/// The Component Model specification has no notion of a "task group"[^1], but
/// we define one here in order to enable embedders to associate guest->host
/// calls with corresponding host->guest calls in a predictable way.
///
/// A `TaskGroupId` is allocated whenever a guest task is created for a
/// host->guest call, and `handle_start` is called.  That task is considered the
/// "root" task for the task group, and any subtasks transitively created by it
/// will also be considered part of that task group.  Whenever the runtime
/// switches (Component-Model-level) threads, it will call `handle_exit` for the
/// group to which the old thread belonged, if any, and call `handle_enter` for
/// the group to which the new thread belongs.  Only once all the threads of all
/// those tasks have exited (and the guest has dropped any subtask handles
/// referring to any of those tasks) will the `TaskGroupId` be deallocated, at
/// which point `handle_finish` will be called.
///
/// Note that a given `TaskGroupId` may be reused after `handle_finish` is
/// called, so implementations of this trait must take care to reset any state
/// associated with it.
///
/// Each of these functions may return an error, in which case any running guest
/// code will trap, the store will be poisoned such that it cannot be used to
/// run any further guest code, and the error will propagate back to the
/// host->guest caller.  Furthermore, if and when the store is poisoned due to
/// any unrecoverable error (whether it was produced by the hook, the guest, or
/// the host), the currently-entered group will be exited (i.e. `handle_exit`
/// called), if any, and any started group will be finished
/// (i.e. `handle_finish` called).
///
/// As of this writing, https://github.com/WebAssembly/component-model/pull/730
/// (which adds `thread.set-task` and related intrinsics) has not yet been
/// merged.  Once it has, and Wasmtime adds support for that feature, it will be
/// possible for guest threads to change their task; in that case, the thread
/// will effectively join whatever group the new task belongs to, which might
/// not be the same as that of the old task.  In addition, the new
/// `thread.get-task` intrinsic will give the guest another way (besides subtask
/// handles) to keep tasks alive beyond the point when all their threads have
/// exited or switched tasks, in which case the group it belongs to will not be
/// disposed until all such tasks have been dropped using `task.drop`.
///
/// [^1]: Although it does _imply_ such a notion in the discussion of ["semantic
/// tail
/// calls"](https://github.com/WebAssembly/component-model/blob/d1daf829e2da2293091c105121383ffbc3b3515b/design/mvp/Concurrency.md?plain=1#L339-L3410.
pub trait TaskGroupHook: Send + Sync + 'static {
    /// Handle notification that a new task group has been created (i.e. a
    /// host->guest call has been prepared).
    fn handle_start(&mut self, id: TaskGroupId) -> Result<()>;
    /// Handle notification that the runtime has switched to a thread belonging to
    /// the specified task group.
    fn handle_enter(&mut self, id: TaskGroupId) -> Result<()>;
    /// Handle notification that the runtime has switched away from a thread
    /// belonging to the specified task group.
    fn handle_exit(&mut self, id: TaskGroupId) -> Result<()>;
    /// Handle notification that the specified task group has been disposed of
    /// (i.e. the task created for the host->guest call for which the task group
    /// was created has exited, along with any and all subtasks transitively
    /// created by that task, and the guest has dropped any and all handles to
    /// those tasks).
    fn handle_finish(&mut self, id: TaskGroupId) -> Result<()>;
}

impl<T> Store<T> {
    /// Convenience wrapper for [`StoreContextMut::task_group_hook`]
    pub fn task_group_hook(&mut self, hook: impl TaskGroupHook) {
        self.as_context_mut().task_group_hook(hook);
    }
}

impl<T> StoreContextMut<'_, T> {
    /// Set a [`TaskGroupHook`] for this store.
    ///
    /// This will overwrite any hook that was previously set.
    pub fn task_group_hook(self, hook: impl TaskGroupHook) {
        self.0
            .concurrent_state_mut_without_forcing_current_thread()
            .task_group_hook = Some(Box::new(hook));
    }
}

impl StoreOpaque {
    pub(crate) fn clean_up_task_groups(&mut self) {
        if !self.concurrency_support() {
            return;
        }

        // Note that we ignore all errors here since, if we've reached here,
        // it's either because we've already poisoned the store and are in the
        // process of propagating the error which caused it to be poisoned or
        // because we're disposing of the store entirely.

        let state = self.concurrent_state_mut_without_forcing_current_thread();
        if let Some(mut hook) = state.task_group_hook.take() {
            let thread = state.unforced_current_thread;
            if let Ok(Some(group)) = thread.group(state) {
                _ = hook.handle_exit(group);
            }

            let groups = state
                .table
                .get_mut()
                .iter_mut()
                .filter_map(|(rep, entry)| {
                    if entry.downcast_mut::<TaskGroup>().is_some() {
                        Some(TableId::new(rep))
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();

            for group in groups {
                state.delete(group).unwrap();
                _ = hook.handle_finish(TaskGroupId(group));
            }

            state.task_group_hook = Some(hook);
        }
    }
}

impl CurrentThread {
    fn group(&self, state: &mut ConcurrentState) -> Result<Option<TaskGroupId>> {
        Ok(match self {
            Self::Guest(thread) | Self::DeferredHost(thread) => {
                Some(state.get_mut(thread.task)?.group)
            }
            Self::Host(task) => Some(state.get_mut(*task)?.group),
            Self::None => None,
        })
    }
}

impl ConcurrentState {
    pub(super) fn handle_thread_switch(
        &mut self,
        old: CurrentThread,
        new: CurrentThread,
    ) -> Result<()> {
        let old_group = old.group(self)?;
        let new_group = new.group(self)?;
        if let (true, Some(hook)) = ((old_group != new_group), &mut self.task_group_hook) {
            if let Some(group) = old_group {
                hook.handle_exit(group)?;
            }

            if let Some(group) = new_group {
                hook.handle_enter(group)?;
            }
        }
        Ok(())
    }

    pub(super) fn make_task_group(&mut self) -> Result<TaskGroupId> {
        let group = TaskGroupId(self.push(TaskGroup { ref_count: 1 })?);
        if let Some(hook) = &mut self.task_group_hook {
            hook.handle_start(group)?;
        }
        log::trace!("new {group:?}");
        Ok(group)
    }

    pub(super) fn increment_group_ref_count(&mut self, group: TaskGroupId) -> Result<()> {
        let count = &mut self.get_mut(group.0)?.ref_count;
        *count += 1;
        log::trace!("increment {group:?} to {count}");
        Ok(())
    }

    pub(super) fn decrement_group_ref_count(&mut self, group: TaskGroupId) -> Result<()> {
        let count = &mut self.get_mut(group.0)?.ref_count;
        assert!(*count >= 1);
        *count -= 1;
        log::trace!("decrement {group:?} to {count}");
        if *count == 0 {
            self.delete(group.0)?;
            if let Some(hook) = &mut self.task_group_hook {
                hook.handle_finish(group)?;
            }
        }
        Ok(())
    }
}
