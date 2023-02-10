use crate::codegen::ir::{ArgumentExtension, ArgumentPurpose};
use anyhow::Result;
use cranelift::codegen::ir::types::*;
use cranelift::codegen::ir::{AbiParam, Signature, Type};
use cranelift::codegen::isa::CallConv;

use arbitrary::Unstructured;

/// A trait for generating random Cranelift datastructures.
pub trait CraneliftArbitrary {
    fn _type(&mut self) -> Result<Type>;
    fn callconv(&mut self) -> Result<CallConv>;
    fn abi_param(&mut self) -> Result<AbiParam>;
    fn signature(&mut self, max_params: usize, max_rets: usize) -> Result<Signature>;
}

impl<'a> CraneliftArbitrary for &mut Unstructured<'a> {
    fn _type(&mut self) -> Result<Type> {
        // TODO: It would be nice if we could get these directly from cranelift
        let scalars = [
            I8, I16, I32, I64, I128, F32, F64,
            // R32, R64,
        ];
        // TODO: vector types

        let ty = self.choose(&scalars[..])?;
        Ok(*ty)
    }

    fn callconv(&mut self) -> Result<CallConv> {
        // TODO: Generate random CallConvs per target
        Ok(CallConv::SystemV)
    }

    fn abi_param(&mut self) -> Result<AbiParam> {
        let value_type = self._type()?;
        // TODO: There are more argument purposes to be explored...
        let purpose = ArgumentPurpose::Normal;
        let extension = if value_type.is_int() {
            *self.choose(&[
                ArgumentExtension::Sext,
                ArgumentExtension::Uext,
                ArgumentExtension::None,
            ])?
        } else {
            ArgumentExtension::None
        };

        Ok(AbiParam {
            value_type,
            purpose,
            extension,
        })
    }

    fn signature(&mut self, max_params: usize, max_rets: usize) -> Result<Signature> {
        let callconv = self.callconv()?;
        let mut sig = Signature::new(callconv);

        for _ in 0..max_params {
            sig.params.push(self.abi_param()?);
        }

        for _ in 0..max_rets {
            sig.returns.push(self.abi_param()?);
        }

        Ok(sig)
    }
}
