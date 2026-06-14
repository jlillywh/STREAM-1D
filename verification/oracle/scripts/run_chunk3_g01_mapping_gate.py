#!/usr/bin/env python3
"""Chunk 3.3 — g01 bridge/culvert mapping smoke (Beaver bundle)."""

from __future__ import annotations

import os
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path

ORACLE = Path(__file__).resolve().parents[1]
ROOT = ORACLE.parents[1]
LOG = ORACLE / "logs" / "chunk3_g01_mapping_latest.log"


def main() -> int:
    LOG.parent.mkdir(parents=True, exist_ok=True)
    lines: list[str] = []

    def log(msg: str = "") -> None:
        print(msg)
        lines.append(msg)

    log("=== Chunk 3.3 g01 mapping gate ===")
    log(f"Repo: {ROOT}")
    log(f"Date: {datetime.now(timezone.utc).astimezone().isoformat(timespec='seconds')}")
    log()

    py = sys.executable
    env = os.environ.copy()
    python_pkg = str(ROOT / "python")
    prev = env.get("PYTHONPATH", "")
    env["PYTHONPATH"] = f"{python_pkg}{os.pathsep}{prev}" if prev else python_pkg

    try:
        import stream1d  # noqa: F401
    except ImportError:
        log("SKIP: stream1d extension not built — run: maturin develop --features python")
        LOG.write_text("\n".join(lines) + "\n", encoding="utf-8")
        return 0

    cmd = [py, str(ORACLE / "scripts" / "test_chunk3_g01_mapping.py")]
    print("\n--- 3.3 test_chunk3_g01_mapping ---")
    print(" ".join(cmd))
    try:
        subprocess.run(cmd, cwd=ROOT, env=env, check=True)
        log("test_chunk3_g01_mapping: OK")
    except subprocess.CalledProcessError as exc:
        log(f"\nFAILED: exit code {exc.returncode}")
        LOG.write_text("\n".join(lines) + "\n", encoding="utf-8")
        return exc.returncode or 1

    smoke = [py, str(ORACLE / "scripts" / "smoke_beaver_parse.py")]
    print("\n--- 3.3 smoke_beaver_parse ---")
    try:
        subprocess.run(smoke, cwd=ROOT, env=env, check=True)
        log("smoke_beaver_parse: OK")
    except subprocess.CalledProcessError as exc:
        log(f"\nFAILED: exit code {exc.returncode}")
        LOG.write_text("\n".join(lines) + "\n", encoding="utf-8")
        return exc.returncode or 1

    log()
    log("=== Chunk 3.3 g01 mapping gate complete ===")
    log(f"Log: {LOG}")
    LOG.write_text("\n".join(lines) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
