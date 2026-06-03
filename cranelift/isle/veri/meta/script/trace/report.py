#!/usr/bin/env python3

import sys
from collections import Counter, namedtuple

TOP_K = 32

# Trace events.
class EventInstruction(namedtuple("TraceInstruction", ["opcode", "output_types", "input_types", "features"])):
    def is_ctrl(self):
        return self.has_any_feature("terminator", "branch", "call")

    def is_mem(self):
        return self.has_any_feature("load", "store")

    def is_fp(self):
        return self.has_any_type("f32", "f64")

    def has_type(self, ty):
        return ty in self.output_types or ty in self.input_types

    def has_any_type(self, *tys):
        return any(self.has_type(ty) for ty in tys)

    def has_feature(self, feature):
        return (feature in self.features)

    def has_any_feature(self, *features):
        return any(self.has_feature(feature) for feature in features)


class EventRule(namedtuple("TraceRule", ["name", "pos"])):
    pass

# Trace parsing.

def parse_trace(lines):
    trace = []
    for line in lines:
        parts = line.rstrip().split(None, 3)
        if len(parts) == 0 or parts[0] != "TRACE":
            continue
        assert len(parts) == 4
        assert parts[1] == "-"
        typ = parts[2].rstrip(":")
        fields = parts[3].split(",")
        # TRACE - inst: trap
        if typ == "inst":
            assert len(fields) == 4
            trace.append(EventInstruction(
                opcode=fields[0],
                output_types=fields[1].split(":"),
                input_types=fields[2].split(":"),
                features=fields[3].split(":"),
            ))
        # TRACE - rule: ,src/isa/x64/inst.isle line 4101
        elif typ == "rule":
            assert len(fields) == 2
            trace.append(EventRule(
                name=fields[0],
                pos=fields[1],
            ))
        else:
            assert False, f"unknown trace type: {typ}"
    return trace


# Report generation.

def rule_stats(exclude_fp=False, exclude_mem=False, exclude_ctrl=False):
    counts = Counter()
    names = {}

    # Ingest the trace.
    exclude = False
    for event in parse_trace(sys.stdin):
        # Instruction event: starting a new lowering.
        if isinstance(event, EventInstruction):
            # Should we exclude this instruction?
            exclude = False
            if exclude_fp:
                exclude |= event.is_fp()
            if exclude_mem:
                exclude |= event.is_mem()
            if exclude_ctrl:
                exclude |= event.is_ctrl()
            continue

        # Rule event: ISLE rule fired in lowering.
        elif isinstance(event, EventRule):
            if exclude:
                continue
            counts[event.pos] += 1
            names.setdefault(event.pos, event.name)

        else:
            assert False, "unknown trace event"

    # How many uses (times a rule was triggered) were of named rules?
    named_uses = sum(n for (pos, n) in counts.items() if names.get(pos))
    total_uses = sum(counts.values())
    print(f'\nNamed uses: {named_uses}/{total_uses} = '
          f'{named_uses/total_uses:.1%}')

    # How many covered rules (used at least once) were named?
    named_covered = sum(1 for (i, c) in counts.items() if names.get(i))
    total_covered = len(counts)
    print(f'\nNamed covered: {named_covered}/{total_covered} = '
          f'{named_covered/total_covered:.1%}')

    # Print the most frequently triggered rules, for fun.
    print(f'Top {TOP_K} most commonly used rules:')
    for pos, count in counts.most_common(TOP_K):
        print(count, pos, names[pos])



if __name__ == "__main__":
    rule_stats(
        '--no-fp' in sys.argv[1:],
        '--no-mem' in sys.argv[1:],
        '--no-ctrl' in sys.argv[1:],
    )
