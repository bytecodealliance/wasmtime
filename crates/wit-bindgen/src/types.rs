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
    pub fn analyze(&mut self, iface: &Interface) {
        for (t, _) in iface.types.iter() {
            self.type_id_info(iface, t);
        }
        for f in iface.functions.iter() {
            for (_, ty) in f.params.iter() {
                self.set_param_result_ty(
                    iface,
                    ty,
                    TypeInfo {
                        param: true,
                        ..TypeInfo::default()
                    },
                );
            }
            for ty in f.results.iter_types() {
                self.set_param_result_ty(
                    iface,
                    ty,
                    TypeInfo {
                        result: true,
                        ..TypeInfo::default()
                    },
                );
            }
        }
    }

    pub fn get(&self, id: TypeId) -> TypeInfo {
        self.type_info[&id]
    }

    fn type_id_info(&mut self, iface: &Interface, ty: TypeId) -> TypeInfo {
        if let Some(info) = self.type_info.get(&ty) {
            return *info;
        }
        let mut info = TypeInfo::default();
        match &iface.types[ty].kind {
            TypeDefKind::Record(r) => {
                for field in r.fields.iter() {
                    info |= self.type_info(iface, &field.ty);
                }
            }
            TypeDefKind::Tuple(t) => {
                for ty in t.types.iter() {
                    info |= self.type_info(iface, ty);
                }
            }
            TypeDefKind::Flags(_) => {}
            TypeDefKind::Enum(_) => {}
            TypeDefKind::Variant(v) => {
                for case in v.cases.iter() {
                    info |= self.optional_type_info(iface, case.ty.as_ref());
                }
            }
            TypeDefKind::List(ty) => {
                info = self.type_info(iface, ty);
                info.has_list = true;
            }
            TypeDefKind::Type(ty) => {
                info = self.type_info(iface, ty);
            }
            TypeDefKind::Option(ty) => {
                info = self.type_info(iface, ty);
            }
            TypeDefKind::Result(r) => {
                info = self.optional_type_info(iface, r.ok.as_ref());
                info |= self.optional_type_info(iface, r.err.as_ref());
            }
            TypeDefKind::Union(u) => {
                for case in u.cases.iter() {
                    info |= self.type_info(iface, &case.ty);
                }
            }
            TypeDefKind::Future(ty) => {
                info = self.optional_type_info(iface, ty.as_ref());
            }
            TypeDefKind::Stream(stream) => {
                info = self.optional_type_info(iface, stream.element.as_ref());
                info |= self.optional_type_info(iface, stream.end.as_ref());
            }
        }
        self.type_info.insert(ty, info);
        info
    }

    fn type_info(&mut self, iface: &Interface, ty: &Type) -> TypeInfo {
        let mut info = TypeInfo::default();
        match ty {
            Type::String => info.has_list = true,
            Type::Id(id) => return self.type_id_info(iface, *id),
            _ => {}
        }
        info
    }

    fn optional_type_info(&mut self, iface: &Interface, ty: Option<&Type>) -> TypeInfo {
        match ty {
            Some(ty) => self.type_info(iface, ty),
            None => TypeInfo::default(),
        }
    }

    fn set_param_result_id(&mut self, iface: &Interface, ty: TypeId, info: TypeInfo) {
        match &iface.types[ty].kind {
            TypeDefKind::Record(r) => {
                for field in r.fields.iter() {
                    self.set_param_result_ty(iface, &field.ty, info)
                }
            }
            TypeDefKind::Tuple(t) => {
                for ty in t.types.iter() {
                    self.set_param_result_ty(iface, ty, info)
                }
            }
            TypeDefKind::Flags(_) => {}
            TypeDefKind::Enum(_) => {}
            TypeDefKind::Variant(v) => {
                for case in v.cases.iter() {
                    self.set_param_result_optional_ty(iface, case.ty.as_ref(), info)
                }
            }
            TypeDefKind::List(ty) | TypeDefKind::Type(ty) | TypeDefKind::Option(ty) => {
                self.set_param_result_ty(iface, ty, info)
            }
            TypeDefKind::Result(r) => {
                self.set_param_result_optional_ty(iface, r.ok.as_ref(), info);
                let mut info2 = info;
                info2.error = info.result;
                self.set_param_result_optional_ty(iface, r.err.as_ref(), info2);
            }
            TypeDefKind::Union(u) => {
                for case in u.cases.iter() {
                    self.set_param_result_ty(iface, &case.ty, info)
                }
            }
            TypeDefKind::Future(ty) => self.set_param_result_optional_ty(iface, ty.as_ref(), info),
            TypeDefKind::Stream(stream) => {
                self.set_param_result_optional_ty(iface, stream.element.as_ref(), info);
                self.set_param_result_optional_ty(iface, stream.end.as_ref(), info);
            }
        }
    }

    fn set_param_result_ty(&mut self, iface: &Interface, ty: &Type, info: TypeInfo) {
        match ty {
            Type::Id(id) => {
                self.type_id_info(iface, *id);
                let cur = self.type_info.get_mut(id).unwrap();
                let prev = *cur;
                *cur |= info;
                if prev != *cur {
                    self.set_param_result_id(iface, *id, info);
                }
            }
            _ => {}
        }
    }

    fn set_param_result_optional_ty(
        &mut self,
        iface: &Interface,
        ty: Option<&Type>,
        info: TypeInfo,
    ) {
        match ty {
            Some(ty) => self.set_param_result_ty(iface, ty, info),
            None => (),
        }
    }
}
