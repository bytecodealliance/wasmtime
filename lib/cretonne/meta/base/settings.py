"""
Cretonne shared settings.

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
        Run the Cretonne IL verifier at strategic times during compilation.

        This makes compilation slower but catches many bugs. The verifier is
        disabled by default, except when reading Cretonne IL from a text file.
        """,
        default=True)

is_64bit = BoolSetting("Enable 64-bit code generation")

is_pic = BoolSetting("Enable Position-Independent Code generation")

return_at_end = BoolSetting(
        """
        Generate functions with at most a single return instruction at the
        end of the function.

        This guarantees that functions do not have any internal return
        instructions. Either they never return, or they have a single return
        instruction at the end.
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

is_compressed = BoolSetting("Enable compressed instructions")

enable_float = BoolSetting(
        """Enable the use of floating-point instructions""",
        default=True)

enable_simd = BoolSetting(
        """Enable the use of SIMD instructions.""",
        default=True)

enable_atomics = BoolSetting(
        """Enable the use of atomic instructions""",
        default=True)

#
# Settings specific to the `spiderwasm` calling convention.
#
spiderwasm_prologue_words = NumSetting(
        """
        Number of pointer-sized words pushed by the spiderwasm prologue.

        Functions with the `spiderwasm` calling convention don't generate their
        own prologue and epilogue. They depend on externally generated code
        that pushes a fixed number of words in the prologue and restores them
        in the epilogue.

        This setting configures the number of pointer-sized words pushed on the
        stack when the Cretonne-generated code is entered. This includes the
        pushed return address on Intel ISAs.
        """)

#
# BaldrMonkey requires that not-yet-relocated function addresses be encoded
# as all-ones bitpatterns.
#
allones_funcaddrs = BoolSetting(
        """
        Emit not-yet-relocated function addresses as all-ones bit patterns.
        """)

group.close(globals())
