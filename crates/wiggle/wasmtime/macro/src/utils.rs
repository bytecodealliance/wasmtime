use proc_macro2::Ident;

pub fn param_name(param: &witx::InterfaceFuncParam) -> Ident {
    quote::format_ident!(
        "{}",
        match param.name.as_str() {
            "in" | "type" => format!("r#{}", param.name.as_str()),
            s => s.to_string(),
        }
    )
}
