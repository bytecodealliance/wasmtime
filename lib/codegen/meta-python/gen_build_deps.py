"""
Generate build dependencies for Cargo.

The `build.py` script is invoked by cargo when building lib/codegen to
generate Rust code from the instruction descriptions. Cargo needs to know when
it is necessary to rerun the build script.

If the build script outputs lines of the form:

    cargo:rerun-if-changed=/path/to/file

cargo will rerun the build script when those files have changed since the last
build.
"""
from __future__ import absolute_import, print_function
import os
from os.path import dirname, abspath, join

try:
    from typing import Iterable  # noqa
except ImportError:
    pass


def generate():
    # type: () -> None
    print("Dependencies from meta language directory:")
    meta = dirname(abspath(__file__))
    for (dirpath, _, filenames) in os.walk(meta):
        for f in filenames:
            if f.endswith('.py'):
                print("cargo:rerun-if-changed=" + join(dirpath, f))
