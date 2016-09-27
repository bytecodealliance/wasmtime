"""
Generate constant hash tables.

The `constant_hash` module can generate constant pre-populated hash tables. We
don't attempt parfect hashing, but simply generate an open addressed
quadratically probed hash table.
"""
from __future__ import absolute_import


def simple_hash(s):
    """
    Compute a primitive hash of a string.

    Example:
        >>> hex(simple_hash("Hello"))
        '0x2fa70c01'
        >>> hex(simple_hash("world"))
        '0x5b0c31d5'
    """
    h = 5381
    for c in s:
        h = ((h ^ ord(c)) + ((h >> 6) + (h << 26))) & 0xffffffff
    return h


def next_power_of_two(x):
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


def compute_quadratic(items, hash_function):
    """
    Compute an open addressed, quadratically probed hash table containing
    `items`. The returned table is a list containing the elements of the
    iterable `items` and `None` in unused slots.

    :param items: Iterable set of items to place in hash table.
    :param hash_function: Hash function which takes an item and returns a
            number.

    Simple example (see hash values above, they collide on slot 1):
        >>> compute_quadratic(['Hello', 'world'], simple_hash)
        [None, 'Hello', 'world', None]
    """

    items = list(items)
    # Table size must be a power of two. Aim for >20% unused slots.
    size = next_power_of_two(int(1.20*len(items)))
    table = [None] * size

    for i in items:
        h = hash_function(i) % size
        s = 0
        while table[h] is not None:
            s += 1
            h = (h + s) % size
        table[h] = i

    return table
