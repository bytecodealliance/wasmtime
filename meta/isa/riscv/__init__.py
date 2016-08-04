"""
RISC-V Target
-------------

`RISC-V <http://riscv.org/>`_ is an open instruction set architecture
originally developed at UC Berkeley. It is a RISC-style ISA with either a
32-bit (RV32I) or 64-bit (RV32I) base instruction set and a number of optional
extensions:

RV32M / RV64M
    Integer multiplication and division.

RV32A / RV64A
    Atomics.

RV32F / RV64F
    Single-precision IEEE floating point.

RV32D / RV64D
    Double-precision IEEE floating point.

RV32G / RV64G
    General purpose instruction sets. This represents the union of the I, M, A,
    F, and D instruction sets listed above.

"""

import defs
import encodings

# Re-export the primary target ISA definition.
isa = defs.isa

