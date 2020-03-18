use crate::trap::Trap;
use crate::values::Val;

/// A trait representing a function that can be imported and called from inside
/// WebAssembly.
/// # Example
/// ```
/// use wasmtime::Val;
///
/// struct TimesTwo;
///
/// impl wasmtime::Callable for TimesTwo {
///     fn call(&self, params: &[Val], results: &mut [Val]) -> Result<(), wasmtime::Trap> {
///         let mut value = params[0].unwrap_i32();
///         value *= 2;
///         results[0] = value.into();
///
///         Ok(())
///     }
/// }
///
/// # fn main () -> Result<(), Box<dyn std::error::Error>> {
/// // Simple module that imports our host function ("times_two") and re-exports
/// // it as "run".
/// let wat = r#"
///    (module
///      (func $times_two (import "" "times_two") (param i32) (result i32))
///      (func
///        (export "run")
///        (param i32)
///        (result i32)
///        (local.get 0)
///        (call $times_two))
///    )
/// "#;
///
/// // Initialise environment and our module.
/// let store = wasmtime::Store::default();
/// let module = wasmtime::Module::new(&store, wat)?;
///
/// // Define the type of the function we're going to call.
/// let times_two_type = wasmtime::FuncType::new(
///     // Parameters
///     Box::new([wasmtime::ValType::I32]),
///     // Results
///     Box::new([wasmtime::ValType::I32])
/// );
///
/// // Build a reference to the "times_two" function that can be used.
/// let times_two_function =
///     wasmtime::Func::new(&store, times_two_type, std::rc::Rc::new(TimesTwo));
///
/// // Create module instance that imports our function
/// let instance = wasmtime::Instance::new(
///     &module,
///     &[times_two_function.into()]
/// )?;
///
/// // Get "run" function from the exports.
/// let run_function = instance.exports()[0].func().unwrap();
///
/// // Borrow and call "run". Returning any error message from Wasm as a string.
/// let original = 5i32;
/// let results = run_function
///     .call(&[original.into()])
///     .map_err(|trap| trap.to_string())?;
///
/// // Compare that the results returned matches what we expect.
/// assert_eq!(original * 2, results[0].unwrap_i32());
/// # Ok(())
/// # }
/// ```
pub trait Callable {
    /// What is called when the function is invoked in WebAssembly.
    /// `params` is an immutable list of parameters provided to the function.
    /// `results` is mutable list of results to be potentially set by your
    /// function. Produces a `Trap` if the function encounters any errors.
    fn call(&self, params: &[Val], results: &mut [Val]) -> Result<(), Trap>;
}
