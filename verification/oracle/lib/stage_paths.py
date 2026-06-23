"""Windows-native HEC-RAS project staging paths (GUI-friendly)."""

from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path

# Override with STREAM1D_HECRAS_STAGE (e.g. C:\Users\<you>\Documents\hecras_testing)
_DEFAULT_REL = Path("Documents") / "hecras_testing"


def _windows_userprofile() -> Path | None:
    if sys.platform == "win32":
        profile = os.environ.get("USERPROFILE")
        return Path(profile) if profile else None
    try:
        raw = subprocess.check_output(
            ["cmd.exe", "/c", "echo", "%USERPROFILE%"],
            text=True,
            stderr=subprocess.DEVNULL,
        ).strip().replace("\r", "")
    except (subprocess.CalledProcessError, FileNotFoundError):
        return None
    if not raw or len(raw) < 2 or raw[1] != ":":
        return None
    drive = raw[0].lower()
    rest = raw[2:].replace("\\", "/")
    return Path(f"/mnt/{drive}{rest}")


def hecras_stage_root() -> Path:
    """Root folder for GUI-accessible HEC-RAS test projects."""
    override = os.environ.get("STREAM1D_HECRAS_STAGE", "").strip()
    if override:
        return Path(override)
    profile = _windows_userprofile()
    if profile is None:
        # Last resort (Windows only)
        return Path.home() / "Documents" / "hecras_testing"
    return profile / _DEFAULT_REL


def hecras_stage_dir(project_name: str) -> Path:
    """Per-project stage directory, e.g. .../hecras_testing/reach_mild."""
    return hecras_stage_root() / project_name
