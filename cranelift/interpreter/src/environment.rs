//! Implements the function environment (e.g. a name-to-function mapping) for interpretation.
use cranelift_codegen::ir::{FuncRef, Function};
use cranelift_entity::{entity_impl, PrimaryMap};
use std::collections::HashMap;

/// A function store contains all of the functions that are accessible to an interpreter.
#[derive(Default, Clone)]
pub struct FunctionStore<'a> {
    functions: PrimaryMap<FuncIndex, &'a Function>,
    function_names: HashMap<String, FuncIndex>,
}

/// An opaque reference to a [`Function`](Function) stored in the [FunctionStore].
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FuncIndex(u32);
entity_impl!(FuncIndex, "fn");

/// This is a helpful conversion for instantiating a store from a single [Function].
impl<'a> From<&'a Function> for FunctionStore<'a> {
    fn from(function: &'a Function) -> Self {
        let mut store = FunctionStore::default();
        store.add(function.name.to_string(), function);
        store
    }
}

impl<'a> FunctionStore<'a> {
    /// Add a function by name.
    pub fn add(&mut self, name: String, function: &'a Function) {
        assert!(!self.function_names.contains_key(&name));
        let index = self.functions.push(function);
        self.function_names.insert(name, index);
    }

    /// Retrieve the index of a function in the function store by its `name`.
    pub fn index_of(&self, name: &str) -> Option<FuncIndex> {
        self.function_names.get(name).cloned()
    }

    /// Retrieve a function by its index in the function store.
    pub fn get_by_index(&self, index: FuncIndex) -> Option<&'a Function> {
        self.functions.get(index).cloned()
    }

    /// Retrieve a function by its name.
    pub fn get_by_name(&self, name: &str) -> Option<&'a Function> {
        let index = self.index_of(name)?;
        self.get_by_index(index)
    }

    /// Retrieve a function from a [FuncRef] within a [Function]. TODO this should be optimized, if possible, as
    /// currently it retrieves the function name as a string and performs string matching.
    pub fn get_from_func_ref(
        &self,
        func_ref: FuncRef,
        function: &Function,
    ) -> Option<&'a Function> {
        self.get_by_name(&get_function_name(func_ref, function))
    }
}

/// Retrieve a function name from a [FuncRef] within a [Function]. TODO this should be optimized, if possible, as
/// currently it retrieves the function name as a string and performs string matching.
fn get_function_name(func_ref: FuncRef, function: &Function) -> String {
    function
        .dfg
        .ext_funcs
        .get(func_ref)
        .expect("function to exist")
        .name
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use cranelift_codegen::ir::{ExternalName, Signature};
    use cranelift_codegen::isa::CallConv;

    #[test]
    fn addition() {
        let mut env = FunctionStore::default();
        let a = "a";
        let f = Function::new();

        env.add(a.to_string(), &f);
        assert!(env.get_by_name(a).is_some());
    }

    #[test]
    fn nonexistence() {
        let env = FunctionStore::default();
        assert!(env.get_by_name("a").is_none());
    }

    #[test]
    fn from() {
        let name = ExternalName::testcase("test");
        let signature = Signature::new(CallConv::Fast);
        let func = &Function::with_name_signature(name, signature);
        let env: FunctionStore = func.into();
        assert_eq!(env.index_of("%test"), Some(FuncIndex::from_u32(0)));
    }
}
