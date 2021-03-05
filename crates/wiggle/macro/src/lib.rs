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
///       function is generated which takes ABI-level arguments, along with
///       a ref that impls the module trait, and a `GuestMemory` implementation.
///       Users typically won't use these abi-level functions: The `wasmtime-wiggle`
///       and `lucet-wiggle` crates adapt these to work with a particular WebAssembly
///       engine.
///
///     * A public "module trait" is defined (called the module name, in
///       SnakeCase) which has a `&self` method for each function in the
///       module. These methods takes idiomatic Rust types for each argument
///       and return `Result<($return_types),$error_type>`
///
/// Arguments are provided using Rust struct value syntax.
///
/// * `witx` takes a list of string literal paths. Paths are relative to the
///   CARGO_MANIFEST_DIR of the crate where the macro is invoked. Alternatively,
///   `witx_literal` takes a string containing a complete witx document.
/// * Optional: `errors` takes a mapping of witx identifiers to types, e.g
///   `{ errno => YourErrnoType }`. This allows you to use the `UserErrorConversion`
///   trait to map these rich errors into the flat witx type, or to terminate
///   WebAssembly execution by trapping.
/// * Optional: `async_` takes a set of witx modules and functions which are
///   made Rust `async` functions in the module trait.
///
/// ## Example
///
/// ```
/// use wiggle::GuestPtr;
/// wiggle::from_witx!({
///     witx_literal: "
///         (typename $errno
///           (enum (@witx tag u32)
///             $ok
///             $invalid_arg
///             $io
///             $overflow))
///          (typename $alias_to_float f32)
///          (module $example
///            (@interface func (export \"int_float_args\")
///              (param $an_int u32)
///              (param $some_floats (list f32))
///              (result $r (expected (error $errno))))
///            (@interface func (export \"double_int_return_float\")
///              (param $an_int u32)
///              (result $r (expected $alias_to_float (error $errno)))))
///     ",
///     errors: { errno => YourRichError },
///     async_: { example::double_int_return_float },
/// });
///
/// /// Witx generates a set of traits, which the user must impl on a
/// /// type they define. We call this the ctx type. It stores any context
/// /// these functions need to execute.
/// pub struct YourCtxType {}
///
/// /// Witx provides a hook to translate "rich" (arbitrary Rust type) errors
/// /// into the flat error enums used at the WebAssembly interface. You will
/// /// need to impl the `types::UserErrorConversion` trait to provide a translation
/// /// from this rich type.
/// #[derive(Debug)]
/// pub enum YourRichError {
///     InvalidArg(String),
///     Io(std::io::Error),
///     Overflow,
///     Trap(String),
/// }
///
/// /// The above witx text contains one module called `$example`. So, we must
/// /// implement this one method trait for our ctx type.
/// #[wiggle::async_trait(?Send)]
/// /// We specified in the `async_` field that `example::double_int_return_float`
/// /// is an asynchronous method. Therefore, we use the `async_trait` proc macro
/// /// (re-exported by wiggle from the crate of the same name) to define this
/// /// trait, so that `double_int_return_float` can be an `async fn`.
/// impl example::Example for YourCtxType {
///     /// The arrays module has two methods, shown here.
///     /// Note that the `GuestPtr` type comes from `wiggle`,
///     /// whereas the witx-defined types like `Excuse` and `Errno` come
///     /// from the `pub mod types` emitted by the `wiggle::from_witx!`
///     /// invocation above.
///     fn int_float_args(&self, _int: u32, _floats: &GuestPtr<[f32]>)
///         -> Result<(), YourRichError> {
///         unimplemented!()
///     }
///     async fn double_int_return_float(&self, int: u32)
///         -> Result<f32, YourRichError> {
///         Ok(int.checked_mul(2).ok_or(YourRichError::Overflow)? as f32)
///     }
/// }
///
/// /// For all types used in the `error` an `expected` in the witx document,
/// /// you must implement `GuestErrorType` which tells wiggle-generated
/// /// code what value to return when the method returns Ok(...).
/// impl wiggle::GuestErrorType for types::Errno {
///     fn success() -> Self {
///         unimplemented!()
///     }
/// }
///
/// /// The `types::GuestErrorConversion` trait is also generated with a method for
/// /// each type used in the `error` position. This trait allows wiggle-generated
/// /// code to convert a `wiggle::GuestError` into the right error type. The trait
/// /// must be implemented for the user's ctx type.
///
/// impl types::GuestErrorConversion for YourCtxType {
///     fn into_errno(&self, _e: wiggle::GuestError) -> types::Errno {
///         unimplemented!()
///     }
/// }
///
/// /// If you specify a `error` mapping to the macro, you must implement the
/// /// `types::UserErrorConversion` for your ctx type as well. This trait gives
/// /// you an opportunity to store or log your rich error type, while returning
/// /// a basic witx enum to the WebAssembly caller. It also gives you the ability
/// /// to terminate WebAssembly execution by trapping.
///
/// impl types::UserErrorConversion for YourCtxType {
///     fn errno_from_your_rich_error(&self, e: YourRichError)
///         -> Result<types::Errno, wiggle::Trap>
///     {
///         println!("Rich error: {:?}", e);
///         match e {
///             YourRichError::InvalidArg{..} => Ok(types::Errno::InvalidArg),
///             YourRichError::Io{..} => Ok(types::Errno::Io),
///             YourRichError::Overflow => Ok(types::Errno::Overflow),
///             YourRichError::Trap(s) => Err(wiggle::Trap::String(s)),
///         }
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
    let names = wiggle_generate::Names::new(quote!(wiggle));

    let error_transform =
        wiggle_generate::CodegenSettings::new(&config.errors, &config.async_, &doc)
            .expect("validating codegen settings");

    let code = wiggle_generate::generate(&doc, &names, &error_transform);
    let metadata = if cfg!(feature = "wiggle_metadata") {
        wiggle_generate::generate_metadata(&doc, &names)
    } else {
        quote!()
    };

    TokenStream::from(quote! { #code #metadata })
}
