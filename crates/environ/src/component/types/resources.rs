use crate::component::{
    ComponentTypes, ResourceIndex, RuntimeComponentInstanceIndex, TypeResourceTable,
    TypeResourceTableIndex,
};
use std::collections::HashMap;
use wasmparser::types;

/// TODO
/// TODO: this is a very special cache
#[derive(Default, Clone)]
pub struct ResourcesBuilder {
    resource_id_to_table_index: HashMap<types::ResourceId, TypeResourceTableIndex>,
    resource_id_to_resource_index: HashMap<types::ResourceId, ResourceIndex>,
    current_instance: Option<RuntimeComponentInstanceIndex>,
}

impl ResourcesBuilder {
    /// TODO
    pub fn convert(
        &mut self,
        id: types::ResourceId,
        types: &mut ComponentTypes,
    ) -> TypeResourceTableIndex {
        *self
            .resource_id_to_table_index
            .entry(id)
            .or_insert_with(|| {
                let ty = self.resource_id_to_resource_index[&id];
                let instance = self.current_instance.expect("current instance not set");
                types
                    .resource_tables
                    .push(TypeResourceTable { ty, instance })
            })
    }

    /// TODO
    pub fn register_component_entity_type<'a>(
        &mut self,
        types: types::TypesRef<'a>,
        ty: types::ComponentEntityType,
        path: &mut Vec<&'a str>,
        register: &mut dyn FnMut(&[&'a str]) -> ResourceIndex,
    ) {
        match ty {
            types::ComponentEntityType::Instance(id) => {
                let ty = types
                    .type_from_id(id)
                    .unwrap()
                    .as_component_instance_type()
                    .unwrap();
                for (name, ty) in ty.exports.iter() {
                    path.push(name);
                    self.register_component_entity_type(types, *ty, path, register);
                    path.pop();
                }
            }
            types::ComponentEntityType::Type { created, .. } => {
                let id = match types.type_from_id(created).unwrap() {
                    types::Type::Resource(id) => *id,
                    _ => return,
                };
                self.resource_id_to_resource_index
                    .entry(id)
                    .or_insert_with(|| register(path));
            }

            // TODO: comment why not needed
            types::ComponentEntityType::Func(_)
            | types::ComponentEntityType::Module(_)
            | types::ComponentEntityType::Component(_)
            | types::ComponentEntityType::Value(_) => {}
        }
    }

    /// TODO
    pub fn register_resource<'a>(
        &mut self,
        types: types::TypesRef<'a>,
        id: types::TypeId,
        ty: ResourceIndex,
    ) {
        let id = match types.type_from_id(id).unwrap() {
            types::Type::Resource(id) => *id,
            _ => unreachable!(),
        };
        let prev = self.resource_id_to_resource_index.insert(id, ty);
        assert!(prev.is_none());
    }

    /// TODO
    pub fn set_current_instance(&mut self, instance: RuntimeComponentInstanceIndex) {
        self.current_instance = Some(instance);
    }
}
