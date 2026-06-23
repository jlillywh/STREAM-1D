"""Repository root helpers for oracle scripts."""

from __future__ import annotations

import os
import subprocess
from pathlib import Path

ORACLE_ROOT = Path(__file__).resolve().parents[1]
REPO_ROOT = ORACLE_ROOT.parents[1]


def repo_root_for_wsl(root: Path | None = None) -> str:
    """Map a Windows or UNC repo path to a WSL POSIX path for ``wsl bash -lc`` verify."""
    override = os.environ.get("STREAM1D_REPO_ROOT", "").strip()
    if override:
        return override

    resolved = (root or REPO_ROOT).resolve()
    text = str(resolved).replace("/", "\\")
    marker = "\\wsl.localhost\\"
    lower = text.lower()
    if marker in lower:
        rest = text[lower.index(marker) + len(marker) :]
        _distro, *parts = rest.split("\\")
        return "/" + "/".join(p for p in parts if p)
    if str(resolved).startswith("/"):
        return str(resolved)

    result = subprocess.run(
        ["wsl", "wslpath", "-u", text],
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode == 0 and result.stdout.strip():
        return result.stdout.strip()
    raise RuntimeError(
        "Could not map repo path to WSL. Run verify from Linux/WSL directly, "
        "set STREAM1D_REPO_ROOT, or pass an explicit path."
    )
