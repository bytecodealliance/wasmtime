use quote::quote;
use syn::DeriveInput;
use syn::Result;

pub fn derive_into_dyn_ast_ref(input: &DeriveInput) -> Result<impl quote::ToTokens> {
    let ty = &input.ident;

    let opts = crate::PeepmaticOpts::from_attrs(&mut input.attrs.clone())?;
    if opts.no_into_dyn_node {
        return Ok(quote! {});
    }

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    Ok(quote! {
        impl #impl_generics From<&'a #ty #ty_generics> for DynAstRef<'a, TOperator> #where_clause {
            #[inline]
            fn from(x: &'a #ty #ty_generics) -> Self {
                Self::#ty(x)
            }
        }
    })
}
