use anyhow::Result;
use std::str::FromStr;
use target_lexicon::Triple;
use wasmtime_cranelift::builder;
use wasmtime_environ::{Compiler, Tunables};

pub fn build_arm32_compiler() -> Result<(Box<dyn Compiler>, Tunables)> {
    let triple = Triple::from_str("armv7-unknown-linux-gnueabihf")
        .map_err(|e| anyhow::anyhow!("failed to parse target triple: {}", e))?;
    let tunables = Tunables::default_for_target(&triple)
        .map_err(|e| anyhow::anyhow!("failed to create default tunables for target: {}", e))?;
    let mut b = builder(Some(triple))
        .map_err(|e| anyhow::anyhow!("failed to create compiler builder: {}", e))?;
    b.set_tunables(tunables.clone())
        .map_err(|e| anyhow::anyhow!("failed to set tunables: {}", e))?;
    let compiler = b
        .build()
        .map_err(|e| anyhow::anyhow!("failed to build compiler: {}", e))?;
    Ok((compiler, tunables))
}

/// Compile a wasm module's first function and extract the machine code.
/// Returns: (bytes, alignment, module_translation).
#[cfg(test)]
mod tests {
    use super::*;
    use cranelift_codegen::ir::{AbiParam, Function, InstBuilder, Signature, UserFuncName, types};
    use cranelift_codegen::isa::{CallConv, lookup};
    use cranelift_codegen::{Context, settings};
    use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
    use cranelift_module::{Linkage, Module, default_libcall_names};
    use cranelift_object::{ObjectBuilder, ObjectModule};

    #[test]
    fn test_object_emission() {
        let triple = Triple::from_str("armv7-unknown-linux-gnueabihf").unwrap();
        let Some(isa) = lookup(triple.clone()).ok() else {
            return;
        };
        let Ok(isa) = isa.finish(settings::Flags::new(settings::builder())) else {
            return;
        };
        let mut module = ObjectModule::new(
            ObjectBuilder::new(isa.clone(), "test", default_libcall_names()).unwrap(),
        );

        let mut sig_a = Signature::new(CallConv::triple_default(&triple));
        sig_a.params.push(AbiParam::new(types::I32));
        sig_a.returns.push(AbiParam::new(types::I32));

        let func_a_id = module
            .declare_function("func_a", Linkage::Local, &sig_a)
            .unwrap();

        let mut ctx_a = Context::new();
        ctx_a.func =
            Function::with_name_signature(UserFuncName::user(0, func_a_id.as_u32()), sig_a.clone());
        let mut func_ctx = FunctionBuilderContext::new();
        {
            let mut bcx = FunctionBuilder::new(&mut ctx_a.func, &mut func_ctx);
            let block = bcx.create_block();
            bcx.switch_to_block(block);
            bcx.append_block_params_for_function_params(block);
            let param = bcx.block_params(block)[0];
            let result = bcx.ins().iadd_imm_s(param, 1);
            bcx.ins().return_(&[result]);
            bcx.seal_all_blocks();
            bcx.finalize(isa.frontend_config());
        }
        module.define_function(func_a_id, &mut ctx_a).unwrap();

        let mut sig_b = Signature::new(CallConv::triple_default(&triple));
        sig_b.params.push(AbiParam::new(types::I32));
        sig_b.returns.push(AbiParam::new(types::I32));

        let func_b_id = module
            .declare_function("func_b", Linkage::Local, &sig_b)
            .unwrap();

        let mut ctx_b = Context::new();
        ctx_b.func =
            Function::with_name_signature(UserFuncName::user(0, func_b_id.as_u32()), sig_b.clone());
        {
            let mut bcx = FunctionBuilder::new(&mut ctx_b.func, &mut func_ctx);
            let block = bcx.create_block();
            bcx.switch_to_block(block);
            bcx.append_block_params_for_function_params(block);
            let param = bcx.block_params(block)[0];
            let local_func = module.declare_func_in_func(func_a_id, bcx.func);
            let result = bcx.ins().call(local_func, &[param]);
            let return_val = bcx.inst_results(result)[0];
            bcx.ins().return_(&[return_val]);
            bcx.seal_all_blocks();
            bcx.finalize(isa.frontend_config());
        }
        module.define_function(func_b_id, &mut ctx_b).unwrap();

        let _ = module.finish().emit().unwrap();
    }
}
