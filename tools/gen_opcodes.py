#!/usr/bin/env python3
"""Parse 6502 opcode reference HTML and emit define_opcodes! macro rows."""

from __future__ import annotations

import argparse
import json
import re
from dataclasses import dataclass
from html.parser import HTMLParser
from pathlib import Path
from typing import Dict, List, Optional, Tuple


@dataclass(frozen=True)
class OpcodeEntry:
    code: int
    mnemonic: str
    length: int
    cycles: int
    mode: str
    note: Optional[str] = None


class H2PreCollector(HTMLParser):
    """Collect pairs of <h2> titles and their subsequent <pre> contents."""

    def __init__(self) -> None:
        super().__init__()
        self._recording_h2 = False
        self._current_h2: List[str] = []
        self._last_h2: Optional[str] = None
        self._recording_pre = False
        self._current_pre: List[str] = []
        self.sections: List[Tuple[str, str]] = []

    def handle_starttag(self, tag: str, attrs: List[Tuple[str, Optional[str]]]) -> None:
        tag = tag.lower()
        if tag == "h2":
            self._recording_h2 = True
            self._current_h2 = []
        elif tag == "pre":
            self._recording_pre = True
            self._current_pre = []

    def handle_endtag(self, tag: str) -> None:
        tag = tag.lower()
        if tag == "h2":
            self._recording_h2 = False
            self._last_h2 = "".join(self._current_h2).strip()
        elif tag == "pre":
            if self._recording_pre and self._last_h2:
                pre_text = "".join(self._current_pre)
                self.sections.append((self._last_h2, pre_text))
            self._recording_pre = False
            self._current_pre = []

    def handle_data(self, data: str) -> None:
        if self._recording_h2:
            self._current_h2.append(data)
        elif self._recording_pre:
            self._current_pre.append(data)


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
    "Accumulator": "Accumulator",
    "Implied": "NoneAddressing",
    "Relative": "Relative",
}


def parse_html_sections(html_text: str) -> List[Tuple[str, str]]:
    parser = H2PreCollector()
    parser.feed(html_text)
    return parser.sections


def first_token_mnemonic(title: str) -> Optional[str]:
    first = title.strip().split()
    if not first:
        return None
    token = first[0]
    return token if token.isalpha() and token.isupper() and len(token) <= 3 else None


def normalize_mnemonic(text: str) -> str:
    match = re.search(r"[A-Z]{3}", text)
    if not match:
        raise ValueError(f"Cannot parse mnemonic from: {text}")
    return match.group(0)


def parse_standard_table(mnemonic: str, pre_text: str) -> List[OpcodeEntry]:
    entries: List[OpcodeEntry] = []
    clean_mnemonic = normalize_mnemonic(mnemonic)
    for raw_line in pre_text.splitlines():
        line = raw_line.rstrip()
        if not line or line.lstrip().startswith("+"):
            continue
        if "MODE" in line and "HEX" in line:
            continue
        parts = re.split(r"\s{2,}", line.strip())
        if len(parts) < 5:
            continue
        mode_name, _syntax, hex_code, length_str, cycles_str = parts[:5]
        if not hex_code.startswith("$"):
            continue
        try:
            opcode_value = int(hex_code[1:], 16)
            length = int(length_str)
            extra = cycles_str.endswith("+")
            cycles = int(cycles_str.rstrip("+"))
            mode_variant = MODE_NAME_TO_VARIANT[mode_name]
        except (ValueError, KeyError):
            continue
        note = "+1 if page crossed" if extra else None
        entries.append(
            OpcodeEntry(
                code=opcode_value,
                mnemonic=clean_mnemonic,
                length=length,
                cycles=cycles,
                mode=mode_variant,
                note=note,
            )
        )
    return entries


def parse_branch_block(pre_text: str) -> List[OpcodeEntry]:
    entries: List[OpcodeEntry] = []
    for raw_line in pre_text.splitlines():
        line = raw_line.strip()
        if not line or line.upper().startswith("MNEMONIC"):
            continue
        parts = re.split(r"\s{2,}", line)
        if len(parts) < 2 or not parts[-1].startswith("$"):
            continue
        mnemonic = normalize_mnemonic(parts[0])
        opcode_value = int(parts[-1][1:], 16)
        entries.append(
            OpcodeEntry(
                code=opcode_value,
                mnemonic=mnemonic,
                length=2,
                cycles=2,
                mode="Relative",
                note="+1 if branch taken, +1 more if page crossed",
            )
        )
    return entries


def parse_flag_block(pre_text: str) -> List[OpcodeEntry]:
    entries: List[OpcodeEntry] = []
    for raw_line in pre_text.splitlines():
        line = raw_line.strip()
        if not line or line.upper().startswith("MNEMONIC"):
            continue
        parts = re.split(r"\s{2,}", line)
        if len(parts) < 2 or not parts[-1].startswith("$"):
            continue
        mnemonic = normalize_mnemonic(parts[0])
        opcode_value = int(parts[-1][1:], 16)
        entries.append(
            OpcodeEntry(
                code=opcode_value,
                mnemonic=mnemonic,
                length=1,
                cycles=2,
                mode="NoneAddressing",
            )
        )
    return entries


def parse_register_block(pre_text: str) -> List[OpcodeEntry]:
    entries: List[OpcodeEntry] = []
    for raw_line in pre_text.splitlines():
        line = raw_line.strip()
        if not line or line.upper().startswith("MNEMONIC"):
            continue
        parts = re.split(r"\s{2,}", line)
        if len(parts) < 2 or not parts[-1].startswith("$"):
            continue
        mnemonic = normalize_mnemonic(parts[0])
        opcode_value = int(parts[-1][1:], 16)
        entries.append(
            OpcodeEntry(
                code=opcode_value,
                mnemonic=mnemonic,
                length=1,
                cycles=2,
                mode="NoneAddressing",
            )
        )
    return entries


def parse_stack_block(pre_text: str) -> List[OpcodeEntry]:
    entries: List[OpcodeEntry] = []
    for raw_line in pre_text.splitlines():
        line = raw_line.strip()
        if not line or line.upper().startswith("MNEMONIC"):
            continue
        parts = re.split(r"\s{2,}", line)
        if len(parts) < 3 or not parts[-2].startswith("$"):
            continue
        mnemonic = normalize_mnemonic(parts[0])
        opcode_value = int(parts[-2][1:], 16)
        cycles = int(parts[-1])
        entries.append(
            OpcodeEntry(
                code=opcode_value,
                mnemonic=mnemonic,
                length=1,
                cycles=cycles,
                mode="NoneAddressing",
            )
        )
    return entries


def parse_opcodes(html_text: str) -> Dict[int, OpcodeEntry]:
    opcodes: Dict[int, OpcodeEntry] = {}
    for title, pre in parse_html_sections(html_text):
        header_line = next((line.strip() for line in pre.splitlines() if line.strip()), "")
        title_lower = title.lower()
        mnemonic = first_token_mnemonic(title)
        entries: List[OpcodeEntry] = []
        if mnemonic and "mode" in header_line.lower():
            entries = parse_standard_table(mnemonic, pre)
        elif "branch" in title_lower:
            entries = parse_branch_block(pre)
        elif title_lower.startswith("flag"):
            entries = parse_flag_block(pre)
        elif title_lower.startswith("register"):
            entries = parse_register_block(pre)
        elif title_lower.startswith("stack"):
            entries = parse_stack_block(pre)
        if not entries:
            continue
        for entry in entries:
            if entry.code in opcodes:
                raise ValueError(f"Duplicate opcode 0x{entry.code:02X}")
            opcodes[entry.code] = entry
    return opcodes


def opcodes_to_dict(opcodes: Dict[int, OpcodeEntry]) -> Dict[str, Dict[str, object]]:
    return {
        f"0x{code:02X}": {
            "mnemonic": entry.mnemonic,
            "length": entry.length,
            "cycles": entry.cycles,
            "mode": entry.mode,
            "note": entry.note,
        }
        for code, entry in sorted(opcodes.items())
    }


def format_define_opcodes(opcodes: Dict[int, OpcodeEntry]) -> str:
    lines = ["define_opcodes!("]
    for code in sorted(opcodes):
        entry = opcodes[code]
        comment = f" /*{entry.note}*/" if entry.note else ""
        lines.append(
            f"    0x{code:02x}, \"{entry.mnemonic}\", {entry.length}, {entry.cycles}, {comment} {entry.mode};".replace(
                "  ", " "
            ).replace(" ,", ",")
        )
    lines.append(")")
    return "\n".join(lines)


def format_match_dispatch(opcodes: Dict[int, OpcodeEntry]) -> str:
    """Generate Rust match arms grouped by mnemonic, preserving opcode order."""

    grouped: Dict[str, List[int]] = {}
    for code, entry in sorted(opcodes.items()):
        grouped.setdefault(entry.mnemonic, []).append(code)

    lines: List[str] = ["match code {"]
    for mnemonic, codes in grouped.items():
        arms = " | ".join(f"0x{code:02X}" for code in codes)
        lines.append(
            "    "
            + arms
            + " => {\n        todo!(\""
            + mnemonic
            + "\");\n    },"
        )
    lines.append("    _ => todo!(),")
    lines.append("}")
    return "\n".join(lines)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--html",
        type=Path,
        default=Path(__file__).with_suffix("").parent / "data" / "6502opcodes.html",
        help="Path to the saved 6502 opcode HTML file.",
    )
    parser.add_argument(
        "--format",
        choices=["macro", "json", "match"],
        default="macro",
        help="Select output format (macro, json, or match).",
    )
    args = parser.parse_args()

    html_path = args.html
    html_text = html_path.read_text(encoding="utf-8")
    opcodes = parse_opcodes(html_text)
    print(f"Extracted {len(opcodes)} opcodes")

    if args.format == "json":
        print(json.dumps(opcodes_to_dict(opcodes), indent=2))
    elif args.format == "match":
        print(format_match_dispatch(opcodes))
    else:
        print(format_define_opcodes(opcodes))


if __name__ == "__main__":
    main()