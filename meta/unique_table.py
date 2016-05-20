"""
Generate a table of unique items.

The `UniqueTable` class collects items into an array, removing duplicates. Each
item is mapped to its offset in the final array.

This is a compression technique for compile-time generated tables.
"""


class UniqueTable:
    """
    Collect items into the `table` list, removing duplicates.
    """
    def __init__(self):
        # List of items added in order.
        self.table = list()
        # Map item -> index.
        self.index = dict()

    def add(self, item):
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
        self.table = list()
        # Map seq -> index.
        self.index = dict()

    def add(self, seq):
        """
        Add a sequence of items to the table. If the table already contains the
        items in `seq` in the same order, use those instead.

        Return the offset into `self.table` of the beginning of `seq`.
        """
        if len(seq) == 0:
            return 0
        seq = tuple(seq)
        if seq in self.index:
            return self.index[seq]

        idx = len(self.table)
        self.table.extend(seq)

        # Add seq and all sub-sequences to `index`.
        for length in range(1, len(seq) + 1):
            for offset in range(len(seq) - length + 1):
                self.index[seq[offset:offset+length]] = idx + offset

        return idx
