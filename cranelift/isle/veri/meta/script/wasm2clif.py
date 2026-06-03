#!/usr/bin/env python3

import sys
import pathlib
import itertools
import logging
import collections
import re
import csv
import argparse
import json


INDENT = 4*" "

Operator = collections.namedtuple("Operator", ["op", "proposal"])

def read_wasm_operators_csv(f):
    r = csv.reader(f)
    ops = []
    for row in r:
        assert len(row) == 2
        op = Operator(row[0], row[1])
        ops.append(op)
    return ops


Arm = collections.namedtuple("Arm", ["pattern", "body"])

class Parser:
    def __init__(self, lines):
        self.lines = lines

    def parse(self):
        # Find function start
        self.skip_to("pub fn translate_operator")

        # Find switch start
        self.skip_to(f"{INDENT}match op {{")

        # Parse arms.
        translations = []
        while True:
            arm = self.parse_arm()
            if arm is None:
                break

            logging.debug(f"pattern: {arm.pattern}")
            assert len(arm.pattern) > 0
            logging.debug(f"body: {arm.body}")
            assert len(arm.body) > 0

            translation = derive_arm_translation(arm)
            translations.append(translation)

        return translations

    @staticmethod
    def is_comment(line):
        trim = line.lstrip()
        return trim.startswith("/*") or trim.startswith("//") or trim.startswith("*")

    def parse_arm(self):
        # Collect pattern
        pattern = ""
        for line in self.lines:
            single_line = self.parse_arm_single_line(line)
            if single_line is not None:
                return single_line
            if line.startswith(f"{INDENT}}};"):
                return None
            pattern += line
            if line.endswith("=> {\n"):
                break

        # Collect body
        body = ""
        for line in self.lines:
            body += line
            if line.startswith(f"{INDENT}{INDENT}}}"):
                break

        return Arm(pattern, body)

    @staticmethod
    def parse_arm_single_line(line):
        if "Operator::" not in line:
            return None
        if " => " not in line:
            return None
        if not line.endswith(",\n"):
            return None
        parts = line.split(" => ")
        assert len(parts) == 2
        return Arm(parts[0], parts[1])

    def skip_to(self, target):
        for line in self.lines:
            if line.startswith(target):
                logging.debug(f"found target: {target}")
                return
        raise ValueError(f"could not find target: {target}")


Translation = collections.namedtuple("Translation", ["operators", "instructions"])

def derive_arm_translation(arm):
    # Parse operators
    operators = re.findall(r'Operator::(\w+)', arm.pattern, flags=re.MULTILINE)

    # Parse instructions.
    instructions = re.findall(r'builder\.ins\(\)\.(\w+)\(', arm.body, flags=re.MULTILINE)

    # Parse opcodes.
    opcodes = re.findall(r'ir::Opcode::(\w+)', arm.body, flags=re.MULTILINE)
    instructions.extend(opcode.lower() for opcode in opcodes)

    # Special cases
    if "translate_icmp(" in arm.body:
        instructions.append("icmp")
        instructions.append("uextend")
    if "translate_fcmp(" in arm.body:
        instructions.append("fcmp")
        instructions.append("uextend")
    if "translate_store(" in arm.body or "translate_load(" in arm.body:
        # prepare_addr
        instructions.append("uadd_overflow_trap")
        # bounds_checks::bounds_check_and_compute_addr
        instructions.append("icmp")
        instructions.append("isub")
        instructions.append("iconst")

    # Deduplicate and sort
    instructions = sorted(list(set(instructions)))

    return Translation(operators, instructions)


def build_wasm_to_clif(ops, translations):
    data = {
        "operators": list(op._asdict() for op in ops),
        "translations": list(t._asdict() for t in translations),
    }
    return data


def code_translator_path():
    self_dir = pathlib.Path(__file__).parent.resolve()
    rel_path = "../../../../wasm/src/code_translator.rs"
    return self_dir.joinpath(rel_path)


def main(args):
    # Options.
    parser = argparse.ArgumentParser(description='Derive WASM to CLIF mapping')
    parser.add_argument('--wasm-ops', required=True, type=argparse.FileType('r'), help="wasm operators csv file")
    parser.add_argument('--output', type=argparse.FileType('w'), default=sys.stdout)
    parser.add_argument('--log-level', default="info")
    opts = parser.parse_args(args)
    logging.basicConfig(level=opts.log_level.upper())

    # Read WASM operators.
    ops = read_wasm_operators_csv(opts.wasm_ops)

    # Parse code translator.
    path = code_translator_path()
    with open(path) as lines:
        parser = Parser(lines)
        translations = parser.parse()

    # Build and write dataset.
    data = build_wasm_to_clif(ops, translations)
    json.dump(data, opts.output, indent="\t")
    opts.output.write("\n")


if __name__ == "__main__":
    main(sys.argv[1:])
