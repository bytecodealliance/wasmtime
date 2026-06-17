use std::sync::Arc;

use crate::program::Program;
use crate::types::field_type_by_index;
use cranelift_isle::{
    error::{Errors, ErrorsBuilder},
    files::Files,
    sema::{ExternalSig, ReturnKind, TermEnv, TermId, Type, TypeEnv, TypeId},
    trie_again::{self, Binding, BindingId, RuleSet},
};

pub fn build_trie(termenv: &TermEnv, files: Arc<Files>) -> Result<Vec<(TermId, RuleSet)>, Errors> {
    let (terms, errors) = trie_again::build(termenv);
    if errors.is_empty() {
        Ok(terms)
    } else {
        Err(ErrorsBuilder::new()
            .errors(errors)
            .files(files.clone())
            .build())
    }
}

#[derive(Clone, Debug)]
pub enum BindingType {
    Base(TypeId),
    Option(Box<BindingType>),
    Tuple(Vec<BindingType>),
}

impl BindingType {
    pub fn display(&self, tyenv: &TypeEnv) -> String {
        match self {
            BindingType::Base(type_id) => {
                let ty = &tyenv.types[type_id.index()];
                ty.name(tyenv).to_string()
            }
            BindingType::Option(inner) => format!("Option({})", inner.display(tyenv)),
            BindingType::Tuple(inners) => format!(
                "({inners})",
                inners = inners
                    .iter()
                    .map(|inner| inner.display(tyenv))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }
}

/// Determine the type of a given binding.
pub fn binding_type(
    binding: &Binding,
    term_id: TermId,
    prog: &Program,
    // TODO(mbm): is there a less ugly way to do binding lookup here?
    lookup_binding: impl Fn(BindingId) -> Binding,
) -> BindingType {
    match binding {
        Binding::ConstInt { ty, .. }
        | Binding::ConstBool { ty, .. }
        | Binding::MakeVariant { ty, .. }
        | Binding::MakeStruct { ty, .. } => BindingType::Base(*ty),

        Binding::ConstPrim { val } => BindingType::Base(prog.tyenv.const_types[val]),

        Binding::Argument { index } => {
            let term = &prog.termenv.terms[term_id.index()];
            BindingType::Base(term.arg_tys[index.index()])
        }

        Binding::Extractor { term, .. } => {
            // Determine the extractor signature.
            let term = &prog.termenv.terms[term.index()];
            let sig = term
                .extractor_sig(&prog.tyenv)
                .expect("term should have extractor signature");
            external_sig_return_type(&sig)
        }

        Binding::Constructor { term, .. } => {
            // Determine the constructor signature.
            let term = &prog.termenv.terms[term.index()];
            let sig = term
                .constructor_sig(&prog.tyenv)
                .expect("term should have constructor signature");
            external_sig_return_type(&sig)
        }

        Binding::MakeSome { inner } => {
            let inner_binding = lookup_binding(*inner);
            let inner_ty = binding_type(&inner_binding, term_id, prog, lookup_binding);
            BindingType::Option(Box::new(inner_ty))
        }

        Binding::MatchSome { source } => {
            let source_binding = lookup_binding(*source);
            let source_ty = binding_type(&source_binding, term_id, prog, lookup_binding);
            match source_ty {
                BindingType::Option(ty) => *ty,
                _ => unreachable!("source of match some should be an option"),
            }
        }

        Binding::MatchVariant {
            source,
            variant,
            field,
        } => {
            // Lookup type ID for the underlying enum.
            let source_binding = lookup_binding(*source);
            let source_ty = binding_type(&source_binding, term_id, prog, lookup_binding);
            let source_type_id = match source_ty {
                BindingType::Base(type_id) => type_id,
                _ => unreachable!("source of match variant should be a base type"),
            };

            // Lookup variant.
            let enum_ty = &prog.tyenv.types[source_type_id.index()];
            let variant = match enum_ty {
                Type::Enum { variants, .. } => &variants[variant.index()],
                _ => unreachable!("source match variant should be an enum"),
            };

            // Lookup field type.
            BindingType::Base(field_type_by_index(&variant.fields, field.index()))
        }

        Binding::ExtractStruct { source, field } => {
            // Lookup type ID for the underlying struct.
            let source_binding = lookup_binding(*source);
            let source_ty = binding_type(&source_binding, term_id, prog, lookup_binding);
            let source_type_id = match source_ty {
                BindingType::Base(type_id) => type_id,
                _ => unreachable!("source of extract_struct should be a base type"),
            };

            // Lookup field type.
            let struct_ty = &prog.tyenv.types[source_type_id.index()];
            let fields = match struct_ty {
                Type::Struct { fields, .. } => fields,
                _ => unreachable!("source of extract_struct should be a struct"),
            };
            BindingType::Base(field_type_by_index(fields, field.index()))
        }

        Binding::MatchTuple { source, field } => {
            let source_binding = lookup_binding(*source);
            let source_ty = binding_type(&source_binding, term_id, prog, lookup_binding);
            match source_ty {
                BindingType::Tuple(tys) => tys[field.index()].clone(),
                _ => unreachable!("source type should be a tuple"),
            }
        }

        Binding::Iterator { .. } => unimplemented!("iterator bindings not supported"),
    }
}

fn external_sig_return_type(sig: &ExternalSig) -> BindingType {
    // Multiple return types are represented as a tuple.
    let ty = if sig.ret_tys.len() == 1 {
        BindingType::Base(sig.ret_tys[0])
    } else {
        BindingType::Tuple(sig.ret_tys.iter().copied().map(BindingType::Base).collect())
    };

    // Fallible terms return option type.
    match sig.ret_kind {
        ReturnKind::Option => BindingType::Option(Box::new(ty)),
        ReturnKind::Plain => ty,
        ReturnKind::Iterator => unimplemented!("extractor iterator return"),
    }
}
