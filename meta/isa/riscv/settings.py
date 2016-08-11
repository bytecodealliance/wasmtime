"""
RISC-V settings.
"""

from cretonne import SettingGroup, BoolSetting
from cretonne.predicates import And
import cretonne.settings as shared
from defs import isa

isa.settings = SettingGroup('riscv', parent=shared.group)

supports_m = BoolSetting("CPU supports the 'M' extension (mul/div)")
supports_a = BoolSetting("CPU supports the 'A' extension (atomics)")
supports_f = BoolSetting("CPU supports the 'F' extension (float)")
supports_d = BoolSetting("CPU supports the 'D' extension (double)")

full_float = And(shared.enable_simd, supports_f, supports_d)

isa.settings.close(globals())
