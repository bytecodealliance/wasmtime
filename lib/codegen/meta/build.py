# Second-level build script.
#
# This script is run from lib/codegen/build.rs to generate Rust files.

from __future__ import absolute_import
import argparse
import isa
import gen_types
import gen_instr
import gen_settings
import gen_build_deps
import gen_encoding
import gen_legalizer
import gen_registers
import gen_binemit


def main():
    # type: () -> None
    parser = argparse.ArgumentParser(
            description='Generate sources for Cranelift.')
    parser.add_argument('--out-dir', help='set output directory')

    args = parser.parse_args()
    out_dir = args.out_dir

    isas = isa.all_isas()

    gen_types.generate(out_dir)
    gen_instr.generate(isas, out_dir)
    gen_settings.generate(isas, out_dir)
    gen_encoding.generate(isas, out_dir)
    gen_legalizer.generate(isas, out_dir)
    gen_registers.generate(isas, out_dir)
    gen_binemit.generate(isas, out_dir)
    gen_build_deps.generate()


if __name__ == "__main__":
    main()
