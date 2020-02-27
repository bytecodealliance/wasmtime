use super::{atom_token, int_repr_tokens};
use crate::names::Names;

use proc_macro2::TokenStream;
use quote::quote;

pub(super) fn define_enum(names: &Names, name: &witx::Id, e: &witx::EnumDatatype) -> TokenStream {
    let ident = names.type_(&name);

    let repr = int_repr_tokens(e.repr);
    let abi_repr = atom_token(match e.repr {
        witx::IntRepr::U8 | witx::IntRepr::U16 | witx::IntRepr::U32 => witx::AtomType::I32,
        witx::IntRepr::U64 => witx::AtomType::I64,
    });

    let mut variant_names = vec![];
    let mut tryfrom_repr_cases = vec![];
    let mut to_repr_cases = vec![];
    let mut to_display = vec![];

    for (n, variant) in e.variants.iter().enumerate() {
        let variant_name = names.enum_variant(&variant.name);
        let docs = variant.docs.trim();
        let ident_str = ident.to_string();
        let variant_str = variant_name.to_string();
        tryfrom_repr_cases.push(quote!(#n => Ok(#ident::#variant_name)));
        to_repr_cases.push(quote!(#ident::#variant_name => #n as #repr));
        to_display.push(quote!(#ident::#variant_name => format!("{} ({}::{}({}))", #docs, #ident_str, #variant_str, #repr::from(*self))));
        variant_names.push(variant_name);
    }

    quote! {
        #[repr(#repr)]
        #[derive(Copy, Clone, Debug, ::std::hash::Hash, Eq, PartialEq)]
        pub enum #ident {
            #(#variant_names),*
        }

        impl ::std::fmt::Display for #ident {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                let to_str = match self {
                    #(#to_display,)*
                };
                write!(f, "{}", to_str)
            }
        }

        impl ::std::convert::TryFrom<#repr> for #ident {
            type Error = wiggle_runtime::GuestError;
            fn try_from(value: #repr) -> Result<#ident, wiggle_runtime::GuestError> {
                match value as usize {
                    #(#tryfrom_repr_cases),*,
                    _ => Err(wiggle_runtime::GuestError::InvalidEnumValue(stringify!(#ident))),
                }
            }
        }

        impl ::std::convert::TryFrom<#abi_repr> for #ident {
            type Error = wiggle_runtime::GuestError;
            fn try_from(value: #abi_repr) -> Result<#ident, wiggle_runtime::GuestError> {
                #ident::try_from(value as #repr)
            }
        }

        impl From<#ident> for #repr {
            fn from(e: #ident) -> #repr {
                match e {
                    #(#to_repr_cases),*
                }
            }
        }

        impl From<#ident> for #abi_repr {
            fn from(e: #ident) -> #abi_repr {
                #repr::from(e) as #abi_repr
            }
        }

        impl<'a> wiggle_runtime::GuestType<'a> for #ident {
            fn size() -> u32 {
                ::std::mem::size_of::<#repr>() as u32
            }

            fn align() -> u32 {
                ::std::mem::align_of::<#repr>() as u32
            }

            fn name() -> String {
                stringify!(#ident).to_owned()
            }

            fn validate(location: &wiggle_runtime::GuestPtr<'a, #ident>) -> Result<(), wiggle_runtime::GuestError> {
                use ::std::convert::TryFrom;
                let raw: #repr = unsafe { (location.as_raw() as *const #repr).read() };
                let _ = #ident::try_from(raw)?;
                Ok(())
            }

            fn read(location: &wiggle_runtime::GuestPtr<#ident>) -> Result<#ident, wiggle_runtime::GuestError> {
                // Perform validation as part of as_ref:
                let r = location.as_ref()?;
                Ok(*r)
            }

            fn write(&self, location: &wiggle_runtime::GuestPtrMut<#ident>) {
                let val: #repr = #repr::from(*self);
                unsafe { (location.as_raw() as *mut #repr).write(val) };
            }
        }

        impl<'a> wiggle_runtime::GuestTypeTransparent<'a> for #ident {}
    }
}
