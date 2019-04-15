"""
x86 settings.
"""
from __future__ import absolute_import
from cdsl.settings import SettingGroup, BoolSetting, Preset
from cdsl.predicates import And, Not
import base.settings as shared
from .defs import ISA

ISA.settings = SettingGroup('x86', parent=shared.group)

# The has_* settings here correspond to CPUID bits.

# CPUID.01H:ECX
has_sse3 = BoolSetting("SSE3: CPUID.01H:ECX.SSE3[bit 0]")
has_ssse3 = BoolSetting("SSSE3: CPUID.01H:ECX.SSSE3[bit 9]")
has_sse41 = BoolSetting("SSE4.1: CPUID.01H:ECX.SSE4_1[bit 19]")
has_sse42 = BoolSetting("SSE4.2: CPUID.01H:ECX.SSE4_2[bit 20]")
has_popcnt = BoolSetting("POPCNT: CPUID.01H:ECX.POPCNT[bit 23]")
has_avx = BoolSetting("AVX: CPUID.01H:ECX.AVX[bit 28]")

# CPUID.(EAX=07H, ECX=0H):EBX
has_bmi1 = BoolSetting("BMI1: CPUID.(EAX=07H, ECX=0H):EBX.BMI1[bit 3]")
has_bmi2 = BoolSetting("BMI2: CPUID.(EAX=07H, ECX=0H):EBX.BMI2[bit 8]")

# CPUID.EAX=80000001H:ECX
has_lzcnt = BoolSetting("LZCNT: CPUID.EAX=80000001H:ECX.LZCNT[bit 5]")


# The use_* settings here are used to determine if a feature can be used.

use_sse41 = And(has_sse41)
use_sse42 = And(has_sse42, use_sse41)
use_popcnt = And(has_popcnt, has_sse42)
use_bmi1 = And(has_bmi1)
use_lzcnt = And(has_lzcnt)

is_pic = And(shared.is_pic)
not_is_pic = Not(shared.is_pic)
all_ones_funcaddrs_and_not_is_pic = And(shared.allones_funcaddrs,
                                        Not(shared.is_pic))
not_all_ones_funcaddrs_and_not_is_pic = And(Not(shared.allones_funcaddrs),
                                            Not(shared.is_pic))

# Presets corresponding to x86 CPUs.

baseline = Preset()

nehalem = Preset(
        has_sse3, has_ssse3, has_sse41, has_sse42, has_popcnt)
haswell = Preset(nehalem, has_bmi1, has_bmi2, has_lzcnt)
broadwell = Preset(haswell)
skylake = Preset(broadwell)
cannonlake = Preset(skylake)
icelake = Preset(cannonlake)

znver1 = Preset(
        has_sse3, has_ssse3, has_sse41, has_sse42, has_popcnt,
        has_bmi1, has_bmi2, has_lzcnt)

ISA.settings.close(globals())
