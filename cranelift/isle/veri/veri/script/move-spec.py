#!/usr/bin/env python3
"""
Extract top-level Lisp-style forms whose head symbol is in EXTRACT_TOP_LEVEL_TERMS,
leaving the original file unchanged and writing the matching forms to a side file.

For example, if EXTRACT_TOP_LEVEL_TERMS includes "spec", then top-level forms like

    (spec ...)

will be copied into a sibling output file.

Behavior:
- does not modify the input file
- extracts complete top-level forms only
- preserves the exact original text of each extracted form
- ignores nested matching forms
- handles nested parentheses, strings, and `;` line comments

Default output naming:
    input.isle -> input.extracted.isle

Usage:
    python extract_top_level_forms.py file.isle
    python extract_top_level_forms.py a.isle b.isle
    python extract_top_level_forms.py --suffix .matched.isle file.isle
    python extract_top_level_forms.py --output-dir extracted/ *.isle
"""

from __future__ import annotations

import argparse
import pathlib
from dataclasses import dataclass

# Edit this list.
EXTRACT_TOP_LEVEL_TERMS = [
    "spec",
    "model",
    "attr",
    "instantiate",
    "form",
    "state",
    "macro"
    # "rule",
]


@dataclass
class Span:
    start: int
    end: int


def is_symbol_char(ch: str) -> bool:
    return not ch.isspace() and ch not in '();"'


def skip_string(s: str, i: int) -> int:
    # s[i] == '"'
    i += 1
    while i < len(s):
        if s[i] == "\\":
            i += 2
        elif s[i] == '"':
            return i + 1
        else:
            i += 1
    return i


def skip_line_comment(s: str, i: int) -> int:
    # s[i] == ';'
    while i < len(s) and s[i] != "\n":
        i += 1
    return i


def read_head_symbol(s: str, i: int) -> tuple[str | None, int]:
    """
    Read the head symbol of a list starting just after '(' at s[i].
    Returns (symbol_or_none, new_index).
    """
    n = len(s)

    while i < n:
        ch = s[i]
        if ch.isspace():
            i += 1
        elif ch == ";":
            i = skip_line_comment(s, i)
        elif ch == '"':
            return None, i
        else:
            break

    start = i
    while i < n and is_symbol_char(s[i]):
        i += 1

    if i == start:
        return None, i
    return s[start:i], i


def skip_balanced_list(s: str, i: int) -> int:
    """
    Skip a balanced parenthesized form starting at s[i] == '('.
    Returns the index just after the closing ')', or len(s) if unmatched.
    """
    assert s[i] == "("
    depth = 0
    n = len(s)

    while i < n:
        ch = s[i]
        if ch == '"':
            i = skip_string(s, i)
        elif ch == ";":
            i = skip_line_comment(s, i)
        elif ch == "(":
            depth += 1
            i += 1
        elif ch == ")":
            depth -= 1
            i += 1
            if depth == 0:
                return i
        else:
            i += 1

    return i


def find_top_level_spans(s: str, terms_to_extract: set[str]) -> list[Span]:
    spans: list[Span] = []
    i = 0
    n = len(s)
    depth = 0

    while i < n:
        ch = s[i]

        if ch == '"':
            i = skip_string(s, i)
            continue

        if ch == ";":
            i = skip_line_comment(s, i)
            continue

        if ch == "(":
            if depth == 0:
                form_start = i
                head, _after_head = read_head_symbol(s, i + 1)
                form_end = skip_balanced_list(s, i)
                if head in terms_to_extract:
                    spans.append(Span(form_start, form_end))
                i = form_end
                continue
            else:
                depth += 1
                i += 1
                continue

        if ch == ")":
            depth = max(0, depth - 1)
            i += 1
            continue

        i += 1

    return spans


def extract_spans(s: str, spans: list[Span]) -> str:
    pieces = [s[sp.start:sp.end] for sp in spans]
    if not pieces:
        return ""
    return "\n\n".join(piece.rstrip() for piece in pieces) + "\n"


def default_output_path(input_path: pathlib.Path, suffix: str) -> pathlib.Path:
    if input_path.suffix:
        return input_path.with_name(f"{input_path.stem}{suffix}")
    return input_path.with_name(input_path.name + suffix)


def process_file(path: pathlib.Path, output_path: pathlib.Path) -> int:
    original = path.read_text(encoding="utf-8")
    terms_to_extract = set(EXTRACT_TOP_LEVEL_TERMS)
    spans = find_top_level_spans(original, terms_to_extract)
    extracted = extract_spans(original, spans)
    output_path.write_text(extracted, encoding="utf-8")
    print(f"{path} -> {output_path}: wrote {len(spans)} matching top-level form(s)")
    return len(spans)


def parse_args() -> argparse.Namespace:
    p = argparse.ArgumentParser()
    p.add_argument("files", nargs="+", help="Files to process")
    p.add_argument(
        "--suffix",
        default=".extracted.isle",
        help="Suffix for side output files (default: .extracted.isle)",
    )
    p.add_argument(
        "--output-dir",
        default=None,
        help="Optional directory for extracted files",
    )
    return p.parse_args()


if __name__ == "__main__":
    args = parse_args()

    output_dir = pathlib.Path(args.output_dir) if args.output_dir else None
    if output_dir is not None:
        output_dir.mkdir(parents=True, exist_ok=True)

    for name in args.files:
        input_path = pathlib.Path(name)
        if output_dir is None:
            output_path = default_output_path(input_path, args.suffix)
        else:
            output_path = output_dir / default_output_path(input_path, args.suffix).name
        process_file(input_path, output_path)