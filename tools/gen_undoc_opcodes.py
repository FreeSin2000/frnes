#!/usr/bin/env python3
"""Emit `define_opcodes!` macro rows for undocumented 6502 opcodes."""

from __future__ import annotations

import argparse
import quopri
import re
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable, List, Optional


HEADING_RE = re.compile(r"^([A-Z]{3})\b")

MODE_NAME_TO_VARIANT = {
    "Immediate": "Immediate",
    "Zero Page": "ZeroPage",
    "Zero Page,X": "ZeroPage_X",
    "Zero Page,Y": "ZeroPage_Y",
    "Absolute": "Absolute",
    "Absolute,X": "Absolute_X",
    "Absolute,Y": "Absolute_Y",
    "Indirect": "Indirect",
    "Indirect,X": "Indirect_X",
    "Indirect,Y": "Indirect_Y",
    "Implied": "NoneAddressing",
    "Relative": "Relative",
    "Accumulator": "Accumulator",
}


@dataclass(frozen=True)
class UndocEntry:
    code: int
    mnemonic: str
    length: int
    cycles: int
    mode: str
    comment: Optional[str] = None


def normalize_mode_name(raw: str) -> str:
    text = raw.strip()
    if not text:
        raise ValueError("Empty addressing mode entry")
    replacements = {
        "(Indirect,X)": "Indirect,X",
        "(Indirect),Y": "Indirect,Y",
        "(Indirect,Y)": "Indirect,Y",
        "(Indirect)": "Indirect",
    }
    for needle, repl in replacements.items():
        text = text.replace(needle, repl)
    text = text.replace(" ,", ",").replace(", ", ",")
    text = re.sub(r"\s+", " ", text)
    normalized = text.strip()
    if normalized not in MODE_NAME_TO_VARIANT:
        raise KeyError(f"Unknown addressing mode label: '{raw}' -> '{normalized}'")
    return MODE_NAME_TO_VARIANT[normalized]


def is_heading_line(line: str, next_line: Optional[str]) -> Optional[str]:
    match = HEADING_RE.match(line.strip())
    if not match or next_line is None:
        return None
    next_stripped = next_line.strip()
    if next_stripped and set(next_stripped) <= {"="}:
        return match.group(1)
    return None


def parse_cycles_field(raw: str) -> tuple[int, Optional[str]]:
    field = raw.strip()
    comment: Optional[str] = None
    if "*" in field:
        field = field.replace("*", "").strip()
        comment = "+1 if page crossed"
    if field in {"", "-"}:
        return 0, comment
    return int(field), comment


def parse_table_row(line: str) -> Optional[UndocEntry]:
    if line.strip().startswith("-"):
        return None
    if "|" not in line:
        return None
    parts = [part.strip() for part in line.split("|")]
    if len(parts) < 5:
        return None
    mode_raw, mnemonic_raw, opcode_raw, size_raw, cycles_raw = parts[:5]
    if not opcode_raw.startswith("$"):
        return None
    if mode_raw.startswith("Addressing") or mnemonic_raw.startswith("Mnemonics"):
        return None
    try:
        opcode = int(opcode_raw[1:], 16)
        size = int(size_raw)
    except ValueError:
        return None
    cycles, star_comment = parse_cycles_field(cycles_raw)
    mode = normalize_mode_name(mode_raw)
    mnemonic_token = mnemonic_raw.split()
    if not mnemonic_token:
        return None
    comment = star_comment
    return UndocEntry(
        code=opcode,
        mnemonic=mnemonic_token[0].upper(),
        length=size,
        cycles=cycles,
        mode=mode,
        comment=comment,
    )


def parse_document(text: str) -> List[UndocEntry]:
    lines = text.splitlines()
    entries: List[UndocEntry] = []
    current_heading: Optional[str] = None
    in_table = False
    for idx, line in enumerate(lines):
        next_line = lines[idx + 1] if idx + 1 < len(lines) else None
        heading = is_heading_line(line, next_line)
        if heading:
            current_heading = heading
            in_table = False
            continue
        stripped = line.strip()
        if not stripped:
            in_table = False
            continue
        if stripped.startswith("Addressing") and current_heading:
            in_table = True
            continue
        if not in_table:
            continue
        entry = parse_table_row(line)
        if entry is not None:
            entries.append(entry)
    return entries


def format_entries(entries: Iterable[UndocEntry], wrap: bool) -> str:
    sorted_entries = sorted(entries, key=lambda e: e.code)
    if not sorted_entries:
        return ""
    lines: List[str] = []
    if wrap:
        lines.append("define_opcodes!(")
    indent = "    "
    for entry in sorted_entries:
        line = f"{indent}0x{entry.code:02x}, \"{entry.mnemonic}\", {entry.length}, {entry.cycles}, "
        if entry.comment:
            line += f"/*{entry.comment}*/ "
        line += f"{entry.mode};"
        lines.append(line.rstrip())
    if wrap:
        lines.append(")")
    return "\n".join(lines)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--input",
        type=Path,
        default=Path(__file__).with_suffix("").parent / "data" / "undocumented_opcodes.txt",
        help="Path to 'undocumented_opcodes.txt'.",
    )
    parser.add_argument(
        "--wrap",
        action="store_true",
        help="Wrap the output with define_opcodes!(...) for convenience.",
    )
    args = parser.parse_args()

    raw_bytes = args.input.read_bytes()
    try:
        decoded_bytes = quopri.decodestring(raw_bytes)
    except Exception:
        decoded_bytes = raw_bytes
    text = decoded_bytes.decode("utf-8", errors="replace")
    entries = parse_document(text)
    if not entries:
        raise SystemExit("No opcode entries were parsed from the document.")
    print(format_entries(entries, wrap=args.wrap))


if __name__ == "__main__":
    main()