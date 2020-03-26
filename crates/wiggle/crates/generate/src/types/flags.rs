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
        #[repr(transparent)]
        #[derive(Copy, Clone, Debug, ::std::hash::Hash, Eq, PartialEq)]
        pub struct #ident(#repr);

        impl #ident {
            #(pub const #names_: #ident = #ident(#values_);)*

            #[inline]
            pub const fn empty() -> Self {
                #ident(0)
            }

            #[inline]
            pub const fn all() -> Self {
                #ident(#(#values_)|*)
            }

            #[inline]
            pub fn contains(&self, other: &#ident) -> bool {
                !*self & *other == Self::empty()
            }
        }

        impl ::std::fmt::Display for #ident {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                let mut first = true;
                #(
                    if self.0 & #values_ == #values_ {
                        if !first {
                            f.write_str("|")?;
                        }
                        first = false;
                        f.write_fmt(format_args!("{}", stringify!(#names_).to_lowercase()))?;
                    }
                )*
                if first {
                    f.write_str("empty")?;
                }
                f.write_fmt(format_args!(" ({:#x})", self.0))?;
                Ok(())
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
            type Error = wiggle::GuestError;
            fn try_from(value: #repr) -> Result<Self, wiggle::GuestError> {
                if #repr::from(!#ident::all()) & value != 0 {
                    Err(wiggle::GuestError::InvalidFlagValue(stringify!(#ident)))
                } else {
                    Ok(#ident(value))
                }
            }
        }

        impl ::std::convert::TryFrom<#abi_repr> for #ident {
            type Error = wiggle::GuestError;
            fn try_from(value: #abi_repr) -> Result<#ident, wiggle::GuestError> {
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

        impl<'a> wiggle::GuestType<'a> for #ident {
            fn guest_size() -> u32 {
                #repr::guest_size()
            }

            fn guest_align() -> usize {
                #repr::guest_align()
            }

            fn read(location: &wiggle::GuestPtr<#ident>) -> Result<#ident, wiggle::GuestError> {
                use std::convert::TryFrom;
                let reprval = #repr::read(&location.cast())?;
                let value = #ident::try_from(reprval)?;
                Ok(value)
            }

            fn write(location: &wiggle::GuestPtr<'_, #ident>, val: Self) -> Result<(), wiggle::GuestError> {
                let val: #repr = #repr::from(val);
                #repr::write(&location.cast(), val)
            }
        }
        unsafe impl <'a> wiggle::GuestTypeTransparent<'a> for #ident {
            #[inline]
            fn validate(location: *mut #ident) -> Result<(), wiggle::GuestError> {
                use std::convert::TryFrom;
                // Validate value in memory using #ident::try_from(reprval)
                let reprval = unsafe { (location as *mut #repr).read() };
                let _val = #ident::try_from(reprval)?;
                Ok(())
            }
        }

    }
}
