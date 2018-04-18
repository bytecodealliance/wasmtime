"""Defining instruction set architectures."""
from __future__ import absolute_import
from collections import OrderedDict
from .predicates import And, TypePredicate
from .registers import RegClass, Register, Stack
from .ast import Apply
from .types import ValueType
from .instructions import InstructionGroup

# The typing module is only required by mypy, and we don't use these imports
# outside type comments.
try:
    from typing import Tuple, Union, Any, Iterable, Sequence, List, Set, Dict, TYPE_CHECKING  # noqa
    if TYPE_CHECKING:
        from .instructions import MaybeBoundInst, InstructionFormat  # noqa
        from .predicates import PredNode, PredKey  # noqa
        from .settings import SettingGroup  # noqa
        from .registers import RegBank  # noqa
        from .xform import XFormGroup  # noqa
        OperandConstraint = Union[RegClass, Register, int, Stack]
        ConstraintSeq = Union[OperandConstraint, Tuple[OperandConstraint, ...]]
        # Instruction specification for encodings. Allows for predicated
        # instructions.
        InstSpec = Union[MaybeBoundInst, Apply]
        BranchRange = Sequence[int]
        # A recipe predicate consisting of an ISA predicate and an instruction
        # predicate.
        RecipePred = Tuple[PredNode, PredNode]
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
        self.legalize_codes = OrderedDict()  # type: OrderedDict[XFormGroup, int]  # noqa
        # Unique copies of all predicates.
        self._predicates = dict()  # type: Dict[PredKey, PredNode]

        assert InstructionGroup._current is None,\
            "InstructionGroup {} is still open"\
            .format(InstructionGroup._current.name)

    def __str__(self):
        # type: () -> str
        return self.name

    def finish(self):
        # type: () -> TargetISA
        """
        Finish the definition of a target ISA after adding all CPU modes and
        settings.

        This computes some derived properties that are used in multiple
        places.

        :returns self:
        """
        self._collect_encoding_recipes()
        self._collect_predicates()
        self._collect_regclasses()
        self._collect_legalize_codes()
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
                    assert recipe.number is None
                    recipe.number = len(rcps)
                    rcps.add(recipe)
                    self.all_recipes.append(recipe)
                    # Make sure ISA predicates are registered.
                    if recipe.isap:
                        recipe.isap = self.unique_pred(recipe.isap)
                        self.settings.number_predicate(recipe.isap)
                    recipe.instp = self.unique_pred(recipe.instp)

    def _collect_predicates(self):
        # type: () -> None
        """
        Collect and number all predicates in use.

        Ensures that all ISA predicates have an assigned bit number in
        `self.settings`.
        """
        self.instp_number = OrderedDict()  # type: OrderedDict[PredNode, int]
        for cpumode in self.cpumodes:
            for enc in cpumode.encodings:
                instp = enc.instp
                if instp and instp not in self.instp_number:
                    # assign predicate number starting from 0.
                    n = len(self.instp_number)
                    self.instp_number[instp] = n

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
            # Always get the pressure tracking classes in first.
            if bank.pressure_tracking:
                self.regclasses.extend(bank.toprcs)

        # The limit on the number of top-level register classes can be raised.
        # This should be coordinated with the `MAX_TRACKED_TOPRCS` constant in
        # `isa/registers.rs`.
        assert len(self.regclasses) <= 4, "Too many top-level register classes"

        # Get the remaining top-level register classes which may exceed
        # `MAX_TRACKED_TOPRCS`.
        for bank in self.regbanks:
            if not bank.pressure_tracking:
                self.regclasses.extend(bank.toprcs)

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

    def _collect_legalize_codes(self):
        # type: () -> None
        """
        Make sure all legalization transforms have been assigned a code.
        """
        for cpumode in self.cpumodes:
            self.legalize_code(cpumode.default_legalize)
            for x in cpumode.type_legalize.values():
                self.legalize_code(x)

    def legalize_code(self, xgrp):
        # type: (XFormGroup) -> int
        """
        Get the legalization code for the transform group `xgrp`. Assign one if
        necessary.

        Each target ISA has its own list of legalization actions with
        associated legalize codes that appear in the encoding tables.

        This method is used to maintain the registry of legalization actions
        and their table codes.
        """
        if xgrp in self.legalize_codes:
            code = self.legalize_codes[xgrp]
        else:
            code = len(self.legalize_codes)
            self.legalize_codes[xgrp] = code
        return code

    def unique_pred(self, pred):
        # type: (PredNode) -> PredNode
        """
        Get a unique predicate that is equivalent to `pred`.
        """
        if pred is None:
            return pred
        # TODO: We could actually perform some algebraic simplifications. It's
        # not clear if it is worthwhile.
        k = pred.predicate_key()
        if k in self._predicates:
            return self._predicates[k]
        self._predicates[k] = pred
        return pred


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

        # Tables for configuring legalization actions when no valid encoding
        # exists for an instruction.
        self.default_legalize = None  # type: XFormGroup
        self.type_legalize = OrderedDict()  # type: OrderedDict[ValueType, XFormGroup]  # noqa

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

    def legalize_type(self, default=None, **kwargs):
        # type: (XFormGroup, **XFormGroup) -> None
        """
        Configure the legalization action per controlling type variable.

        Instructions that have a controlling type variable mentioned in one of
        the arguments will be legalized according to the action specified here
        instead of  using the `legalize_default` action.

        The keyword arguments are value type names:

            mode.legalize_type(i8=widen, i16=widen, i32=expand)

        The `default` argument specifies the action to take for controlling
        type variables that don't have an explicitly configured action.
        """
        if default is not None:
            self.default_legalize = default

        for name, xgrp in kwargs.items():
            ty = ValueType.by_name(name)
            self.type_legalize[ty] = xgrp

    def legalize_monomorphic(self, xgrp):
        # type: (XFormGroup) -> None
        """
        Configure the legalization action to take for monomorphic instructions
        which don't have a controlling type variable.

        See also `legalize_type()` for polymorphic instructions.
        """
        self.type_legalize[None] = xgrp

    def get_legalize_action(self, ty):
        # type: (ValueType) -> XFormGroup
        """
        Get the legalization action to use for `ty`.
        """
        return self.type_legalize.get(ty, self.default_legalize)


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
    - A `Stack` specifying a value in a stack slot.

    The `branch_range` argument must be provided for recipes that can encode
    branch instructions. It is an `(origin, bits)` tuple describing the exact
    range that can be encoded in a branch instruction.

    For ISAs that use CPU flags in `iflags` and `fflags` value types, the
    `clobbers_flags` is used to indicate instruction encodings that clobbers
    the CPU flags, so they can't be used where a flag value is live.

    :param name: Short mnemonic name for this recipe.
    :param format: All encoded instructions must have this
            :py:class:`InstructionFormat`.
    :param size: Number of bytes in the binary encoded instruction.
    :param ins: Tuple of register constraints for value operands.
    :param outs: Tuple of register constraints for results.
    :param branch_range: `(origin, bits)` range for branches.
    :param clobbers_flags: This instruction clobbers `iflags` and `fflags`.
    :param instp: Instruction predicate.
    :param isap: ISA predicate.
    :param emit: Rust code for binary emission.
    """

    def __init__(
            self,
            name,                 # type: str
            format,               # type: InstructionFormat
            size,                 # type: int
            ins,                  # type: ConstraintSeq
            outs,                 # type: ConstraintSeq
            branch_range=None,    # type: BranchRange
            clobbers_flags=True,  # type: bool
            instp=None,           # type: PredNode
            isap=None,            # type: PredNode
            emit=None             # type: str
            ):
        # type: (...) -> None
        self.name = name
        self.format = format
        assert size >= 0
        self.size = size
        self.branch_range = branch_range
        self.clobbers_flags = clobbers_flags
        self.instp = instp
        self.isap = isap
        self.emit = emit
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
                assert (isinstance(c, RegClass)
                        or isinstance(c, Register)
                        or isinstance(c, Stack))
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

    def fixed_ops(self):
        # type: () -> Tuple[Set[Register], Set[Register]]
        """
        Return two sets of registers representing the fixed input and output
        operands.
        """
        i = set(r for r in self.ins if isinstance(r, Register))
        o = set(r for r in self.outs if isinstance(r, Register))
        return (i, o)

    def recipe_pred(self):
        # type: () -> RecipePred
        """
        Get the combined recipe predicate which includes both the ISA predicate
        and the instruction predicate.

        Return `None` if this recipe has neither predicate.
        """
        if self.isap is None and self.instp is None:
            return None
        else:
            return (self.isap, self.instp)


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

            # Add secondary type variables to the instruction predicate.
            # This is already included by Apply.inst_predicate() above.
            if len(self.typevars) > 1:
                for tv, vt in zip(self.inst.other_typevars, self.typevars[1:]):
                    # A None tv is an 'any' wild card: `ishl.i32.any`.
                    if vt is None:
                        continue
                    typred = TypePredicate.typevar_check(self.inst, tv, vt)
                    instp = And.combine(instp, typred)

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

        # Record specific predicates. Note that the recipe also has predicates.
        self.instp = self.cpumode.isa.unique_pred(instp)
        self.isap = self.cpumode.isa.unique_pred(isap)

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
