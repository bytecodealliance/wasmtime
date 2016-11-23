"""
Intel settings.
"""
from __future__ import absolute_import
from cdsl.settings import SettingGroup
import base.settings as shared
from .defs import ISA

ISA.settings = SettingGroup('intel', parent=shared.group)

ISA.settings.close(globals())
