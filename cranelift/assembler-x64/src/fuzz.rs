//! A fuzz testing oracle for roundtrip assembly-disassembly.
//!
//! This contains manual implementations of the `Arbitrary` trait for types
//! throughout this crate to avoid depending on the `arbitrary` crate
//! unconditionally (use the `fuzz` feature instead).

use std::string::{String, ToString};
use std::vec::Vec;
use std::{format, println};

use crate::{
    AmodeOffset, AmodeOffsetPlusKnownOffset, AsReg, CodeSink, DeferredTarget, Feature, Features,
    Fixed, Gpr, Inst, KnownOffset, NonRspGpr, Registers, TrapCode, Xmm,
};
use arbitrary::{Arbitrary, Result, Unstructured};
use capstone::{Capstone, arch::BuildsCapstone, arch::BuildsCapstoneSyntax, arch::x86};

/// Take a random assembly instruction and check its encoding and
/// pretty-printing against a known-good disassembler.
///
/// This uses Capstone as the disassembler oracle; see [`roundtrip_with`] for
/// the oracle-agnostic core.
///
/// # Panics
///
/// This function panics to express failure as expected by the `arbitrary`
/// fuzzer infrastructure. It may fail during assembly, disassembly, or when
/// comparing the disassembled strings.
pub fn roundtrip(inst: &Inst<FuzzRegs>) {
    // The bundled capstone build does not disassemble AVX-VNNI instructions, so
    // the roundtrip oracle has no reference to compare against; skip them. Their
    // encodings are covered by dedicated filetests.
    if features_mention(inst.features(), Feature::avx_vnni) {
        return;
    }

    roundtrip_with(inst, "capstone", disassemble_capstone, capstone_matches);
}

/// Like [`roundtrip`], but uses Intel XED as the disassembler oracle instead of
/// Capstone.
///
/// XED understands newer encodings (e.g. APX) that the bundled Capstone does
/// not, so this is a useful second oracle. It is only available with the
/// `fuzz-xed` feature (which requires building XED from source).
///
/// # Panics
///
/// See [`roundtrip`].
#[cfg(feature = "fuzz-xed")]
pub fn roundtrip_xed(inst: &Inst<FuzzRegs>) {
    roundtrip_with(inst, "xed", disassemble_xed, xed_matches);
}

/// The oracle-agnostic core of [`roundtrip`]: assemble `inst`, disassemble the
/// resulting bytes with the provided `disassemble` oracle, and check that the
/// oracle's pretty-printed output matches the assembler's own `to_string`,
/// where "matches" is defined by the oracle-specific `matches` predicate
/// (`matches(expected_from_oracle, actual_from_assembler)`).
///
/// The `oracle` name is only used to label diagnostic output on failure.
fn roundtrip_with(
    inst: &Inst<FuzzRegs>,
    oracle: &str,
    disassemble: impl Fn(&[u8], &Inst<FuzzRegs>) -> String,
    matches: impl Fn(&str, &str) -> bool,
) {
    // Check that we can actually assemble this instruction.
    let assembled = assemble(inst);
    let expected = disassemble(&assembled, inst);

    // Check that our pretty-printed output matches the known-good output. Trim
    // off the instruction offset first.
    let expected = expected.split_once(' ').unwrap().1;
    let actual = inst.to_string();
    if !matches(expected, &actual) {
        println!("> {inst}");
        println!("  debug: {inst:x?}");
        println!("  assembled: {}", pretty_print_hexadecimal(&assembled));
        println!("  expected ({oracle}): {expected}");
        println!("  actual (to_string):  {actual}");
        assert_eq!(expected, &actual);
    }
}

/// Whether an instruction's feature term references `target`; used to skip
/// instructions the disassembler oracle cannot handle.
fn features_mention(features: &Features, target: Feature) -> bool {
    match features {
        Features::And(a, b) | Features::Or(a, b) => {
            features_mention(a, target) || features_mention(b, target)
        }
        Features::Feature(f) => *f == target,
    }
}

/// Comparison predicate for the Capstone oracle: exact match, or match after
/// applying Capstone-specific normalization ([`fix_up`]) to the assembler
/// output.
fn capstone_matches(expected: &str, actual: &str) -> bool {
    expected == actual || expected.trim() == fix_up(actual)
}

/// Use this assembler to emit machine code into a byte buffer.
///
/// This will skip any traps or label registrations, but this is fine for the
/// single-instruction disassembly we're doing here.
fn assemble(inst: &Inst<FuzzRegs>) -> Vec<u8> {
    let mut sink = TestCodeSink::default();
    inst.encode(&mut sink);
    sink.patch_labels_as_if_they_referred_to_end();
    sink.buf
}

#[derive(Default)]
struct TestCodeSink {
    buf: Vec<u8>,
    offsets_using_label: Vec<usize>,
}

impl TestCodeSink {
    /// References to labels, e.g. RIP-relative addressing, is stored with an
    /// adjustment that takes into account the distance from the relative offset
    /// to the end of the instruction, where the offset is relative to. That
    /// means that to indeed make the offset relative to the end of the
    /// instruction, which is what we pretend all labels are bound to, it's
    /// required that this adjustment is taken into account.
    ///
    /// This function will iterate over all labels bound to this code sink and
    /// pretend the label is found at the end of the `buf`. That means that the
    /// distance from the label to the end of `buf` minus 4, which is the width
    /// of the offset, is added to what's already present in the encoding buffer.
    ///
    /// This is effectively undoing the `bytes_at_end` adjustment that's part of
    /// `Amode::RipRelative` addressing.
    fn patch_labels_as_if_they_referred_to_end(&mut self) {
        let len = i32::try_from(self.buf.len()).unwrap();
        for offset in self.offsets_using_label.iter() {
            let range = self.buf[*offset..].first_chunk_mut::<4>().unwrap();
            let offset = i32::try_from(*offset).unwrap() + 4;
            let rel_distance = len - offset;
            *range = (i32::from_le_bytes(*range) + rel_distance).to_le_bytes();
        }
    }
}

impl CodeSink for TestCodeSink {
    fn put1(&mut self, v: u8) {
        self.buf.extend_from_slice(&[v]);
    }

    fn put2(&mut self, v: u16) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    fn put4(&mut self, v: u32) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    fn put8(&mut self, v: u64) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    fn add_trap(&mut self, _: TrapCode) {}

    fn use_target(&mut self, _: DeferredTarget) {
        let offset = self.buf.len();
        self.offsets_using_label.push(offset);
    }

    fn known_offset(&self, target: KnownOffset) -> i32 {
        panic!("unsupported known target {target:?}")
    }
}

/// Disassemble a single instruction with Capstone, returning its AT&T-syntax
/// string. This is the default [`roundtrip`] oracle.
///
/// Building a new `Capstone` each time is suboptimal (TODO).
fn disassemble_capstone(assembled: &[u8], original: &Inst<FuzzRegs>) -> String {
    let cs = Capstone::new()
        .x86()
        .mode(x86::ArchMode::Mode64)
        .syntax(x86::ArchSyntax::Att)
        .detail(true)
        .build()
        .expect("failed to create Capstone object");
    let insts = cs
        .disasm_all(assembled, 0x0)
        .expect("failed to disassemble");

    if insts.len() != 1 {
        println!("> {original}");
        println!("  debug: {original:x?}");
        println!("  assembled: {}", pretty_print_hexadecimal(&assembled));
        assert_eq!(insts.len(), 1, "not a single instruction");
    }

    let inst = insts.first().expect("at least one instruction");
    if assembled.len() != inst.len() {
        println!("> {original}");
        println!("  debug: {original:x?}");
        println!("  assembled: {}", pretty_print_hexadecimal(&assembled));
        println!(
            "  capstone-assembled: {}",
            pretty_print_hexadecimal(inst.bytes())
        );
        assert_eq!(assembled.len(), inst.len(), "extra bytes not disassembled");
    }

    inst.to_string()
}

/// Disassemble a single instruction with Intel XED, returning a string in the
/// same shape as [`disassemble_capstone`] (a leading offset token, a space,
/// then the AT&T-syntax instruction) so that [`roundtrip_with`] can compare it
/// uniformly.
#[cfg(feature = "fuzz-xed")]
fn disassemble_xed(assembled: &[u8], original: &Inst<FuzzRegs>) -> String {
    use core::ffi::c_void;
    use std::sync::Once;
    use xed_sys::*;

    // XED requires a one-time global table initialization before any decode.
    static INIT: Once = Once::new();
    // SAFETY: `xed_tables_init` is safe to call; `Once` guarantees it runs
    // exactly once even across threads.
    INIT.call_once(|| unsafe { xed_tables_init() });

    // SAFETY: all of the following are standard XED decode/format calls
    // operating on stack-allocated, properly initialized structures.
    unsafe {
        let mut xedd: xed_decoded_inst_t = core::mem::zeroed();
        xed_decoded_inst_zero(&mut xedd);
        xed_decoded_inst_set_mode(&mut xedd, XED_MACHINE_MODE_LONG_64, XED_ADDRESS_WIDTH_64b);

        let error = xed_decode(
            &mut xedd,
            assembled.as_ptr(),
            assembled.len() as core::ffi::c_uint,
        );
        if error != XED_ERROR_NONE {
            println!("> {original}");
            println!("  debug: {original:x?}");
            println!("  assembled: {}", pretty_print_hexadecimal(assembled));
            let name = core::ffi::CStr::from_ptr(xed_error_enum_t2str(error));
            panic!("xed failed to decode: {}", name.to_string_lossy());
        }

        // XED must consume exactly the bytes we emitted; a shorter length means
        // trailing bytes were not part of the instruction.
        let decoded_len = xed_decoded_inst_get_length(&xedd) as usize;
        if decoded_len != assembled.len() {
            println!("> {original}");
            println!("  debug: {original:x?}");
            println!("  assembled: {}", pretty_print_hexadecimal(assembled));
            assert_eq!(
                decoded_len,
                assembled.len(),
                "xed did not consume all bytes"
            );
        }

        // Format in AT&T syntax to match the assembler's own pretty-printing.
        let mut buf = [0i8; 256];
        let ok = xed_format_context(
            XED_SYNTAX_ATT,
            &xedd,
            buf.as_mut_ptr(),
            buf.len() as core::ffi::c_int,
            0,
            core::ptr::null_mut::<c_void>(),
            None,
        );
        assert!(ok != 0, "xed failed to format instruction");

        let disasm = core::ffi::CStr::from_ptr(buf.as_ptr())
            .to_string_lossy()
            .into_owned();

        // Prepend a fake offset token so the shape matches Capstone's
        // `0x0: <inst>` output that `roundtrip_with` expects.
        format!("0: {disasm}")
    }
}

fn pretty_print_hexadecimal(hex: &[u8]) -> String {
    use core::fmt::Write;
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
/// pretty-printing rules; e.g. `capstone` will:
/// - omit the `0x` prefix when printing `0x0` as `0`.
/// - omit the `0x` prefix when print small values (less than 10)
/// - print negative values as `-0x...` (signed hex) instead of `0xff...`
///   (normal hex)
/// - print `mov` immediates as base-10 instead of base-16 (?!).
fn replace_signed_immediates(dis: &str) -> alloc::borrow::Cow<'_, str> {
    match dis.find('$') {
        None => dis.into(),
        Some(idx) => {
            let (prefix, rest) = dis.split_at(idx + 1); // Skip the '$'.
            let (_, rest) = chomp("-", rest); // Skip the '-' if it's there.
            let (_, rest) = chomp("0x", rest); // Skip the '0x' if it's there.
            let n = rest.chars().take_while(char::is_ascii_hexdigit).count();
            let (hex, rest) = rest.split_at(n); // Split at next non-hex character.
            let simm = if dis.starts_with("mov") {
                u64::from_str_radix(hex, 16).unwrap().to_string()
            } else {
                match hex.len() {
                    1 | 2 => hex_print_signed_imm!(hex, u8 => i8),
                    4 => hex_print_signed_imm!(hex, u16 => i16),
                    8 => hex_print_signed_imm!(hex, u32 => i32),
                    16 => hex_print_signed_imm!(hex, u64 => i64),
                    _ => panic!("unexpected length for hex: {hex}"),
                }
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
    assert_eq!(
        replace_signed_immediates("movq $0xffffffff864ae103, %rsi"),
        "movq $18446744071667638531, %rsi"
    );
}

/// Remove everything after the first semicolon in the disassembly and trim any
/// trailing spaces. This is necessary to remove the implicit operands we end up
/// printing for Cranelift's sake.
fn remove_after_semicolon(dis: &str) -> &str {
    match dis.find(';') {
        None => dis,
        Some(idx) => {
            let (prefix, _) = dis.split_at(idx);
            prefix.trim()
        }
    }
}

#[test]
fn remove_after_parenthesis_test() {
    assert_eq!(
        remove_after_semicolon("imulb 0x7658eddd(%rcx) ;; implicit: %ax"),
        "imulb 0x7658eddd(%rcx)"
    );
}

/// Run some post-processing on the disassembly to make it match Capstone.
fn fix_up(dis: &str) -> alloc::borrow::Cow<'_, str> {
    let dis = remove_after_semicolon(dis);
    replace_signed_immediates(&dis)
}

/// Comparison predicate for the Intel XED oracle.
///
/// XED decodes the same instructions as the assembler but prints them with a
/// number of different conventions. The differences we reconcile here are:
///
/// - Cosmetic whitespace (XED's double space after the mnemonic, its lack of
///   spaces inside memory operands), an explicit SIB scale of 1, and the AT&T
///   indirect-branch marker `*` (as in `jmpq *%rax`).
/// - Numeric formatting: XED always prints hex while the assembler prints small
///   values in decimal. Normalized for `$` immediates, memory displacements,
///   and bare branch targets.
/// - XED omits the AT&T operand-size suffix on the mnemonic (`adc` vs `adcw`)
///   when an operand makes the width unambiguous, or conversely adds one the
///   assembler omits (`rcpssl` vs `rcpss`).
/// - XED appends a vector-length marker (`x`/`y`/`z` for 128/256/512-bit) to
///   some VEX/EVEX mnemonics (`vpalignrx` vs `vpalignr`).
/// - Legacy prefixes (`lock`, `rep*`, ...) printed as a leading token.
/// - Condition-code aliases (`cmovnb` vs `cmovae`).
/// - The AT&T/Intel spellings of the sign/zero-extend convert instructions
///   (`cltq` vs `cdqe`) and the move-with-extension instructions (`movzbl` vs
///   `movzxb`, `movslq` vs `movsxdl`).
/// - `movabs`(`q`) (assembler) vs plain `mov` (XED) for the imm64 move.
/// - Implicit operands the assembler prints but XED omits (the `%xmm0` mask of
///   the SSE4.1 variable blends, the `$1` count of shift/rotate-by-one).
/// - The SSE/AVX compare pseudo-ops, where the assembler bakes the predicate
///   into the mnemonic (`vcmpneqsd`) but XED uses a predicate immediate
///   (`vcmpsd $0x4, ...`).
///
/// Rather than blindly stripping suffixes from the assembler mnemonic--which
/// would corrupt mnemonics that legitimately end in those letters, like `mul`
/// or `call`--we use XED's mnemonic as ground truth: a suffix is only dropped
/// if doing so makes the two mnemonics exactly equal.
#[cfg(feature = "fuzz-xed")]
fn xed_matches(expected: &str, actual: &str) -> bool {
    let actual = remove_after_semicolon(actual);

    // Normalize runs of whitespace to a single space, and drop spaces that
    // follow a comma, so cosmetic spacing differences (XED's double space after
    // the mnemonic, and its lack of spaces inside memory operands like
    // `(%rsi,%rdx,2)`) don't matter. Also drop an explicit SIB scale of 1,
    // which XED prints (`(%rbp,%rsi,1)`) but the assembler omits. Finally, drop
    // the AT&T indirect-branch marker `*` (as in `jmpq *%rax`), which the
    // assembler prints but XED does not; `*` has no other use in this syntax.
    fn normalize_ws(s: &str) -> String {
        let collapsed = s.split_whitespace().collect::<Vec<_>>().join(" ");
        collapsed
            .replace(", ", ",")
            .replace(",1)", ")")
            .replace('*', "")
    }
    let expected = canonicalize_displacements(&canonicalize_immediates(&normalize_ws(expected)));
    let actual = canonicalize_displacements(&canonicalize_immediates(&normalize_ws(actual)));
    if expected == actual {
        return true;
    }

    // Split "mnemonic operands" into the leading mnemonic and the remainder.
    fn split_mnemonic(s: &str) -> (&str, &str) {
        match s.split_once(' ') {
            Some((m, rest)) => (m, rest),
            None => (s, ""),
        }
    }

    // Strip any leading legacy instruction prefixes (`lock`, `rep*`, ...) that
    // both disassemblers print as a separate leading token, so the mnemonic
    // normalization below operates on the real operation mnemonic rather than
    // the prefix. Both sides decode the same bytes, so their prefixes agree.
    fn strip_legacy_prefixes(s: &str) -> &str {
        let mut s = s;
        while let Some((head, rest)) = s.split_once(' ') {
            if matches!(
                head,
                "lock" | "rep" | "repe" | "repz" | "repne" | "repnz" | "data16" | "bnd" | "notrack"
            ) {
                s = rest;
            } else {
                break;
            }
        }
        s
    }
    let expected = strip_legacy_prefixes(&expected);
    let actual = strip_legacy_prefixes(&actual);

    let (exp_mnemonic, exp_ops) = split_mnemonic(expected);
    let (act_mnemonic, act_ops) = split_mnemonic(actual);

    // XED makes the implicit shift/rotate-by-one count explicit (`sarl $0x1, X`
    // vs the assembler's `sarl X`); drop a leading `$0x1,` for that family so
    // the operand lists line up.
    fn is_shift_rotate(m: &str) -> bool {
        const ROOTS: [&str; 8] = ["sal", "sar", "shl", "shr", "rol", "ror", "rcl", "rcr"];
        ROOTS.contains(&m)
            || m.strip_suffix(['b', 'w', 'l', 'q'])
                .is_some_and(|r| ROOTS.contains(&r))
    }
    fn strip_shift_one<'a>(m: &str, ops: &'a str) -> &'a str {
        if is_shift_rotate(m) {
            if let Some(rest) = ops.strip_prefix("$0x1,") {
                return rest;
            }
        }
        ops
    }

    // The SSE4.1 variable-blend instructions (`blendvps`, `blendvpd`,
    // `pblendvb`) take an implicit `%xmm0` mask, which the assembler prints as
    // an explicit leading operand but XED omits. This only ever appears on the
    // assembler (`actual`) side, so strip it there alone--stripping it from XED
    // too would corrupt cases where `%xmm0` is also a real explicit operand.
    fn strip_implicit_xmm0<'a>(m: &str, ops: &'a str) -> &'a str {
        if m.starts_with("blendv") || m.starts_with("pblendv") {
            if let Some(rest) = ops.strip_prefix("%xmm0,") {
                return rest;
            }
        }
        ops
    }
    let exp_ops = strip_shift_one(exp_mnemonic, exp_ops);
    let act_ops = strip_implicit_xmm0(act_mnemonic, strip_shift_one(act_mnemonic, act_ops));

    // Canonicalize any whole-operand bare number (e.g. a relative branch target
    // like `jnp 5` vs `jnp 0x5`) into the shared hex form. Operands that are
    // not pure numbers (registers, memory references) are left untouched.
    fn canonicalize_operand_numbers(ops: &str) -> String {
        ops.split(',')
            .map(|op| canonicalize_one_number(op).unwrap_or_else(|| op.to_string()))
            .collect::<Vec<_>>()
            .join(",")
    }
    let exp_ops = canonicalize_operand_numbers(exp_ops);
    let act_ops = canonicalize_operand_numbers(act_ops);

    // SSE/AVX compare pseudo-ops: the assembler bakes the comparison predicate
    // into the mnemonic (`vcmpneqsd`) while XED uses the generic mnemonic plus a
    // leading predicate immediate (`vcmpsd $0x4, ...`). Convert whichever side
    // is a pseudo-op into the generic `mnemonic + $imm` form and compare.
    let exp_cmp = split_cmp_pseudo(exp_mnemonic).map(|(b, n)| (b, prepend_predicate(n, &exp_ops)));
    let act_cmp = split_cmp_pseudo(act_mnemonic).map(|(b, n)| (b, prepend_predicate(n, &act_ops)));
    if exp_cmp.is_some() || act_cmp.is_some() {
        let (exp_base, exp_cops) =
            exp_cmp.unwrap_or_else(|| (exp_mnemonic.to_string(), exp_ops.clone()));
        let (act_base, act_cops) =
            act_cmp.unwrap_or_else(|| (act_mnemonic.to_string(), act_ops.clone()));
        if exp_cops == act_cops
            && (exp_base == act_base
                || act_base.strip_suffix(['b', 'w', 'l', 'q']).as_deref() == Some(&exp_base)
                || exp_base.strip_suffix(['b', 'w', 'l', 'q']).as_deref() == Some(&act_base)
                || exp_base.strip_suffix(['x', 'y', 'z']).as_deref() == Some(&act_base))
        {
            return true;
        }
    }

    if exp_ops != act_ops {
        return false;
    }

    if act_mnemonic == exp_mnemonic {
        return true;
    }

    // The assembler mnemonic is the XED mnemonic plus a single trailing
    // operand-size suffix (`adcw` vs `adc`).
    if act_mnemonic.strip_suffix(['b', 'w', 'l', 'q']) == Some(exp_mnemonic) {
        return true;
    }

    // ...or vice versa: XED adds an operand-size suffix that the assembler
    // omits (`rcpssl` vs `rcpss`) when a memory operand makes the width
    // otherwise unstated.
    if exp_mnemonic.strip_suffix(['b', 'w', 'l', 'q']) == Some(act_mnemonic) {
        return true;
    }

    // The XED mnemonic is the assembler mnemonic plus a trailing vector-length
    // marker (`vpalignrx` vs `vpalignr`).
    if exp_mnemonic.strip_suffix(['x', 'y', 'z']) == Some(act_mnemonic) {
        return true;
    }

    // XED and the assembler may spell the same condition code differently
    // (`cmovnb` vs `cmovae`). Canonicalize both mnemonics' condition codes and
    // compare; a match means they name the same conditional instruction.
    if let (Some(exp_canon), Some(act_canon)) = (
        canonical_condition_mnemonic(exp_mnemonic),
        canonical_condition_mnemonic(act_mnemonic),
    ) {
        if exp_canon == act_canon {
            return true;
        }
    }

    // The sign/zero-extending conversion instructions have distinct AT&T and
    // Intel mnemonics for the same opcode; the assembler prints the AT&T form
    // (`cltq`) while XED prints the Intel form (`cdqe`) even in AT&T syntax.
    if canonical_convert_mnemonic(exp_mnemonic) == canonical_convert_mnemonic(act_mnemonic) {
        return true;
    }

    // The move-with-extension instructions spell their operand sizes
    // differently: the assembler encodes both source and destination sizes in
    // the mnemonic (`movzbl` = zero-extend byte to long) while XED uses a
    // single source-size suffix (`movzxb`), or omits it entirely (`movzx`) when
    // a register source already states the width. Canonicalize both--taking the
    // source size from the mnemonic or, failing that, the source operand--and
    // compare. The operand lists are already known equal here.
    if let (Some(exp_canon), Some(act_canon)) = (
        canonical_movext_mnemonic(exp_mnemonic, &exp_ops),
        canonical_movext_mnemonic(act_mnemonic, &act_ops),
    ) {
        if exp_canon == act_canon {
            return true;
        }
    }

    // The assembler names the imm64/moffs move `movabs`(`q`); XED prints plain
    // `mov`. Strip the `abs` marker, then allow the usual operand-size suffix
    // difference (`movq` vs `mov`).
    let exp_dm = exp_mnemonic
        .strip_prefix("movabs")
        .map(|s| format!("mov{s}"));
    let act_dm = act_mnemonic
        .strip_prefix("movabs")
        .map(|s| format!("mov{s}"));
    if exp_dm.is_some() || act_dm.is_some() {
        let e = exp_dm.as_deref().unwrap_or(exp_mnemonic);
        let a = act_dm.as_deref().unwrap_or(act_mnemonic);
        if e == a
            || a.strip_suffix(['b', 'w', 'l', 'q']) == Some(e)
            || e.strip_suffix(['b', 'w', 'l', 'q']) == Some(a)
        {
            return true;
        }
    }

    false
}

/// Map an SSE/AVX compare-predicate mnemonic fragment (as baked into the AT&T
/// pseudo-op mnemonics, e.g. `neq`) to its immediate predicate value.
#[cfg(feature = "fuzz-xed")]
fn cmp_predicate(name: &str) -> Option<u8> {
    Some(match name {
        "eq" => 0,
        "lt" => 1,
        "le" => 2,
        "unord" => 3,
        "neq" => 4,
        "nlt" => 5,
        "nle" => 6,
        "ord" => 7,
        "eq_uq" => 8,
        "nge" => 9,
        "ngt" => 10,
        "false" => 11,
        "neq_oq" => 12,
        "ge" => 13,
        "gt" => 14,
        "true" => 15,
        "eq_os" => 16,
        "lt_oq" => 17,
        "le_oq" => 18,
        "unord_s" => 19,
        "neq_us" => 20,
        "nlt_uq" => 21,
        "nle_uq" => 22,
        "ord_s" => 23,
        "eq_us" => 24,
        "nge_uq" => 25,
        "ngt_uq" => 26,
        "false_os" => 27,
        "neq_os" => 28,
        "ge_oq" => 29,
        "gt_oq" => 30,
        "true_us" => 31,
        _ => return None,
    })
}

/// Recognize an SSE/AVX compare pseudo-op mnemonic (`cmpneqps`, `vcmpltsd`,
/// ...) and split it into its generic base mnemonic (`cmpps`, `vcmpsd`) and the
/// predicate immediate value. Returns `None` for any other mnemonic.
#[cfg(feature = "fuzz-xed")]
fn split_cmp_pseudo(m: &str) -> Option<(String, u8)> {
    let (prefix, rest) = match m.strip_prefix('v') {
        Some(r) => ("v", r),
        None => ("", m),
    };
    let rest = rest.strip_prefix("cmp")?;
    for ty in ["ps", "pd", "ss", "sd"] {
        if let Some(pred) = rest.strip_suffix(ty) {
            if let Some(n) = cmp_predicate(pred) {
                return Some((format!("{prefix}cmp{ty}"), n));
            }
        }
    }
    None
}

/// Prepend a predicate immediate (`$0xN`) to an operand list, matching the
/// generic-form operand ordering used by XED for the compare instructions.
#[cfg(feature = "fuzz-xed")]
fn prepend_predicate(n: u8, ops: &str) -> String {
    if ops.is_empty() {
        format!("$0x{n:x}")
    } else {
        format!("$0x{n:x},{ops}")
    }
}

/// The size letter implied by a source register operand (`%r11w` -> `w`), or
/// `None` for a memory operand (whose size cannot be read off the operand).
#[cfg(feature = "fuzz-xed")]
fn reg_source_size(op: &str) -> Option<&'static str> {
    let reg = op.strip_prefix('%')?;
    if reg.contains('(') {
        return None; // memory operand
    }
    const B: &[&str] = &[
        "al", "bl", "cl", "dl", "sil", "dil", "spl", "bpl", "ah", "bh", "ch", "dh",
    ];
    const W: &[&str] = &["ax", "bx", "cx", "dx", "si", "di", "sp", "bp"];
    const D: &[&str] = &["eax", "ebx", "ecx", "edx", "esi", "edi", "esp", "ebp"];
    if B.contains(&reg) || (reg.starts_with('r') && reg.ends_with('b')) {
        Some("b")
    } else if W.contains(&reg) || (reg.starts_with('r') && reg.ends_with('w')) {
        Some("w")
    } else if D.contains(&reg) || (reg.starts_with('r') && reg.ends_with('d')) {
        Some("d")
    } else {
        Some("q")
    }
}

/// Canonicalize a move-with-zero/sign-extension mnemonic so the assembler's
/// two-size AT&T spelling and XED's spelling compare equal.
///
/// The assembler writes `mov{z,s}<src><dst>` (e.g. `movzbl`, `movswq`). XED
/// writes `mov{z,s}x<src>` (e.g. `movzxb`, `movsxd`) or, when a register source
/// already states the width, just `mov{z,s}x`. Both agree on the destination
/// via the register operand, so we reduce each to `mov{z,s}x<src>` with the
/// source size taken from the mnemonic when present, else inferred from the
/// source operand `ops`; the 32-bit source is normalized (`l` and `d` both mean
/// doubleword). Returns `None` for any mnemonic that is not one of these.
#[cfg(feature = "fuzz-xed")]
fn canonical_movext_mnemonic(m: &str, ops: &str) -> Option<String> {
    fn norm_src(s: &str) -> &str {
        match s {
            "l" | "d" => "d",
            other => other,
        }
    }
    let source_op = ops.split(',').next().unwrap_or(ops);
    for kind in ['z', 's'] {
        let prefix = format!("mov{kind}");
        let Some(rest) = m.strip_prefix(&prefix) else {
            continue;
        };
        // XED form: `x`, optionally followed by a single source-size letter
        // (`movzxb`, or bare `movzx`). `movsxd` may carry a redundant trailing
        // operand-size suffix (`movsxdl` = movsxd + l).
        if let Some(src) = rest.strip_prefix('x') {
            let src = match src.strip_suffix(['b', 'w', 'l', 'q']) {
                Some(s) if !s.is_empty() => s,
                _ => src,
            };
            return match src.len() {
                0 => reg_source_size(source_op).map(|s| format!("mov{kind}x{}", norm_src(s))),
                1 => Some(format!("mov{kind}x{}", norm_src(src))),
                _ => None,
            };
        }
        // Assembler form: exactly a source-size then destination-size letter
        // (`movzbl`). Anything else (e.g. `movsd`, `movsldup`) is not a
        // move-with-extension mnemonic.
        if rest.len() == 2 {
            return Some(format!("mov{kind}x{}", norm_src(&rest[0..1])));
        }
        return None;
    }
    None
}

/// Canonicalize the sign/zero-extending "convert" instructions, whose AT&T and
/// Intel mnemonics differ for the same opcode (e.g. AT&T `cltq` vs Intel
/// `cdqe`). Returns the input unchanged if it is not one of these mnemonics.
#[cfg(feature = "fuzz-xed")]
fn canonical_convert_mnemonic(m: &str) -> &str {
    match m {
        "cbtw" | "cbw" => "cbw",
        "cwtl" | "cwde" => "cwde",
        "cltq" | "cdqe" => "cdqe",
        "cwtd" | "cwd" => "cwd",
        "cltd" | "cdq" => "cdq",
        "cqto" | "cqo" => "cqo",
        other => other,
    }
}

/// Canonicalize a conditional-instruction mnemonic so that different spellings
/// of the same condition code compare equal.
///
/// x86 condition codes have multiple mnemonic aliases that denote the identical
/// flag test, e.g. `ae` (above-or-equal), `nb` (not-below), and `nc`
/// (not-carry) are the same condition. The assembler and XED may pick different
/// aliases, so for the conditional families (`cmov`, `set`, and the `j`
/// conditional jumps) we split off an optional trailing operand-size suffix,
/// map the condition code to a canonical representative, and return
/// `prefix + canonical-cc` (dropping the size suffix, which is already implied
/// by the operands that have been matched separately).
///
/// Returns `None` if `m` is not a recognized conditional mnemonic, so callers
/// can fall through to other comparisons.
#[cfg(feature = "fuzz-xed")]
fn canonical_condition_mnemonic(m: &str) -> Option<String> {
    // Map every condition-code alias to a canonical representative. Aliases on
    // the same line denote the same condition.
    fn canonical_cc(cc: &str) -> Option<&'static str> {
        Some(match cc {
            "e" | "z" => "e",
            "ne" | "nz" => "ne",
            "b" | "c" | "nae" => "b",
            "ae" | "nb" | "nc" => "ae",
            "be" | "na" => "be",
            "a" | "nbe" => "a",
            "l" | "nge" => "l",
            "ge" | "nl" => "ge",
            "le" | "ng" => "le",
            "g" | "nle" => "g",
            "p" | "pe" => "p",
            "np" | "po" => "np",
            "o" => "o",
            "no" => "no",
            "s" => "s",
            "ns" => "ns",
            _ => return None,
        })
    }

    // The conditional families we normalize. `j` must be tried last so that
    // longer prefixes (`cmov`) are matched first.
    for prefix in ["cmov", "set", "j"] {
        let Some(rest) = m.strip_prefix(prefix) else {
            continue;
        };

        // `rest` is the condition code, possibly followed by a single
        // operand-size suffix (`cmovbq` = `cmov` + `b` + `q`). Only treat a
        // trailing size character as a suffix when the remainder is itself a
        // valid condition code; this avoids mis-parsing codes that genuinely
        // end in a size-like letter (`nl`, `nb`).
        let cc = match rest.strip_suffix(['b', 'w', 'l', 'q']) {
            Some(stripped) if canonical_cc(stripped).is_some() => stripped,
            _ => rest,
        };

        return canonical_cc(cc).map(|c| format!("{prefix}{c}"));
    }

    None
}

/// Rewrite every `$`-prefixed immediate in a disassembly string into a single
/// canonical form so that decimal-vs-hex and signedness differences between the
/// assembler and XED don't cause spurious mismatches.
///
/// The assembler prints small immediates in decimal (`$1`) and larger ones in
/// hex (`$0xb143`), while XED always prints hex (`$0x1`). We parse each
/// immediate's numeric value (handling an optional leading `-` and `0x`) and
/// re-emit it as `$0x{:x}` of its `u64` two's-complement value.
#[cfg(feature = "fuzz-xed")]
fn canonicalize_immediates(dis: &str) -> String {
    let mut out = String::with_capacity(dis.len());
    let mut rest = dis;
    while let Some(idx) = rest.find('$') {
        out.push_str(&rest[..idx]);
        // Everything after the '$'.
        let after = &rest[idx + 1..];
        let (neg, num) = match after.strip_prefix('-') {
            Some(n) => (true, n),
            None => (false, after),
        };
        let (radix, digits) = match num.strip_prefix("0x") {
            Some(d) => (16, d),
            None => (10, num),
        };
        let n = digits.chars().take_while(|c| c.is_digit(radix)).count();
        if n == 0 {
            // Not actually an immediate we can parse; keep the '$' literally.
            out.push('$');
            rest = after;
            continue;
        }
        let (value_str, tail) = digits.split_at(n);
        let value = u64::from_str_radix(value_str, radix).unwrap_or(0);
        let value = if neg { value.wrapping_neg() } else { value };
        out.push_str(&format!("$0x{value:x}"));
        rest = tail;
    }
    out.push_str(rest);
    out
}

/// Parse a complete numeric token of the form `[-]?(0x)?<digits>` and re-emit
/// it in the canonical `0x{:x}` form used throughout XED normalization. Returns
/// `None` if `s` is not entirely a valid number.
#[cfg(feature = "fuzz-xed")]
fn canonicalize_one_number(s: &str) -> Option<String> {
    let (neg, num) = match s.strip_prefix('-') {
        Some(n) => (true, n),
        None => (false, s),
    };
    let (radix, digits) = match num.strip_prefix("0x") {
        Some(d) => (16, d),
        None => (10, num),
    };
    if digits.is_empty() || !digits.chars().all(|c| c.is_digit(radix)) {
        return None;
    }
    let value = u64::from_str_radix(digits, radix).ok()?;
    let value = if neg { value.wrapping_neg() } else { value };
    Some(format!("0x{value:x}"))
}

/// Rewrite every numeric memory displacement--the number immediately preceding
/// a `(` base/index group--into the same canonical `0x{:x}` form used for
/// immediates, so decimal-vs-hex differences in displacements don't cause
/// spurious mismatches (`-8(%rax)` vs `-0x8(%rax)`).
#[cfg(feature = "fuzz-xed")]
fn canonicalize_displacements(dis: &str) -> String {
    let mut out = String::with_capacity(dis.len());
    for ch in dis.chars() {
        if ch != '(' {
            out.push(ch);
            continue;
        }
        // Walk backwards over any trailing displacement token in `out`:
        // hex digits, an optional `0x` prefix, and an optional leading `-`.
        let b = out.as_bytes();
        let mut start = out.len();
        while start > 0 && b[start - 1].is_ascii_hexdigit() {
            start -= 1;
        }
        if start >= 2 && &out[start - 2..start] == "0x" {
            start -= 2;
        }
        if start > 0 && b[start - 1] == b'-' {
            start -= 1;
        }
        if let Some(canon) = canonicalize_one_number(&out[start..]) {
            out.truncate(start);
            out.push_str(&canon);
        }
        out.push('(');
    }
    out
}

/// Fuzz-specific registers.
///
/// For the fuzzer, we do not need any fancy register types; see [`FuzzReg`].
#[derive(Clone, Arbitrary, Debug)]
pub struct FuzzRegs;

impl Registers for FuzzRegs {
    type ReadGpr = FuzzReg;
    type ReadWriteGpr = FuzzReg;
    type WriteGpr = FuzzReg;
    type ReadXmm = FuzzReg;
    type ReadWriteXmm = FuzzReg;
    type WriteXmm = FuzzReg;
}

/// A simple `u8` register type for fuzzing only.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FuzzReg(u8);

impl<'a> Arbitrary<'a> for FuzzReg {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        Ok(Self(u.int_in_range(0..=15)?))
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

impl Arbitrary<'_> for AmodeOffset {
    fn arbitrary(u: &mut Unstructured<'_>) -> Result<Self> {
        // Custom implementation to try to generate some "interesting" offsets.
        // For example choose either an arbitrary 8-bit or 32-bit number as the
        // base, and then optionally shift that number to the left to create
        // multiples of constants. This can help stress some of the more
        // interesting encodings in EVEX instructions for example.
        let base = if u.arbitrary()? {
            i32::from(u.arbitrary::<i8>()?)
        } else {
            u.arbitrary::<i32>()?
        };
        Ok(match u.int_in_range(0..=5)? {
            0 => AmodeOffset::ZERO,
            n => AmodeOffset::new(base << (n - 1)),
        })
    }
}

impl Arbitrary<'_> for AmodeOffsetPlusKnownOffset {
    fn arbitrary(u: &mut Unstructured<'_>) -> Result<Self> {
        // For now, we don't generate offsets (TODO).
        Ok(Self {
            simm32: AmodeOffset::arbitrary(u)?,
            offset: None,
        })
    }
}

impl<R: AsReg, const E: u8> Arbitrary<'_> for Fixed<R, E> {
    fn arbitrary(_: &mut Unstructured<'_>) -> Result<Self> {
        Ok(Self::new(E))
    }
}

impl<R: AsReg> Arbitrary<'_> for NonRspGpr<R> {
    fn arbitrary(u: &mut Unstructured<'_>) -> Result<Self> {
        use crate::gpr::enc::*;
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
impl<'a, R: AsReg> Arbitrary<'a> for Xmm<R> {
    fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self> {
        Ok(Self(R::new(u.int_in_range(0..=15)?)))
    }
}

/// Helper trait that's used to be the same as `Registers` except with an extra
/// `for<'a> Arbitrary<'a>` bound on all of the associated types.
pub trait RegistersArbitrary:
    Registers<
        ReadGpr: for<'a> Arbitrary<'a>,
        ReadWriteGpr: for<'a> Arbitrary<'a>,
        WriteGpr: for<'a> Arbitrary<'a>,
        ReadXmm: for<'a> Arbitrary<'a>,
        ReadWriteXmm: for<'a> Arbitrary<'a>,
        WriteXmm: for<'a> Arbitrary<'a>,
    >
{
}

impl<R> RegistersArbitrary for R
where
    R: Registers,
    R::ReadGpr: for<'a> Arbitrary<'a>,
    R::ReadWriteGpr: for<'a> Arbitrary<'a>,
    R::WriteGpr: for<'a> Arbitrary<'a>,
    R::ReadXmm: for<'a> Arbitrary<'a>,
    R::ReadWriteXmm: for<'a> Arbitrary<'a>,
    R::WriteXmm: for<'a> Arbitrary<'a>,
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

    #[test]
    fn callq() {
        for i in -500..500 {
            println!("immediate: {i}");
            let inst = crate::inst::callq_d::new(i);
            roundtrip(&inst.into());
        }
    }

    /// Same as [`smoke`], but exercises the Intel XED oracle. Only available
    /// with the `fuzz-xed` feature.
    ///
    /// XED decodes the same bytes as the assembler but pretty-prints them with
    /// a number of different conventions; the [`xed_matches`] predicate
    /// reconciles them (operand-size suffixes, vector-length markers,
    /// whitespace, immediate/displacement/branch-target formatting, explicit
    /// SIB scales, legacy prefixes, condition-code aliases, the AT&T/Intel
    /// convert mnemonics, `movabs`, the move-with-extension mnemonics, implicit
    /// operands, and the compare pseudo-ops). Run explicitly with
    /// `cargo test --features fuzz-xed -- smoke_xed`.
    #[cfg(feature = "fuzz-xed")]
    #[test]
    fn smoke_xed() {
        let count = AtomicUsize::new(0);
        arbtest(|u| {
            let inst: Inst<FuzzRegs> = u.arbitrary()?;
            roundtrip_xed(&inst);
            println!("#{}: {inst}", count.fetch_add(1, Ordering::SeqCst));
            Ok(())
        })
        .budget_ms(1_000);
    }
}
