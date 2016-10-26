"""
Source code generator.

The `srcgen` module contains generic helper routines and classes for generating
source code.

"""
from __future__ import absolute_import
import sys
import os
import re

try:
    from typing import Any  # noqa
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
        self.line(fmt.format(*args))

    def comment(self, s):
        """Add a comment line."""
        self.line('// ' + s)

    def doc_comment(self, s):
        """Add a (multi-line) documentation comment."""
        s = re.sub('^', self.indent + '/// ', s, flags=re.M) + '\n'
        self.lines.append(s)
