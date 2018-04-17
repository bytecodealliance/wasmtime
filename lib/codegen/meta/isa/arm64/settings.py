"""
ARM64 settings.
"""
from __future__ import absolute_import
from cdsl.settings import SettingGroup
import base.settings as shared
from .defs import ISA

ISA.settings = SettingGroup('arm64', parent=shared.group)

ISA.settings.close(globals())
