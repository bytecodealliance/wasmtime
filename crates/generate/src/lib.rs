extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;

const WITX_PATH: &'static str = "crates/WASI/phases/snapshot/witx/wasi_snapshot_preview1.witx";

#[proc_macro]
pub fn from_witx(args: TokenStream) -> TokenStream {
    let doc = witx::load(&[WITX_PATH]).unwrap();
    TokenStream::new()
    // TokenStream::from(raw_types::gen(
    //     TokenStream2::from(args),
    //     raw_types::Mode::Host,
    // ))
}
