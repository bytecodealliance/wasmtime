"""
Generate sources with type info.

This generates a `types.rs` file which is included in
`lib/codegen/ir/types.rs`. The file provides constant definitions for the most
commonly used types, including all of the scalar types.

This ensures that Python and Rust use the same type numbering.
"""
from __future__ import absolute_import
import srcgen
from cdsl.types import ValueType
import base.types  # noqa

try:
    from typing import Iterable  # noqa
except ImportError:
    pass


def emit_type(ty, fmt):
    # type: (ValueType, srcgen.Formatter) -> None
    """
    Emit a constant definition of a single value type.
    """
    name = ty.name.upper()
    fmt.doc_comment(ty.__doc__)
    fmt.line(
            'pub const {}: Type = Type({:#x});'
            .format(name, ty.number))
    fmt.line()


def emit_vectors(bits, fmt):
    # type: (int, srcgen.Formatter) -> None
    """
    Emit definition for all vector types with `bits` total size.
    """
    size = bits // 8
    for ty in ValueType.all_lane_types:
        mb = ty.membytes
        if mb == 0 or mb >= size:
            continue
        emit_type(ty.by(size // mb), fmt)


def emit_types(fmt):
    # type: (srcgen.Formatter) -> None
    for spec in ValueType.all_special_types:
        emit_type(spec, fmt)
    for ty in ValueType.all_lane_types:
        emit_type(ty, fmt)
    # Emit vector definitions for common SIMD sizes.
    emit_vectors(64, fmt)
    emit_vectors(128, fmt)
    emit_vectors(256, fmt)
    emit_vectors(512, fmt)


def generate(out_dir):
    # type: (str) -> None
    fmt = srcgen.Formatter()
    emit_types(fmt)
    fmt.update_file('types.rs', out_dir)
