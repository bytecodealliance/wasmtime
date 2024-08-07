//! Encoding support for pulley bytecode.

use crate::imms::*;
use crate::opcode::{ExtendedOpcode, Opcode};
use crate::regs::*;

trait Encode {
    fn encode<E>(&self, sink: &mut E)
    where
        E: Extend<u8>;
}

impl Encode for u8 {
    fn encode<E>(&self, sink: &mut E)
    where
        E: Extend<u8>,
    {
        sink.extend(core::iter::once(*self));
    }
}

impl Encode for u16 {
    fn encode<E>(&self, sink: &mut E)
    where
        E: Extend<u8>,
    {
        sink.extend(self.to_le_bytes());
    }
}

impl Encode for u32 {
    fn encode<E>(&self, sink: &mut E)
    where
        E: Extend<u8>,
    {
        sink.extend(self.to_le_bytes());
    }
}

impl Encode for u64 {
    fn encode<E>(&self, sink: &mut E)
    where
        E: Extend<u8>,
    {
        sink.extend(self.to_le_bytes());
    }
}

impl Encode for i8 {
    fn encode<E>(&self, sink: &mut E)
    where
        E: Extend<u8>,
    {
        sink.extend(core::iter::once(*self as u8));
    }
}

impl Encode for i16 {
    fn encode<E>(&self, sink: &mut E)
    where
        E: Extend<u8>,
    {
        sink.extend(self.to_le_bytes());
    }
}

impl Encode for i32 {
    fn encode<E>(&self, sink: &mut E)
    where
        E: Extend<u8>,
    {
        sink.extend(self.to_le_bytes());
    }
}

impl Encode for i64 {
    fn encode<E>(&self, sink: &mut E)
    where
        E: Extend<u8>,
    {
        sink.extend(self.to_le_bytes());
    }
}

impl Encode for XReg {
    fn encode<E>(&self, sink: &mut E)
    where
        E: Extend<u8>,
    {
        sink.extend(core::iter::once(self.to_u8()));
    }
}

impl Encode for FReg {
    fn encode<E>(&self, sink: &mut E)
    where
        E: Extend<u8>,
    {
        sink.extend(core::iter::once(self.to_u8()));
    }
}

impl Encode for VReg {
    fn encode<E>(&self, sink: &mut E)
    where
        E: Extend<u8>,
    {
        sink.extend(core::iter::once(self.to_u8()));
    }
}

impl Encode for PcRelOffset {
    fn encode<E>(&self, sink: &mut E)
    where
        E: Extend<u8>,
    {
        i32::from(*self).encode(sink);
    }
}

impl<R: Reg> Encode for BinaryOperands<R> {
    fn encode<E>(&self, sink: &mut E)
    where
        E: Extend<u8>,
    {
        self.to_bits().encode(sink);
    }
}

macro_rules! impl_encoders {
    (
        $(
            $( #[$attr:meta] )*
                $snake_name:ident = $name:ident $( {
                $(
                    $( #[$field_attr:meta] )*
                    $field:ident : $field_ty:ty
                ),*
            } )? ;
        )*
    ) => {
        $(
            $( #[$attr] )*
            pub fn $snake_name<E>(into: &mut E $( $( , $field : impl Into<$field_ty> )* )? )
            where
                E: Extend<u8>,
            {
                into.extend(core::iter::once(Opcode::$name as u8));
                $(
                    $(
                        $field.into().encode(into);
                    )*
                )?
            }
        )*
    };
}
for_each_op!(impl_encoders);

macro_rules! impl_extended_encoders {
    (
        $(
            $( #[$attr:meta] )*
                $snake_name:ident = $name:ident $( {
                    $(
                        $( #[$field_attr:meta] )*
                        $field:ident : $field_ty:ty
                    ),*
                } )? ;
        )*
    ) => {
        $(
            $( #[$attr] )*
            pub fn $snake_name<E>(into: &mut E $( $( , $field : impl Into<$field_ty> )* )? )
            where
                E: Extend<u8>,
            {
                into.extend(core::iter::once(Opcode::ExtendedOp as u8));
                into.extend((ExtendedOpcode::$name as u16).to_le_bytes());
                $(
                    $(
                        $field.into().encode(into);
                    )*
                )?
            }
        )*
    };
}
for_each_extended_op!(impl_extended_encoders);
