"""
Cretonne shared settings.

This module defines settings are are relevant for all code generators.
"""

from . import SettingGroup, BoolSetting

group = SettingGroup('shared')

enable_simd = BoolSetting("Enable the use of SIMD instructions", default=True)

group.close(globals())
