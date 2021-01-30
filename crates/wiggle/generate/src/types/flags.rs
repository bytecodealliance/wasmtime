use super::{atom_token, int_repr_tokens};
use crate::names::Names;

use proc_macro2::{Literal, TokenStream};
use quote::quote;
use std::convert::TryFrom;

pub(super) fn define_flags(names: &Names, name: &witx::Id, f: &witx::FlagsDatatype) -> TokenStream {
    let rt = names.runtime_mod();
    let ident = names.type_(&name);
    let repr = int_repr_tokens(f.repr);
    let abi_repr = atom_token(match f.repr {
        witx::IntRepr::U8 | witx::IntRepr::U16 | witx::IntRepr::U32 => witx::AtomType::I32,
        witx::IntRepr::U64 => witx::AtomType::I64,
    });

    let mut names_ = vec![];
    let mut values_ = vec![];
    for (i, f) in f.flags.iter().enumerate() {
        let name = names.flag_member(&f.name);
        let value = 1u128
            .checked_shl(u32::try_from(i).expect("flag value overflow"))
            .expect("flag value overflow");
        let value_token = Literal::u128_unsuffixed(value);
        names_.push(name);
        values_.push(value_token);
    }

    quote! {
        #rt::bitflags::bitflags! {
            pub struct #ident: #repr {
                #(const #names_ = #values_;)*
            }
        }

        impl ::std::fmt::Display for #ident {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                f.write_str(stringify!(#ident))?;
                f.write_str("(")?;
                ::std::fmt::Debug::fmt(self, f)?;
                f.write_str(" (0x")?;
                ::std::fmt::LowerHex::fmt(&self.bits, f)?;
                f.write_str("))")?;
                Ok(())
            }
        }

        impl ::std::convert::TryFrom<#repr> for #ident {
            type Error = #rt::GuestError;
            fn try_from(value: #repr) -> Result<Self, #rt::GuestError> {
                if #repr::from(!#ident::all()) & value != 0 {
                    Err(#rt::GuestError::InvalidFlagValue(stringify!(#ident)))
                } else {
                    Ok(#ident { bits: value })
                }
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
                e.bits
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

            fn read(location: &#rt::GuestPtr<#ident>) -> Result<#ident, #rt::GuestError> {
                use std::convert::TryFrom;
                let reprval = #repr::read(&location.cast())?;
                let value = #ident::try_from(reprval)?;
                Ok(value)
            }

            fn write(location: &#rt::GuestPtr<'_, #ident>, val: Self) -> Result<(), #rt::GuestError> {
                let val: #repr = #repr::from(val);
                #repr::write(&location.cast(), val)
            }
        }
        unsafe impl<'a> #rt::GuestTypeTransparent<'a> for #ident {
            #[inline]
            fn validate(location: *mut #ident) -> Result<(), #rt::GuestError> {
                use std::convert::TryFrom;
                // Validate value in memory using #ident::try_from(reprval)
                let reprval = unsafe { (location as *mut #repr).read() };
                let _val = #ident::try_from(reprval)?;
                Ok(())
            }
        }

    }
}

impl super::WiggleType for witx::FlagsDatatype {
    fn impls_display(&self) -> bool {
        true
    }
}
