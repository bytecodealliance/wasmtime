"""
RISC-V settings.
"""

from cretonne import SettingGroup, BoolSetting
from defs import isa

isa.settings = SettingGroup('riscv')

supports_m = BoolSetting("CPU supports the 'M' extension (mul/div)")
supports_a = BoolSetting("CPU supports the 'A' extension (atomics)")
supports_f = BoolSetting("CPU supports the 'F' extension (float)")
supports_d = BoolSetting("CPU supports the 'D' extension (double)")

isa.settings.close(globals())
