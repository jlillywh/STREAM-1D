"""Patch simple_channel.u02 downstream BC for HEC-RAS 7 (Normal Depth / friction slope)."""

from __future__ import annotations

import re
from pathlib import Path

from .hecras_text_io import read_ras_lines, write_ras_lines

RIVER = "Simple Creek"
REACH = "Trapezoid Reach"
DOWNSTREAM_STATION = "0.0"
DEFAULT_SLOPE = 0.001

_DS_LOC = (
    f"Boundary Location={RIVER}    ,{REACH} ,{DOWNSTREAM_STATION}    ,"
    "        ,                ,                ,                "
)


def _ensure_1d_friction_slope_flag(lines: list[str]) -> list[str]:
    """HEC-RAS 7 1D Normal Depth BCs use ``Friction Slope=<s>,0`` in emitted files."""
    out: list[str] = []
    for line in lines:
        if line.startswith("Friction Slope="):
            payload = line.split("=", 1)[1].strip()
            if "," not in payload:
                slope = payload.split()[0]
                line = f"Friction Slope={slope},0"
        out.append(line)
    return out


def _replace_downstream_block(lines: list[str], *, friction_slope: float) -> list[str]:
    """Keep only Boundary Location + Friction Slope on the RM 0.0 block."""
    loc_re = re.compile(
        rf"^Boundary Location={re.escape(RIVER)}\s*,\s*{re.escape(REACH)}\s*,\s*0\.0\b"
    )
    out: list[str] = []
    i = 0
    while i < len(lines):
        line = lines[i]
        if loc_re.match(line):
            out.append(_DS_LOC)
            out.append(f"Friction Slope={friction_slope:g},0")
            i += 1
            while i < len(lines) and not lines[i].startswith("Boundary Location="):
                i += 1
            continue
        out.append(line)
        i += 1
    return out


def patch_friction_slope_ds(
    u02_path: Path,
    *,
    friction_slope: float = DEFAULT_SLOPE,
) -> dict[str, object]:
    """
    Ensure RM 0.0 is a clean Normal Depth (friction slope) downstream BC.

    Uses ras-commander when available (strips stray Stage Hydrograph blocks),
    then enforces the `,0` flag HEC-RAS 7 expects on 1D river boundaries.
    """
    if not u02_path.is_file():
        raise FileNotFoundError(u02_path)

    meta: dict[str, object] = {"path": str(u02_path), "ras_commander": False}
    try:
        from ras_commander import RasUnsteady

        result = RasUnsteady.set_normal_depth_boundary(
            u02_path,
            friction_slope=friction_slope,
            river=RIVER,
            reach=REACH,
            station=DOWNSTREAM_STATION,
        )
        meta["ras_commander"] = True
        meta["ras_result"] = result
    except ImportError:
        lines = read_ras_lines(u02_path)
        lines = _replace_downstream_block(lines, friction_slope=friction_slope)
        write_ras_lines(u02_path, lines)
    except ValueError as exc:
        meta["ras_commander_error"] = str(exc)
        lines = read_ras_lines(u02_path)
        lines = _replace_downstream_block(lines, friction_slope=friction_slope)
        write_ras_lines(u02_path, lines)

    lines = _ensure_1d_friction_slope_flag(read_ras_lines(u02_path))
    write_ras_lines(u02_path, lines)

    text = "\n".join(lines)
    if "Friction Slope=" not in text:
        raise ValueError(f"{u02_path.name}: missing Friction Slope after patch")
    if "Stage Hydrograph=" in text:
        raise ValueError(f"{u02_path.name}: downstream Stage Hydrograph still present")
    meta["friction_line"] = next(ln for ln in lines if ln.startswith("Friction Slope="))
    return meta
