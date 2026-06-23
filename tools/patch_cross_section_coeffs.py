#!/usr/bin/env python3
"""Insert coeff_contraction/coeff_expansion into CrossSection literals missing them."""

from __future__ import annotations

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
INSERT = "coeff_contraction: None,\n            coeff_expansion: None,"

# Match CrossSection { ... } blocks without coeff_contraction (non-greedy, brace-aware-ish).
BLOCK = re.compile(
    r"CrossSection\s*\{(?P<body>[^{}]*(?:\{[^{}]*\}[^{}]*)*)\}",
    re.MULTILINE | re.DOTALL,
)


def patch_block(body: str) -> str:
    if "coeff_contraction" in body:
        return body
    lines = body.split("\n")
    out: list[str] = []
    inserted = False
    for line in lines:
        out.append(line)
        stripped = line.strip()
        if not inserted and stripped in {
            "guide_banks: None,",
            "ineffective_flow_areas: None,",
            "blocked_obstructions: None,",
            "is_overbank: None,",
        }:
            indent = line[: len(line) - len(line.lstrip())]
            out.append(f"{indent}coeff_contraction: None,")
            out.append(f"{indent}coeff_expansion: None,")
            inserted = True
    if not inserted:
        # Append before closing (last non-empty line in body)
        indent = "            "
        for line in reversed(lines):
            if line.strip():
                indent = line[: len(line) - len(line.lstrip())]
                break
        if out and out[-1].strip() == "":
            out.insert(-1, f"{indent}coeff_expansion: None,")
            out.insert(-1, f"{indent}coeff_contraction: None,")
        else:
            out.append(f"{indent}coeff_contraction: None,")
            out.append(f"{indent}coeff_expansion: None,")
    return "\n".join(out)


def patch_file(path: Path) -> bool:
    text = path.read_text(encoding="utf-8")
    if "CrossSection {" not in text:
        return False

    changed = False

    def repl(match: re.Match[str]) -> str:
        nonlocal changed
        body = match.group("body")
        if "coeff_contraction" in body:
            return match.group(0)
        changed = True
        return "CrossSection {" + patch_block(body) + "}"

    new_text = BLOCK.sub(repl, text)
    if changed:
        path.write_text(new_text, encoding="utf-8")
    return changed


def main() -> None:
    changed_files: list[str] = []
    for path in sorted(ROOT.rglob("*.rs")):
        if "target" in path.parts:
            continue
        if patch_file(path):
            changed_files.append(str(path.relative_to(ROOT)))
    print(f"Patched {len(changed_files)} files:")
    for name in changed_files:
        print(name)


if __name__ == "__main__":
    main()
