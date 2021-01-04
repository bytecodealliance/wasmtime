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

/// Add packed double-precision floating-point values from xmm2/mem to xmm1 and store result in  
/// xmm1 (SSE2).
pub static ADDPD: [u8; 3] = [0x66, 0x0f, 0x58];

/// Add packed single-precision floating-point values from xmm2/mem to xmm1 and store result in  
/// xmm1 (SSE).
pub static ADDPS: [u8; 2] = [0x0f, 0x58];

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

/// Select packed single-precision floating-point values from xmm1 and xmm2/m128
/// from mask specified in XMM0 and store the values into xmm1 (SSE4.1).
pub static BLENDVPS: [u8; 4] = [0x66, 0x0f, 0x38, 0x14];

/// Select packed double-precision floating-point values from xmm1 and xmm2/m128
/// from mask specified in XMM0 and store the values into xmm1 (SSE4.1).
pub static BLENDVPD: [u8; 4] = [0x66, 0x0f, 0x38, 0x15];

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

/// Compare packed double-precision floating-point value in xmm2/m32 and xmm1 using bits 2:0 of
/// imm8 as comparison predicate (SSE2).
pub static CMPPD: [u8; 3] = [0x66, 0x0f, 0xc2];

/// Compare packed single-precision floating-point value in xmm2/m32 and xmm1 using bits 2:0 of
/// imm8 as comparison predicate (SSE).
pub static CMPPS: [u8; 2] = [0x0f, 0xc2];

/// Convert four packed signed doubleword integers from xmm2/mem to four packed single-precision
/// floating-point values in xmm1 (SSE2).
pub static CVTDQ2PS: [u8; 2] = [0x0f, 0x5b];

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

/// Convert four packed single-precision floating-point values from xmm2/mem to four packed signed
/// doubleword values in xmm1 using truncation (SSE2).
pub static CVTTPS2DQ: [u8; 3] = [0xf3, 0x0f, 0x5b];

/// Convert with truncation scalar double-precision floating-point value to signed
/// integer.
pub static CVTTSD2SI: [u8; 3] = [0xf2, 0x0f, 0x2c];

/// Convert with truncation scalar single-precision floating-point value to integer.
pub static CVTTSS2SI: [u8; 3] = [0xf3, 0x0f, 0x2c];

/// Unsigned divide for {16,32,64}-bit.
pub static DIV: [u8; 1] = [0xf7];

/// Divide packed double-precision floating-point values in xmm1 by packed double-precision
/// floating-point values in xmm2/mem (SSE2).
pub static DIVPD: [u8; 3] = [0x66, 0x0f, 0x5e];

/// Divide packed single-precision floating-point values in xmm1 by packed single-precision
/// floating-point values in xmm2/mem (SSE).
pub static DIVPS: [u8; 2] = [0x0f, 0x5e];

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

/// Return the maximum packed double-precision floating-point values between xmm1 and xmm2/m128
/// (SSE2).
pub static MAXPD: [u8; 3] = [0x66, 0x0f, 0x5f];

/// Return the maximum packed single-precision floating-point values between  xmm1 and xmm2/m128
/// (SSE).
pub static MAXPS: [u8; 2] = [0x0f, 0x5f];

/// Return the maximum scalar double-precision floating-point value between
/// xmm2/m64 and xmm1.
pub static MAXSD: [u8; 3] = [0xf2, 0x0f, 0x5f];

/// Return the maximum scalar single-precision floating-point value between
/// xmm2/m32 and xmm1.
pub static MAXSS: [u8; 3] = [0xf3, 0x0f, 0x5f];

/// Return the minimum packed double-precision floating-point values between xmm1 and xmm2/m128
/// (SSE2).
pub static MINPD: [u8; 3] = [0x66, 0x0f, 0x5d];

/// Return the minimum packed single-precision floating-point values between xmm1 and xmm2/m128
/// (SSE).
pub static MINPS: [u8; 2] = [0x0f, 0x5d];

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

/// Multiply packed double-precision floating-point values from xmm2/mem to xmm1 and store result
/// in xmm1 (SSE2).
pub static MULPD: [u8; 3] = [0x66, 0x0f, 0x59];

/// Multiply packed single-precision floating-point values from xmm2/mem to xmm1 and store result
/// in xmm1 (SSE).
pub static MULPS: [u8; 2] = [0x0f, 0x59];

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

/// Compute the absolute value of bytes in xmm2/m128 and store the unsigned result in xmm1 (SSSE3).
pub static PABSB: [u8; 4] = [0x66, 0x0f, 0x38, 0x1c];

/// Compute the absolute value of 32-bit integers in xmm2/m128 and store the unsigned result in
/// xmm1 (SSSE3).
pub static PABSD: [u8; 4] = [0x66, 0x0f, 0x38, 0x1e];

/// Compute the absolute value of 16-bit integers in xmm2/m128 and store the unsigned result in
/// xmm1 (SSSE3).
pub static PABSW: [u8; 4] = [0x66, 0x0f, 0x38, 0x1d];

/// Converts 8 packed signed word integers from xmm1 and from xmm2/m128 into 16 packed signed byte
/// integers in xmm1 using signed saturation (SSE2).
pub static PACKSSWB: [u8; 3] = [0x66, 0x0f, 0x63];

/// Converts 4 packed signed doubleword integers from xmm1 and from xmm2/m128 into 8 packed signed
/// word integers in xmm1 using signed saturation (SSE2).
pub static PACKSSDW: [u8; 3] = [0x66, 0x0f, 0x6b];

/// Converts 8 packed signed word integers from xmm1 and from xmm2/m128 into 16 packed unsigned byte
/// integers in xmm1 using unsigned saturation (SSE2).
pub static PACKUSWB: [u8; 3] = [0x66, 0x0f, 0x67];

/// Converts 4 packed signed doubleword integers from xmm1 and from xmm2/m128 into 8 unpacked signed
/// word integers in xmm1 using unsigned saturation (SSE4.1).
pub static PACKUSDW: [u8; 4] = [0x66, 0x0f, 0x38, 0x2b];

/// Add packed byte integers from xmm2/m128 and xmm1 (SSE2).
pub static PADDB: [u8; 3] = [0x66, 0x0f, 0xfc];

/// Add packed doubleword integers from xmm2/m128 and xmm1 (SSE2).
pub static PADDD: [u8; 3] = [0x66, 0x0f, 0xfe];

/// Add packed quadword integers from xmm2/m128 and xmm1 (SSE2).
pub static PADDQ: [u8; 3] = [0x66, 0x0f, 0xd4];

/// Add packed word integers from xmm2/m128 and xmm1 (SSE2).
pub static PADDW: [u8; 3] = [0x66, 0x0f, 0xfd];

/// Add packed signed byte integers from xmm2/m128 and xmm1 saturate the results (SSE).
pub static PADDSB: [u8; 3] = [0x66, 0x0f, 0xec];

/// Add packed signed word integers from xmm2/m128 and xmm1 saturate the results (SSE).
pub static PADDSW: [u8; 3] = [0x66, 0x0f, 0xed];

/// Add packed unsigned byte integers from xmm2/m128 and xmm1 saturate the results (SSE).
pub static PADDUSB: [u8; 3] = [0x66, 0x0f, 0xdc];

/// Add packed unsigned word integers from xmm2/m128 and xmm1 saturate the results (SSE).
pub static PADDUSW: [u8; 3] = [0x66, 0x0f, 0xdd];

/// Concatenate destination and source operands, extract a byte-aligned result into xmm1 that is
/// shifted to the right by the constant number of bytes in imm8 (SSSE3).
pub static PALIGNR: [u8; 4] = [0x66, 0x0f, 0x3a, 0x0f];

/// Bitwise AND of xmm2/m128 and xmm1 (SSE2).
pub static PAND: [u8; 3] = [0x66, 0x0f, 0xdb];

/// Bitwise AND NOT of xmm2/m128 and xmm1 (SSE2).
pub static PANDN: [u8; 3] = [0x66, 0x0f, 0xdf];

/// Average packed unsigned byte integers from xmm2/m128 and xmm1 with rounding (SSE2).
pub static PAVGB: [u8; 3] = [0x66, 0x0f, 0xE0];

/// Average packed unsigned word integers from xmm2/m128 and xmm1 with rounding (SSE2).
pub static PAVGW: [u8; 3] = [0x66, 0x0f, 0xE3];

/// Select byte values from xmm1 and xmm2/m128 from mask specified in the high bit of each byte
/// in XMM0 and store the values into xmm1 (SSE4.1).
pub static PBLENDVB: [u8; 4] = [0x66, 0x0f, 0x38, 0x10];

/// Select words from xmm1 and xmm2/m128 from mask specified in imm8 and store the values into xmm1
/// (SSE4.1).
pub static PBLENDW: [u8; 4] = [0x66, 0x0f, 0x3a, 0x0e];

/// Compare packed data for equal (SSE2).
pub static PCMPEQB: [u8; 3] = [0x66, 0x0f, 0x74];

/// Compare packed data for equal (SSE2).
pub static PCMPEQD: [u8; 3] = [0x66, 0x0f, 0x76];

/// Compare packed data for equal (SSE4.1).
pub static PCMPEQQ: [u8; 4] = [0x66, 0x0f, 0x38, 0x29];

/// Compare packed data for equal (SSE2).
pub static PCMPEQW: [u8; 3] = [0x66, 0x0f, 0x75];

/// Compare packed signed byte integers for greater than (SSE2).
pub static PCMPGTB: [u8; 3] = [0x66, 0x0f, 0x64];

/// Compare packed signed doubleword integers for greater than (SSE2).
pub static PCMPGTD: [u8; 3] = [0x66, 0x0f, 0x66];

/// Compare packed signed quadword integers for greater than (SSE4.2).
pub static PCMPGTQ: [u8; 4] = [0x66, 0x0f, 0x38, 0x37];

/// Compare packed signed word integers for greater than (SSE2).
pub static PCMPGTW: [u8; 3] = [0x66, 0x0f, 0x65];

/// Extract doubleword or quadword, depending on REX.W (SSE4.1).
pub static PEXTR: [u8; 4] = [0x66, 0x0f, 0x3a, 0x16];

/// Extract byte (SSE4.1).
pub static PEXTRB: [u8; 4] = [0x66, 0x0f, 0x3a, 0x14];

/// Extract word (SSE4.1). There is a 3-byte SSE2 variant that can also move to m/16.
pub static PEXTRW: [u8; 4] = [0x66, 0x0f, 0x3a, 0x15];

/// Insert doubleword or quadword, depending on REX.W (SSE4.1).
pub static PINSR: [u8; 4] = [0x66, 0x0f, 0x3a, 0x22];

/// Insert byte (SSE4.1).
pub static PINSRB: [u8; 4] = [0x66, 0x0f, 0x3a, 0x20];

/// Insert word (SSE2).
pub static PINSRW: [u8; 3] = [0x66, 0x0f, 0xc4];

/// Compare packed signed byte integers in xmm1 and xmm2/m128 and store packed maximum values in
/// xmm1 (SSE4.1).
pub static PMAXSB: [u8; 4] = [0x66, 0x0f, 0x38, 0x3c];

/// Compare packed signed doubleword integers in xmm1 and xmm2/m128 and store packed maximum
/// values in xmm1 (SSE4.1).
pub static PMAXSD: [u8; 4] = [0x66, 0x0f, 0x38, 0x3d];

/// Compare packed signed word integers in xmm1 and xmm2/m128 and store packed maximum values in
/// xmm1 (SSE2).
pub static PMAXSW: [u8; 3] = [0x66, 0x0f, 0xee];

/// Compare packed unsigned byte integers in xmm1 and xmm2/m128 and store packed maximum values in
/// xmm1 (SSE2).
pub static PMAXUB: [u8; 3] = [0x66, 0x0f, 0xde];

/// Compare packed unsigned doubleword integers in xmm1 and xmm2/m128 and store packed maximum
/// values in xmm1 (SSE4.1).
pub static PMAXUD: [u8; 4] = [0x66, 0x0f, 0x38, 0x3f];

/// Compare packed unsigned word integers in xmm1 and xmm2/m128 and store packed maximum values in
/// xmm1 (SSE4.1).
pub static PMAXUW: [u8; 4] = [0x66, 0x0f, 0x38, 0x3e];

/// Compare packed signed byte integers in xmm1 and xmm2/m128 and store packed minimum values in
/// xmm1 (SSE4.1).
pub static PMINSB: [u8; 4] = [0x66, 0x0f, 0x38, 0x38];

/// Compare packed signed doubleword integers in xmm1 and xmm2/m128 and store packed minimum
/// values in xmm1 (SSE4.1).
pub static PMINSD: [u8; 4] = [0x66, 0x0f, 0x38, 0x39];

/// Compare packed signed word integers in xmm1 and xmm2/m128 and store packed minimum values in
/// xmm1 (SSE2).
pub static PMINSW: [u8; 3] = [0x66, 0x0f, 0xea];

/// Compare packed unsigned byte integers in xmm1 and xmm2/m128 and store packed minimum values in
/// xmm1 (SSE2).
pub static PMINUB: [u8; 3] = [0x66, 0x0f, 0xda];

/// Compare packed unsigned doubleword integers in xmm1 and xmm2/m128 and store packed minimum
/// values in xmm1 (SSE4.1).
pub static PMINUD: [u8; 4] = [0x66, 0x0f, 0x38, 0x3b];

/// Compare packed unsigned word integers in xmm1 and xmm2/m128 and store packed minimum values in
/// xmm1 (SSE4.1).
pub static PMINUW: [u8; 4] = [0x66, 0x0f, 0x38, 0x3a];

/// Sign extend 8 packed 8-bit integers in the low 8 bytes of xmm2/m64 to 8 packed 16-bit
/// integers in xmm1 (SSE4.1).
pub static PMOVSXBW: [u8; 4] = [0x66, 0x0f, 0x38, 0x20];

/// Sign extend 4 packed 16-bit integers in the low 8 bytes of xmm2/m64 to 4 packed 32-bit
/// integers in xmm1 (SSE4.1).
pub static PMOVSXWD: [u8; 4] = [0x66, 0x0f, 0x38, 0x23];

/// Sign extend 2 packed 32-bit integers in the low 8 bytes of xmm2/m64 to 2 packed 64-bit
/// integers in xmm1 (SSE4.1).
pub static PMOVSXDQ: [u8; 4] = [0x66, 0x0f, 0x38, 0x25];

/// Zero extend 8 packed 8-bit integers in the low 8 bytes of xmm2/m64 to 8 packed 16-bit
/// integers in xmm1 (SSE4.1).
pub static PMOVZXBW: [u8; 4] = [0x66, 0x0f, 0x38, 0x30];

/// Zero extend 4 packed 16-bit integers in the low 8 bytes of xmm2/m64 to 4 packed 32-bit
/// integers in xmm1 (SSE4.1).
pub static PMOVZXWD: [u8; 4] = [0x66, 0x0f, 0x38, 0x33];

/// Zero extend 2 packed 32-bit integers in the low 8 bytes of xmm2/m64 to 2 packed 64-bit
/// integers in xmm1 (SSE4.1).
pub static PMOVZXDQ: [u8; 4] = [0x66, 0x0f, 0x38, 0x35];

/// Multiply the packed signed word integers in xmm1 and xmm2/m128, and store the low 16 bits of
/// the results in xmm1 (SSE2).
pub static PMULLW: [u8; 3] = [0x66, 0x0f, 0xd5];

/// Multiply the packed doubleword signed integers in xmm1 and xmm2/m128 and store the low 32
/// bits of each product in xmm1 (SSE4.1).
pub static PMULLD: [u8; 4] = [0x66, 0x0f, 0x38, 0x40];

/// Multiply the packed quadword signed integers in xmm2 and xmm3/m128 and store the low 64
/// bits of each product in xmm1 (AVX512VL/DQ). Requires an EVEX encoding.
pub static VPMULLQ: [u8; 4] = [0x66, 0x0f, 0x38, 0x40];

/// Multiply packed unsigned doubleword integers in xmm1 by packed unsigned doubleword integers
/// in xmm2/m128, and store the quadword results in xmm1 (SSE2).
pub static PMULUDQ: [u8; 3] = [0x66, 0x0f, 0xf4];

/// Multiply the packed word integers, add adjacent doubleword results.
pub static PMADDWD: [u8; 3] = [0x66, 0x0f, 0xf5];

/// Pop top of stack into r{16,32,64}; increment stack pointer.
pub static POP_REG: [u8; 1] = [0x58];

/// Returns the count of number of bits set to 1.
pub static POPCNT: [u8; 3] = [0xf3, 0x0f, 0xb8];

/// Bitwise OR of xmm2/m128 and xmm1 (SSE2).
pub static POR: [u8; 3] = [0x66, 0x0f, 0xeb];

/// Shuffle bytes in xmm1 according to contents of xmm2/m128 (SSE3).
pub static PSHUFB: [u8; 4] = [0x66, 0x0f, 0x38, 0x00];

/// Shuffle the doublewords in xmm2/m128 based on the encoding in imm8 and
/// store the result in xmm1 (SSE2).
pub static PSHUFD: [u8; 3] = [0x66, 0x0f, 0x70];

/// Shift words in xmm1 by imm8; the direction and sign-bit behavior is controlled by the RRR
/// digit used in the ModR/M byte (SSE2).
pub static PS_W_IMM: [u8; 3] = [0x66, 0x0f, 0x71];

/// Shift doublewords in xmm1 by imm8; the direction and sign-bit behavior is controlled by the RRR
/// digit used in the ModR/M byte (SSE2).
pub static PS_D_IMM: [u8; 3] = [0x66, 0x0f, 0x72];

/// Shift quadwords in xmm1 by imm8; the direction and sign-bit behavior is controlled by the RRR
/// digit used in the ModR/M byte (SSE2).
pub static PS_Q_IMM: [u8; 3] = [0x66, 0x0f, 0x73];

/// Shift words in xmm1 left by xmm2/m128 while shifting in 0s (SSE2).
pub static PSLLW: [u8; 3] = [0x66, 0x0f, 0xf1];

/// Shift doublewords in xmm1 left by xmm2/m128 while shifting in 0s (SSE2).
pub static PSLLD: [u8; 3] = [0x66, 0x0f, 0xf2];

/// Shift quadwords in xmm1 left by xmm2/m128 while shifting in 0s (SSE2).
pub static PSLLQ: [u8; 3] = [0x66, 0x0f, 0xf3];

/// Shift words in xmm1 right by xmm2/m128 while shifting in 0s (SSE2).
pub static PSRLW: [u8; 3] = [0x66, 0x0f, 0xd1];

/// Shift doublewords in xmm1 right by xmm2/m128 while shifting in 0s (SSE2).
pub static PSRLD: [u8; 3] = [0x66, 0x0f, 0xd2];

/// Shift quadwords in xmm1 right by xmm2/m128 while shifting in 0s (SSE2).
pub static PSRLQ: [u8; 3] = [0x66, 0x0f, 0xd3];

/// Shift words in xmm1 right by xmm2/m128 while shifting in sign bits (SSE2).
pub static PSRAW: [u8; 3] = [0x66, 0x0f, 0xe1];

/// Shift doublewords in xmm1 right by xmm2/m128 while shifting in sign bits (SSE2).
pub static PSRAD: [u8; 3] = [0x66, 0x0f, 0xe2];

/// Subtract packed byte integers in xmm2/m128 from packed byte integers in xmm1 (SSE2).
pub static PSUBB: [u8; 3] = [0x66, 0x0f, 0xf8];

/// Subtract packed word integers in xmm2/m128 from packed word integers in xmm1 (SSE2).
pub static PSUBW: [u8; 3] = [0x66, 0x0f, 0xf9];

/// Subtract packed doubleword integers in xmm2/m128 from doubleword byte integers in xmm1 (SSE2).
pub static PSUBD: [u8; 3] = [0x66, 0x0f, 0xfa];

/// Subtract packed quadword integers in xmm2/m128 from xmm1 (SSE2).
pub static PSUBQ: [u8; 3] = [0x66, 0x0f, 0xfb];

/// Subtract packed signed byte integers in xmm2/m128 from packed signed byte integers in xmm1
/// and saturate results (SSE2).
pub static PSUBSB: [u8; 3] = [0x66, 0x0f, 0xe8];

/// Subtract packed signed word integers in xmm2/m128 from packed signed word integers in xmm1
/// and saturate results (SSE2).
pub static PSUBSW: [u8; 3] = [0x66, 0x0f, 0xe9];

/// Subtract packed unsigned byte integers in xmm2/m128 from packed unsigned byte integers in xmm1
/// and saturate results (SSE2).
pub static PSUBUSB: [u8; 3] = [0x66, 0x0f, 0xd8];

/// Subtract packed unsigned word integers in xmm2/m128 from packed unsigned word integers in xmm1
/// and saturate results (SSE2).
pub static PSUBUSW: [u8; 3] = [0x66, 0x0f, 0xd9];

/// Set ZF if xmm2/m128 AND xmm1 result is all 0s; set CF if xmm2/m128 AND NOT xmm1 result is all
/// 0s (SSE4.1).
pub static PTEST: [u8; 4] = [0x66, 0x0f, 0x38, 0x17];

/// Unpack and interleave high-order bytes from xmm1 and xmm2/m128 into xmm1 (SSE2).
pub static PUNPCKHBW: [u8; 3] = [0x66, 0x0f, 0x68];

/// Unpack and interleave high-order words from xmm1 and xmm2/m128 into xmm1 (SSE2).
pub static PUNPCKHWD: [u8; 3] = [0x66, 0x0f, 0x69];

/// Unpack and interleave high-order doublewords from xmm1 and xmm2/m128 into xmm1 (SSE2).
pub static PUNPCKHDQ: [u8; 3] = [0x66, 0x0f, 0x6A];

/// Unpack and interleave high-order quadwords from xmm1 and xmm2/m128 into xmm1 (SSE2).
pub static PUNPCKHQDQ: [u8; 3] = [0x66, 0x0f, 0x6D];

/// Unpack and interleave low-order bytes from xmm1 and xmm2/m128 into xmm1 (SSE2).
pub static PUNPCKLBW: [u8; 3] = [0x66, 0x0f, 0x60];

/// Unpack and interleave low-order words from xmm1 and xmm2/m128 into xmm1 (SSE2).
pub static PUNPCKLWD: [u8; 3] = [0x66, 0x0f, 0x61];

/// Unpack and interleave low-order doublewords from xmm1 and xmm2/m128 into xmm1 (SSE2).
pub static PUNPCKLDQ: [u8; 3] = [0x66, 0x0f, 0x62];

/// Unpack and interleave low-order quadwords from xmm1 and xmm2/m128 into xmm1 (SSE2).
pub static PUNPCKLQDQ: [u8; 3] = [0x66, 0x0f, 0x6C];

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

/// Compute the square root of the packed double-precision floating-point values and store the
/// result in xmm1 (SSE2).
pub static SQRTPD: [u8; 3] = [0x66, 0x0f, 0x51];

/// Compute the square root of the packed double-precision floating-point values and store the
/// result in xmm1 (SSE).
pub static SQRTPS: [u8; 2] = [0x0f, 0x51];

/// Compute square root of scalar double-precision floating-point value.
pub static SQRTSD: [u8; 3] = [0xf2, 0x0f, 0x51];

/// Compute square root of scalar single-precision value.
pub static SQRTSS: [u8; 3] = [0xf3, 0x0f, 0x51];

/// Subtract r{16,32,64} from r/m of same size.
pub static SUB: [u8; 1] = [0x29];

/// Subtract packed double-precision floating-point values in xmm2/mem from xmm1 and store result
/// in xmm1 (SSE2).
pub static SUBPD: [u8; 3] = [0x66, 0x0f, 0x5c];

/// Subtract packed single-precision floating-point values in xmm2/mem from xmm1 and store result
/// in xmm1 (SSE).
pub static SUBPS: [u8; 2] = [0x0f, 0x5c];

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

/// Convert four packed unsigned doubleword integers from xmm2/m128/m32bcst to packed
/// single-precision floating-point values in xmm1 with writemask k1. Rounding behavior
/// is controlled by MXCSR but can be overriden by EVEX.L'L in static rounding mode
/// (AVX512VL, AVX512F).
pub static VCVTUDQ2PS: [u8; 3] = [0xf2, 0x0f, 0x7a];

/// imm{16,32} XOR r/m{16,32,64}, possibly sign-extended.
pub static XOR_IMM: [u8; 1] = [0x81];

/// r/m{16,32,64} XOR sign-extended imm8.
pub static XOR_IMM8_SIGN_EXTEND: [u8; 1] = [0x83];

/// r/m{16,32,64} XOR register of the same size.
pub static XOR: [u8; 1] = [0x31];

/// Bitwise logical XOR of packed double-precision floating-point values.
pub static XORPD: [u8; 3] = [0x66, 0x0f, 0x57];

/// Bitwise logical XOR of packed single-precision floating-point values.
pub static XORPS: [u8; 2] = [0x0f, 0x57];
