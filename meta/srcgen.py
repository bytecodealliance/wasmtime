"""
Source code generator.

The `srcgen` module contains generic helper routines and classes for generating
source code.

"""

import sys
import os

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
        >>> f.line('Back again')
        >>> f.writelines()
        Hello line 1
          // Nested comment
        Back again

    """

    shiftwidth = 2

    def __init__(self):
        self.indent = ''
        self.lines = []

    def indent_push(self):
        """Increase current indentation level by one."""
        self.indent += ' ' * self.shiftwidth

    def indent_pop(self):
        """Decrease indentation by one level."""
        assert self.indent != '', 'Already at top level indentation'
        self.indent = self.indent[0:-self.shiftwidth]

    def line(self, s):
        """And an indented line."""
        self.lines.append('{}{}\n'.format(self.indent, s))

    def writelines(self, f=None):
        """Write all lines to `f`."""
        if not f:
            f = sys.stdout
        f.writelines(self.lines)

    def update_file(self, filename, directory):
        if directory is not None:
            filename = os.path.join(directory, filename)
        with open(filename, 'w') as f:
            self.writelines(f)

    def comment(self, s):
        """Add a comment line."""
        self.line('// ' + s)

if __name__ == "__main__":
    import doctest
    doctest.testmod()
