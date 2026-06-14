#!/usr/bin/env python3
"""Chunk 3.1 — steady culvert parity gate (ConSpan / FHWA + linked steady oracle)."""

from __future__ import annotations

import os
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path

ORACLE = Path(__file__).resolve().parents[1]
ROOT = ORACLE.parents[1]
LOG = ORACLE / "logs" / "chunk3_culvert_steady_latest.log"


def run_step(
    label: str,
    cmd: list[str],
    *,
    cwd: Path | None = None,
    env: dict[str, str] | None = None,
) -> None:
    print(f"\n--- {label} ---")
    print(" ".join(cmd))
    subprocess.run(cmd, cwd=cwd or ROOT, env=env, check=True)


def main() -> int:
    LOG.parent.mkdir(parents=True, exist_ok=True)
    lines: list[str] = []

    def log(msg: str = "") -> None:
        print(msg)
        lines.append(msg)

    log("=== Chunk 3.1 steady culvert gate ===")
    log(f"Repo: {ROOT}")
    log(f"Date: {datetime.now(timezone.utc).astimezone().isoformat(timespec='seconds')}")
    log()

    py = sys.executable
    python_pkg = str(ROOT / "python")
    env = os.environ.copy()
    prev = env.get("PYTHONPATH", "")
    env["PYTHONPATH"] = f"{python_pkg}{os.pathsep}{prev}" if prev else python_pkg

    try:
        run_step(
            "3.1.1 cargo culvert_hecras_verification",
            ["cargo", "test", "--test", "culvert_hecras_verification"],
        )
        log("3.1.1: OK")

        try:
            import stream1d  # noqa: F401

            run_step(
                "3.1.2 python ConSpan profiles",
                [py, str(ROOT / "python" / "test_hecras_culvert_verification.py")],
                env=env,
            )
            log("3.1.2: OK")
        except ImportError:
            log("3.1.2: SKIP (stream1d extension not built)")

        run_step(
            "3.1.3 linked steady ConSpan oracle",
            [
                py,
                str(ORACLE / "run_linked_verify.py"),
                "--scenario",
                str(ORACLE / "scenarios" / "conspan_steady_linked.json"),
            ],
            env=env,
        )
        log("3.1.3: OK")
    except subprocess.CalledProcessError as exc:
        log(f"\nFAILED: exit code {exc.returncode}")
        LOG.write_text("\n".join(lines) + "\n", encoding="utf-8")
        return exc.returncode or 1

    log()
    log("=== Chunk 3.1 culvert steady gate complete ===")
    log(f"Log: {LOG}")
    LOG.write_text("\n".join(lines) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
