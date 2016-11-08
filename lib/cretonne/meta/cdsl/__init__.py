"""
Cretonne DSL classes.

This module defines the classes that are used to define Cretonne instructions
and other entitties.
"""
from __future__ import absolute_import
import re


camel_re = re.compile('(^|_)([a-z])')


def camel_case(s):
    # type: (str) -> str
    """Convert the string s to CamelCase"""
    return camel_re.sub(lambda m: m.group(2).upper(), s)
