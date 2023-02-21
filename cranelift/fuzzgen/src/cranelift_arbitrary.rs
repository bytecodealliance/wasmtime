use crate::codegen::ir::{ArgumentExtension, ArgumentPurpose};
use anyhow::Result;
use cranelift::codegen::data_value::DataValue;
use cranelift::codegen::ir::types::*;
use cranelift::codegen::ir::{AbiParam, Signature, Type};
use cranelift::codegen::isa::CallConv;

use arbitrary::Unstructured;
use cranelift::prelude::{Ieee32, Ieee64};

/// A trait for generating random Cranelift datastructures.
pub trait CraneliftArbitrary {
    fn _type(&mut self) -> Result<Type>;
    fn callconv(&mut self) -> Result<CallConv>;
    fn abi_param(&mut self) -> Result<AbiParam>;
    fn signature(&mut self, max_params: usize, max_rets: usize) -> Result<Signature>;
    fn datavalue(&mut self, ty: Type) -> Result<DataValue>;
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

    fn datavalue(&mut self, ty: Type) -> Result<DataValue> {
        Ok(match ty {
            ty if ty.is_int() => {
                let imm = match ty {
                    I8 => self.arbitrary::<i8>()? as i128,
                    I16 => self.arbitrary::<i16>()? as i128,
                    I32 => self.arbitrary::<i32>()? as i128,
                    I64 => self.arbitrary::<i64>()? as i128,
                    I128 => self.arbitrary::<i128>()?,
                    _ => unreachable!(),
                };
                DataValue::from_integer(imm, ty)?
            }
            // f{32,64}::arbitrary does not generate a bunch of important values
            // such as Signaling NaN's / NaN's with payload, so generate floats from integers.
            F32 => DataValue::F32(Ieee32::with_bits(self.arbitrary::<u32>()?)),
            F64 => DataValue::F64(Ieee64::with_bits(self.arbitrary::<u64>()?)),
            _ => unimplemented!(),
        })
    }
}
