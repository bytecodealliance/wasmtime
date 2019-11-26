use syn::parse::{Parse, ParseStream};
use syn::Visibility;
use syn::{Ident, Path, Result};

pub(crate) struct TransformAttributes {
    pub module: Option<Path>,
    pub visibility: Option<Visibility>,
    pub context: Option<Path>,
}

impl Parse for TransformAttributes {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut module = None;
        let mut context = None;
        let mut visibility = None;

        while !input.is_empty() {
            let i: Ident = input.parse()?;
            match i.to_string().as_str() {
                "module" => {
                    let content;
                    parenthesized!(content in input);
                    module = Some(content.parse::<Path>()?);
                }
                "context" => {
                    let content;
                    parenthesized!(content in input);
                    context = Some(content.parse::<Path>()?);
                }
                "visibility" => {
                    let content;
                    parenthesized!(content in input);
                    visibility = Some(content.parse::<Visibility>()?);
                }
                _ => {
                    return Err(input.error(format!("unexpected attr name {}", i.to_string())));
                }
            }
            if input.is_empty() {
                break;
            }
            input.parse::<Token![,]>()?;
        }
        Ok(TransformAttributes {
            module,
            context,
            visibility,
        })
    }
}
