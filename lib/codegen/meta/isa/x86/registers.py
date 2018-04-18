"""
x86 register banks.

While the floating-point registers are straight-forward, the general purpose
register bank has a few quirks on x86. We have these encodings of the 8-bit
registers:

         I32 I64  |  16b 32b  64b
    000  AL  AL   |  AX  EAX  RAX
    001  CL  CL   |  CX  ECX  RCX
    010  DL  DL   |  DX  EDX  RDX
    011  BL  BL   |  BX  EBX  RBX
    100  AH  SPL  |  SP  ESP  RSP
    101  CH  BPL  |  BP  EBP  RBP
    110  DH  SIL  |  SI  ESI  RSI
    111  BH  DIL  |  DI  EDI  RDI

Here, the I64 column refers to the registers you get with a REX prefix. Without
the REX prefix, you get the I32 registers.

The 8-bit registers are not that useful since WebAssembly only has i32 and i64
data types, and the H-registers even less so. Rather than trying to model the
H-registers accurately, we'll avoid using them in both I32 and I64 modes.
"""
from __future__ import absolute_import
from cdsl.registers import RegBank, RegClass, Stack
from .defs import ISA


IntRegs = RegBank(
        'IntRegs', ISA,
        'General purpose registers',
        units=16, prefix='r',
        names='rax rcx rdx rbx rsp rbp rsi rdi'.split())

FloatRegs = RegBank(
        'FloatRegs', ISA,
        'SSE floating point registers',
        units=16, prefix='xmm')

FlagRegs = RegBank(
        'FlagRegs', ISA,
        'Flag registers',
        units=1,
        pressure_tracking=False,
        names=['rflags'])

GPR = RegClass(IntRegs)
# Certain types of deref encodings cannot be used with all registers.
#   R13/RBP cannot be used with zero-offset load or store instructions.
#   R12 cannot be used with a non-SIB-byte encoding of all derefs.
GPR_DEREF_SAFE = GPR.without(GPR.rsp, GPR.r12)
GPR_ZERO_DEREF_SAFE = GPR_DEREF_SAFE.without(GPR.rbp, GPR.r13)
GPR8 = GPR[0:8]
GPR8_DEREF_SAFE = GPR8.without(GPR.rsp)
GPR8_ZERO_DEREF_SAFE = GPR8_DEREF_SAFE.without(GPR.rbp)
ABCD = GPR[0:4]
FPR = RegClass(FloatRegs)
FPR8 = FPR[0:8]
FLAG = RegClass(FlagRegs)

# Constraints for stack operands.

# Stack operand with a 32-bit signed displacement from either RBP or RSP.
StackGPR32 = Stack(GPR)
StackFPR32 = Stack(FPR)

RegClass.extract_names(globals())
