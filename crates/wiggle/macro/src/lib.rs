extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;

/// This macro expands to a set of `pub` Rust modules:
///
/// * The `types` module contains definitions for each `typename` declared in
///   the witx document. Type names are translated to the Rust-idiomatic
///   CamelCase.
///
/// * For each `module` defined in the witx document, a Rust module is defined
///   containing definitions for that module. Module names are translated to the
///   Rust-idiomatic snake\_case.
///
///     * For each `@interface func` defined in a witx module, an abi-level
///       function is generated which takes ABI-level arguments, along with a
///       "context" struct (whose type is given by the `ctx` field in the
///       macro invocation) and a `GuestMemory` implementation.
///
///     * A public "module trait" is defined (called the module name, in
///       SnakeCase) which has a `&self` method for each function in the
///       module. These methods takes idiomatic Rust types for each argument
///       and return `Result<($return_types),$error_type>`
///
/// Arguments are provided using Rust struct value syntax.
///
/// * `witx` takes a list of string literal paths. Paths are relative to the
///   CARGO_MANIFEST_DIR of the crate where the macro is invoked.
/// * `ctx` takes a type name. This type must implement all of the module
///    traits
///
/// ## Example
///
/// ```
/// use wiggle::{GuestPtr, GuestErrorType};
///
/// /// The test witx file `arrays.witx` lives in the test directory. For a
/// /// full-fledged example with runtime tests, see `tests/arrays.rs` and
/// /// the rest of the files in that directory.
/// wiggle::from_witx!({
///     witx: ["../tests/arrays.witx"],
///     ctx: YourCtxType,
/// });
///
/// /// The `ctx` type for this wiggle invocation.
/// pub struct YourCtxType {}
///
/// /// `arrays.witx` contains one module called `arrays`. So, we must
/// /// implement this one method trait for our ctx type:
/// impl arrays::Arrays for YourCtxType {
///     /// The arrays module has two methods, shown here.
///     /// Note that the `GuestPtr` type comes from `wiggle`,
///     /// whereas the witx-defined types like `Excuse` and `Errno` come
///     /// from the `pub mod types` emitted by the `wiggle::from_witx!`
///     /// invocation above.
///     fn reduce_excuses(&self, _a: &GuestPtr<[GuestPtr<types::Excuse>]>)
///         -> Result<types::Excuse, types::Errno> {
///         unimplemented!()
///     }
///     fn populate_excuses(&self, _a: &GuestPtr<[GuestPtr<types::Excuse>]>)
///         -> Result<(), types::Errno> {
///         unimplemented!()
///     }
/// }
///
/// /// For all types used in the `Error` position of a `Result` in the module
/// /// traits, you must implement `GuestErrorType` which tells wiggle-generated
/// /// code what value to return when the method returns Ok(...).
/// impl GuestErrorType for types::Errno {
///     fn success() -> Self {
///         unimplemented!()
///     }
/// }
///
/// /// The `types::GuestErrorConversion` trait is also generated with a method for
/// /// each type used in the `Error` position. This trait allows wiggle-generated
/// /// code to convert a `wiggle::GuestError` into the right error type. The trait
/// /// must be implemented for the user's `ctx` type.
///
/// impl types::GuestErrorConversion for YourCtxType {
///     fn into_errno(&self, _e: wiggle::GuestError) -> types::Errno {
///         unimplemented!()
///     }
/// }
///
/// # fn main() { println!("this fools doc tests into compiling the above outside a function body")
/// # }
/// ```
#[proc_macro]
pub fn from_witx(args: TokenStream) -> TokenStream {
    let config = parse_macro_input!(args as wiggle_generate::Config);

    let doc = config.load_document();
    let names = wiggle_generate::Names::new(&config.ctx.name, quote!(wiggle));

    let error_transform = wiggle_generate::ErrorTransform::new(&config.errors, &doc)
        .expect("validating error transform");

    let code = wiggle_generate::generate(&doc, &names, &error_transform);
    let metadata = if cfg!(feature = "wiggle_metadata") {
        wiggle_generate::generate_metadata(&doc, &names)
    } else {
        quote!()
    };

    TokenStream::from(quote! { #code #metadata })
}
