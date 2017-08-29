"""
RISC-V settings.
"""
from __future__ import absolute_import
from cdsl.settings import SettingGroup, BoolSetting
from cdsl.predicates import And
import base.settings as shared
from .defs import ISA

ISA.settings = SettingGroup('riscv', parent=shared.group)

supports_m = BoolSetting("CPU supports the 'M' extension (mul/div)")
supports_a = BoolSetting("CPU supports the 'A' extension (atomics)")
supports_f = BoolSetting("CPU supports the 'F' extension (float)")
supports_d = BoolSetting("CPU supports the 'D' extension (double)")

enable_m = BoolSetting(
        "Enable the use of 'M' instructions if available",
        default=True)

enable_e = BoolSetting(
        "Enable the 'RV32E' instruction set with only 16 registers")

use_m = And(supports_m, enable_m)
use_a = And(supports_a, shared.enable_atomics)
use_f = And(supports_f, shared.enable_float)
use_d = And(supports_d, shared.enable_float)

full_float = And(shared.enable_simd, supports_f, supports_d)

ISA.settings.close(globals())
