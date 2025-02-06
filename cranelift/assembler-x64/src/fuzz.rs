//! A fuzz testing oracle for roundtrip assembly-disassembly.
//!
//! This contains manual implementations of the `Arbitrary` trait for types
//! throughout this crate to avoid depending on the `arbitrary` crate
//! unconditionally (use the `fuzz` feature instead).

use crate::{AsReg, Gpr, Inst, NonRspGpr, Registers, Simm32, Simm32PlusKnownOffset};
use arbitrary::{Arbitrary, Result, Unstructured};
use capstone::{arch::x86, arch::BuildsCapstone, arch::BuildsCapstoneSyntax, Capstone};

/// Take a random assembly instruction and check its encoding and
/// pretty-printing against a known-good disassembler.
///
/// # Panics
///
/// This function panics to express failure as expected by the `arbitrary`
/// fuzzer infrastructure. It may fail during assembly, disassembly, or when
/// comparing the disassembled strings.
pub fn roundtrip(inst: &Inst<FuzzRegs>) {
    // Check that we can actually assemble this instruction.
    let assembled = assemble(inst);
    let expected = disassemble(&assembled);

    // Check that our pretty-printed output matches the known-good output. Trim
    // off the instruction offset first.
    let expected = expected.split_once(' ').unwrap().1;
    let actual = inst.to_string();
    if expected != actual && expected != replace_signed_immediates(&actual) {
        println!("> {inst}");
        println!("  debug: {inst:x?}");
        println!("  assembled: {}", pretty_print_hexadecimal(&assembled));
        assert_eq!(expected, &actual);
    }
}

/// Use this assembler to emit machine code into a byte buffer.
///
/// This will skip any traps or label registrations, but this is fine for the
/// single-instruction disassembly we're doing here.
fn assemble(insn: &Inst<FuzzRegs>) -> Vec<u8> {
    let mut buffer = Vec::new();
    let offsets: Vec<i32> = Vec::new();
    insn.encode(&mut buffer, &offsets);
    buffer
}

/// Building a new `Capstone` each time is suboptimal (TODO).
fn disassemble(assembled: &[u8]) -> String {
    let cs = Capstone::new()
        .x86()
        .mode(x86::ArchMode::Mode64)
        .syntax(x86::ArchSyntax::Att)
        .detail(true)
        .build()
        .expect("failed to create Capstone object");
    let insns = cs
        .disasm_all(assembled, 0x0)
        .expect("failed to disassemble");
    assert_eq!(insns.len(), 1, "not a single instruction: {assembled:x?}");
    let insn = insns.first().expect("at least one instruction");
    assert_eq!(assembled.len(), insn.len());
    insn.to_string()
}

fn pretty_print_hexadecimal(hex: &[u8]) -> String {
    use std::fmt::Write;
    let mut s = String::with_capacity(hex.len() * 2);
    for b in hex {
        write!(&mut s, "{b:02X}").unwrap();
    }
    s
}

/// See `replace_signed_immediates`.
macro_rules! hex_print_signed_imm {
    ($hex:expr, $from:ty => $to:ty) => {{
        let imm = <$from>::from_str_radix($hex, 16).unwrap() as $to;
        let mut simm = String::new();
        if imm < 0 {
            simm.push_str("-");
        }
        let abs = match imm.checked_abs() {
            Some(i) => i,
            None => <$to>::MIN,
        };
        if imm > -10 && imm < 10 {
            simm.push_str(&format!("{:x}", abs));
        } else {
            simm.push_str(&format!("0x{:x}", abs));
        }
        simm
    }};
}

/// Replace signed immediates in the disassembly with their unsigned hexadecimal
/// equivalent. This is only necessary to match `capstone`'s complex
/// pretty-printing rules; e.g. `capsone` will:
/// - omit the `0x` prefix when printing `0x0` as `0`.
/// - omit the `0x` prefix when print small values (less than 10)
/// - print negative values as `-0x...` (signed hex) instead of `0xff...`
///   (normal hex)
fn replace_signed_immediates(dis: &str) -> std::borrow::Cow<str> {
    match dis.find("$") {
        None => dis.into(),
        Some(idx) => {
            let (prefix, rest) = dis.split_at(idx + 1); // Skip the '$'.
            let (_, rest) = chomp("-", rest); // Skip the '-' if it's there.
            let (_, rest) = chomp("0x", rest); // Skip the '0x' if it's there.
            let n = rest.chars().take_while(char::is_ascii_hexdigit).count();
            let (hex, rest) = rest.split_at(n); // Split at next non-hex character.
            let simm = match hex.len() {
                1 | 2 => hex_print_signed_imm!(hex, u8 => i8),
                4 => hex_print_signed_imm!(hex, u16 => i16),
                8 => hex_print_signed_imm!(hex, u32 => i32),
                16 => hex_print_signed_imm!(hex, u64 => i64),
                _ => panic!("unexpected length for hex: {hex}"),
            };
            format!("{prefix}{simm}{rest}").into()
        }
    }
}

// See `replace_signed_immediates`.
fn chomp<'a>(pat: &str, s: &'a str) -> (&'a str, &'a str) {
    if s.starts_with(pat) {
        s.split_at(pat.len())
    } else {
        ("", s)
    }
}

#[test]
fn replace() {
    assert_eq!(
        replace_signed_immediates("andl $0xffffff9a, %r11d"),
        "andl $-0x66, %r11d"
    );
    assert_eq!(
        replace_signed_immediates("xorq $0xffffffffffffffbc, 0x7f139ecc(%r9)"),
        "xorq $-0x44, 0x7f139ecc(%r9)"
    );
    assert_eq!(
        replace_signed_immediates("subl $0x3ca77a19, -0x1a030f40(%r14)"),
        "subl $0x3ca77a19, -0x1a030f40(%r14)"
    );
}

/// Fuzz-specific registers.
///
/// For the fuzzer, we do not need any fancy register types; see [`FuzzReg`].
#[derive(Arbitrary, Debug)]
pub struct FuzzRegs;

impl Registers for FuzzRegs {
    type ReadGpr = FuzzReg;
    type ReadWriteGpr = FuzzReg;
}

/// A simple `u8` register type for fuzzing only.
#[derive(Clone, Copy, Debug)]
pub struct FuzzReg(u8);

impl<'a> Arbitrary<'a> for FuzzReg {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        Ok(Self::new(u.int_in_range(0..=15)?))
    }
}

impl AsReg for FuzzReg {
    fn new(enc: u8) -> Self {
        Self(enc)
    }
    fn enc(&self) -> u8 {
        self.0
    }
}

impl Arbitrary<'_> for Simm32PlusKnownOffset {
    fn arbitrary(u: &mut Unstructured<'_>) -> Result<Self> {
        // For now, we don't generate offsets (TODO).
        Ok(Self {
            simm32: Simm32::arbitrary(u)?,
            offset: None,
        })
    }
}
impl<R: AsReg> Arbitrary<'_> for NonRspGpr<R> {
    fn arbitrary(u: &mut Unstructured<'_>) -> Result<Self> {
        use crate::reg::enc::*;
        let gpr = u.choose(&[
            RAX, RCX, RDX, RBX, RBP, RSI, RDI, R8, R9, R10, R11, R12, R13, R14, R15,
        ])?;
        Ok(Self::new(R::new(*gpr)))
    }
}
impl<'a, R: AsReg> Arbitrary<'a> for Gpr<R> {
    fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self> {
        Ok(Self(R::new(u.int_in_range(0..=15)?)))
    }
}

/// Helper trait that's used to be the same as `Registers` except with an extra
/// `for<'a> Arbitrary<'a>` bound on all of the associated types.
pub trait RegistersArbitrary:
    Registers<ReadGpr: for<'a> Arbitrary<'a>, ReadWriteGpr: for<'a> Arbitrary<'a>>
{
}

impl<R> RegistersArbitrary for R
where
    R: Registers,
    R::ReadGpr: for<'a> Arbitrary<'a>,
    R::ReadWriteGpr: for<'a> Arbitrary<'a>,
{
}

#[cfg(test)]
mod test {
    use super::*;
    use arbtest::arbtest;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn smoke() {
        let count = AtomicUsize::new(0);
        arbtest(|u| {
            let inst: Inst<FuzzRegs> = u.arbitrary()?;
            roundtrip(&inst);
            println!("#{}: {inst}", count.fetch_add(1, Ordering::SeqCst));
            Ok(())
        })
        .budget_ms(1_000);

        // This will run the `roundtrip` fuzzer for one second. To repeatably
        // test a single input, append `.seed(0x<failing seed>)`.
    }
}
