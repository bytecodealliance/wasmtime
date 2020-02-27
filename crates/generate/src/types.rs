use crate::lifetimes::{anon_lifetime, LifetimeExt};
use crate::names::Names;

use proc_macro2::{Literal, TokenStream};
use quote::quote;
use std::convert::TryFrom;
use witx::Layout;

pub fn define_datatype(names: &Names, namedtype: &witx::NamedType) -> TokenStream {
    match &namedtype.tref {
        witx::TypeRef::Name(alias_to) => define_alias(names, &namedtype.name, &alias_to),
        witx::TypeRef::Value(v) => match &**v {
            witx::Type::Enum(e) => define_enum(names, &namedtype.name, &e),
            witx::Type::Int(i) => define_int(names, &namedtype.name, &i),
            witx::Type::Flags(f) => define_flags(names, &namedtype.name, &f),
            witx::Type::Struct(s) => {
                if !s.needs_lifetime() {
                    define_copy_struct(names, &namedtype.name, &s)
                } else {
                    define_ptr_struct(names, &namedtype.name, &s)
                }
            }
            witx::Type::Union(u) => define_union(names, &namedtype.name, &u),
            witx::Type::Handle(h) => define_handle(names, &namedtype.name, &h),
            witx::Type::Builtin(b) => define_builtin(names, &namedtype.name, *b),
            witx::Type::Pointer(p) => define_witx_pointer(
                names,
                &namedtype.name,
                quote!(wiggle_runtime::GuestPtrMut),
                p,
            ),
            witx::Type::ConstPointer(p) => {
                define_witx_pointer(names, &namedtype.name, quote!(wiggle_runtime::GuestPtr), p)
            }
            witx::Type::Array(arr) => define_witx_array(names, &namedtype.name, &arr),
        },
    }
}

fn define_alias(names: &Names, name: &witx::Id, to: &witx::NamedType) -> TokenStream {
    let ident = names.type_(name);
    let rhs = names.type_(&to.name);
    if to.tref.needs_lifetime() {
        quote!(pub type #ident<'a> = #rhs<'a>;)
    } else {
        quote!(pub type #ident = #rhs;)
    }
}

fn define_int(names: &Names, name: &witx::Id, i: &witx::IntDatatype) -> TokenStream {
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
            type Error = wiggle_runtime::GuestError;
            fn try_from(value: #repr) -> Result<Self, wiggle_runtime::GuestError> {
                Ok(#ident(value))
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

fn define_flags(names: &Names, name: &witx::Id, f: &witx::FlagsDatatype) -> TokenStream {
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

fn define_enum(names: &Names, name: &witx::Id, e: &witx::EnumDatatype) -> TokenStream {
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

fn define_handle(names: &Names, name: &witx::Id, h: &witx::HandleDatatype) -> TokenStream {
    let ident = names.type_(name);
    let size = h.mem_size_align().size as u32;
    let align = h.mem_size_align().align as u32;
    quote! {
        #[derive(Copy, Clone, Debug, ::std::hash::Hash, Eq, PartialEq)]
        pub struct #ident(u32);

        impl From<#ident> for u32 {
            fn from(e: #ident) -> u32 {
                e.0
            }
        }

        impl From<#ident> for i32 {
            fn from(e: #ident) -> i32 {
                e.0 as i32
            }
        }

        impl From<u32> for #ident {
            fn from(e: u32) -> #ident {
                #ident(e)
            }
        }
        impl From<i32> for #ident {
            fn from(e: i32) -> #ident {
                #ident(e as u32)
            }
        }

        impl ::std::fmt::Display for #ident {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                write!(f, "{}({})", stringify!(#ident), self.0)
            }
        }

        impl<'a> wiggle_runtime::GuestType<'a> for #ident {
            fn size() -> u32 {
                #size
            }

            fn align() -> u32 {
                #align
            }

            fn name() -> String {
                stringify!(#ident).to_owned()
            }

            fn validate(ptr: &wiggle_runtime::GuestPtr<#ident>) -> Result<(), wiggle_runtime::GuestError> {
                Ok(())
            }

            fn read(location: &wiggle_runtime::GuestPtr<'a, #ident>) -> Result<#ident, wiggle_runtime::GuestError> {
                let r = location.as_ref()?;
                Ok(*r)
            }

            fn write(&self, location: &wiggle_runtime::GuestPtrMut<'a, Self>) {
                unsafe { (location.as_raw() as *mut #ident).write(*self) };
            }
        }

        impl<'a> wiggle_runtime::GuestTypeTransparent<'a> for #ident {}
    }
}

fn define_builtin(names: &Names, name: &witx::Id, builtin: witx::BuiltinType) -> TokenStream {
    let ident = names.type_(name);
    let built = names.builtin_type(builtin, quote!('a));
    if builtin.needs_lifetime() {
        quote!(pub type #ident<'a> = #built;)
    } else {
        quote!(pub type #ident = #built;)
    }
}

fn define_copy_struct(names: &Names, name: &witx::Id, s: &witx::StructDatatype) -> TokenStream {
    let ident = names.type_(name);
    let size = s.mem_size_align().size as u32;
    let align = s.mem_size_align().align as u32;

    let member_decls = s.members.iter().map(|m| {
        let name = names.struct_member(&m.name);
        let type_ = names.type_ref(&m.tref, anon_lifetime());
        quote!(pub #name: #type_)
    });
    let member_valids = s.member_layout().into_iter().map(|ml| {
        let type_ = names.type_ref(&ml.member.tref, anon_lifetime());
        let offset = ml.offset as u32;
        let fieldname = names.struct_member(&ml.member.name);
        quote! {
            #type_::validate(
                &ptr.cast(#offset).map_err(|e|
                    wiggle_runtime::GuestError::InDataField{
                        typename: stringify!(#ident).to_owned(),
                        field: stringify!(#fieldname).to_owned(),
                        err: Box::new(e),
                    })?
                ).map_err(|e|
                    wiggle_runtime::GuestError::InDataField {
                        typename: stringify!(#ident).to_owned(),
                        field: stringify!(#fieldname).to_owned(),
                        err: Box::new(e),
                    })?;
        }
    });

    quote! {
        #[repr(C)]
        #[derive(Copy, Clone, Debug, ::std::hash::Hash, Eq, PartialEq)]
        pub struct #ident {
            #(#member_decls),*
        }

        impl<'a> wiggle_runtime::GuestType<'a> for #ident {
            fn size() -> u32 {
                #size
            }

            fn align() -> u32 {
                #align
            }

            fn name() -> String {
                stringify!(#ident).to_owned()
            }

            fn validate(ptr: &wiggle_runtime::GuestPtr<#ident>) -> Result<(), wiggle_runtime::GuestError> {
                #(#member_valids)*
                Ok(())
            }

            fn read(location: &wiggle_runtime::GuestPtr<'a, #ident>) -> Result<#ident, wiggle_runtime::GuestError> {
                let r = location.as_ref()?;
                Ok(*r)
            }

            fn write(&self, location: &wiggle_runtime::GuestPtrMut<'a, Self>) {
                unsafe { (location.as_raw() as *mut #ident).write(*self) };
            }
        }

        impl<'a> wiggle_runtime::GuestTypeTransparent<'a> for #ident {}
    }
}

fn define_ptr_struct(names: &Names, name: &witx::Id, s: &witx::StructDatatype) -> TokenStream {
    let ident = names.type_(name);
    let size = s.mem_size_align().size as u32;
    let align = s.mem_size_align().align as u32;

    let member_names = s.members.iter().map(|m| names.struct_member(&m.name));
    let member_decls = s.members.iter().map(|m| {
        let name = names.struct_member(&m.name);
        let type_ = match &m.tref {
            witx::TypeRef::Name(nt) => names.type_(&nt.name),
            witx::TypeRef::Value(ty) => match &**ty {
                witx::Type::Builtin(builtin) => names.builtin_type(*builtin, quote!('a)),
                witx::Type::Pointer(pointee) => {
                    let pointee_type = names.type_ref(&pointee, quote!('a));
                    quote!(wiggle_runtime::GuestPtrMut<'a, #pointee_type>)
                }
                witx::Type::ConstPointer(pointee) => {
                    let pointee_type = names.type_ref(&pointee, quote!('a));
                    quote!(wiggle_runtime::GuestPtr<'a, #pointee_type>)
                }
                _ => unimplemented!("other anonymous struct members"),
            },
        };
        quote!(pub #name: #type_)
    });
    let member_valids = s.member_layout().into_iter().map(|ml| {
        let type_ = match &ml.member.tref {
            witx::TypeRef::Name(nt) => names.type_(&nt.name),
            witx::TypeRef::Value(ty) => match &**ty {
                witx::Type::Builtin(builtin) => names.builtin_type(*builtin, quote!('a)),
                witx::Type::Pointer(pointee) => {
                    let pointee_type = names.type_ref(&pointee, anon_lifetime());
                    quote!(wiggle_runtime::GuestPtrMut::<#pointee_type>)
                }
                witx::Type::ConstPointer(pointee) => {
                    let pointee_type = names.type_ref(&pointee, anon_lifetime());
                    quote!(wiggle_runtime::GuestPtr::<#pointee_type>)
                }
                _ => unimplemented!("other anonymous struct members"),
            },
        };
        let offset = ml.offset as u32;
        let fieldname = names.struct_member(&ml.member.name);
        quote! {
            #type_::validate(
                &ptr.cast(#offset).map_err(|e|
                    wiggle_runtime::GuestError::InDataField{
                        typename: stringify!(#ident).to_owned(),
                        field: stringify!(#fieldname).to_owned(),
                        err: Box::new(e),
                    })?
                ).map_err(|e|
                    wiggle_runtime::GuestError::InDataField {
                        typename: stringify!(#ident).to_owned(),
                        field: stringify!(#fieldname).to_owned(),
                        err: Box::new(e),
                    })?;
        }
    });

    let member_reads = s.member_layout().into_iter().map(|ml| {
        let name = names.struct_member(&ml.member.name);
        let offset = ml.offset as u32;
        match &ml.member.tref {
            witx::TypeRef::Name(nt) => {
                let type_ = names.type_(&nt.name);
                quote! {
                    let #name = <#type_ as wiggle_runtime::GuestType>::read(&location.cast(#offset)?)?;
                }
            }
            witx::TypeRef::Value(ty) => match &**ty {
                witx::Type::Builtin(builtin) => {
                    let type_ = names.builtin_type(*builtin, anon_lifetime());
                    quote! {
                    let #name = <#type_ as wiggle_runtime::GuestType>::read(&location.cast(#offset)?)?;
                    }
                }
                witx::Type::Pointer(pointee) => {
                    let pointee_type = names.type_ref(&pointee, anon_lifetime());
                    quote! {
                        let #name = <wiggle_runtime::GuestPtrMut::<#pointee_type> as wiggle_runtime::GuestType>::read(&location.cast(#offset)?)?;
                    }
                }
                witx::Type::ConstPointer(pointee) => {
                    let pointee_type = names.type_ref(&pointee, anon_lifetime());
                    quote! {
                        let #name = <wiggle_runtime::GuestPtr::<#pointee_type> as wiggle_runtime::GuestType>::read(&location.cast(#offset)?)?;
                    }
                }
                _ => unimplemented!("other anonymous struct members"),
            },
        }
    });

    let member_writes = s.member_layout().into_iter().map(|ml| {
        let name = names.struct_member(&ml.member.name);
        let offset = ml.offset as u32;
        quote! {
            self.#name.write(&location.cast(#offset).expect("cast to inner member"));
        }
    });

    quote! {
        #[derive(Clone)]
        pub struct #ident<'a> {
            #(#member_decls),*
        }

        impl<'a> wiggle_runtime::GuestType<'a> for #ident<'a> {
            fn size() -> u32 {
                #size
            }

            fn align() -> u32 {
                #align
            }

            fn name() -> String {
                stringify!(#ident).to_owned()
            }

            fn validate(ptr: &wiggle_runtime::GuestPtr<'a, #ident<'a>>) -> Result<(), wiggle_runtime::GuestError> {
                #(#member_valids)*
                Ok(())
            }

            fn read(location: &wiggle_runtime::GuestPtr<'a, #ident<'a>>) -> Result<#ident<'a>, wiggle_runtime::GuestError> {
                #(#member_reads)*
                Ok(#ident { #(#member_names),* })
            }

            fn write(&self, location: &wiggle_runtime::GuestPtrMut<'a, Self>) {
                #(#member_writes)*
            }
        }
    }
}

fn union_validate(
    names: &Names,
    typename: TokenStream,
    u: &witx::UnionDatatype,
    ulayout: &witx::UnionLayout,
) -> TokenStream {
    let tagname = names.type_(&u.tag.name);
    let contents_offset = ulayout.contents_offset as u32;

    let with_err = |f: &str| -> TokenStream {
        quote!(|e| wiggle_runtime::GuestError::InDataField {
            typename: stringify!(#typename).to_owned(),
            field: #f.to_owned(),
            err: Box::new(e),
        })
    };

    let tag_err = with_err("<tag>");
    let variant_validation = u.variants.iter().map(|v| {
        let err = with_err(v.name.as_str());
        let variantname = names.enum_variant(&v.name);
        if let Some(tref) = &v.tref {
            let lifetime = anon_lifetime();
            let varianttype = names.type_ref(tref, lifetime.clone());
            quote! {
                #tagname::#variantname => {
                    let variant_ptr = ptr.cast::<#varianttype>(#contents_offset).map_err(#err)?;
                    <#varianttype as wiggle_runtime::GuestType>::validate(&variant_ptr).map_err(#err)?;
                }
            }
        } else {
            quote! { #tagname::#variantname => {} }
        }
    });

    quote! {
        let tag = *ptr.cast::<#tagname>(0).map_err(#tag_err)?.as_ref().map_err(#tag_err)?;
        match tag {
            #(#variant_validation)*
        }
        Ok(())
    }
}

fn define_union(names: &Names, name: &witx::Id, u: &witx::UnionDatatype) -> TokenStream {
    let ident = names.type_(name);
    let size = u.mem_size_align().size as u32;
    let align = u.mem_size_align().align as u32;
    let ulayout = u.union_layout();
    let contents_offset = ulayout.contents_offset as u32;

    let lifetime = quote!('a);

    let variants = u.variants.iter().map(|v| {
        let var_name = names.enum_variant(&v.name);
        if let Some(tref) = &v.tref {
            let var_type = names.type_ref(&tref, lifetime.clone());
            quote!(#var_name(#var_type))
        } else {
            quote!(#var_name)
        }
    });

    let tagname = names.type_(&u.tag.name);

    let read_variant = u.variants.iter().map(|v| {
        let variantname = names.enum_variant(&v.name);
        if let Some(tref) = &v.tref {
            let varianttype = names.type_ref(tref, lifetime.clone());
            quote! {
                #tagname::#variantname => {
                    let variant_ptr = location.cast::<#varianttype>(#contents_offset).expect("union variant ptr validated");
                    let variant_val = <#varianttype as wiggle_runtime::GuestType>::read(&variant_ptr)?;
                    Ok(#ident::#variantname(variant_val))
                }
            }
        } else {
            quote! { #tagname::#variantname => Ok(#ident::#variantname), }
        }
    });

    let write_variant = u.variants.iter().map(|v| {
        let variantname = names.enum_variant(&v.name);
        let write_tag = quote! {
            let tag_ptr = location.cast::<#tagname>(0).expect("union tag ptr TODO error report");
            let mut tag_ref = tag_ptr.as_ref_mut().expect("union tag ref TODO error report");
            *tag_ref = #tagname::#variantname;
        };
        if let Some(tref) = &v.tref {
            let varianttype = names.type_ref(tref, lifetime.clone());
            quote! {
                #ident::#variantname(contents) => {
                    #write_tag
                    let variant_ptr = location.cast::<#varianttype>(#contents_offset).expect("union variant ptr validated");
                    <#varianttype as wiggle_runtime::GuestType>::write(&contents, &variant_ptr);
                }
            }
        } else {
            quote! {
                #ident::#variantname => {
                    #write_tag
                }
            }
        }
    });
    let validate = union_validate(names, ident.clone(), u, &ulayout);

    if !u.needs_lifetime() {
        // Type does not have a lifetime parameter:
        quote! {
            #[derive(Clone, Debug, PartialEq)]
            pub enum #ident {
                #(#variants),*
            }

            impl<'a> wiggle_runtime::GuestType<'a> for #ident {
                fn size() -> u32 {
                    #size
                }

                fn align() -> u32 {
                    #align
                }

                fn name() -> String {
                    stringify!(#ident).to_owned()
                }

                fn validate(ptr: &wiggle_runtime::GuestPtr<'a, #ident>) -> Result<(), wiggle_runtime::GuestError> {
                    #validate
                }

                fn read(location: &wiggle_runtime::GuestPtr<'a, #ident>)
                        -> Result<Self, wiggle_runtime::GuestError> {
                    <#ident as wiggle_runtime::GuestType>::validate(location)?;
                    let tag = *location.cast::<#tagname>(0).expect("validated tag ptr").as_ref().expect("validated tag ref");
                    match tag {
                        #(#read_variant)*
                    }

                }

                fn write(&self, location: &wiggle_runtime::GuestPtrMut<'a, #ident>) {
                    match self {
                        #(#write_variant)*
                    }
                }
            }
        }
    } else {
        quote! {
            #[derive(Clone)]
            pub enum #ident<#lifetime> {
                #(#variants),*
            }

            impl<#lifetime> wiggle_runtime::GuestType<#lifetime> for #ident<#lifetime> {
                fn size() -> u32 {
                    #size
                }

                fn align() -> u32 {
                    #align
                }

                fn name() -> String {
                    stringify!(#ident).to_owned()
                }

                fn validate(ptr: &wiggle_runtime::GuestPtr<#lifetime, #ident<#lifetime>>) -> Result<(), wiggle_runtime::GuestError> {
                    #validate
                }

                fn read(location: &wiggle_runtime::GuestPtr<#lifetime, #ident<#lifetime>>)
                        -> Result<Self, wiggle_runtime::GuestError> {
                    <#ident as wiggle_runtime::GuestType>::validate(location)?;
                    let tag = *location.cast::<#tagname>(0).expect("validated tag ptr").as_ref().expect("validated tag ref");
                    match tag {
                        #(#read_variant)*
                    }

                }

                fn write(&self, location: &wiggle_runtime::GuestPtrMut<#lifetime, #ident<#lifetime>>) {
                    match self {
                        #(#write_variant)*
                    }
                }
            }
        }
    }
}

fn define_witx_pointer(
    names: &Names,
    name: &witx::Id,
    pointer_type: TokenStream,
    pointee: &witx::TypeRef,
) -> TokenStream {
    let ident = names.type_(name);
    let pointee_type = names.type_ref(pointee, quote!('a));

    quote!(pub type #ident<'a> = #pointer_type<'a, #pointee_type>;)
}

fn define_witx_array(names: &Names, name: &witx::Id, arr_raw: &witx::TypeRef) -> TokenStream {
    let ident = names.type_(name);
    let pointee_type = names.type_ref(arr_raw, quote!('a));
    quote!(pub type #ident<'a> = wiggle_runtime::GuestArray<'a, #pointee_type>;)
}

fn int_repr_tokens(int_repr: witx::IntRepr) -> TokenStream {
    match int_repr {
        witx::IntRepr::U8 => quote!(u8),
        witx::IntRepr::U16 => quote!(u16),
        witx::IntRepr::U32 => quote!(u32),
        witx::IntRepr::U64 => quote!(u64),
    }
}
fn atom_token(atom: witx::AtomType) -> TokenStream {
    match atom {
        witx::AtomType::I32 => quote!(i32),
        witx::AtomType::I64 => quote!(i64),
        witx::AtomType::F32 => quote!(f32),
        witx::AtomType::F64 => quote!(f64),
    }
}
