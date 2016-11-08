"""Defining instruction set architectures."""
from __future__ import absolute_import
from .predicates import And

# The typing module is only required by mypy, and we don't use these imports
# outside type comments.
try:
    from typing import Tuple, Union, Any, Iterable, Sequence, TYPE_CHECKING  # noqa
    from .instructions import MaybeBoundInst, InstructionGroup, InstructionFormat  # noqa
    from .predicates import Predicate, FieldPredicate  # noqa
    from .settings import SettingGroup  # noqa
    from .types import ValueType  # noqa
    AnyPredicate = Union[Predicate, FieldPredicate]
except ImportError:
    pass


class TargetISA(object):
    """
    A target instruction set architecture.

    The `TargetISA` class collects everything known about a target ISA.

    :param name: Short mnemonic name for the ISA.
    :param instruction_groups: List of `InstructionGroup` instances that are
        relevant for this ISA.
    """

    def __init__(self, name, instruction_groups):
        # type: (str, Sequence[InstructionGroup]) -> None
        self.name = name
        self.settings = None  # type: SettingGroup
        self.instruction_groups = instruction_groups
        self.cpumodes = list()  # type: List[CPUMode]

    def finish(self):
        # type: () -> TargetISA
        """
        Finish the definition of a target ISA after adding all CPU modes and
        settings.

        This computes some derived properties that are used in multilple
        places.

        :returns self:
        """
        self._collect_encoding_recipes()
        self._collect_predicates()
        return self

    def _collect_encoding_recipes(self):
        # type: () -> None
        """
        Collect and number all encoding recipes in use.
        """
        self.all_recipes = list()  # type: List[EncRecipe]
        rcps = set()  # type: Set[EncRecipe]
        for cpumode in self.cpumodes:
            for enc in cpumode.encodings:
                recipe = enc.recipe
                if recipe not in rcps:
                    recipe.number = len(rcps)
                    rcps.add(recipe)
                    self.all_recipes.append(recipe)

    def _collect_predicates(self):
        # type: () -> None
        """
        Collect and number all predicates in use.

        Sets `instp.number` for all used instruction predicates and places them
        in `self.all_instps` in numerical order.

        Ensures that all ISA predicates have an assigned bit number in
        `self.settings`.
        """
        self.all_instps = list()  # type: List[AnyPredicate]
        instps = set()  # type: Set[AnyPredicate]
        for cpumode in self.cpumodes:
            for enc in cpumode.encodings:
                instp = enc.instp
                if instp and instp not in instps:
                    # assign predicate number starting from 0.
                    instp.number = len(instps)
                    instps.add(instp)
                    self.all_instps.append(instp)

                # All referenced ISA predicates must have a number in
                # `self.settings`. This may cause some parent predicates to be
                # replicated here, which is OK.
                if enc.isap:
                    self.settings.number_predicate(enc.isap)


class CPUMode(object):
    """
    A CPU mode determines which instruction encodings are active.

    All instruction encodings are associated with exactly one `CPUMode`, and
    all CPU modes are associated with exactly one `TargetISA`.

    :param name: Short mnemonic name for the CPU mode.
    :param target: Associated `TargetISA`.
    """

    def __init__(self, name, isa):
        # type: (str, TargetISA) -> None
        self.name = name
        self.isa = isa
        self.encodings = []  # type: List[Encoding]
        isa.cpumodes.append(self)

    def __str__(self):
        # type: () -> str
        return self.name

    def enc(self, *args, **kwargs):
        """
        Add a new encoding to this CPU mode.

        Arguments are the `Encoding constructor arguments, except for the first
        `CPUMode argument which is implied.
        """
        self.encodings.append(Encoding(self, *args, **kwargs))


class EncRecipe(object):
    """
    A recipe for encoding instructions with a given format.

    Many different instructions can be encoded by the same recipe, but they
    must all have the same instruction format.

    :param name: Short mnemonic name for this recipe.
    :param format: All encoded instructions must have this
            :py:class:`InstructionFormat`.
    """

    def __init__(self, name, format, instp=None, isap=None):
        # type: (str, InstructionFormat, AnyPredicate, AnyPredicate) -> None
        self.name = name
        self.format = format
        self.instp = instp
        self.isap = isap
        if instp:
            assert instp.predicate_context() == format
        self.number = None  # type: int

    def __str__(self):
        # type: () -> str
        return self.name


class Encoding(object):
    """
    Encoding for a concrete instruction.

    An `Encoding` object ties an instruction opcode with concrete type
    variables together with and encoding recipe and encoding bits.

    :param cpumode: The CPU mode where the encoding is active.
    :param inst: The :py:class:`Instruction` or :py:class:`BoundInstruction`
                 being encoded.
    :param recipe: The :py:class:`EncRecipe` to use.
    :param encbits: Additional encoding bits to be interpreted by `recipe`.
    :param instp: Instruction predicate, or `None`.
    :param isap: ISA predicate, or `None`.
    """

    def __init__(self, cpumode, inst, recipe, encbits, instp=None, isap=None):
        # type: (CPUMode, MaybeBoundInst, EncRecipe, int, AnyPredicate, AnyPredicate) -> None # noqa
        assert isinstance(cpumode, CPUMode)
        assert isinstance(recipe, EncRecipe)
        self.inst, self.typevars = inst.fully_bound()
        self.cpumode = cpumode
        assert self.inst.format == recipe.format, (
                "Format {} must match recipe: {}".format(
                    self.inst.format, recipe.format))
        self.recipe = recipe
        self.encbits = encbits
        # Combine recipe predicates with the manually specified ones.
        self.instp = And.combine(recipe.instp, instp)
        self.isap = And.combine(recipe.isap, isap)

    def __str__(self):
        # type: () -> str
        return '[{}#{:02x}]'.format(self.recipe, self.encbits)

    def ctrl_typevar(self):
        # type: () -> ValueType
        """
        Get the controlling type variable for this encoding or `None`.
        """
        if self.typevars:
            return self.typevars[0]
        else:
            return None
