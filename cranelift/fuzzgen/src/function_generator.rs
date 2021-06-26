use anyhow::Result;
use arbitrary::Unstructured;
use cranelift::codegen::ir::types::*;
use cranelift::codegen::ir::{AbiParam, ExternalName, Function, Opcode, Signature, Type};
use cranelift::codegen::isa::CallConv;
use cranelift::frontend::{FunctionBuilder, FunctionBuilderContext, Variable};
use cranelift::prelude::{EntityRef, InstBuilder};

fn insert_opcode_arity_0(
    _fgen: &mut FunctionGenerator,
    builder: &mut FunctionBuilder,
    opcode: Opcode,
    _args: &'static [Type],
    _rets: &'static [Type],
) -> Result<()> {
    builder.ins().NullAry(opcode, INVALID);
    Ok(())
}

fn insert_opcode_arity_2(
    fgen: &mut FunctionGenerator,
    builder: &mut FunctionBuilder,
    opcode: Opcode,
    args: &'static [Type],
    rets: &'static [Type],
) -> Result<()> {
    let arg0 = fgen.get_variable_of_type(builder, args[0])?;
    let arg0 = builder.use_var(arg0);

    let arg1 = fgen.get_variable_of_type(builder, args[1])?;
    let arg1 = builder.use_var(arg1);

    let typevar = rets[0];
    let (inst, dfg) = builder.ins().Binary(opcode, typevar, arg0, arg1);

    let val = dfg.first_result(inst);
    let var = fgen.create_var(builder, typevar)?;
    builder.def_var(var, val);

    Ok(())
}

type OpcodeInserter = fn(
    fgen: &mut FunctionGenerator,
    builder: &mut FunctionBuilder,
    Opcode,
    &'static [Type],
    &'static [Type],
) -> Result<()>;

// TODO: Do we have a way to get this information automatically?
const OPCODE_SIGNATURES: &'static [(
    Opcode,
    &'static [Type], // Args
    &'static [Type], // Rets
    OpcodeInserter,
)] = &[
    (Opcode::Nop, &[], &[], insert_opcode_arity_0),
    // Iadd
    (Opcode::Iadd, &[I8, I8], &[I8], insert_opcode_arity_2),
    (Opcode::Iadd, &[I16, I16], &[I16], insert_opcode_arity_2),
    (Opcode::Iadd, &[I32, I32], &[I32], insert_opcode_arity_2),
    (Opcode::Iadd, &[I64, I64], &[I64], insert_opcode_arity_2),
    // Isub
    (Opcode::Isub, &[I8, I8], &[I8], insert_opcode_arity_2),
    (Opcode::Isub, &[I16, I16], &[I16], insert_opcode_arity_2),
    (Opcode::Isub, &[I32, I32], &[I32], insert_opcode_arity_2),
    (Opcode::Isub, &[I64, I64], &[I64], insert_opcode_arity_2),
    // Imul
    (Opcode::Imul, &[I8, I8], &[I8], insert_opcode_arity_2),
    (Opcode::Imul, &[I16, I16], &[I16], insert_opcode_arity_2),
    (Opcode::Imul, &[I32, I32], &[I32], insert_opcode_arity_2),
    (Opcode::Imul, &[I64, I64], &[I64], insert_opcode_arity_2),
    // Udiv
    (Opcode::Udiv, &[I8, I8], &[I8], insert_opcode_arity_2),
    (Opcode::Udiv, &[I16, I16], &[I16], insert_opcode_arity_2),
    (Opcode::Udiv, &[I32, I32], &[I32], insert_opcode_arity_2),
    (Opcode::Udiv, &[I64, I64], &[I64], insert_opcode_arity_2),
    // Sdiv
    (Opcode::Sdiv, &[I8, I8], &[I8], insert_opcode_arity_2),
    (Opcode::Sdiv, &[I16, I16], &[I16], insert_opcode_arity_2),
    (Opcode::Sdiv, &[I32, I32], &[I32], insert_opcode_arity_2),
    (Opcode::Sdiv, &[I64, I64], &[I64], insert_opcode_arity_2),
];

pub struct FunctionGenerator<'r, 'data>
where
    'data: 'r,
{
    u: &'r mut Unstructured<'data>,
    vars: Vec<(Type, Variable)>,
}

impl<'r, 'data> FunctionGenerator<'r, 'data>
where
    'data: 'r,
{
    pub fn new(u: &'r mut Unstructured<'data>) -> Self {
        Self { u, vars: vec![] }
    }

    fn generate_callconv(&mut self) -> Result<CallConv> {
        // TODO: Generate random CallConvs per target
        // Ok(CallConv::Fast)
        Ok(CallConv::SystemV)
    }

    fn generate_type(&mut self) -> Result<Type> {
        // TODO: It would be nice if we could get these directly from cranelift
        let scalars = [
            // IFLAGS, FFLAGS,
            // B1, B8, B16, B32, B64, B128,
            I8, I16, I32, I64,
            // I128,
            // F32, F64,
            // R32, R64,
        ];
        // TODO: vector types

        let ty = self.u.choose(&scalars[..])?;
        Ok(*ty)
    }

    fn generate_abi_param(&mut self) -> Result<AbiParam> {
        // TODO: Generate more advanced abi params (structs/purposes/extensions/etc...)
        let ty = self.generate_type()?;
        Ok(AbiParam::new(ty))
    }

    fn generate_signature(&mut self) -> Result<Signature> {
        let callconv = self.generate_callconv()?;
        let mut sig = Signature::new(callconv);

        // TODO: Unconstrain this
        for _ in 0..self.u.int_in_range(0..=8)? {
            sig.params.push(self.generate_abi_param()?);
        }

        for _ in 0..self.u.int_in_range(0..=8)? {
            sig.returns.push(self.generate_abi_param()?);
        }

        Ok(sig)
    }

    /// Creates a new var
    fn create_var(&mut self, builder: &mut FunctionBuilder, ty: Type) -> Result<Variable> {
        let id = self.vars.len();
        let var = Variable::new(id);
        builder.declare_var(var, ty);
        self.vars.push((ty, var));
        Ok(var)
    }

    // TODO: Rename this
    fn vars_of_type(&self, ty: Type) -> Vec<Variable> {
        self.vars
            .iter()
            .filter(|(var_ty, _)| *var_ty == ty)
            .map(|(_, v)| *v)
            .collect()
    }

    /// Get a variable of type `ty`, either reusing a old var, or generating a new one with a
    /// `iconst`/`fconst`.
    fn get_variable_of_type(
        &mut self,
        builder: &mut FunctionBuilder,
        ty: Type,
    ) -> Result<Variable> {
        // TODO: global vars

        let mut opts: Vec<
            fn(
                fgen: &mut FunctionGenerator,
                builder: &mut FunctionBuilder,
                ty: Type,
            ) -> Result<Variable>,
        > = Vec::with_capacity(2);

        // Generate new
        opts.push(|fgen, builder, ty| {
            let imm64 = match ty {
                I8 => fgen.u.arbitrary::<i8>()? as i64,
                I16 => fgen.u.arbitrary::<i16>()? as i64,
                I32 => fgen.u.arbitrary::<i32>()? as i64,
                I64 => fgen.u.arbitrary::<i64>()?,
                _ => unreachable!(),
            };
            let var = fgen.create_var(builder, ty)?;
            let val = builder.ins().iconst(ty, imm64);
            builder.def_var(var, val);

            Ok(var)
        });

        // Reuse var
        if self.vars_of_type(ty).len() != 0 {
            opts.push(|fg, _, ty| {
                let opts = fg.vars_of_type(ty);
                let var = fg.u.choose(&opts[..])?;
                Ok(*var)
            });
        }

        let f = self.u.choose(&opts[..])?;
        f(self, builder, ty)
    }

    fn generate_return(&mut self, builder: &mut FunctionBuilder) -> Result<()> {
        let ret_params = builder.func.signature.returns.clone();

        let vars = ret_params
            .iter()
            .map(|p| self.get_variable_of_type(builder, p.value_type))
            .collect::<Result<Vec<_>>>()?;

        let vals = vars
            .into_iter()
            .map(|v| builder.use_var(v))
            .collect::<Vec<_>>();

        builder.ins().return_(&vals[..]);
        Ok(())
    }

    /// Inserts a random instruction into the block
    fn generate_instruction(&mut self, builder: &mut FunctionBuilder) -> Result<()> {
        let (op, args, rets, inserter) = *self.u.choose(OPCODE_SIGNATURES)?;
        inserter(self, builder, op, args, rets)
    }

    pub fn generate(mut self) -> Result<Function> {
        let sig = self.generate_signature()?;

        let mut fn_builder_ctx = FunctionBuilderContext::new();
        let mut func = Function::with_name_signature(ExternalName::user(0, 0), sig.clone());

        let mut builder = FunctionBuilder::new(&mut func, &mut fn_builder_ctx);
        let block0 = builder.create_block();

        let mut block_vars = vec![];
        for param in &sig.params {
            let var = self.create_var(&mut builder, param.value_type)?;
            block_vars.push(var);
        }
        builder.append_block_params_for_function_params(block0);
        builder.switch_to_block(block0);
        builder.seal_block(block0);

        // TODO: Cleanup
        for (i, _) in sig.params.iter().enumerate() {
            let var = block_vars[i];
            let block_param = builder.block_params(block0)[i];
            builder.def_var(var, block_param);
            let _ = builder.use_var(block_vars[i]);
        }

        // TODO: Unconstrain this
        for _ in 0..self.u.int_in_range(0..=16)? {
            self.generate_instruction(&mut builder)?;
        }

        // TODO: We should make this part of the regular instruction selection
        self.generate_return(&mut builder)?;

        builder.finalize();

        Ok(func)
    }
}
