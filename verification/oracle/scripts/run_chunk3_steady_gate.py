#!/usr/bin/env python3
"""Chunk 3 — full steady inline structure parity gate (3.1 + 3.2 + 3.3)."""

from __future__ import annotations

import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path

ORACLE = Path(__file__).resolve().parents[1]
ROOT = ORACLE.parents[1]
SCRIPTS = ORACLE / "scripts"
LOG = ORACLE / "logs" / "chunk3_steady_latest.log"

GATES = [
    ("3.1 culvert steady", SCRIPTS / "run_chunk3_culvert_steady_gate.py"),
    ("3.2 bridge steady", SCRIPTS / "run_chunk3_bridge_steady_gate.py"),
    ("3.3 g01 mapping", SCRIPTS / "run_chunk3_g01_mapping_gate.py"),
]


def main() -> int:
    LOG.parent.mkdir(parents=True, exist_ok=True)
    lines: list[str] = []
    py = sys.executable

    def log(msg: str = "") -> None:
        print(msg)
        lines.append(msg)

    log("=== Chunk 3 steady structure parity gate ===")
    log(f"Repo: {ROOT}")
    log(f"Date: {datetime.now(timezone.utc).astimezone().isoformat(timespec='seconds')}")
    log()

    for label, script in GATES:
        print(f"\n========== {label} ==========")
        try:
            subprocess.run([py, str(script)], cwd=ROOT, check=True)
            log(f"{label}: OK")
        except subprocess.CalledProcessError as exc:
            log(f"\nFAILED at {label}: exit code {exc.returncode}")
            LOG.write_text("\n".join(lines) + "\n", encoding="utf-8")
            return exc.returncode or 1

    log()
    log("=== Chunk 3 complete — ready for Chunk 4 ===")
    log(f"Log: {LOG}")
    LOG.write_text("\n".join(lines) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
