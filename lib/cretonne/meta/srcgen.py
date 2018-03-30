"""
Source code generator.

The `srcgen` module contains generic helper routines and classes for generating
source code.

"""
from __future__ import absolute_import
import sys
import os
from collections import OrderedDict

try:
    from typing import Any, List, Set, Tuple  # noqa
except ImportError:
    pass


class Formatter(object):
    """
    Source code formatter class.

    - Collect source code to be written to a file.
    - Keep track of indentation.

    Indentation example:

        >>> f = Formatter()
        >>> f.line('Hello line 1')
        >>> f.writelines()
        Hello line 1
        >>> f.indent_push()
        >>> f.comment('Nested comment')
        >>> f.indent_pop()
        >>> f.format('Back {} again', 'home')
        >>> f.writelines()
        Hello line 1
            // Nested comment
        Back home again

    """

    shiftwidth = 4

    def __init__(self):
        # type: () -> None
        self.indent = ''
        self.lines = []  # type: List[str]

    def indent_push(self):
        # type: () -> None
        """Increase current indentation level by one."""
        self.indent += ' ' * self.shiftwidth

    def indent_pop(self):
        # type: () -> None
        """Decrease indentation by one level."""
        assert self.indent != '', 'Already at top level indentation'
        self.indent = self.indent[0:-self.shiftwidth]

    def line(self, s=None):
        # type: (str) -> None
        """Add an indented line."""
        if s:
            self.lines.append('{}{}\n'.format(self.indent, s))
        else:
            self.lines.append('\n')

    def outdented_line(self, s):
        # type: (str) -> None
        """
        Emit a line outdented one level.

        This is used for '} else {' and similar things inside a single indented
        block.
        """
        self.lines.append('{}{}\n'.format(self.indent[0:-self.shiftwidth], s))

    def writelines(self, f=None):
        # type: (Any) -> None
        """Write all lines to `f`."""
        if not f:
            f = sys.stdout
        f.writelines(self.lines)

    def update_file(self, filename, directory):
        # type: (str, str) -> None
        if directory is not None:
            filename = os.path.join(directory, filename)
        with open(filename, 'w') as f:
            self.writelines(f)

    class _IndentedScope(object):
        def __init__(self, fmt, after):
            # type: (Formatter, str) -> None
            self.fmt = fmt
            self.after = after

        def __enter__(self):
            # type: () -> None
            self.fmt.indent_push()

        def __exit__(self, t, v, tb):
            # type: (object, object, object) -> None
            self.fmt.indent_pop()
            if self.after:
                self.fmt.line(self.after)

    def indented(self, before=None, after=None):
        # type: (str, str) -> Formatter._IndentedScope
        """
        Return a scope object for use with a `with` statement:

            >>> f = Formatter()
            >>> with f.indented('prefix {', '} suffix'):
            ...     f.line('hello')
            >>> f.writelines()
            prefix {
                hello
            } suffix

        The optional `before` and `after` parameters are surrounding lines
        which are *not* indented.
        """
        if before:
            self.line(before)
        return Formatter._IndentedScope(self, after)

    def format(self, fmt, *args):
        # type: (str, *Any) -> None
        self.line(fmt.format(*args))

    def multi_line(self, s):
        # type: (str) -> None
        """Add one or more lines after stripping common indentation."""
        for l in parse_multiline(s):
            self.line(l)

    def comment(self, s):
        # type: (str) -> None
        """Add a comment line."""
        self.line('// ' + s)

    def doc_comment(self, s):
        # type: (str) -> None
        """Add a (multi-line) documentation comment."""
        for l in parse_multiline(s):
            self.line('/// ' + l if l else '///')

    def match(self, m):
        # type: (Match) -> None
        """
        Add a match expression.

        Example:

            >>> f = Formatter()
            >>> m = Match('x')
            >>> m.arm('Orange', ['a', 'b'], 'some body')
            >>> m.arm('Yellow', ['a', 'b'], 'some body')
            >>> m.arm('Green', ['a', 'b'], 'different body')
            >>> m.arm('Blue', ['x', 'y'], 'some body')
            >>> f.match(m)
            >>> f.writelines()
            match x {
                Orange { a, b } |
                Yellow { a, b } => {
                    some body
                }
                Green { a, b } => {
                    different body
                }
                Blue { x, y } => {
                    some body
                }
            }

        """
        with self.indented('match {} {{'.format(m.expr), '}'):
            for (fields, body), names in m.arms.items():
                with self.indented('', '}'):
                    names_left = len(names)
                    for name in names.keys():
                        fields_str = ', '.join(fields)
                        if len(fields) != 0:
                            fields_str = '{{ {} }} '.format(fields_str)
                        names_left -= 1
                        if names_left > 0:
                            suffix = '|'
                        else:
                            suffix = '=> {'
                        self.outdented_line(name + ' ' + fields_str + suffix)
                        if names_left == 0:
                            self.multi_line(body)


def _indent(s):
    # type: (str) -> int
    """
    Compute the indentation of s, or None of an empty line.

    Example:
        >>> _indent("foo")
        0
        >>> _indent("    bar")
        4
        >>> _indent("   ")
        >>> _indent("")
    """
    t = s.lstrip()
    return len(s) - len(t) if t else None


def parse_multiline(s):
    # type: (str) -> List[str]
    """
    Given a multi-line string, split it into a sequence of lines after
    stripping a common indentation, as described in the "trim" function
    from PEP 257. This is useful for strings defined with doc strings:
        >>> parse_multiline('\\n    hello\\n    world\\n')
        ['hello', 'world']
    """
    if not s:
        return []
    # Convert tabs to spaces (following the normal Python rules)
    # and split into a list of lines:
    lines = s.expandtabs().splitlines()
    # Determine minimum indentation (first line doesn't count):
    indent = sys.maxsize
    for line in lines[1:]:
        stripped = line.lstrip()
        if stripped:
            indent = min(indent, len(line) - len(stripped))
    # Remove indentation (first line is special):
    trimmed = [lines[0].strip()]
    if indent < sys.maxsize:
        for line in lines[1:]:
            trimmed.append(line[indent:].rstrip())
    # Strip off trailing and leading blank lines:
    while trimmed and not trimmed[-1]:
        trimmed.pop()
    while trimmed and not trimmed[0]:
        trimmed.pop(0)
    return trimmed


class Match(object):
    """
    Match formatting class.

    Match objects collect all the information needed to emit a Rust `match`
    expression, automatically deduplicating overlapping identical arms.

    Example:

        >>> m = Match('x')
        >>> m.arm('Orange', ['a', 'b'], 'some body')
        >>> m.arm('Yellow', ['a', 'b'], 'some body')
        >>> m.arm('Green', ['a', 'b'], 'different body')
        >>> m.arm('Blue', ['x', 'y'], 'some body')
        >>> assert(len(m.arms) == 3)

    Note that this class is ignorant of Rust types, and considers two fields
    with the same name to be equivalent.
    """

    def __init__(self, expr):
        # type: (str) -> None
        self.expr = expr
        self.arms = OrderedDict()  # type: OrderedDict[Tuple[Tuple[str, ...], str], OrderedDict[str, None]]  # noqa

    def arm(self, name, fields, body):
        # type: (str, List[str], str) -> None
        key = (tuple(fields), body)
        if key not in self.arms:
            self.arms[key] = OrderedDict()
        self.arms[key][name] = None
