//! Static, named definitions of instruction opcodes.

/// Empty opcode for use as a default.
pub static EMPTY: [u8; 0] = [];

/// Add with carry flag r{16,32,64} to r/m of the same size.
pub static ADC: [u8; 1] = [0x11];

/// Add r{16,32,64} to r/m of the same size.
pub static ADD: [u8; 1] = [0x01];

/// Add imm{16,32} to r/m{16,32,64}, possibly sign-extended.
pub static ADD_IMM: [u8; 1] = [0x81];

/// Add sign-extended imm8 to r/m{16,32,64}.
pub static ADD_IMM8_SIGN_EXTEND: [u8; 1] = [0x83];

/// Add the low double-precision floating-point value from xmm2/mem to xmm1
/// and store the result in xmm1.
pub static ADDSD: [u8; 3] = [0xf2, 0x0f, 0x58];

/// Add the low single-precision floating-point value from xmm2/mem to xmm1
/// and store the result in xmm1.
pub static ADDSS: [u8; 3] = [0xf3, 0x0f, 0x58];

/// r/m{16,32,64} AND register of the same size (Intel docs have a typo).
pub static AND: [u8; 1] = [0x21];

/// imm{16,32} AND r/m{16,32,64}, possibly sign-extended.
pub static AND_IMM: [u8; 1] = [0x81];

/// r/m{16,32,64} AND sign-extended imm8.
pub static AND_IMM8_SIGN_EXTEND: [u8; 1] = [0x83];

/// Return the bitwise logical AND NOT of packed single-precision floating-point
/// values in xmm1 and xmm2/mem.
pub static ANDNPS: [u8; 2] = [0x0f, 0x55];

/// Return the bitwise logical AND of packed single-precision floating-point values
/// in xmm1 and xmm2/mem.
pub static ANDPS: [u8; 2] = [0x0f, 0x54];

/// Bit scan forward (stores index of first encountered 1 from the front).
pub static BIT_SCAN_FORWARD: [u8; 2] = [0x0f, 0xbc];

/// Bit scan reverse (stores index of first encountered 1 from the back).
pub static BIT_SCAN_REVERSE: [u8; 2] = [0x0f, 0xbd];

/// Call near, relative, displacement relative to next instruction (sign-extended).
pub static CALL_RELATIVE: [u8; 1] = [0xe8];

/// Move r/m{16,32,64} if overflow (OF=1).
pub static CMOV_OVERFLOW: [u8; 2] = [0x0f, 0x40];

/// Compare imm{16,32} with r/m{16,32,64} (sign-extended if 64).
pub static CMP_IMM: [u8; 1] = [0x81];

/// Compare imm8 with r/m{16,32,64}.
pub static CMP_IMM8: [u8; 1] = [0x83];

/// Compare r{16,32,64} with r/m of the same size.
pub static CMP_REG: [u8; 1] = [0x39];

/// Convert scalar double-precision floating-point value to scalar single-precision
/// floating-point value.
pub static CVTSD2SS: [u8; 3] = [0xf2, 0x0f, 0x5a];

/// Convert doubleword integer to scalar double-precision floating-point value.
pub static CVTSI2SD: [u8; 3] = [0xf2, 0x0f, 0x2a];

/// Convert doubleword integer to scalar single-precision floating-point value.
pub static CVTSI2SS: [u8; 3] = [0xf3, 0x0f, 0x2a];

/// Convert scalar single-precision floating-point value to scalar double-precision
/// float-point value.
pub static CVTSS2SD: [u8; 3] = [0xf3, 0x0f, 0x5a];

/// Convert with truncation scalar double-precision floating-point value to signed
/// integer.
pub static CVTTSD2SI: [u8; 3] = [0xf2, 0x0f, 0x2c];

/// Convert with truncation scalar single-precision floating-point value to integer.
pub static CVTTSS2SI: [u8; 3] = [0xf3, 0x0f, 0x2c];

/// Unsigned divide for {16,32,64}-bit.
pub static DIV: [u8; 1] = [0xf7];

/// Divide low double-precision floating-point value in xmm1 by low double-precision
/// floating-point value in xmm2/m64.
pub static DIVSD: [u8; 3] = [0xf2, 0x0f, 0x5e];

/// Divide low single-precision floating-point value in xmm1 by low single-precision
/// floating-point value in xmm2/m32.
pub static DIVSS: [u8; 3] = [0xf3, 0x0f, 0x5e];

/// Signed divide for {16,32,64}-bit.
pub static IDIV: [u8; 1] = [0xf7];

/// Signed multiply for {16,32,64}-bit, generic registers.
pub static IMUL: [u8; 2] = [0x0f, 0xaf];

/// Signed multiply for {16,32,64}-bit, storing into RDX:RAX.
pub static IMUL_RDX_RAX: [u8; 1] = [0xf7];

/// Insert scalar single-precision floating-point value.
pub static INSERTPS: [u8; 4] = [0x66, 0x0f, 0x3a, 0x21];

/// Either:
///  1. Jump near, absolute indirect, RIP = 64-bit offset from register or memory.
///  2. Jump far, absolute indirect, address given in m16:64.
pub static JUMP_ABSOLUTE: [u8; 1] = [0xff];

/// Jump near, relative, RIP = RIP + 32-bit displacement sign extended to 64 bits.
pub static JUMP_NEAR_RELATIVE: [u8; 1] = [0xe9];

/// Jump near (rel32) if overflow (OF=1).
pub static JUMP_NEAR_IF_OVERFLOW: [u8; 2] = [0x0f, 0x80];

/// Jump short, relative, RIP = RIP + 8-bit displacement sign extended to 64 bits.
pub static JUMP_SHORT: [u8; 1] = [0xeb];

/// Jump short (rel8) if equal (ZF=1).
pub static JUMP_SHORT_IF_EQUAL: [u8; 1] = [0x74];

/// Jump short (rel8) if not equal (ZF=0).
pub static JUMP_SHORT_IF_NOT_EQUAL: [u8; 1] = [0x75];

/// Jump short (rel8) if overflow (OF=1).
pub static JUMP_SHORT_IF_OVERFLOW: [u8; 1] = [0x70];

/// Store effective address for m in register r{16,32,64}.
pub static LEA: [u8; 1] = [0x8d];

/// Count the number of leading zero bits.
pub static LZCNT: [u8; 3] = [0xf3, 0x0f, 0xbd];

/// Return the maximum scalar double-precision floating-point value between
/// xmm2/m64 and xmm1.
pub static MAXSD: [u8; 3] = [0xf2, 0x0f, 0x5f];

/// Return the maximum scalar single-precision floating-point value between
/// xmm2/m32 and xmm1.
pub static MAXSS: [u8; 3] = [0xf3, 0x0f, 0x5f];

/// Return the minimum scalar double-precision floating-point value between
/// xmm2/m64 and xmm1.
pub static MINSD: [u8; 3] = [0xf2, 0x0f, 0x5d];

/// Return the minimum scalar single-precision floating-point value between
/// xmm2/m32 and xmm1.
pub static MINSS: [u8; 3] = [0xf3, 0x0f, 0x5d];

/// Move r8 to r/m8.
pub static MOV_BYTE_STORE: [u8; 1] = [0x88];

/// Move imm{16,32,64} to same-sized register.
pub static MOV_IMM: [u8; 1] = [0xb8];

/// Move imm{16,32} to r{16,32,64}, sign-extended if 64-bit target.
pub static MOV_IMM_SIGNEXTEND: [u8; 1] = [0xc7];

/// Move {r/m16, r/m32, r/m64} to same-sized register.
pub static MOV_LOAD: [u8; 1] = [0x8b];

/// Move r16 to r/m16.
pub static MOV_STORE_16: [u8; 2] = [0x66, 0x89];

/// Move {r16, r32, r64} to same-sized register or memory.
pub static MOV_STORE: [u8; 1] = [0x89];

/// Move aligned packed single-precision floating-point values from x/m to xmm (SSE).
pub static MOVAPS_LOAD: [u8; 2] = [0x0f, 0x28];

/// Move doubleword from r/m32 to xmm (SSE2). Quadword with REX prefix.
pub static MOVD_LOAD_XMM: [u8; 3] = [0x66, 0x0f, 0x6e];

/// Move doubleword from xmm to r/m32 (SSE2). Quadword with REX prefix.
pub static MOVD_STORE_XMM: [u8; 3] = [0x66, 0x0f, 0x7e];

/// Move packed single-precision floating-point values low to high (SSE).
pub static MOVLHPS: [u8; 2] = [0x0f, 0x16];

/// Move scalar double-precision floating-point value (from reg/mem to reg).
pub static MOVSD_LOAD: [u8; 3] = [0xf2, 0x0f, 0x10];

/// Move scalar double-precision floating-point value (from reg to reg/mem).
pub static MOVSD_STORE: [u8; 3] = [0xf2, 0x0f, 0x11];

/// Move scalar single-precision floating-point value (from reg to reg/mem).
pub static MOVSS_STORE: [u8; 3] = [0xf3, 0x0f, 0x11];

/// Move scalar single-precision floating-point-value (from reg/mem to reg).
pub static MOVSS_LOAD: [u8; 3] = [0xf3, 0x0f, 0x10];

/// Move byte to register with sign-extension.
pub static MOVSX_BYTE: [u8; 2] = [0x0f, 0xbe];

/// Move word to register with sign-extension.
pub static MOVSX_WORD: [u8; 2] = [0x0f, 0xbf];

/// Move doubleword to register with sign-extension.
pub static MOVSXD: [u8; 1] = [0x63];

/// Move unaligned packed single-precision floating-point from x/m to xmm (SSE).
pub static MOVUPS_LOAD: [u8; 2] = [0x0f, 0x10];

/// Move unaligned packed single-precision floating-point value from xmm to x/m (SSE).
pub static MOVUPS_STORE: [u8; 2] = [0x0f, 0x11];

/// Move byte to register with zero-extension.
pub static MOVZX_BYTE: [u8; 2] = [0x0f, 0xb6];

/// Move word to register with zero-extension.
pub static MOVZX_WORD: [u8; 2] = [0x0f, 0xb7];

/// Unsigned multiply for {16,32,64}-bit.
pub static MUL: [u8; 1] = [0xf7];

/// Multiply the low double-precision floating-point value in xmm2/m64 by the
/// low double-precision floating-point value in xmm1.
pub static MULSD: [u8; 3] = [0xf2, 0x0f, 0x59];

/// Multiply the low single-precision floating-point value in xmm2/m32 by the
/// low single-precision floating-point value in xmm1.
pub static MULSS: [u8; 3] = [0xf3, 0x0f, 0x59];

/// Reverse each bit of r/m{16,32,64}.
pub static NOT: [u8; 1] = [0xf7];

/// r{16,32,64} OR register of same size.
pub static OR: [u8; 1] = [0x09];

/// imm{16,32} OR r/m{16,32,64}, possibly sign-extended.
pub static OR_IMM: [u8; 1] = [0x81];

/// r/m{16,32,64} OR sign-extended imm8.
pub static OR_IMM8_SIGN_EXTEND: [u8; 1] = [0x83];

/// Return the bitwise logical OR of packed single-precision values in xmm and x/m (SSE).
pub static ORPS: [u8; 2] = [0x0f, 0x56];

/// Add packed byte integers from xmm2/m128 and xmm1 (SSE2).
pub static PADDB: [u8; 3] = [0x66, 0x0f, 0xfc];

/// Add packed doubleword integers from xmm2/m128 and xmm1 (SSE2).
pub static PADDD: [u8; 3] = [0x66, 0x0f, 0xfe];

/// Add packed quadword integers from xmm2/m128 and xmm1 (SSE2).
pub static PADDQ: [u8; 3] = [0x66, 0x0f, 0xd4];

/// Add packed word integers from xmm2/m128 and xmm1 (SSE2).
pub static PADDW: [u8; 3] = [0x66, 0x0f, 0xfd];

/// Compare packed data for equal (SSE2).
pub static PCMPEQB: [u8; 3] = [0x66, 0x0f, 0x74];

/// Compare packed data for equal (SSE2).
pub static PCMPEQD: [u8; 3] = [0x66, 0x0f, 0x76];

/// Compare packed data for equal (SSE4.1).
pub static PCMPEQQ: [u8; 4] = [0x66, 0x0f, 0x38, 0x29];

/// Compare packed data for equal (SSE2).
pub static PCMPEQW: [u8; 3] = [0x66, 0x0f, 0x75];

/// Extract doubleword or quadword, depending on REX.W (SSE4.1).
pub static PEXTR: [u8; 4] = [0x66, 0x0f, 0x3a, 0x16];

/// Extract byte (SSE4.1).
pub static PEXTRB: [u8; 4] = [0x66, 0x0f, 0x3a, 0x14];

/// Extract word (SSE2). There is a 4-byte SSE4.1 variant that can also move to m/16.
pub static PEXTRW_SSE2: [u8; 3] = [0x66, 0x0f, 0xc5];

/// Insert doubleword or quadword, depending on REX.W (SSE4.1).
pub static PINSR: [u8; 4] = [0x66, 0x0f, 0x3a, 0x22];

/// Insert byte (SSE4.1).
pub static PINSRB: [u8; 4] = [0x66, 0x0f, 0x3a, 0x20];

/// Insert word (SSE2).
pub static PINSRW: [u8; 3] = [0x66, 0x0f, 0xc4];

/// Pop top of stack into r{16,32,64}; increment stack pointer.
pub static POP_REG: [u8; 1] = [0x58];

/// Returns the count of number of bits set to 1.
pub static POPCNT: [u8; 3] = [0xf3, 0x0f, 0xb8];

/// Shuffle bytes in xmm1 according to contents of xmm2/m128 (SSE3).
pub static PSHUFB: [u8; 4] = [0x66, 0x0f, 0x38, 0x00];

/// Shuffle the doublewords in xmm2/m128 based on the encoding in imm8 and
/// store the result in xmm1 (SSE2).
pub static PSHUFD: [u8; 3] = [0x66, 0x0f, 0x70];

/// Push r{16,32,64}.
pub static PUSH_REG: [u8; 1] = [0x50];

/// Logical exclusive OR (SSE2).
pub static PXOR: [u8; 3] = [0x66, 0x0f, 0xef];

/// Near return to calling procedure.
pub static RET_NEAR: [u8; 1] = [0xc3];

/// General rotation opcode. Kind of rotation depends on encoding.
pub static ROTATE_CL: [u8; 1] = [0xd3];

/// General rotation opcode. Kind of rotation depends on encoding.
pub static ROTATE_IMM8: [u8; 1] = [0xc1];

/// Round scalar doubl-precision floating-point values.
pub static ROUNDSD: [u8; 4] = [0x66, 0x0f, 0x3a, 0x0b];

/// Round scalar single-precision floating-point values.
pub static ROUNDSS: [u8; 4] = [0x66, 0x0f, 0x3a, 0x0a];

/// Subtract with borrow r{16,32,64} from r/m of the same size.
pub static SBB: [u8; 1] = [0x19];

/// Set byte if overflow (OF=1).
pub static SET_BYTE_IF_OVERFLOW: [u8; 2] = [0x0f, 0x90];

/// Compute square root of scalar double-precision floating-point value.
pub static SQRTSD: [u8; 3] = [0xf2, 0x0f, 0x51];

/// Compute square root of scalar single-precision value.
pub static SQRTSS: [u8; 3] = [0xf3, 0x0f, 0x51];

/// Subtract r{16,32,64} from r/m of same size.
pub static SUB: [u8; 1] = [0x29];

/// Subtract the low double-precision floating-point value in xmm2/m64 from xmm1
/// and store the result in xmm1.
pub static SUBSD: [u8; 3] = [0xf2, 0x0f, 0x5c];

/// Subtract the low single-precision floating-point value in xmm2/m32 from xmm1
/// and store the result in xmm1.
pub static SUBSS: [u8; 3] = [0xf3, 0x0f, 0x5c];

/// AND r8 with r/m8; set SF, ZF, PF according to result.
pub static TEST_BYTE_REG: [u8; 1] = [0x84];

/// AND {r16, r32, r64} with r/m of the same size; set SF, ZF, PF according to result.
pub static TEST_REG: [u8; 1] = [0x85];

/// Count the number of trailing zero bits.
pub static TZCNT: [u8; 3] = [0xf3, 0x0f, 0xbc];

/// Compare low double-precision floating-point values in xmm1 and xmm2/mem64
/// and set the EFLAGS flags accordingly.
pub static UCOMISD: [u8; 3] = [0x66, 0x0f, 0x2e];

/// Compare low single-precision floating-point values in xmm1 and xmm2/mem32
/// and set the EFLAGS flags accordingly.
pub static UCOMISS: [u8; 2] = [0x0f, 0x2e];

/// Raise invalid opcode instruction.
pub static UNDEFINED2: [u8; 2] = [0x0f, 0x0b];

/// imm{16,32} XOR r/m{16,32,64}, possibly sign-extended.
pub static XOR_IMM: [u8; 1] = [0x81];

/// r/m{16,32,64} XOR sign-extended imm8.
pub static XOR_IMM8_SIGN_EXTEND: [u8; 1] = [0x83];

/// r/m{16,32,64} XOR register of the same size.
pub static XOR: [u8; 1] = [0x31];

/// r/m8 XOR r8.
pub static XORB: [u8; 1] = [0x30];

/// Bitwise logical XOR of packed double-precision floating-point values.
pub static XORPD: [u8; 3] = [0x66, 0x0f, 0x57];

/// Bitwise logical XOR of packed single-precision floating-point values.
pub static XORPS: [u8; 2] = [0x0f, 0x57];
