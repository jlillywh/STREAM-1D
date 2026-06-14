#!/usr/bin/env python3
"""
Extract WSEL time series from HEC-RAS plan HDF for linked verify (Chunk 8 stub).

Usage:
  python3 verification/oracle/scripts/extract_hdf_wsel.py \\
    --hdf verification/oracle/projects/beaver/beaver.p03.hdf \\
    --scenario verification/oracle/scenarios/beaver_unsteady_linked.json \\
    --out verification/oracle/projects/beaver/reference_wsel_hdf_plan03.json

Requires: h5py (and optionally ras-commander for path discovery).
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path


def _find_wsel_datasets(hdf_path: Path) -> list[tuple[str, tuple]]:
    import h5py

    found: list[tuple[str, tuple]] = []
    with h5py.File(hdf_path, "r") as f:
        def visit(name: str, obj) -> None:
            if isinstance(obj, h5py.Dataset) and "WSEL" in name.upper():
                found.append((name, obj.shape))

        f.visititems(visit)
    return found


def main() -> int:
    parser = argparse.ArgumentParser(description="Extract HDF WSEL for oracle (stub)")
    parser.add_argument("--hdf", type=Path, required=True, help="Path to plan .hdf")
    parser.add_argument("--scenario", type=Path, help="Linked scenario JSON for checkpoint RMs")
    parser.add_argument("--out", type=Path, help="Output JSON path")
    parser.add_argument("--hecras-version", default="6.x")
    args = parser.parse_args()

    if not args.hdf.is_file():
        print(f"ERROR: HDF not found: {args.hdf}", file=sys.stderr)
        print(
            "Run HEC-RAS plan 03 locally, enable Write HDF5, then point --hdf at the output.",
            file=sys.stderr,
        )
        return 2

    datasets = _find_wsel_datasets(args.hdf)
    print(f"Found {len(datasets)} WSEL-like datasets in {args.hdf}")
    for name, shape in datasets[:20]:
        print(f"  {name} {shape}")
    if len(datasets) > 20:
        print(f"  ... and {len(datasets) - 20} more")

    checkpoints = []
    if args.scenario and args.scenario.is_file():
        scenario = json.loads(args.scenario.read_text(encoding="utf-8"))
        for rm in scenario.get("compare", {}).get("checkpoints_rm", []):
            checkpoints.append({"river_mile": float(rm), "times_s": [], "wsel_ft": []})

    stub = {
        "schema_version": 1,
        "source": "hecras_hdf",
        "hecras_version": args.hecras_version,
        "plan": "03",
        "hdf_path": str(args.hdf),
        "status": "stub — populate times_s/wsel_ft per RM from HDF datasets above",
        "checkpoints": checkpoints,
    }

    if args.out:
        args.out.parent.mkdir(parents=True, exist_ok=True)
        args.out.write_text(json.dumps(stub, indent=2) + "\n", encoding="utf-8")
        print(f"Wrote stub: {args.out}")

    print(
        "\nNext: map RAS 6.x HDF cross-section output to checkpoint RMs using ras-commander "
        "or manual inspection of dataset paths listed above."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
