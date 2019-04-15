"""Classes for describing settings and groups of settings."""
from __future__ import absolute_import
from collections import OrderedDict
from .predicates import Predicate

try:
    from typing import Tuple, Set, List, Dict, Any, Union, TYPE_CHECKING  # noqa
    BoolOrPresetOrDict = Union['BoolSetting', 'Preset', Dict['Setting', Any]]
    if TYPE_CHECKING:
        from .predicates import PredLeaf, PredNode, PredKey  # noqa
except ImportError:
    pass


class Setting(object):
    """
    A named setting variable that can be configured externally to Cranelift.

    Settings are normally not named when they are created. They get their name
    from the `extract_names` method.
    """

    def __init__(self, doc):
        # type: (str) -> None
        self.name = None  # type: str  # Assigned later by `extract_names()`.
        self.__doc__ = doc
        # Offset of byte in settings vector containing this setting.
        self.byte_offset = None  # type: int
        # Index into the generated DESCRIPTORS table.
        self.descriptor_index = None  # type: int

        self.group = SettingGroup.append(self)

    def __str__(self):
        # type: () -> str
        return '{}.{}'.format(self.group.name, self.name)

    def default_byte(self):
        # type: () -> int
        raise NotImplementedError("default_byte is an abstract method")

    def byte_for_value(self, value):
        # type: (Any) -> int
        """Get the setting byte value that corresponds to `value`"""
        raise NotImplementedError("byte_for_value is an abstract method")

    def byte_mask(self):
        # type: () -> int
        """Get a mask of bits in our byte that are relevant to this setting."""
        # Only BoolSetting has a different mask.
        return 0xff


class BoolSetting(Setting):
    """
    A named setting with a boolean on/off value.

    :param doc: Documentation string.
    :param default: The default value of this setting.
    """

    def __init__(self, doc, default=False):
        # type: (str, bool) -> None
        super(BoolSetting, self).__init__(doc)
        self.default = default
        self.bit_offset = None  # type: int

    def default_byte(self):
        # type: () -> int
        """
        Get the default value of this setting, as a byte that can be bitwise
        or'ed with the other booleans sharing the same byte.
        """
        if self.default:
            return 1 << self.bit_offset
        else:
            return 0

    def byte_for_value(self, value):
        # type: (Any) -> int
        if value:
            return 1 << self.bit_offset
        else:
            return 0

    def byte_mask(self):
        # type: () -> int
        return 1 << self.bit_offset

    def predicate_context(self):
        # type: () -> SettingGroup
        """
        Return the context where this setting can be evaluated as a (leaf)
        predicate.
        """
        return self.group

    def predicate_key(self):
        # type: () -> PredKey
        assert self.name, "Can't compute key before setting is named"
        return ('setting', self.group.name, self.name)

    def predicate_leafs(self, leafs):
        # type: (Set[PredLeaf]) -> None
        leafs.add(self)

    def rust_predicate(self, prec):
        # type: (int) -> str
        """
        Return the Rust code to compute the value of this setting.

        The emitted code assumes that the setting group exists as a local
        variable.
        """
        return '{}.{}()'.format(self.group.name, self.name)


class NumSetting(Setting):
    """
    A named setting with an integral value in the range 0--255.

    :param doc: Documentation string.
    :param default: The default value of this setting.
    """

    def __init__(self, doc, default=0):
        # type: (str, int) -> None
        super(NumSetting, self).__init__(doc)
        assert default == int(default)
        assert default >= 0 and default <= 255
        self.default = default

    def default_byte(self):
        # type: () -> int
        return self.default

    def byte_for_value(self, value):
        # type: (Any) -> int
        assert isinstance(value, int), "NumSetting must be set to an int"
        assert value >= 0 and value <= 255
        return value


class EnumSetting(Setting):
    """
    A named setting with an enumerated set of possible values.

    The default value is always the first enumerator.

    :param doc: Documentation string.
    :param args: Tuple of unique strings representing the possible values.
    """

    def __init__(self, doc, *args):
        # type: (str, *str) -> None
        super(EnumSetting, self).__init__(doc)
        assert len(args) > 0, "EnumSetting must have at least one value"
        self.values = tuple(str(x) for x in args)
        self.default = self.values[0]

    def default_byte(self):
        # type: () -> int
        return 0

    def byte_for_value(self, value):
        # type: (Any) -> int
        return self.values.index(value)


class SettingGroup(object):
    """
    A group of settings.

    Whenever a :class:`Setting` object is created, it is added to the currently
    open group. A setting group must be closed explicitly before another can be
    opened.

    :param name: Short mnemonic name for setting group.
    :param parent: Parent settings group.
    """

    # The currently open setting group.
    _current = None  # type: SettingGroup

    def __init__(self, name, parent=None):
        # type: (str, SettingGroup) -> None
        self.name = name
        self.parent = parent
        self.settings = []  # type: List[Setting]
        # Named predicates computed from settings in this group or its
        # parents.
        self.named_predicates = OrderedDict()  # type: OrderedDict[str, Predicate]  # noqa
        # All boolean predicates that can be accessed by number. This includes:
        # - All boolean settings in this group.
        # - All named predicates.
        # - Added anonymous predicates, see `number_predicate()`.
        # - Added parent predicates that are replicated in this group.
        # Maps predicate -> number.
        self.predicate_number = OrderedDict()  # type: OrderedDict[PredNode, int]  # noqa
        self.presets = []  # type: List[Preset]

        # Fully qualified Rust module name. See gen_settings.py.
        self.qual_mod = None  # type: str

        self.open()

    def open(self):
        # type: () -> None
        """
        Open this setting group such that future new settings are added to this
        group.
        """
        assert SettingGroup._current is None, (
                "Can't open {} since {} is already open"
                .format(self, SettingGroup._current))
        SettingGroup._current = self

    def close(self, globs=None):
        # type: (Dict[str, Any]) -> None
        """
        Close this setting group. This function must be called before opening
        another setting group.

        :param globs: Pass in `globals()` to run `extract_names` on all
            settings defined in the module.
        """
        assert SettingGroup._current is self, (
                "Can't close {}, the open setting group is {}"
                .format(self, SettingGroup._current))
        SettingGroup._current = None
        if globs:
            # Ensure that named predicates are ordered in a deterministic way
            # that the Rust crate may simply reproduce, by pushing entries into
            # a vector that we'll sort by name later.
            named_predicates = []

            for name, obj in globs.items():
                if isinstance(obj, Setting):
                    assert obj.name is None, obj.name
                    obj.name = name
                if isinstance(obj, Predicate):
                    named_predicates.append((name, obj))
                if isinstance(obj, Preset):
                    assert obj.name is None, obj.name
                    obj.name = name

            named_predicates.sort(key=lambda x: x[0])
            for (name, obj) in named_predicates:
                self.named_predicates[name] = obj

        self.layout()

    @staticmethod
    def append(setting):
        # type: (Setting) -> SettingGroup
        g = SettingGroup._current
        assert g, "Open a setting group before defining settings."
        g.settings.append(setting)
        return g

    @staticmethod
    def append_preset(preset):
        # type: (Preset) -> SettingGroup
        g = SettingGroup._current
        assert g, "Open a setting group before defining presets."
        g.presets.append(preset)
        return g

    def number_predicate(self, pred):
        # type: (PredNode) -> int
        """
        Make sure that `pred` has an assigned number, and will be included in
        this group's bit vector.

        The numbered predicates include:
        - `BoolSetting` settings that belong to this group.
        - `Predicate` instances in `named_predicates`.
        - `Predicate` instances without a name.
        - Settings or computed predicates that belong to the parent group, but
          need to be accessible by number in this group.

        The numbered predicates are referenced by the encoding tables as ISA
        predicates. See the `isap` field on `Encoding`.

        :returns: The assigned predicate number in this group.
        """
        if pred in self.predicate_number:
            return self.predicate_number[pred]
        else:
            number = len(self.predicate_number)
            self.predicate_number[pred] = number
            return number

    def layout(self):
        # type: () -> None
        """
        Compute the layout of the byte vector used to represent this settings
        group.

        The byte vector contains the following entries in order:

        1. Byte-sized settings like `NumSetting` and `EnumSetting`.
        2. `BoolSetting` settings.
        3. Precomputed named predicates.
        4. Other numbered predicates, including anonymous predicates and parent
           predicates that need to be accessible by number.

        Set `self.settings_size` to the length of the byte vector prefix that
        contains the settings. All bytes after that are computed, not
        configured.

        Set `self.boolean_offset` to the beginning of the numbered predicates,
        2. in the list above.

        Assign `byte_offset` and `bit_offset` fields in all settings.

        After calling this method, no more settings can be added, but
        additional predicates can be made accessible with `number_predicate()`.
        """
        assert len(self.predicate_number) == 0, "Too late for layout"

        # Assign the non-boolean settings.
        byte_offset = 0
        for s in self.settings:
            if not isinstance(s, BoolSetting):
                s.byte_offset = byte_offset
                byte_offset += 1

        # Then the boolean settings.
        self.boolean_offset = byte_offset
        for s in self.settings:
            if isinstance(s, BoolSetting):
                number = self.number_predicate(s)
                s.byte_offset = byte_offset + number // 8
                s.bit_offset = number % 8

        # This is the end of the settings. Round up to a whole number of bytes.
        self.boolean_settings = len(self.predicate_number)
        self.settings_size = self.byte_size()

        # Now assign numbers to all our named predicates.
        for name, pred in self.named_predicates.items():
            self.number_predicate(pred)

    def byte_size(self):
        # type: () -> int
        """
        Compute the number of bytes required to hold all settings and
        precomputed predicates.

        This is the size of the byte-sized settings plus all the numbered
        predicate bits rounded up to a whole number of bytes.
        """
        return self.boolean_offset + (len(self.predicate_number) + 7) // 8


class Preset(object):
    """
    A collection of setting values that are applied at once.

    A `Preset` represents a shorthand notation for applying a number of
    settings at once. Example:

        nehalem = Preset(has_sse41, has_cmov, has_avx=0)

    Enabling the `nehalem` setting is equivalent to enabling `has_sse41` and
    `has_cmov` while disabling the `has_avx` setting.
    """

    def __init__(self, *args):
        # type: (*BoolOrPresetOrDict) -> None
        self.name = None  # type: str  # Assigned later by `SettingGroup`.
        # Each tuple provides the value for a setting.
        self.values = list()  # type: List[Tuple[Setting, Any]]

        for arg in args:
            if isinstance(arg, Preset):
                # Any presets in args are immediately expanded.
                self.values.extend(arg.values)
            elif isinstance(arg, dict):
                # A dictionary of key: value pairs.
                self.values.extend(arg.items())
            else:
                # A BoolSetting to enable.
                assert isinstance(arg, BoolSetting)
                self.values.append((arg, True))

        self.group = SettingGroup.append_preset(self)
        # Index into the generated DESCRIPTORS table.
        self.descriptor_index = None  # type: int

    def layout(self):
        # type: () -> List[Tuple[int, int]]
        """
        Compute a list of (mask, byte) pairs that incorporate all values in
        this preset.

        The list will have an entry for each setting byte in the settings
        group.
        """
        lst = [(0, 0)] * self.group.settings_size

        # Apply setting values in order.
        for s, v in self.values:
            ofs = s.byte_offset
            s_mask = s.byte_mask()
            s_val = s.byte_for_value(v)
            assert (s_val & ~s_mask) == 0
            l_mask, l_val = lst[ofs]
            # Accumulated mask of modified bits.
            l_mask |= s_mask
            # Overwrite the relevant bits with the new value.
            l_val = (l_val & ~s_mask) | s_val
            lst[ofs] = (l_mask, l_val)

        return lst
