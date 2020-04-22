use super::{atom_token, int_repr_tokens};
use crate::names::Names;

use proc_macro2::TokenStream;
use quote::quote;

pub(super) fn define_int(names: &Names, name: &witx::Id, i: &witx::IntDatatype) -> TokenStream {
    let rt = names.runtime_mod();
    let ident = names.type_(&name);
    let repr = int_repr_tokens(i.repr);
    let abi_repr = atom_token(match i.repr {
        witx::IntRepr::U8 | witx::IntRepr::U16 | witx::IntRepr::U32 => witx::AtomType::I32,
        witx::IntRepr::U64 => witx::AtomType::I64,
    });
    let consts = i
        .consts
        .iter()
        .map(|r#const| {
            let const_ident = names.int_member(&r#const.name);
            let value = r#const.value;
            quote!(pub const #const_ident: #ident = #ident(#value))
        })
        .collect::<Vec<_>>();

    quote! {
        #[repr(transparent)]
        #[derive(Copy, Clone, Debug, ::std::hash::Hash, Eq, PartialEq)]
        pub struct #ident(#repr);

        impl #ident {
            #(#consts;)*
        }

        impl ::std::fmt::Display for #ident {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                write!(f, "{:?}", self)
            }
        }

        impl ::std::convert::TryFrom<#repr> for #ident {
            type Error = #rt::GuestError;
            fn try_from(value: #repr) -> Result<Self, #rt::GuestError> {
                Ok(#ident(value))
            }
        }

        impl ::std::convert::TryFrom<#abi_repr> for #ident {
            type Error = #rt::GuestError;
            fn try_from(value: #abi_repr) -> Result<#ident, #rt::GuestError> {
                #ident::try_from(value as #repr)
            }
        }

        impl From<#ident> for #repr {
            fn from(e: #ident) -> #repr {
                e.0
            }
        }

        impl From<#ident> for #abi_repr {
            fn from(e: #ident) -> #abi_repr {
                #repr::from(e) as #abi_repr
            }
        }

        impl<'a> #rt::GuestType<'a> for #ident {
            fn guest_size() -> u32 {
                #repr::guest_size()
            }

            fn guest_align() -> usize {
                #repr::guest_align()
            }

            fn read(location: &#rt::GuestPtr<'a, #ident>) -> Result<#ident, #rt::GuestError> {
                Ok(#ident(#repr::read(&location.cast())?))

            }

            fn write(location: &#rt::GuestPtr<'_, #ident>, val: Self) -> Result<(), #rt::GuestError> {
                #repr::write(&location.cast(), val.0)
            }
        }

        unsafe impl<'a> #rt::GuestTypeTransparent<'a> for #ident {
            #[inline]
            fn validate(_location: *mut #ident) -> Result<(), #rt::GuestError> {
                // All bit patterns accepted
                Ok(())
            }
        }

    }
}
