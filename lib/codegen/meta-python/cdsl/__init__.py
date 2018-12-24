"""
Cranelift DSL classes.

This module defines the classes that are used to define Cranelift instructions
and other entities.
"""
from __future__ import absolute_import
import re


camel_re = re.compile('(^|_)([a-z])')


def camel_case(s):
    # type: (str) -> str
    """Convert the string s to CamelCase:
        >>> camel_case('x')
        'X'
        >>> camel_case('camel_case')
        'CamelCase'
    """
    return camel_re.sub(lambda m: m.group(2).upper(), s)


def is_power_of_two(x):
    # type: (int) -> bool
    """Check if `x` is a power of two:
        >>> is_power_of_two(0)
        False
        >>> is_power_of_two(1)
        True
        >>> is_power_of_two(2)
        True
        >>> is_power_of_two(3)
        False
    """
    return x > 0 and x & (x-1) == 0


def next_power_of_two(x):
    # type: (int) -> int
    """
    Compute the next power of two that is greater than `x`:
        >>> next_power_of_two(0)
        1
        >>> next_power_of_two(1)
        2
        >>> next_power_of_two(2)
        4
        >>> next_power_of_two(3)
        4
        >>> next_power_of_two(4)
        8
    """
    s = 1
    while x & (x + 1) != 0:
        x |= x >> s
        s *= 2
    return x + 1
