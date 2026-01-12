use wasmparser::ValType;

use crate::component::info::Accessor;
use crate::component::{ComponentContext, ComponentInstanceState, WIZER_INSTANCE};
use crate::snapshot::Snapshot;
use crate::{InstanceState, SnapshotVal};

/// Snapshot result of a component.
///
/// Currently this only contains the state for instances found within the
/// component, and each instance's snapshot is defined as the same core wasm
/// snapshot state.
pub struct ComponentSnapshot {
    /// List of snapshots for corresponding module indices.
    ///
    /// Note that modules don't get a snapshot if they aren't instantiated.
    pub(crate) modules: Vec<(u32, Snapshot)>,
}

pub async fn snapshot(
    component: &ComponentContext<'_>,
    ctx: &mut impl ComponentInstanceState,
) -> ComponentSnapshot {
    let mut modules = Vec::new();

    for (module_index, module) in component.core_modules() {
        // Ignore uninstantiated modules.
        if !component.core_instantiations.contains_key(&module_index) {
            continue;
        }

        // Use the core-level snapshotting routines to collect a
        // snapshot of the module. Requests for core-level items are
        // redirected through the component-level `ctx` provided.
        let snapshot = crate::snapshot::snapshot(
            module,
            &mut ViaComponent {
                ctx,
                module_index,
                component,
            },
        )
        .await;
        modules.push((module_index, snapshot));
    }

    ComponentSnapshot { modules }
}

/// Implementation of `InstanceState` via component model primitives.
struct ViaComponent<'a, 'b, S> {
    ctx: &'a mut S,
    component: &'a ComponentContext<'b>,
    module_index: u32,
}

impl<'a, S> ViaComponent<'a, '_, S>
where
    S: ComponentInstanceState,
{
    fn get_accessor(&self, name: &str) -> &'a Accessor {
        let accessors = self.component.accessors.as_ref().unwrap();
        accessors
            .iter()
            .find(|a| match a {
                Accessor::Global {
                    module_index,
                    core_export_name,
                    ..
                }
                | Accessor::Memory {
                    module_index,
                    core_export_name,
                    ..
                } => *module_index == self.module_index && core_export_name == name,
            })
            .unwrap()
    }
}

impl<S> InstanceState for ViaComponent<'_, '_, S>
where
    S: ComponentInstanceState,
{
    async fn global_get(&mut self, name: &str, _: ValType) -> SnapshotVal {
        let Accessor::Global {
            accessor_export_name,
            ty,
            ..
        } = self.get_accessor(name)
        else {
            panic!("expected global accessor for {name}");
        };
        match ty {
            wasmparser::ValType::I32 => SnapshotVal::I32(
                self.ctx
                    .call_func_ret_s32(WIZER_INSTANCE, accessor_export_name)
                    .await,
            ),
            wasmparser::ValType::I64 => SnapshotVal::I64(
                self.ctx
                    .call_func_ret_s64(WIZER_INSTANCE, accessor_export_name)
                    .await,
            ),
            wasmparser::ValType::F32 => SnapshotVal::F32(
                self.ctx
                    .call_func_ret_f32(WIZER_INSTANCE, accessor_export_name)
                    .await,
            ),
            wasmparser::ValType::F64 => SnapshotVal::F64(
                self.ctx
                    .call_func_ret_f64(WIZER_INSTANCE, accessor_export_name)
                    .await,
            ),
            wasmparser::ValType::V128 => todo!(),
            wasmparser::ValType::Ref(_) => unreachable!(),
        }
    }

    async fn memory_contents(&mut self, name: &str, contents: impl FnOnce(&[u8]) + Send) {
        let Accessor::Memory {
            accessor_export_name,
            ..
        } = self.get_accessor(name)
        else {
            panic!("expected memory accessor for {name}");
        };
        self.ctx
            .call_func_ret_list_u8(WIZER_INSTANCE, accessor_export_name, contents)
            .await
    }
}
