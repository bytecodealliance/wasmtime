extern crate proc_macro;

use proc_macro::TokenStream;
use syn::parse_macro_input;

#[proc_macro]
pub fn from_witx(args: TokenStream) -> TokenStream {
    let mut config = parse_macro_input!(args as wiggle_generate::Config);
    config.witx.make_paths_relative_to(
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR env var"),
    );
    let doc = witx::load(&config.witx.paths).expect("loading witx");
    TokenStream::from(wiggle_generate::generate(&doc, &config))
}
