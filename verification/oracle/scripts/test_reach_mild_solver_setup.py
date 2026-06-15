#!/usr/bin/env python3
"""Unit checks for HEC-RAS station parsing and reach_mild mapper."""

from __future__ import annotations

import sys
from pathlib import Path

ORACLE = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ORACLE))

from lib.hecras_geom_parser import parse_g01  # noqa: E402
from lib.ras_headless import _parse_station_rm  # noqa: E402
from lib.reach_mapper import build_reach_unsteady_inputs  # noqa: E402


def test_station_tokens() -> None:
    assert _parse_station_rm("20.208*") == 20.208
    assert _parse_station_rm("20.189") == 20.189
    assert _parse_station_rm(" 20.000 ") == 20.0


def test_reach_mild_uses_g01_geometry() -> None:
    project = ORACLE / "projects" / "reach_mild"
    geom = parse_g01(project / "reach_mild.g01")
    payload, _ = build_reach_unsteady_inputs(project)
    assert len(payload["cross_sections"]) == len(geom.cross_sections)
    g01_xs = geom.cross_sections[0]
    payload_xs = payload["cross_sections"][0]
    assert payload_xs["x"] == g01_xs.x
    assert payload_xs["y"] == g01_xs.y
    assert payload_xs.get("is_overbank") is not None


def main() -> int:
    test_station_tokens()
    test_reach_mild_uses_g01_geometry()
    print("OK")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
