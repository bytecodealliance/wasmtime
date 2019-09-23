use syn::{token, FnArg, Ident, Pat, Path, ReturnType, Signature, Type};

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum PtrOrRef {
    Ptr,
    Ref,
}

pub(crate) enum ParameterType<'a> {
    VMContextMutPtr,
    SelfRef(Option<token::Mut>),
    Context(Option<(PtrOrRef, Option<token::Mut>)>),
    Ptr(&'a Type, PtrOrRef, Option<token::Mut>),
    Simple(&'a Type),
}

pub(crate) enum Return<'a> {
    Ptr(&'a Type, PtrOrRef, Option<token::Mut>),
    Simple(&'a Type),
}

pub(crate) struct Parameter<'a> {
    pub(crate) id: Option<&'a Ident>,
    pub(crate) ty: ParameterType<'a>,
}

pub(crate) struct MethodSignature<'a> {
    pub(crate) params: Vec<Parameter<'a>>,
    pub(crate) result: Option<Return<'a>>,
    pub(crate) original_params: Vec<Option<&'a Type>>,
    pub(crate) original_result: Option<&'a Type>,
}

pub(crate) fn read_signature<'a>(
    sig: &'a Signature,
    context: &Option<Path>,
) -> MethodSignature<'a> {
    let mut params = Vec::new();
    let mut original_params = Vec::new();
    for i in &sig.inputs {
        match i {
            FnArg::Typed(t) => {
                let id = Some(if let Pat::Ident(ref id) = *t.pat {
                    assert!(id.attrs.is_empty());
                    assert!(id.subpat.is_none());
                    &id.ident
                } else {
                    panic!("no id");
                });
                let ty = match *t.ty {
                    Type::Ptr(ref pt) => match *pt.elem {
                        Type::Path(ref p)
                            if p.path.is_ident("VMContext") && pt.mutability.is_some() =>
                        {
                            ParameterType::VMContextMutPtr
                        }
                        Type::Path(ref p) if Some(&p.path) == context.as_ref() => {
                            ParameterType::Context(Some((PtrOrRef::Ptr, pt.mutability.clone())))
                        }
                        _ => ParameterType::Ptr(&t.ty, PtrOrRef::Ptr, pt.mutability),
                    },
                    Type::Path(ref tp) => {
                        if context.as_ref().map(|c| *c == tp.path) == Some(true) {
                            ParameterType::Context(None)
                        } else {
                            ParameterType::Simple(&t.ty)
                        }
                    }
                    Type::Reference(ref tr) => {
                        let is_context = if let Type::Path(ref tp) = *tr.elem {
                            context.as_ref().map(|c| *c == tp.path) == Some(true)
                        } else {
                            false
                        };
                        if is_context {
                            ParameterType::Context(Some((PtrOrRef::Ref, tr.mutability.clone())))
                        } else {
                            ParameterType::Ptr(&t.ty, PtrOrRef::Ref, tr.mutability.clone())
                        }
                    }
                    _ => panic!("Unsupported param type declaration"),
                };
                params.push(Parameter { id, ty });
                original_params.push(Some(&*t.ty));
            }
            FnArg::Receiver(r) => {
                assert!(r.attrs.is_empty());
                assert!(r.reference.is_some(), "self needs reference");
                params.push(Parameter {
                    id: None,
                    ty: ParameterType::SelfRef(r.mutability.clone().into()),
                });
                original_params.push(None);
            }
        }
    }
    let (result, original_result) = if let ReturnType::Type(_, ref rt) = sig.output {
        (
            Some(match **rt {
                Type::Ptr(ref pt) => Return::Ptr(&**rt, PtrOrRef::Ptr, pt.mutability.clone()),
                Type::Path(_) => Return::Simple(&**rt),
                _ => panic!("Unsupported result type declaration"),
            }),
            Some(&**rt),
        )
    } else {
        (None, None)
    };
    MethodSignature {
        params,
        result,
        original_params,
        original_result,
    }
}
