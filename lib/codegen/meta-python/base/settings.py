"""
Cranelift shared settings.

This module defines settings relevant for all code generators.
"""
from __future__ import absolute_import
from cdsl.settings import SettingGroup, BoolSetting, EnumSetting, NumSetting

group = SettingGroup('shared')

opt_level = EnumSetting(
        """
        Optimization level:

        - default: Very profitable optimizations enabled, none slow.
        - best: Enable all optimizations
        - fastest: Optimize for compile time by disabling most optimizations.
        """,
        'default', 'best', 'fastest')

enable_verifier = BoolSetting(
        """
        Run the Cranelift IR verifier at strategic times during compilation.

        This makes compilation slower but catches many bugs. The verifier is
        disabled by default, except when reading Cranelift IR from a text file.
        """,
        default=True)

# Note that Cranelift doesn't currently need an is_pie flag, because PIE is
# just PIC where symbols can't be pre-empted, which can be expressed with the
# `colocated` flag on external functions and global values.
is_pic = BoolSetting("Enable Position-Independent Code generation")

colocated_libcalls = BoolSetting(
        """
        Use colocated libcalls.

        Generate code that assumes that libcalls can be declared "colocated",
        meaning they will be defined along with the current function, such that
        they can use more efficient addressing.
        """)

avoid_div_traps = BoolSetting(
        """
        Generate explicit checks around native division instructions to avoid
        their trapping.

        This is primarily used by SpiderMonkey which doesn't install a signal
        handler for SIGFPE, but expects a SIGILL trap for division by zero.

        On ISAs like ARM where the native division instructions don't trap,
        this setting has no effect - explicit checks are always inserted.
        """)

enable_float = BoolSetting(
        """
        Enable the use of floating-point instructions

        Disabling use of floating-point instructions is not yet implemented.
        """,
        default=True)

enable_nan_canonicalization = BoolSetting(
        """
        Enable NaN canonicalization

        This replaces NaNs with a single canonical value, for users requiring
        entirely deterministic WebAssembly computation. This is not required
        by the WebAssembly spec, so it is not enabled by default.
        """,
        default=False)

enable_simd = BoolSetting(
        """Enable the use of SIMD instructions.""",
        default=True)

enable_atomics = BoolSetting(
        """Enable the use of atomic instructions""",
        default=True)

#
# Settings specific to the `baldrdash` calling convention.
#
baldrdash_prologue_words = NumSetting(
        """
        Number of pointer-sized words pushed by the baldrdash prologue.

        Functions with the `baldrdash` calling convention don't generate their
        own prologue and epilogue. They depend on externally generated code
        that pushes a fixed number of words in the prologue and restores them
        in the epilogue.

        This setting configures the number of pointer-sized words pushed on the
        stack when the Cranelift-generated code is entered. This includes the
        pushed return address on x86.
        """)

#
# BaldrMonkey requires that not-yet-relocated function addresses be encoded
# as all-ones bitpatterns.
#
allones_funcaddrs = BoolSetting(
        """
        Emit not-yet-relocated function addresses as all-ones bit patterns.
        """)

#
# Stack probing options.
#
probestack_enabled = BoolSetting(
        """
        Enable the use of stack probes, for calling conventions which support
        this functionality.
        """,
        default=True)

probestack_func_adjusts_sp = BoolSetting(
        """
        Set this to true of the stack probe function modifies the stack pointer
        itself.
        """)

probestack_size_log2 = NumSetting(
        """
        The log2 of the size of the stack guard region.

        Stack frames larger than this size will have stack overflow checked
        by calling the probestack function.

        The default is 12, which translates to a size of 4096.
        """,
        default=12)

#
# Jump table options.
#
jump_tables_enabled = BoolSetting(
        """
        Enable the use of jump tables in generated machine code.
        """,
        default=True)

group.close(globals())
