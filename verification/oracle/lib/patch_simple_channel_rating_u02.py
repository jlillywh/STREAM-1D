"""Patch simple_channel.u03 downstream BC for HEC-RAS 7 (rating curve)."""

from __future__ import annotations

import re
from pathlib import Path

from .hecras_text_io import read_ras_lines, write_ras_lines

RIVER = "Simple Creek"
REACH = "Trapezoid Reach"
DOWNSTREAM_STATION = "0.0"

_DS_LOC = (
    f"Boundary Location={RIVER}    ,{REACH} ,{DOWNSTREAM_STATION}    ,"
    "        ,                ,                ,                "
)


def _format_rating_pairs(rating_q: list[float], rating_wsel: list[float]) -> list[str]:
    """RAS 7 fixed-width: (stage, discharge) pairs, 5 pairs per line."""
    interleaved: list[float] = []
    for wsel, q in zip(rating_wsel, rating_q):
        interleaved.extend([wsel, q])
    lines: list[str] = []
    for i in range(0, len(interleaved), 10):
        row = interleaved[i : i + 10]
        lines.append("".join(f"{v:8.2f}" for v in row))
    return lines


def _replace_downstream_block(
    lines: list[str],
    *,
    rating_q: list[float],
    rating_wsel: list[float],
) -> list[str]:
    loc_re = re.compile(
        rf"^Boundary Location={re.escape(RIVER)}\s*,\s*{re.escape(REACH)}\s*,\s*0\.0\b"
    )
    out: list[str] = []
    i = 0
    while i < len(lines):
        line = lines[i]
        if loc_re.match(line):
            out.append(_DS_LOC)
            out.append(f"Rating Curve= {len(rating_q)} ")
            out.extend(_format_rating_pairs(rating_q, rating_wsel))
            i += 1
            while i < len(lines) and not lines[i].startswith("Boundary Location="):
                i += 1
            continue
        out.append(line)
        i += 1
    return out


def patch_rating_curve_ds(
    u03_path: Path,
    *,
    rating_q: list[float],
    rating_wsel: list[float],
) -> dict[str, object]:
    if not u03_path.is_file():
        raise FileNotFoundError(u03_path)
    if len(rating_q) != len(rating_wsel) or len(rating_q) < 2:
        raise ValueError("rating_q and rating_wsel need >=2 matching points")

    pairs = sorted(zip(rating_q, rating_wsel), key=lambda row: row[0])
    rating_q_sorted = [q for q, _ in pairs]
    rating_wsel_sorted = [w for _, w in pairs]

    meta: dict[str, object] = {"path": str(u03_path), "ras_commander": False}
    try:
        import pandas as pd
        from ras_commander import RasUnsteady

        df = pd.DataFrame({"stage": rating_wsel_sorted, "discharge": rating_q_sorted})
        result = RasUnsteady.set_rating_curve(
            u03_path,
            df,
            river=RIVER,
            reach=REACH,
            station=DOWNSTREAM_STATION,
        )
        meta["ras_commander"] = True
        meta["ras_result"] = result
    except ImportError:
        lines = read_ras_lines(u03_path)
        lines = _replace_downstream_block(
            lines,
            rating_q=rating_q_sorted,
            rating_wsel=rating_wsel_sorted,
        )
        write_ras_lines(u03_path, lines)
    except ValueError as exc:
        meta["ras_commander_error"] = str(exc)
        lines = read_ras_lines(u03_path)
        lines = _replace_downstream_block(
            lines,
            rating_q=rating_q_sorted,
            rating_wsel=rating_wsel_sorted,
        )
        write_ras_lines(u03_path, lines)

    # Re-write with full-precision 8.2f pairs (ras-commander rounds to 0.1).
    lines = read_ras_lines(u03_path)
    lines = _replace_downstream_block(
        lines,
        rating_q=rating_q_sorted,
        rating_wsel=rating_wsel_sorted,
    )
    write_ras_lines(u03_path, lines)

    lines = read_ras_lines(u03_path)
    text = "\n".join(lines)
    if "Rating Curve=" not in text:
        raise ValueError(f"{u03_path.name}: missing Rating Curve after patch")
    if "Friction Slope=" in text or "Stage Hydrograph=" in text:
        raise ValueError(f"{u03_path.name}: stray downstream BC still present")
    meta["rating_line"] = next(ln for ln in lines if ln.startswith("Rating Curve="))
    return meta
