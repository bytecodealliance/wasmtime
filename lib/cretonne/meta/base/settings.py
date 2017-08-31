"""
Cretonne shared settings.

This module defines settings relevant for all code generators.
"""
from __future__ import absolute_import
from cdsl.settings import SettingGroup, BoolSetting, EnumSetting

group = SettingGroup('shared')

opt_level = EnumSetting(
        """
        Optimization level:

        - default: Very profitable optimizations enabled, none slow.
        - best: Enable all optimizations
        - fastest: Optimize for compile time by disabling most optimizations.
        """,
        'default', 'best', 'fastest')

enable_verifier = BoolSetting(
        """
        Run the Cretonne IL verifier at strategic times during compilation.

        This makes compilation slower but catches many bugs. The verifier is
        disabled by default, except when reading Cretonne IL from a text file.
        """)

is_64bit = BoolSetting("Enable 64-bit code generation")

is_compressed = BoolSetting("Enable compressed instructions")

enable_float = BoolSetting(
        """Enable the use of floating-point instructions""",
        default=True)

enable_simd = BoolSetting(
        """Enable the use of SIMD instructions.""",
        default=True)

enable_atomics = BoolSetting(
        """Enable the use of atomic instructions""",
        default=True)

group.close(globals())
