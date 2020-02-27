use super::{atom_token, int_repr_tokens};
use crate::names::Names;

use proc_macro2::{Literal, TokenStream};
use quote::quote;
use std::convert::TryFrom;

pub(super) fn define_flags(names: &Names, name: &witx::Id, f: &witx::FlagsDatatype) -> TokenStream {
    let ident = names.type_(&name);
    let repr = int_repr_tokens(f.repr);
    let abi_repr = atom_token(match f.repr {
        witx::IntRepr::U8 | witx::IntRepr::U16 | witx::IntRepr::U32 => witx::AtomType::I32,
        witx::IntRepr::U64 => witx::AtomType::I64,
    });

    let mut flag_constructors = vec![];
    let mut all_values = 0;
    for (i, f) in f.flags.iter().enumerate() {
        let name = names.flag_member(&f.name);
        let value = 1u128
            .checked_shl(u32::try_from(i).expect("flag value overflow"))
            .expect("flag value overflow");
        let value_token = Literal::u128_unsuffixed(value);
        flag_constructors.push(quote!(pub const #name: #ident = #ident(#value_token)));
        all_values += value;
    }
    let all_values_token = Literal::u128_unsuffixed(all_values);

    let ident_str = ident.to_string();

    quote! {
        #[repr(transparent)]
        #[derive(Copy, Clone, Debug, ::std::hash::Hash, Eq, PartialEq)]
        pub struct #ident(#repr);

        impl #ident {
            #(#flag_constructors);*;
            pub const ALL_FLAGS: #ident = #ident(#all_values_token);

            pub fn contains(&self, other: &#ident) -> bool {
                #repr::from(!*self & *other) == 0 as #repr
            }
        }

        impl ::std::fmt::Display for #ident {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                write!(f, "{}({:#b})", #ident_str, self.0)
            }
        }

        impl ::std::ops::BitAnd for #ident {
            type Output = Self;
            fn bitand(self, rhs: Self) -> Self::Output {
                #ident(self.0 & rhs.0)
            }
        }

        impl ::std::ops::BitAndAssign for #ident {
            fn bitand_assign(&mut self, rhs: Self) {
                *self = *self & rhs
            }
        }

        impl ::std::ops::BitOr for #ident {
            type Output = Self;
            fn bitor(self, rhs: Self) -> Self::Output {
                #ident(self.0 | rhs.0)
            }
        }

        impl ::std::ops::BitOrAssign for #ident {
            fn bitor_assign(&mut self, rhs: Self) {
                *self = *self | rhs
            }
        }

        impl ::std::ops::BitXor for #ident {
            type Output = Self;
            fn bitxor(self, rhs: Self) -> Self::Output {
                #ident(self.0 ^ rhs.0)
            }
        }

        impl ::std::ops::BitXorAssign for #ident {
            fn bitxor_assign(&mut self, rhs: Self) {
                *self = *self ^ rhs
            }
        }

        impl ::std::ops::Not for #ident {
            type Output = Self;
            fn not(self) -> Self::Output {
                #ident(!self.0)
            }
        }

        impl ::std::convert::TryFrom<#repr> for #ident {
            type Error = wiggle_runtime::GuestError;
            fn try_from(value: #repr) -> Result<Self, wiggle_runtime::GuestError> {
                if #repr::from(!#ident::ALL_FLAGS) & value != 0 {
                    Err(wiggle_runtime::GuestError::InvalidFlagValue(stringify!(#ident)))
                } else {
                    Ok(#ident(value))
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
                e.0
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
                Ok(*location.as_ref()?)
            }

            fn write(&self, location: &wiggle_runtime::GuestPtrMut<#ident>) {
                let val: #repr = #repr::from(*self);
                unsafe { (location.as_raw() as *mut #repr).write(val) };
            }
        }

        impl<'a> wiggle_runtime::GuestTypeTransparent<'a> for #ident {}
    }
}
