"""
Intel register banks.

While the floating-point registers are straight-forward, the general purpose
register bank has a few quirks on Intel architectures. We have these encodings
of the 8-bit registers:

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
from cdsl.registers import RegBank, RegClass
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

GPR = RegClass('GPR', IntRegs)
FPR = RegClass('FPR', FloatRegs)
