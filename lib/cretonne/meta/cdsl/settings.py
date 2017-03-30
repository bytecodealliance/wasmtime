"""Classes for describing settings and groups of settings."""
from __future__ import absolute_import
from collections import OrderedDict
from .predicates import Predicate

try:
    from typing import Set, List, Dict, Any, TYPE_CHECKING  # noqa
    if TYPE_CHECKING:
        from .predicates import PredLeaf, PredNode  # noqa
except ImportError:
    pass


class Setting(object):
    """
    A named setting variable that can be configured externally to Cretonne.

    Settings are normally not named when they are created. They get their name
    from the `extract_names` method.
    """

    def __init__(self, doc):
        # type: (str) -> None
        self.name = None  # type: str  # Assigned later by `extract_names()`.
        self.number = None  # type: int
        self.__doc__ = doc
        # Offset of byte in settings vector containing this setting.
        self.byte_offset = None  # type: int
        self.group = SettingGroup.append(self)

    def __str__(self):
        # type: () -> str
        return '{}.{}'.format(self.group.name, self.name)

    def predicate_context(self):
        # type: () -> SettingGroup
        """
        Return the context where this setting can be evaluated as a (leaf)
        predicate.
        """
        return self.group


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
        self.named_predicates = []  # type: List[Predicate]
        # All boolean predicates that can be accessed by number. This includes:
        # - All boolean settings in this group.
        # - All named predicates.
        # - Added anonymous predicates, see `number_predicate()`.
        # - Added parent predicates that are replicated in this group.
        # Maps predicate -> number.
        self.predicate_number = OrderedDict()  # type: OrderedDict[PredNode, int]  # noqa

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
            for name, obj in globs.items():
                if isinstance(obj, Setting):
                    assert obj.name is None, obj.name
                    obj.name = name
                if isinstance(obj, Predicate):
                    assert obj.name is None
                    obj.name = name
                    self.named_predicates.append(obj)
        self.layout()

    @staticmethod
    def append(setting):
        # type: (Setting) -> SettingGroup
        g = SettingGroup._current
        assert g, "Open a setting group before defining settings."
        g.settings.append(setting)
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
        for p in self.named_predicates:
            self.number_predicate(p)

    def byte_size(self):
        # type: () -> int
        """
        Compute the number of bytes required to hold all settings and
        precomputed predicates.

        This is the size of the byte-sized settings plus all the numbered
        predcate bits rounded up to a whole number of bytes.
        """
        return self.boolean_offset + (len(self.predicate_number) + 7) // 8
