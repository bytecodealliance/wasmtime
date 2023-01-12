use std::collections::HashMap;
use wit_parser::*;

#[derive(Default)]
pub struct Types {
    type_info: HashMap<TypeId, TypeInfo>,
}

#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub struct TypeInfo {
    /// Whether or not this type is ever used (transitively) within the
    /// parameter of a function.
    pub param: bool,

    /// Whether or not this type is ever used (transitively) within the
    /// result of a function.
    pub result: bool,

    /// Whether or not this type is ever used (transitively) within the
    /// error case in the result of a function.
    pub error: bool,

    /// Whether or not this type (transitively) has a list.
    pub has_list: bool,
}

impl std::ops::BitOrAssign for TypeInfo {
    fn bitor_assign(&mut self, rhs: Self) {
        self.param |= rhs.param;
        self.result |= rhs.result;
        self.error |= rhs.error;
        self.has_list |= rhs.has_list;
    }
}

impl Types {
    pub fn analyze(&mut self, resolve: &Resolve, world: WorldId) {
        let world = &resolve.worlds[world];
        for (_, item) in world.imports.iter().chain(world.exports.iter()) {
            match item {
                WorldItem::Function(f) => self.type_info_func(resolve, f),
                WorldItem::Interface(id) => {
                    let iface = &resolve.interfaces[*id];

                    for (_, t) in iface.types.iter() {
                        self.type_id_info(resolve, *t);
                    }
                    for (_, f) in iface.functions.iter() {
                        self.type_info_func(resolve, f);
                    }
                }
            }
        }
    }

    fn type_info_func(&mut self, resolve: &Resolve, func: &Function) {
        for (_, ty) in func.params.iter() {
            self.set_param_result_ty(
                resolve,
                ty,
                TypeInfo {
                    param: true,
                    ..TypeInfo::default()
                },
            );
        }
        for ty in func.results.iter_types() {
            self.set_param_result_ty(
                resolve,
                ty,
                TypeInfo {
                    result: true,
                    ..TypeInfo::default()
                },
            );
        }
    }

    pub fn get(&self, id: TypeId) -> TypeInfo {
        self.type_info[&id]
    }

    fn type_id_info(&mut self, resolve: &Resolve, ty: TypeId) -> TypeInfo {
        if let Some(info) = self.type_info.get(&ty) {
            return *info;
        }
        let mut info = TypeInfo::default();
        match &resolve.types[ty].kind {
            TypeDefKind::Record(r) => {
                for field in r.fields.iter() {
                    info |= self.type_info(resolve, &field.ty);
                }
            }
            TypeDefKind::Tuple(t) => {
                for ty in t.types.iter() {
                    info |= self.type_info(resolve, ty);
                }
            }
            TypeDefKind::Flags(_) => {}
            TypeDefKind::Enum(_) => {}
            TypeDefKind::Variant(v) => {
                for case in v.cases.iter() {
                    info |= self.optional_type_info(resolve, case.ty.as_ref());
                }
            }
            TypeDefKind::List(ty) => {
                info = self.type_info(resolve, ty);
                info.has_list = true;
            }
            TypeDefKind::Type(ty) => {
                info = self.type_info(resolve, ty);
            }
            TypeDefKind::Option(ty) => {
                info = self.type_info(resolve, ty);
            }
            TypeDefKind::Result(r) => {
                info = self.optional_type_info(resolve, r.ok.as_ref());
                info |= self.optional_type_info(resolve, r.err.as_ref());
            }
            TypeDefKind::Union(u) => {
                for case in u.cases.iter() {
                    info |= self.type_info(resolve, &case.ty);
                }
            }
            TypeDefKind::Future(ty) => {
                info = self.optional_type_info(resolve, ty.as_ref());
            }
            TypeDefKind::Stream(stream) => {
                info = self.optional_type_info(resolve, stream.element.as_ref());
                info |= self.optional_type_info(resolve, stream.end.as_ref());
            }
            TypeDefKind::Unknown => unreachable!(),
        }
        self.type_info.insert(ty, info);
        info
    }

    fn type_info(&mut self, resolve: &Resolve, ty: &Type) -> TypeInfo {
        let mut info = TypeInfo::default();
        match ty {
            Type::String => info.has_list = true,
            Type::Id(id) => return self.type_id_info(resolve, *id),
            _ => {}
        }
        info
    }

    fn optional_type_info(&mut self, resolve: &Resolve, ty: Option<&Type>) -> TypeInfo {
        match ty {
            Some(ty) => self.type_info(resolve, ty),
            None => TypeInfo::default(),
        }
    }

    fn set_param_result_id(&mut self, resolve: &Resolve, ty: TypeId, info: TypeInfo) {
        match &resolve.types[ty].kind {
            TypeDefKind::Record(r) => {
                for field in r.fields.iter() {
                    self.set_param_result_ty(resolve, &field.ty, info)
                }
            }
            TypeDefKind::Tuple(t) => {
                for ty in t.types.iter() {
                    self.set_param_result_ty(resolve, ty, info)
                }
            }
            TypeDefKind::Flags(_) => {}
            TypeDefKind::Enum(_) => {}
            TypeDefKind::Variant(v) => {
                for case in v.cases.iter() {
                    self.set_param_result_optional_ty(resolve, case.ty.as_ref(), info)
                }
            }
            TypeDefKind::List(ty) | TypeDefKind::Type(ty) | TypeDefKind::Option(ty) => {
                self.set_param_result_ty(resolve, ty, info)
            }
            TypeDefKind::Result(r) => {
                self.set_param_result_optional_ty(resolve, r.ok.as_ref(), info);
                let mut info2 = info;
                info2.error = info.result;
                self.set_param_result_optional_ty(resolve, r.err.as_ref(), info2);
            }
            TypeDefKind::Union(u) => {
                for case in u.cases.iter() {
                    self.set_param_result_ty(resolve, &case.ty, info)
                }
            }
            TypeDefKind::Future(ty) => {
                self.set_param_result_optional_ty(resolve, ty.as_ref(), info)
            }
            TypeDefKind::Stream(stream) => {
                self.set_param_result_optional_ty(resolve, stream.element.as_ref(), info);
                self.set_param_result_optional_ty(resolve, stream.end.as_ref(), info);
            }
            TypeDefKind::Unknown => unreachable!(),
        }
    }

    fn set_param_result_ty(&mut self, resolve: &Resolve, ty: &Type, info: TypeInfo) {
        match ty {
            Type::Id(id) => {
                self.type_id_info(resolve, *id);
                let cur = self.type_info.get_mut(id).unwrap();
                let prev = *cur;
                *cur |= info;
                if prev != *cur {
                    self.set_param_result_id(resolve, *id, info);
                }
            }
            _ => {}
        }
    }

    fn set_param_result_optional_ty(
        &mut self,
        resolve: &Resolve,
        ty: Option<&Type>,
        info: TypeInfo,
    ) {
        match ty {
            Some(ty) => self.set_param_result_ty(resolve, ty, info),
            None => (),
        }
    }
}
