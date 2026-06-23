#!/usr/bin/env python3
"""Write beaver.f01 + beaver.p01 for HEC-RAS steady profile runs (CRLF)."""

from __future__ import annotations

import sys
from pathlib import Path

ORACLE = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ORACLE))

from lib.hecras_text_io import write_ras_lines  # noqa: E402

PROJECT = ORACLE / "projects" / "beaver"
F01 = PROJECT / "beaver.f01"
P01 = PROJECT / "beaver.p01"

RIVER = "Beaver Creek"
REACH = "Kentwood"
UPSTREAM_RM = 5.99
FRICTION_SLOPE = 0.002

# Match bundled u02 / diagnose_beaver_restart.py reference flows (cfs).
PROFILES: list[tuple[str, float]] = [
    ("Initial", 500.0),
    ("Peak", 14000.0),
    ("Recession", 6181.02),
]


def _flow_line(values: list[float]) -> str:
    return " " + " ".join(f"{v:8.2f}" if v < 10000 else f"{v:8.1f}" for v in values)


def build_f01_lines() -> list[str]:
    names = ",".join(name for name, _ in PROFILES)
    flows = [q for _, q in PROFILES]
    lines = [
        "Flow Title=Beaver Creek steady bridge profiles",
        "Program Version=5.00",
        f"Number of Profiles= {len(PROFILES)} ",
        f"Profile Names={names}",
        f"River Rch & RM={RIVER:<16},{REACH:<16},{UPSTREAM_RM:g}    ",
        _flow_line(flows).rstrip(),
    ]
    for i in range(1, len(PROFILES) + 1):
        lines.extend(
            [
                f"Boundary for River Rch & Prof#={RIVER:<16},{REACH:<16}, {i} ",
                "Up Type= 0 ",
                "Dn Type= 3 ",
                f"Dn Slope={FRICTION_SLOPE:g}",
            ]
        )
    lines.extend(
        [
            "DSS Import StartDate=",
            "DSS Import StartTime=",
            "DSS Import EndDate=",
            "DSS Import EndTime=",
            "DSS Import GetInterval= 0 ",
            "DSS Import Interval=",
            "DSS Import GetPeak= 0 ",
            "DSS Import FillOption= 0 ",
        ]
    )
    return lines


def build_p01_lines() -> list[str]:
    return [
        "Plan Title=Beaver Creek steady bridge profiles",
        "Program Version=5.00",
        "Short Identifier=Steady Brdg",
        "Simulation Date=,,,",
        "Geom File=g01",
        "Flow File=f01",
        "Subcritical Flow",
        "K Sum by GR= 0 ",
        "Std Step Tol= 0.01 ",
        "Critical Tol= 0.01 ",
        "Num of Std Step Trials= 20 ",
        "Max Error Tol= 0.33 ",
        "Flow Tol Ratio= 0.001 ",
        "Split Flow NTrial= 30 ",
        "Split Flow Tol= 0.02 ",
        "Split Flow Ratio= 0.02 ",
        "Log Output Level= 0 ",
        "Friction Slope Method= 1 ",
        "Unsteady Friction Slope Method= 2 ",
        "Unsteady Bridges Friction Slope Method= 1 ",
        "Calc Critical at every XS",
        "Parabolic Critical Depth",
        "Global Vel Dist= 1 , 1 , 1 ",
        "Global Log Level= 0 ",
        "CheckData=True",
        "Encroach Param=-1 ,0,0, 0 ",
        "Run HTab= 1 ",
        "Run UNet= 0 ",
        "Run Sediment= 0 ",
        "Run PostProcess= 1 ",
        "Run WQNet= 0 ",
        "Run RASMapper= 0 ",
        "Write HDF5 File= 1 ",
        "HDF Compression= 1 ",
        "HDF Chunk Size= 1 ",
        "HDF Spatial Parts= 1 ",
        "HDF Fixed Rows= 1 ",
        "Echo Input=False",
        "Echo Parameters=False",
        "Echo Output=False",
        "Write Detailed= 1 ",
    ]


def patch_prj(prj_path: Path) -> None:
    lines = prj_path.read_text(encoding="utf-8", errors="replace").splitlines()
    out: list[str] = []
    have_flow = False
    have_p01 = False
    for line in lines:
        if line.startswith("Flow File="):
            out.append("Flow File=f01")
            have_flow = True
            continue
        if line.startswith("Plan File=p01"):
            have_p01 = True
        if line.startswith("Plan Title=") and "steady" in line.lower():
            continue
        if line.startswith("Plan Title=") and "unsteady" in line.lower():
            continue
        out.append(line)
    if not have_flow:
        # Insert after Geom Title line if present.
        inserted: list[str] = []
        for line in out:
            inserted.append(line)
            if line.startswith("Geom Title="):
                inserted.append("Flow File=f01")
                inserted.append("Flow Title=Beaver Creek steady bridge profiles")
                have_flow = True
        out = inserted
    if not have_p01:
        # Add p01 before p03 if possible.
        patched: list[str] = []
        for line in out:
            if line.startswith("Plan File=p03"):
                patched.append("Plan File=p01")
                patched.append("Plan Title=Beaver Creek steady bridge profiles")
                have_p01 = True
            patched.append(line)
            if line.startswith("Plan File=p03"):
                patched.append("Plan Title=Unsteady with 100 yr event")
        out = patched if have_p01 else out + [
            "Plan File=p01",
            "Plan Title=Beaver Creek steady bridge profiles",
        ]
    write_ras_lines(prj_path, out)


def main() -> int:
    write_ras_lines(F01, build_f01_lines())
    write_ras_lines(P01, build_p01_lines())
    patch_prj(PROJECT / "beaver.prj")
    print(f"Wrote {F01}")
    print(f"Wrote {P01}")
    print(f"Updated {PROJECT / 'beaver.prj'}")
    print("Profiles:", ", ".join(f"{n}={q:g} cfs" for n, q in PROFILES))
    print("HEC-RAS: open beaver.prj → Steady Flow Editor (f01) → run plan 01")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
