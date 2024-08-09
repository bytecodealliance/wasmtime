//! Disassembly support for pulley bytecode.

use crate::decode::*;
use crate::imms::*;
use crate::regs::*;
use alloc::string::String;
use core::fmt::Write;

/// A Pulley bytecode disassembler.
///
/// This is implemented as an `OpVisitor`, where you pass a `Disassembler` to a
/// `Decoder` in order to disassemble instructions from a bytecode stream.
///
/// Alternatively, you can use the `Disassembler::disassemble_all` method to
/// disassemble a complete bytecode stream.
pub struct Disassembler<'a> {
    raw_bytecode: &'a [u8],
    bytecode: SafeBytecodeStream<'a>,
    disas: String,
    start: usize,
    temp: String,
}

impl<'a> Disassembler<'a> {
    /// Disassemble every instruction in the given bytecode stream.
    pub fn disassemble_all(bytecode: &'a [u8]) -> Result<String> {
        let mut disas = Self::new(bytecode);
        Decoder::decode_all(&mut disas)?;
        Ok(disas.disas)
    }

    /// Create a new `Disassembler` that can be used to incrementally
    /// disassemble instructions from the given bytecode stream.
    pub fn new(bytecode: &'a [u8]) -> Self {
        Self {
            raw_bytecode: bytecode,
            bytecode: SafeBytecodeStream::new(bytecode),
            disas: String::new(),
            start: 0,
            temp: String::new(),
        }
    }

    /// Get the disassembly thus far.
    pub fn disas(&self) -> &str {
        &self.disas
    }
}

/// Anything inside an instruction that can be disassembled: registers,
/// immediates, etc...
trait Disas {
    fn disas(&self, position: usize, disas: &mut String);
}

impl Disas for XReg {
    fn disas(&self, _position: usize, disas: &mut String) {
        write!(disas, "{self}").unwrap();
    }
}

impl Disas for FReg {
    fn disas(&self, _position: usize, disas: &mut String) {
        write!(disas, "{self}").unwrap();
    }
}

impl Disas for VReg {
    fn disas(&self, _position: usize, disas: &mut String) {
        write!(disas, "{self}").unwrap();
    }
}

impl Disas for i8 {
    fn disas(&self, _position: usize, disas: &mut String) {
        write!(disas, "{self}").unwrap();
    }
}

impl Disas for i16 {
    fn disas(&self, _position: usize, disas: &mut String) {
        write!(disas, "{self}").unwrap();
    }
}

impl Disas for i32 {
    fn disas(&self, _position: usize, disas: &mut String) {
        write!(disas, "{self}").unwrap();
    }
}

impl Disas for i64 {
    fn disas(&self, _position: usize, disas: &mut String) {
        write!(disas, "{self}").unwrap();
    }
}

impl Disas for u8 {
    fn disas(&self, _position: usize, disas: &mut String) {
        write!(disas, "{self}").unwrap();
    }
}

impl Disas for u16 {
    fn disas(&self, _position: usize, disas: &mut String) {
        write!(disas, "{self}").unwrap();
    }
}

impl Disas for u32 {
    fn disas(&self, _position: usize, disas: &mut String) {
        write!(disas, "{self}").unwrap();
    }
}

impl Disas for u64 {
    fn disas(&self, _position: usize, disas: &mut String) {
        write!(disas, "{self}").unwrap();
    }
}

impl Disas for PcRelOffset {
    fn disas(&self, position: usize, disas: &mut String) {
        let offset = isize::try_from(i32::from(*self)).unwrap();
        let target = position.wrapping_add(offset as usize);
        write!(disas, "{offset:#x}    // target = {target:#x}").unwrap()
    }
}

fn disas_list<T: Disas>(position: usize, disas: &mut String, iter: impl IntoIterator<Item = T>) {
    let mut iter = iter.into_iter();
    let Some(first) = iter.next() else { return };
    first.disas(position, disas);

    for item in iter {
        write!(disas, ", ").unwrap();
        item.disas(position, disas);
    }
}

impl<R: Reg + Disas> Disas for BinaryOperands<R> {
    fn disas(&self, position: usize, disas: &mut String) {
        disas_list(position, disas, [self.dst, self.src1, self.src2])
    }
}

impl<R: Reg + Disas> Disas for RegSet<R> {
    fn disas(&self, position: usize, disas: &mut String) {
        disas_list(position, disas, *self)
    }
}

macro_rules! impl_disas {
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
        impl<'a> OpVisitor for Disassembler<'a> {
            type BytecodeStream = SafeBytecodeStream<'a>;

            fn bytecode(&mut self) -> &mut Self::BytecodeStream {
                &mut self.bytecode
            }

            type Return = ();

            fn before_visit(&mut self) {
                self.start = self.bytecode.position();
            }

            fn after_visit(&mut self) {
                let size = self.bytecode.position() - self.start;

                write!(&mut self.disas, "{:8x}: ", self.start).unwrap();
                let mut need_space = false;
                for byte in &self.raw_bytecode[self.start..][..size] {
                    write!(&mut self.disas, "{}{byte:02x}", if need_space { " " } else { "" }).unwrap();
                    need_space = true;
                }
                for _ in 0..11_usize.saturating_sub(size) {
                    write!(&mut self.disas, "   ").unwrap();
                }

                self.disas.push_str(&self.temp);
                self.temp.clear();

                self.disas.push('\n');
            }

            $(
                fn $snake_name(&mut self $( $( , $field : $field_ty )* )? ) {
                    let mnemonic = stringify!($snake_name);
                    write!(&mut self.temp, "{mnemonic}").unwrap();
                    $(
                        let mut need_comma = false;
                        $(
                            let val = $field;
                            if need_comma {
                                write!(&mut self.temp, ",").unwrap();
                            }
                            write!(&mut self.temp, " ").unwrap();
                            val.disas(self.start, &mut self.temp);
                            #[allow(unused_assignments)]
                            { need_comma = true; }
                        )*
                    )?
                }
            )*
        }
    };
}
for_each_op!(impl_disas);

macro_rules! impl_extended_disas {
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
        impl ExtendedOpVisitor for Disassembler<'_> {
            $(
                fn $snake_name(&mut self $( $( , $field : $field_ty )* )? ) {
                    let mnemonic = stringify!($snake_name);
                    write!(&mut self.temp, "{mnemonic}").unwrap();
                    $(
                        let mut need_comma = false;
                        $(
                            let val = $field;
                            if need_comma {
                                write!(&mut self.temp, ",").unwrap();
                            }
                            write!(&mut self.temp, " ").unwrap();
                            val.disas(self.start, &mut self.temp);
                            #[allow(unused_assignments)]
                            { need_comma = true; }
                        )*
                    )?
                }
            )*
        }
    };
}
for_each_extended_op!(impl_extended_disas);
