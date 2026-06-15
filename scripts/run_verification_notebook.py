#!/usr/bin/env python3
"""Build stream1d and run python/stream1d_verification.ipynb (same steps as CI).

Usage:
  python scripts/run_verification_notebook.py           # headless execute
  python scripts/run_verification_notebook.py --serve   # jupyter notebook UI
"""
from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path


def main() -> int:
    root = Path(__file__).resolve().parent.parent
    venv = root / ".venv"
    python = venv / "bin" / "python"

    if not venv.is_dir():
        subprocess.run([sys.executable, "-m", "venv", str(venv)], check=True)

    pip = [str(python), "-m", "pip"]
    subprocess.run(pip + ["install", "--upgrade", "pip", "maturin"], check=True)
    subprocess.run(pip + ["install", "-r", str(root / "requirements.txt")], check=True)
    subprocess.run(
        [str(python), "-m", "maturin", "develop", "--features", "python", "--release"],
        cwd=root,
        check=True,
    )

    notebook_dir = root / "python"
    notebook = notebook_dir / "stream1d_verification.ipynb"
    serve = len(sys.argv) > 1 and sys.argv[1] == "--serve"

    if serve:
        print("Open in your Windows browser (WSL cannot auto-open):")
        os.chdir(notebook_dir)
        os.execv(
            str(python),
            [
                str(python),
                "-m",
                "jupyter",
                "notebook",
                "--no-browser",
                notebook.name,
            ],
        )

    subprocess.run(
        [
            str(python),
            "-m",
            "jupyter",
            "nbconvert",
            "--to",
            "notebook",
            "--execute",
            str(notebook),
            "--output",
            "/tmp/stream1d_verification_executed.ipynb",
            "--ExecutePreprocessor.timeout=600",
        ],
        cwd=notebook_dir,
        check=True,
    )
    print("Notebook executed successfully.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
