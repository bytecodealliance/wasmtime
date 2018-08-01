extern crate cranelift_codegen;
extern crate cranelift_module;
extern crate cranelift_simplejit;

use cranelift_codegen::ir::*;
use cranelift_codegen::settings::*;
use cranelift_module::*;
use cranelift_simplejit::*;

#[test]
fn error_on_incompatible_sig_in_declare_function() {
    let mut module: Module<SimpleJITBackend> = Module::new(SimpleJITBuilder::new());
    let mut sig = Signature {
        params: vec![AbiParam::new(types::I64)],
        returns: vec![],
        call_conv: CallConv::SystemV,
        argument_bytes: None,
    };
    module
        .declare_function("abc", Linkage::Local, &sig)
        .unwrap();
    sig.params[0] = AbiParam::new(types::I32);
    module
        .declare_function("abc", Linkage::Local, &sig)
        .err()
        .unwrap(); // Make sure this is an error
}
