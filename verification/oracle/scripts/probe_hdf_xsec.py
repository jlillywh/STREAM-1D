#!/usr/bin/env python3
"""Probe HEC-RAS plan HDF cross-section WSEL layout (dev helper)."""
from __future__ import annotations

import sys
from pathlib import Path

try:
    import h5py
except ImportError:
    import subprocess

    subprocess.check_call([sys.executable, "-m", "pip", "install", "h5py", "-q", "--break-system-packages"])
    import h5py

ORACLE = Path(__file__).resolve().parents[1]
DEFAULT_HDF = ORACLE / "projects" / "conspan" / "ConSpan.p08.hdf"
hdf = Path(sys.argv[1] if len(sys.argv) > 1 else DEFAULT_HDF)
with h5py.File(hdf, "r") as f:
    hits = []
    def visit(name, obj):
        low = name.lower()
        if any(k in low for k in ("water_surface", "cross section", "cross_sections", "station")):
            hits.append((name, type(obj).__name__))
    f.visititems(visit)
    for name, kind in sorted(hits)[:60]:
        print(kind, name)
