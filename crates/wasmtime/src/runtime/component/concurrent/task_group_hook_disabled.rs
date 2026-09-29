use crate::component::concurrent::{ConcurrentState, CurrentThread};
use crate::error::Result;
use crate::store::StoreOpaque;

/// Represents a "task group" containing the "root" task of a host->guest call,
/// plus any subtasks transitively created by that task.
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct TaskGroupId;

impl StoreOpaque {
    pub(crate) fn clean_up_task_groups(&mut self) {}
}

impl ConcurrentState {
    pub(super) fn switch_threads(
        &mut self,
        _old: CurrentThread,
        _new: CurrentThread,
    ) -> Result<()> {
        Ok(())
    }

    pub(super) fn make_task_group(&mut self) -> Result<TaskGroupId> {
        Ok(TaskGroupId)
    }

    pub(super) fn increment_group_ref_count(&mut self, _group: TaskGroupId) -> Result<()> {
        Ok(())
    }

    pub(super) fn decrement_group_ref_count(&mut self, _group: TaskGroupId) -> Result<()> {
        Ok(())
    }
}
