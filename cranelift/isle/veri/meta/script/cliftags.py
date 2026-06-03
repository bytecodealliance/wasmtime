#!/usr/bin/env python3

import sys
import argparse
import json
import logging

CATEGORIES = {
    # Stack
    "Drop": "stack",
    "Select": "stack",
    "TypedSelect": "stack",

    # Control flow
    "Nop": "control_flow",
    "Unreachable": "control_flow",
    "Block": "control_flow",
    "Loop": "control_flow",
    "If": "control_flow",
    "Else": "control_flow",
    "End": "control_flow",
    "Br": "control_flow",
    "BrIf": "control_flow",
    "BrTable": "control_flow",
    "Return": "control_flow",

    # Calls
    "Call": "calls",
    "CallIndirect": "calls",

    # Memory
    "MemoryGrow": "memory_management",
    "MemorySize": "memory_management",

    # Unary
    "I32Clz": "unary",
    "I64Clz": "unary",
    "I32Ctz": "unary",
    "I64Ctz": "unary",
    "I32Popcnt": "unary",
    "I64Popcnt": "unary",
    "I64ExtendI32S": "unary",
    "I64ExtendI32U": "unary",
    "I32WrapI64": "unary",
    "F32Sqrt": "unary",
    "F64Sqrt": "unary",
    "F32Ceil": "unary",
    "F64Ceil": "unary",
    "F32Floor": "unary",
    "F64Floor": "unary",
    "F32Trunc": "unary",
    "F64Trunc": "unary",
    "F32Nearest": "unary",
    "F64Nearest": "unary",
    "F32Abs": "unary",
    "F64Abs": "unary",
    "F32Neg": "unary",
    "F64Neg": "unary",
    "F64ConvertI64U": "unary",
    "F64ConvertI32U": "unary",
    "F64ConvertI64S": "unary",
    "F64ConvertI32S": "unary",
    "F32ConvertI64S": "unary",
    "F32ConvertI32S": "unary",
    "F32ConvertI64U": "unary",
    "F32ConvertI32U": "unary",
    "F64PromoteF32": "unary",
    "F32DemoteF64": "unary",
    "I64TruncF64S": "unary",
    "I64TruncF32S": "unary",
    "I32TruncF64S": "unary",
    "I32TruncF32S": "unary",
    "I64TruncF64U": "unary",
    "I64TruncF32U": "unary",
    "I32TruncF64U": "unary",
    "I32TruncF32U": "unary",
    "I64TruncSatF64S": "unary",
    "I64TruncSatF32S": "unary",
    "I32TruncSatF64S": "unary",
    "I32TruncSatF32S": "unary",
    "I64TruncSatF64U": "unary",
    "I64TruncSatF32U": "unary",
    "I32TruncSatF64U": "unary",
    "I32TruncSatF32U": "unary",
    "F32ReinterpretI32": "unary",
    "F64ReinterpretI64": "unary",
    "I32ReinterpretF32": "unary",
    "I64ReinterpretF64": "unary",
    "I32Extend8S": "unary",
    "I32Extend16S": "unary",
    "I64Extend8S": "unary",
    "I64Extend16S": "unary",
    "I64Extend32S": "unary",

    # Binary
    "I32Add": "binary",
    "I64Add": "binary",
    "I32And": "binary",
    "I64And": "binary",
    "I32Or": "binary",
    "I64Or": "binary",
    "I32Xor": "binary",
    "I64Xor": "binary",
    "I32Shl": "binary",
    "I64Shl": "binary",
    "I32ShrS": "binary",
    "I64ShrS": "binary",
    "I32ShrU": "binary",
    "I64ShrU": "binary",
    "I32Rotl": "binary",
    "I64Rotl": "binary",
    "I32Rotr": "binary",
    "I64Rotr": "binary",
    "F32Add": "binary",
    "F64Add": "binary",
    "I32Sub": "binary",
    "I64Sub": "binary",
    "F32Sub": "binary",
    "F64Sub": "binary",
    "I32Mul": "binary",
    "I64Mul": "binary",
    "F32Mul": "binary",
    "F64Mul": "binary",
    "F32Div": "binary",
    "F64Div": "binary",
    "I32DivS": "binary",
    "I64DivS": "binary",
    "I32DivU": "binary",
    "I64DivU": "binary",
    "I32RemS": "binary",
    "I64RemS": "binary",
    "I32RemU": "binary",
    "I64RemU": "binary",
    "F32Min": "binary",
    "F64Min": "binary",
    "F32Max": "binary",
    "F64Max": "binary",
    "F32Copysign": "binary",
    "F64Copysign": "binary",

    # Comparisons
    "I32LtS": "comparison",
    "I64LtS": "comparison",
    "I32LtU": "comparison",
    "I64LtU": "comparison",
    "I32LeS": "comparison",
    "I64LeS": "comparison",
    "I32LeU": "comparison",
    "I64LeU": "comparison",
    "I32GtS": "comparison",
    "I64GtS": "comparison",
    "I32GtU": "comparison",
    "I64GtU": "comparison",
    "I32GeS": "comparison",
    "I64GeS": "comparison",
    "I32GeU": "comparison",
    "I64GeU": "comparison",
    "I32Eqz": "comparison",
    "I64Eqz": "comparison",
    "I32Eq": "comparison",
    "I64Eq": "comparison",
    "F32Eq": "comparison",
    "F64Eq": "comparison",
    "I32Ne": "comparison",
    "I64Ne": "comparison",
    "F32Ne": "comparison",
    "F64Ne": "comparison",
    "F32Gt": "comparison",
    "F64Gt": "comparison",
    "F32Ge": "comparison",
    "F64Ge": "comparison",
    "F32Lt": "comparison",
    "F64Lt": "comparison",
    "F32Le": "comparison",
    "F64Le": "comparison",
}

def op_category(op):
    if op.startswith("Local"):
        return "locals"
    if op.startswith("Global"):
        return "globals"
    if "Load" in op:
        return "loads"
    if "Store" in op:
        return "stores"
    if op.endswith("Const"):
        return "const"
    return CATEGORIES.get(op, None)


ALLOW_NO_INSTRUCTIONS = {
    "Drop",
}

def build_clif_tags(data, in_scope_proposals, ignore_categories=None):
    ignore_categories = ignore_categories or set()
    op_proposal = {op["op"]: op["proposal"] for op in data["operators"]}

    clif_tags = dict()
    for translation in data["translations"]:
        for op in translation["operators"]:
            # Check proposal
            proposal = op_proposal[op]
            if proposal not in in_scope_proposals:
                logging.debug(f"{op} proposal not in scope")
                continue
            category = op_category(op)

            # Check category
            assert category is not None, f"no category for {op}"
            if category in ignore_categories:
                logging.debug(f"{op} category not in scope")
                continue

            # Expect corresponding CLIF instructions
            instructions = translation["instructions"]
            assert op in ALLOW_NO_INSTRUCTIONS or len(instructions) > 0, f"no instructions for {op}"
            for instruction in instructions:
                tags = clif_tags.setdefault(instruction, set())
                tags.add(f"wasm_proposal_{proposal}")
                tags.add(f"wasm_category_{category}")

    return {inst: list(sorted(tags)) for inst, tags in clif_tags.items()}


def main(args):
    # Options.
    parser = argparse.ArgumentParser(description='Derive WASM to CLIF mapping')
    parser.add_argument('--data', required=True, type=argparse.FileType('r'), help="wasm to clif data file")
    parser.add_argument('--output', type=argparse.FileType('w'), default=sys.stdout)
    parser.add_argument('--log-level', default="info")
    opts = parser.parse_args(args)
    logging.basicConfig(level=opts.log_level.upper())

    # Read WASM to CLIF data.
    data = json.load(opts.data)

    # Build tags.
    in_scope_proposals = set(["mvp"])
    ignore_categories = set([
        "locals",
        "globals",
        "control_flow",
        "calls",
        "memory_management",
    ])
    clif_tags = build_clif_tags(data, in_scope_proposals, ignore_categories)

    # Write
    json.dump(clif_tags, opts.output, indent="\t")
    opts.output.write("\n")


if __name__ == "__main__":
    main(sys.argv[1:])
