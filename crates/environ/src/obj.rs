//! Utilities for working with object files that operate as Wasmtime's
//! serialization and intermediate format for compiled modules.

use crate::{DefinedFuncIndex, EntityRef, FuncIndex, Module, SignatureIndex};
use object::{File, Object, Symbol, SymbolIndex};

const FUNCTION_PREFIX: &str = "_wasm_function_";
const TRAMPOLINE_PREFIX: &str = "_trampoline_";

/// Returns the symbol name in an object file for the corresponding wasm
/// function index in a module.
pub fn func_symbol_name(index: FuncIndex) -> String {
    format!("{}{}", FUNCTION_PREFIX, index.index())
}

/// Attempts to extract the corresponding function index from a symbol possibly produced by
/// `func_symbol_name`.
pub fn try_parse_func_name(name: &str) -> Option<FuncIndex> {
    let n = name.strip_prefix(FUNCTION_PREFIX)?.parse().ok()?;
    Some(FuncIndex::new(n))
}

/// Returns the symbol name in an object file for the corresponding trampoline
/// for the given signature in a module.
pub fn trampoline_symbol_name(index: SignatureIndex) -> String {
    format!("{}{}", TRAMPOLINE_PREFIX, index.index())
}

/// Attempts to extract the corresponding signature index from a symbol
/// possibly produced by `trampoline_symbol_name`.
pub fn try_parse_trampoline_name(name: &str) -> Option<SignatureIndex> {
    let n = name.strip_prefix(TRAMPOLINE_PREFIX)?.parse().ok()?;
    Some(SignatureIndex::new(n))
}

/// Returns an iterator of the wasm function index and corresponding symbol for
/// the function given a module/compilation artifacts.
///
/// This function only works with Wasmtime's compilation artifacts and isn't
/// supposed to be general purpose otherwise. The shape of the `obj` passed in
/// is implicitly assumed and runtime checks will fail if the `obj` is
/// malformed.
pub fn defined_functions<'a, 'data>(
    module: &'a Module,
    obj: &'a File<'data>,
) -> impl Iterator<Item = (DefinedFuncIndex, Symbol<'data, 'a>)> + 'a {
    // The goal here is to determine the `SymbolIndex` for a `DefinedFuncIndex`
    // without actually synthesizing the name of the string or doing searching
    // in the symbol table otherwise.
    //
    // Given how we construct object files we are guaranteed that all local
    // symbols are defined first, and of those local symbols the first symbols
    // are the defined functions of the module, in sorted order of index. This
    // means that there's a pretty easy mapping from `DefinedFuncIndex` to
    // `SymbolIndex`.
    //
    // Note, though, that we skip the first entry, the 0th symbol, since the
    // `object` crate reserves that for a null symbol of some sort.
    let num_defined = module.functions.len() - module.num_imported_funcs;
    (0..num_defined).map(move |i| {
        let symbol_index = SymbolIndex(i + 1);
        let func_index = DefinedFuncIndex::new(i);
        (func_index, obj.symbol_by_index(symbol_index).unwrap())
    })
}

/// Returns an iterator with the trampolines compiled into an object file.
///
/// Trampolines are yielded as the signature index that the trampoline is for in
/// addition to the symbol that the trampoline corresponds to.
///
/// Like with `defined_functions` this only works with wasmtime-produced object
/// files.
pub fn trampolines<'a, 'data>(
    module: &'a Module,
    obj: &'a File<'data>,
) -> impl Iterator<Item = (SignatureIndex, Symbol<'data, 'a>)> + 'a {
    // Like `defined_functions` above the goal is to go from a signature index
    // to a `Symbol` without any actual parsing or anything like that. This is a
    // bit trickier since `SignatureIndex` isn't a compact space like
    // `DefinedFuncIndex`, and additionally not all signatures have trampolines
    // in a module.
    //
    // This function works by using the `exported_signatures` list in a
    // `Module`, which lists, in sorted order, what trampolines are required for
    // a module. These trampolines are stored in an object file after all the
    // defined functions, which means we can skip the defined functions, the
    // null symbol, and then we're looking at the trampolines.
    let num_defined_funcs = module.functions.len() - module.num_imported_funcs;
    module
        .exported_signatures
        .iter()
        .enumerate()
        .map(move |(i, sig)| {
            let symbol_index = SymbolIndex(i + num_defined_funcs + 1);
            (*sig, obj.symbol_by_index(symbol_index).unwrap())
        })
}
