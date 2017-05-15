"""Defining instruction set architectures."""
from __future__ import absolute_import
from .predicates import And
from .registers import RegClass, Register
from .ast import Apply

# The typing module is only required by mypy, and we don't use these imports
# outside type comments.
try:
    from typing import Tuple, Union, Any, Iterable, Sequence, List, Set, Dict, TYPE_CHECKING  # noqa
    if TYPE_CHECKING:
        from .instructions import MaybeBoundInst, InstructionGroup, InstructionFormat  # noqa
        from .predicates import PredNode  # noqa
        from .settings import SettingGroup  # noqa
        from .types import ValueType  # noqa
        from .registers import RegBank  # noqa
        OperandConstraint = Union[RegClass, Register, int]
        ConstraintSeq = Union[OperandConstraint, Tuple[OperandConstraint, ...]]
        # Instruction specification for encodings. Allows for predicated
        # instructions.
        InstSpec = Union[MaybeBoundInst, Apply]
        BranchRange = Sequence[int]
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
        self.regbanks = list()  # type: List[RegBank]
        self.regclasses = list()  # type: List[RegClass]

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
        self._collect_regclasses()
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
        self.all_instps = list()  # type: List[PredNode]
        instps = set()  # type: Set[PredNode]
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

    def _collect_regclasses(self):
        # type: () -> None
        """
        Collect and number register classes.

        Every register class needs a unique index, and the classes need to be
        topologically ordered.

        We also want all the top-level register classes to be first.
        """
        # Compute subclasses and top-level classes in each bank.
        # Collect the top-level classes so they get numbered consecutively.
        for bank in self.regbanks:
            bank.finish_regclasses()
            self.regclasses.extend(bank.toprcs)

        # The limit on the number of top-level register classes can be raised.
        # This should be coordinated with the `MAX_TOPRCS` constant in
        # `isa/registers.rs`.
        assert len(self.regclasses) <= 4, "Too many top-level register classes"

        # Collect all of the non-top-level register classes.
        # They are numbered strictly after the top-level classes.
        for bank in self.regbanks:
            self.regclasses.extend(
                    rc for rc in bank.classes if not rc.is_toprc())

        for idx, rc in enumerate(self.regclasses):
            rc.index = idx

        # The limit on the number of register classes can be changed. It should
        # be coordinated with the `RegClassMask` and `RegClassIndex` types in
        # `isa/registers.rs`.
        assert len(self.regclasses) <= 32, "Too many register classes"


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
        # type: (*Any, **Any) -> None
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

    The `ins` and `outs` arguments are tuples specifying the register
    allocation constraints for the value operands and results respectively. The
    possible constraints for an operand are:

    - A `RegClass` specifying the set of allowed registers.
    - A `Register` specifying a fixed-register operand.
    - An integer indicating that this result is tied to a value operand, so
      they must use the same register.

    The `branch_range` argument must be provided for recipes that can encode
    branch instructions. It is an `(origin, bits)` tuple describing the exact
    range that can be encoded in a branch instruction.

    :param name: Short mnemonic name for this recipe.
    :param format: All encoded instructions must have this
            :py:class:`InstructionFormat`.
    :param size: Number of bytes in the binary encoded instruction.
    :param: ins Tuple of register constraints for value operands.
    :param: outs Tuple of register constraints for results.
    :param: branch_range `(origin, bits)` range for branches.
    :param: instp Instruction predicate.
    :param: isap ISA predicate.
    """

    def __init__(
            self,
            name,               # type: str
            format,             # type: InstructionFormat
            size,               # type: int
            ins,                # type: ConstraintSeq
            outs,               # type: ConstraintSeq
            branch_range=None,  # type: BranchRange
            instp=None,         # type: PredNode
            isap=None           # type: PredNode
            ):
        # type: (...) -> None
        self.name = name
        self.format = format
        assert size >= 0
        self.size = size
        self.branch_range = branch_range
        self.instp = instp
        self.isap = isap
        if instp:
            assert instp.predicate_context() == format
        self.number = None  # type: int

        self.ins = self._verify_constraints(ins)
        if not format.has_value_list:
            assert len(self.ins) == format.num_value_operands
        self.outs = self._verify_constraints(outs)

    def __str__(self):
        # type: () -> str
        return self.name

    def _verify_constraints(self, seq):
        # type: (ConstraintSeq) -> Sequence[OperandConstraint]
        if not isinstance(seq, tuple):
            seq = (seq,)
        for c in seq:
            if isinstance(c, int):
                # An integer constraint is bound to a value operand.
                # Check that it is in range.
                assert c >= 0 and c < len(self.ins)
            else:
                assert isinstance(c, RegClass) or isinstance(c, Register)
        return seq

    def ties(self):
        # type: () -> Tuple[Dict[int, int], Dict[int, int]]
        """
        Return two dictionaries representing the tied operands.

        The first maps input number to tied output number, the second maps
        output number to tied input number.
        """
        i2o = dict()  # type: Dict[int, int]
        o2i = dict()  # type: Dict[int, int]
        for o, i in enumerate(self.outs):
            if isinstance(i, int):
                i2o[i] = o
                o2i[o] = i
        return (i2o, o2i)


class Encoding(object):
    """
    Encoding for a concrete instruction.

    An `Encoding` object ties an instruction opcode with concrete type
    variables together with and encoding recipe and encoding bits.

    The concrete instruction can be in three different forms:

    1. A naked opcode: `trap` for non-polymorphic instructions.
    2. With bound type variables: `iadd.i32` for polymorphic instructions.
    3. With operands providing constraints: `icmp.i32(intcc.eq, x, y)`.

    If the instruction is polymorphic, all type variables must be provided.

    :param cpumode: The CPU mode where the encoding is active.
    :param inst: The :py:class:`Instruction` or :py:class:`BoundInstruction`
                 being encoded.
    :param recipe: The :py:class:`EncRecipe` to use.
    :param encbits: Additional encoding bits to be interpreted by `recipe`.
    :param instp: Instruction predicate, or `None`.
    :param isap: ISA predicate, or `None`.
    """

    def __init__(self, cpumode, inst, recipe, encbits, instp=None, isap=None):
        # type: (CPUMode, InstSpec, EncRecipe, int, PredNode, PredNode) -> None # noqa
        assert isinstance(cpumode, CPUMode)
        assert isinstance(recipe, EncRecipe)

        # Check for possible instruction predicates in `inst`.
        if isinstance(inst, Apply):
            instp = And.combine(instp, inst.inst_predicate())
            self.inst = inst.inst
            self.typevars = inst.typevars
        else:
            self.inst, self.typevars = inst.fully_bound()

        self.cpumode = cpumode
        assert self.inst.format == recipe.format, (
                "Format {} must match recipe: {}".format(
                    self.inst.format, recipe.format))

        if self.inst.is_branch:
            assert recipe.branch_range, (
                    'Recipe {} for {} must have a branch_range'
                    .format(recipe, self.inst.name))

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
