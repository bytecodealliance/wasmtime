"""
The semantics.types module predefines all the Cretone primitive bitvector
types.
"""
from cdsl.types import BVType
from cdsl.typevar import MAX_BITVEC, int_log2

for width in range(0, int_log2(MAX_BITVEC)+1):
    BVType(2**width)
