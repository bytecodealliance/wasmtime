//! Implements the function environment (e.g. a name-to-function mapping) for interpretation.

use cranelift_codegen::ir::{FuncRef, Function};
use std::collections::HashMap;

#[derive(Default, Clone)]
pub struct FunctionStore<'a> {
    functions: HashMap<FuncRef, &'a Function>,
    function_name_to_func_ref: HashMap<String, FuncRef>,
}

impl<'a> From<&'a Function> for FunctionStore<'a> {
    fn from(f: &'a Function) -> Self {
        let func_ref = FuncRef::from_u32(0);
        let mut function_name_to_func_ref = HashMap::new();
        function_name_to_func_ref.insert(f.name.to_string(), func_ref);
        let mut functions = HashMap::new();
        functions.insert(func_ref, f);
        Self {
            functions,
            function_name_to_func_ref,
        }
    }
}

impl<'a> FunctionStore<'a> {
    /// Add a function by name.
    pub fn add(&mut self, name: String, function: &'a Function) {
        let func_ref = FuncRef::with_number(self.function_name_to_func_ref.len() as u32)
            .expect("a valid function reference");
        self.function_name_to_func_ref.insert(name, func_ref);
        self.functions.insert(func_ref, function);
    }

    /// Retrieve a reference to a function in the environment by its name.
    pub fn index_of(&self, name: &str) -> Option<FuncRef> {
        self.function_name_to_func_ref.get(name).cloned()
    }

    /// Retrieve a function by its function reference.
    pub fn get_by_func_ref(&self, func_ref: FuncRef) -> Option<&'a Function> {
        self.functions.get(&func_ref).cloned()
    }

    /// Retrieve a function by its name.
    pub fn get_by_name(&self, name: &str) -> Option<&'a Function> {
        let func_ref = self.index_of(name)?;
        self.get_by_func_ref(func_ref)
    }
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
        assert_eq!(env.index_of("%test"), FuncRef::with_number(0));
    }
}
