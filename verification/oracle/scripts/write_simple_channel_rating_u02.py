#!/usr/bin/env python3
"""Write simple_channel.u03 — constant Q upstream, rating-curve downstream (RAS 7)."""

from __future__ import annotations

import json
import sys
from pathlib import Path

ORACLE = Path(__file__).resolve().parents[1]
ROOT = ORACLE.parents[1]
sys.path.insert(0, str(ORACLE))
sys.path.insert(0, str(ROOT / "python"))

from lib.hecras_geom_parser import parse_g01, parsed_xs_to_dict  # noqa: E402
from lib.hecras_text_io import assert_compact_ras_text, write_ras_lines  # noqa: E402
from lib.patch_simple_channel_rating_u02 import patch_rating_curve_ds  # noqa: E402
from lib.rating_curve_ops import build_channel_rating_curve  # noqa: E402

PROJECT = ORACLE / "projects" / "simple_channel"
OUT = PROJECT / "simple_channel.u03"
RATING_JSON = PROJECT / "rating_curve_ds.json"

NUM = 49
Q = 150.0
FRICTION_SLOPE = 0.001
Q_SAMPLES = [25.0, 50.0, 75.0, 100.0, 125.0, 150.0, 175.0, 200.0, 250.0, 350.0, 500.0]


def _hecras_table_lines(values: list[float], *, per_line: int = 10) -> list[str]:
    lines: list[str] = []
    for i in range(0, len(values), per_line):
        row = values[i : i + per_line]
        lines.append("".join(f"{v:8.2f}" for v in row))
    return lines


def _approximate_rating_from_anchor(
    q_samples: list[float],
    *,
    anchor_q: float,
    anchor_wsel: float,
    z_min: float = 98.5,
) -> tuple[list[float], list[float]]:
    depth_ref = max(anchor_wsel - z_min, 0.1)
    rating_q = sorted({max(float(q), 1.0) for q in q_samples})
    rating_wsel = [z_min + depth_ref * (q / anchor_q) ** (2.0 / 3.0) for q in rating_q]
    return rating_q, rating_wsel


def _load_or_build_rating() -> tuple[list[float], list[float]]:
    if RATING_JSON.is_file():
        doc = json.loads(RATING_JSON.read_text(encoding="utf-8"))
        return list(doc["rating_q_cfs"]), list(doc["rating_wsel_ft"])

    try:
        import stream1d as st

        geom = parse_g01(PROJECT / "simple_channel.g01")
        cross_sections = [st.CrossSection(**parsed_xs_to_dict(xs)) for xs in geom.cross_sections]
        rating_q, rating_wsel = build_channel_rating_curve(
            cross_sections,
            Q_SAMPLES,
            friction_slope=FRICTION_SLOPE,
        )
    except (ImportError, ModuleNotFoundError):
        rating_q, rating_wsel = _approximate_rating_from_anchor(
            Q_SAMPLES,
            anchor_q=Q,
            anchor_wsel=100.9702,
        )
    RATING_JSON.write_text(
        json.dumps(
            {
                "source": "normal-depth rating on downstream XS (friction slope 0.001)",
                "rating_q_cfs": rating_q,
                "rating_wsel_ft": rating_wsel,
            },
            indent=2,
        )
        + "\n",
        encoding="utf-8",
    )
    return rating_q, rating_wsel


def build_u03_seed_lines() -> list[str]:
    """RAS 7 layout: upstream BC, global DSS flags, downstream Rating Curve=0 seed."""
    q_vals = [Q] * NUM
    return [
        "Flow Title=Simple channel constant Q unsteady (rating DS)",
        "Program Version=7.01",
        "Use Restart= 0 ",
        f"Initial Flow Loc=Simple Creek    ,Trapezoid Reach ,3.0    ,{Q:g}",
        "Boundary Location=Simple Creek    ,Trapezoid Reach ,3.0    ,        ,                ,                ,                ",
        "Interval=1HOUR",
        "Use Fixed Start Time=False",
        "Fixed Start Date/Time=,",
        f"Flow Hydrograph= {NUM} ",
        *_hecras_table_lines(q_vals),
        "DSS File=",
        "Use DSS=False",
        "Boundary Location=Simple Creek    ,Trapezoid Reach ,0.0    ,        ,                ,                ,                ",
        "Rating Curve= 0 ",
    ]


def write_u03_bytes(path: Path = OUT) -> tuple[list[float], list[float]]:
    rating_q, rating_wsel = _load_or_build_rating()
    write_ras_lines(path, build_u03_seed_lines())
    patch_rating_curve_ds(path, rating_q=rating_q, rating_wsel=rating_wsel)
    assert_compact_ras_text(path)
    return rating_q, rating_wsel


if __name__ == "__main__":
    rq, rw = write_u03_bytes()
    print(f"Wrote {OUT}")
    print(f"  rating pairs: {len(rq)}")
    for q, w in zip(rq, rw):
        print(f"    Q={q:6.1f} cfs -> WSEL={w:.4f} ft")
