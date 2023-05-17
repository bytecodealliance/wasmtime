//! Helpers for demangling function names.

/// Demangles a single function name into a user-readable form.
///
/// Currently supported: Rust/C/C++ function names.
pub fn demangle_function_name(writer: &mut impl std::fmt::Write, name: &str) -> std::fmt::Result {
    if let Ok(demangled) = rustc_demangle::try_demangle(name) {
        write!(writer, "{}", demangled)
    } else if let Ok(demangled) = cpp_demangle::Symbol::new(name) {
        write!(writer, "{}", demangled)
    } else {
        write!(writer, "{}", name)
    }
}

/// Demangles a function name if it's provided, or returns a unified representation based on the
/// function index otherwise.
pub fn demangle_function_name_or_index(
    writer: &mut impl std::fmt::Write,
    name: Option<&str>,
    func_id: usize,
) -> std::fmt::Result {
    match name {
        Some(name) => demangle_function_name(writer, name),
        None => write!(writer, "<wasm function {}>", func_id),
    }
}
