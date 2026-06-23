#!/usr/bin/env python3
"""Find unsteady cross-section Water Surface datasets in a HEC-RAS plan HDF."""
from __future__ import annotations

import re
import sys
from pathlib import Path

import h5py

ORACLE = Path(__file__).resolve().parents[1]
DEFAULT_HDF = ORACLE / "projects" / "conspan" / "ConSpan.p08.hdf"
hdf = Path(sys.argv[1] if len(sys.argv) > 1 else DEFAULT_HDF)


def visitor(name: str, obj) -> None:
    if isinstance(obj, h5py.Dataset) and "Water Surface" in name:
        if "Unsteady" in name or "unsteady" in name.lower():
            print("DATASET", name, obj.shape, obj.dtype)


with h5py.File(hdf, "r") as f:
    f.visititems(visitor)

    # Geometry info for station labels (unsteady output block)
    candidates = [
        "Results/Unsteady/Output/Geometry Info/Cross Section Only",
        "Results/Post Process/Steady/Output/Geometry Info/Cross Section Only",
    ]
    for path in candidates:
        if path in f:
            labels = [x.decode() if isinstance(x, bytes) else str(x) for x in f[path][()]]
            print("LABELS", path, len(labels))
            for lab in labels:
                m = re.search(r"(\d+\.?\d*)", lab.replace("*", ""))
                print(" ", lab.strip(), "->", m.group(1) if m else "?")

    # Sample unsteady time series if present
    ts_paths = [
        p
        for p in [
            "Results/Unsteady/Output/Output Blocks/Base Output/Unsteady Time Series/Cross Sections/Water Surface",
            "Results/Unsteady/Output/Output Blocks/Base Output/Summary Output/Cross Sections/Water Surface",
        ]
        if p in f
    ]
    for path in ts_paths:
        ds = f[path]
        print("TS", path, ds.shape, ds.dtype)
        if ds.ndim == 2:
            print("  t0", ds[0, :].tolist())
            print("  t8", ds[8, :].tolist() if ds.shape[0] > 8 else "n/a")
