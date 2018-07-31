"""
Generate a table of unique items.

The `UniqueTable` class collects items into an array, removing duplicates. Each
item is mapped to its offset in the final array.

This is a compression technique for compile-time generated tables.
"""

try:
    from typing import Any, List, Dict, Tuple, Sequence  # noqa
except ImportError:
    pass


class UniqueTable:
    """
    Collect items into the `table` list, removing duplicates.
    """
    def __init__(self):
        # type: () -> None
        # List of items added in order.
        self.table = list()  # type: List[Any]
        # Map item -> index.
        self.index = dict()  # type: Dict[Any, int]

    def add(self, item):
        # type: (Any) -> int
        """
        Add a single item to the table if it isn't already there.

        Return the offset into `self.table` of the item.
        """
        if item in self.index:
            return self.index[item]

        idx = len(self.table)
        self.index[item] = idx
        self.table.append(item)
        return idx


class UniqueSeqTable:
    """
    Collect sequences into the `table` list, removing duplicates.

    Sequences don't have to be of the same length.
    """
    def __init__(self):
        # type: () -> None
        self.table = list()  # type: List[Any]
        # Map seq -> index.
        self.index = dict()  # type: Dict[Tuple[Any, ...], int]

    def add(self, seq):
        # type: (Sequence[Any]) -> int
        """
        Add a sequence of items to the table. If the table already contains the
        items in `seq` in the same order, use those instead.

        Return the offset into `self.table` of the beginning of `seq`.
        """
        if len(seq) == 0:
            return 0
        tseq = tuple(seq)
        if tseq in self.index:
            return self.index[tseq]

        idx = len(self.table)
        self.table.extend(tseq)

        # Add seq and all sub-sequences to `index`.
        index = self.index  # type: Dict[Tuple[Any, ...], int]
        assert index is not None
        for length in range(1, len(tseq) + 1):
            for offset in range(len(tseq) - length + 1):
                key = tseq[offset:offset+length]
                index[key] = idx + offset

        return idx
