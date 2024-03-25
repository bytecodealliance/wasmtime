use crate::codegen::ir::{ArgumentExtension, ArgumentPurpose};
use anyhow::Result;
use cranelift::codegen::data_value::DataValue;
use cranelift::codegen::ir::types::*;
use cranelift::codegen::ir::{AbiParam, Signature};
use cranelift::codegen::isa::CallConv;

use arbitrary::Unstructured;
use cranelift::prelude::{Ieee32, Ieee64};
use target_lexicon::Architecture;

/// A trait for generating random Cranelift datastructures.
pub trait CraneliftArbitrary {
    fn _type(&mut self, simd_enabled: bool) -> Result<Type>;
    fn callconv(&mut self, architecture: Architecture) -> Result<CallConv>;
    fn abi_param(&mut self, simd_enabled: bool) -> Result<AbiParam>;
    fn signature(
        &mut self,
        simd_enabled: bool,
        architecture: Architecture,
        max_params: usize,
        max_rets: usize,
    ) -> Result<Signature>;
    fn datavalue(&mut self, ty: Type) -> Result<DataValue>;
}

impl<'a> CraneliftArbitrary for &mut Unstructured<'a> {
    fn _type(&mut self, simd_enabled: bool) -> Result<Type> {
        // TODO: It would be nice if we could get these directly from cranelift
        let choices: &[Type] = if simd_enabled {
            &[
                I8, I16, I32, I64, I128, // Scalar Integers
                F32, F64, // Scalar Floats
                I8X16, I16X8, I32X4, I64X2, // SIMD Integers
                F32X4, F64X2, // SIMD Floats
            ]
        } else {
            &[I8, I16, I32, I64, I128, F32, F64]
        };

        Ok(*self.choose(choices)?)
    }

    fn callconv(&mut self, architecture: Architecture) -> Result<CallConv> {
        // These are implemented and should work on all backends
        let mut allowed_callconvs = vec![CallConv::Fast, CallConv::Cold, CallConv::SystemV];

        // Fastcall is supposed to work on x86 and aarch64
        if matches!(
            architecture,
            Architecture::X86_64 | Architecture::Aarch64(_)
        ) {
            allowed_callconvs.push(CallConv::WindowsFastcall);
        }

        // AArch64 has a few Apple specific calling conventions
        if matches!(architecture, Architecture::Aarch64(_)) {
            allowed_callconvs.push(CallConv::AppleAarch64);
        }

        // TODO(#6530): The `tail` calling convention is not supported on s390x
        // yet.
        if !matches!(architecture, Architecture::S390x) {
            allowed_callconvs.push(CallConv::Tail);
        }

        // The winch calling convention is supposed to work on x64.
        if matches!(architecture, Architecture::X86_64) {
            allowed_callconvs.push(CallConv::Winch);
        }

        Ok(*self.choose(&allowed_callconvs[..])?)
    }

    fn abi_param(&mut self, simd_enabled: bool) -> Result<AbiParam> {
        let value_type = self._type(simd_enabled)?;
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

    fn signature(
        &mut self,
        mut simd_enabled: bool,
        architecture: Architecture,
        max_params: usize,
        max_rets: usize,
    ) -> Result<Signature> {
        let callconv = self.callconv(architecture)?;

        // Winch doesn't support SIMD yet
        // https://github.com/bytecodealliance/wasmtime/issues/8093
        if callconv == CallConv::Winch {
            simd_enabled = false;
        }

        let mut sig = Signature::new(callconv);

        for _ in 0..max_params {
            sig.params.push(self.abi_param(simd_enabled)?);
        }

        for _ in 0..max_rets {
            sig.returns.push(self.abi_param(simd_enabled)?);
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
            ty if ty.is_vector() && ty.bits() == 128 => {
                DataValue::V128(self.arbitrary::<[u8; 16]>()?)
            }
            _ => unimplemented!(),
        })
    }
}
