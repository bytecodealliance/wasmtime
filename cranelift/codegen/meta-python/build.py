# Second-level build script.
#
# This script is run from cranelift-codegen/build.rs to generate Rust files.

from __future__ import absolute_import
import argparse
import isa
import gen_build_deps
import gen_encoding
import gen_binemit

try:
    from typing import List, Set  # noqa
    from cdsl.isa import TargetISA  # noqa
    from cdsl.instructions import InstructionGroup  # noqa
except ImportError:
    pass


def number_all_instructions(isas):
    # type: (List[TargetISA]) -> None
    seen = set()  # type: Set[InstructionGroup]
    num_inst = 1
    for target_isa in isas:
        for g in target_isa.instruction_groups:
            if g not in seen:
                for i in g.instructions:
                    i.number = num_inst
                    num_inst += 1
                seen.add(g)


def main():
    # type: () -> None
    parser = argparse.ArgumentParser(
            description='Generate sources for Cranelift.')
    parser.add_argument('--out-dir', help='set output directory')

    args = parser.parse_args()
    out_dir = args.out_dir

    isas = isa.all_isas()
    number_all_instructions(isas)

    gen_encoding.generate(isas, out_dir)
    gen_binemit.generate(isas, out_dir)
    gen_build_deps.generate()


if __name__ == "__main__":
    main()
