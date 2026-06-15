#!/usr/bin/env python3
"""Build reach_mild HEC-RAS bundle from ConSpan open-channel cross sections (no culvert)."""

from __future__ import annotations

import re
import sys
from pathlib import Path

ORACLE = Path(__file__).resolve().parents[1]
CONSPAN_G01 = ORACLE / "projects" / "conspan" / "ConSpan.g01"
CONSPAN_U02 = ORACLE / "projects" / "conspan" / "conspan.u02"
OUT_DIR = ORACLE / "projects" / "reach_mild"

# Open-channel RMs only (culvert XS at 20.238 / 20.237 / 20.227 omitted).
REACH_MILD_RMS = (
    20.535,
    20.422,
    20.308,
    20.251,
    20.208,
    20.189,
    20.095,
    20.0,
)

RIVER = "Spring Creek"
REACH = "Culvrt Reach"
UPSTREAM_RM = 20.535
DOWNSTREAM_RM = 20.0

G01_FOOTER = """
LCMann Time=Dec/30/1899 00:00:00
LCMann Region Time=Dec/30/1899 00:00:00
LCMann Table=0
Chan Stop Cuts=-1 

Use User Specified Reach Order=0
GIS Ratio Cuts To Invert=-1
GIS Limit At Bridges=0
Composite Channel Slope=5
"""


def _rm_from_type_line(line: str) -> float | None:
    if not line.startswith("Type RM Length"):
        return None
    match = re.search(r"=\s*1\s*,([0-9.]+)", line)
    if not match:
        return None
    token = match.group(1).rstrip("*").strip()
    return float(token)


def _extract_xs_blocks(g01_text: str, rms: tuple[float, ...]) -> list[str]:
    lines = g01_text.splitlines()
    blocks: list[str] = []
    i = 0
    while i < len(lines):
        line = lines[i]
        if line.startswith("Type RM Length"):
            rm = _rm_from_type_line(line)
            start = i
            i += 1
            while i < len(lines) and not lines[i].startswith("Type RM Length"):
                if lines[i].startswith("LCMann Time="):
                    break
                i += 1
            block = "\n".join(lines[start:i]).rstrip()
            if rm is not None and any(abs(rm - target) < 1e-3 for target in rms):
                blocks.append(block)
            continue
        i += 1

    found = []
    for target in rms:
        match = next(
            (b for b in blocks if abs(_rm_from_type_line(b.splitlines()[0]) - target) < 1e-3),
            None,
        )
        if match is None:
            raise ValueError(f"Missing ConSpan XS for RM {target}")
        found.append(match)
    return found


def _header_from_conspan(g01_text: str) -> str:
    lines = g01_text.splitlines()
    header: list[str] = []
    for line in lines:
        if line.startswith("Type RM Length"):
            break
        if line.startswith("Geom Title="):
            header.append("Geom Title=Reach mild open channel (ConSpan upstream, no culvert)")
            continue
        header.append(line)
    return "\n".join(header).rstrip()


def _parse_type1_reach_segments(g01_text: str) -> list[tuple[float, int]]:
    """Return (RM, reach length ft) for each Type=1 cross section in file order."""
    segments: list[tuple[float, int]] = []
    for line in g01_text.splitlines():
        if not line.startswith("Type RM Length L Ch R = 1"):
            continue
        match = re.search(r"=\s*1\s*,([0-9.]+)\*?\s*,(\d+)", line)
        if not match:
            continue
        segments.append((float(match.group(1)), int(match.group(2))))
    return segments


def _combined_reach_length_ft(
    segments: list[tuple[float, int]],
    rm_from: float,
    rm_to: float,
) -> int:
    """Sum ConSpan reach lengths along the main stem from rm_from down to rm_to."""
    total = 0
    capturing = False
    for rm, length in segments:
        if abs(rm - rm_from) < 1e-3:
            capturing = True
        if capturing:
            if abs(rm - rm_to) < 1e-3:
                break
            total += length
    if total <= 0:
        total = max(1, int(round(abs(rm_from - rm_to) * 5280.0)))
    return total


def _replace_type_rm_length(block: str, rm: float, length: int, *, starred: bool = False) -> str:
    lines = block.splitlines()
    rm_token = f"{rm:.3f}*" if starred else f"{rm:.3f} "
    new_line = f"Type RM Length L Ch R = 1 ,{rm_token} ,{length},{length},{length}"
    if rm <= 20.251 + 1e-3 and length > 100:
        # Preserve ConSpan overbank path proportions for wide upstream sections.
        if abs(rm - 20.535) < 1e-3:
            new_line = f"Type RM Length L Ch R = 1 ,{rm_token} ,{length},{max(1, int(length * 600 / 650))},{max(1, int(length * 500 / 650))}"
        elif abs(rm - 20.422) < 1e-3:
            new_line = f"Type RM Length L Ch R = 1 ,{rm_token} ,{length},{max(1, int(length * 600 / 650))},{max(1, int(length * 500 / 650))}"
        elif abs(rm - 20.308) < 1e-3:
            new_line = f"Type RM Length L Ch R = 1 ,{rm_token} ,{length},{length},{max(1, int(length * 350 / 300))}"
    for idx, line in enumerate(lines):
        if line.startswith("Type RM Length"):
            lines[idx] = new_line
            break
    return "\n".join(lines)


def _recompute_reach_lengths(
    blocks: list[str],
    g01_text: str,
    rms: tuple[float, ...],
) -> list[str]:
    segments = _parse_type1_reach_segments(g01_text)
    updated: list[str] = []
    for i, block in enumerate(blocks):
        rm = rms[i]
        starred = block.splitlines()[0].find("*") >= 0
        if i + 1 < len(rms):
            length = _combined_reach_length_ft(segments, rm, rms[i + 1])
        else:
            length = 0
        updated.append(_replace_type_rm_length(block, rm, length, starred=starred))
    return updated


def write_g01(blocks: list[str], g01_text: str) -> None:
    blocks = _recompute_reach_lengths(blocks, g01_text, REACH_MILD_RMS)
    body = "\n\n".join(blocks)
    content = f"{_header_from_conspan(g01_text)}\n\n{body}{G01_FOOTER}"
    (OUT_DIR / "reach_mild.g01").write_text(content, encoding="utf-8")


def write_u02() -> None:
    """Author reach_mild.u02 (compact CRLF; see scripts/write_reach_mild_u02.py)."""
    import importlib.util

    script = Path(__file__).resolve().parent / "write_reach_mild_u02.py"
    spec = importlib.util.spec_from_file_location("write_reach_mild_u02", script)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"Cannot load {script}")
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    mod.write_u02_bytes(OUT_DIR / "reach_mild.u02")


def write_p02() -> None:
    content = """Plan Title=Reach mild open channel unsteady
Program Version=5.00
Short Identifier=ReachMild02
Simulation Date=01JAN2000,0000,03JAN2000,0000
Geom File=g01
Flow File=u02
Subcritical Flow
Friction Slope Method= 1 
Unsteady Friction Slope Method= 2 
Computation Interval=1HOUR
Output Interval=1HOUR
Run HTab= 1 
Run UNet= 1 
Run PostProcess= 1 
UNET Theta= 1 
UNET ZTol= 0.01 
UNET MxIter= 20 
"""
    (OUT_DIR / "reach_mild.p02").write_text(content, encoding="utf-8")


def write_prj() -> None:
    content = """Proj Title=Reach mild open channel
Program Version=5.00
Current Plan=p02
Default Exp/Contr=0.3,0.1
SI Units=0
English Units=1
Geom File=g01
Geom Title=Reach mild open channel (ConSpan upstream, no culvert)
Unsteady File=u02
Unsteady Title=Reach mild constant Q unsteady
Plan File=p02
Plan Title=Reach mild open channel unsteady
"""
    (OUT_DIR / "reach_mild.prj").write_bytes(
        ("\r\n".join(content.splitlines()) + "\r\n").encode("utf-8")
    )


def main() -> int:
    if not CONSPAN_G01.is_file():
        print(f"ERROR: missing {CONSPAN_G01}", file=sys.stderr)
        return 1
    g01_text = CONSPAN_G01.read_text(encoding="utf-8")
    blocks = _extract_xs_blocks(g01_text, REACH_MILD_RMS)
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    write_g01(blocks, g01_text)
    write_u02()
    write_p02()
    write_prj()
    print(f"Wrote reach_mild bundle ({len(blocks)} cross sections) -> {OUT_DIR}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
