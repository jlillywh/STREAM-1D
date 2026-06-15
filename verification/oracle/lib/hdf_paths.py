"""Locate HEC-RAS plan HDF files for capture scripts."""

from __future__ import annotations

from pathlib import Path


def plan_hdf_candidates(
    project_stem: str,
    plan_key: str,
    *,
    explicit: Path | None = None,
    stage_dir: Path | None = None,
    repo_dir: Path | None = None,
) -> list[Path]:
    plan_key = plan_key.zfill(2)
    names = [f"{project_stem}.p{plan_key}.hdf"]
    candidates: list[Path] = []
    if explicit is not None:
        candidates.append(explicit)
    for base in (stage_dir, repo_dir):
        if base is None:
            continue
        for name in names:
            candidates.append(base / name)
        if base.is_dir():
            candidates.extend(sorted(base.glob("*.p*.hdf"), key=lambda p: p.stat().st_mtime, reverse=True))
    seen: set[str] = set()
    ordered: list[Path] = []
    for path in candidates:
        key = str(path)
        if key in seen:
            continue
        seen.add(key)
        ordered.append(path)
    return ordered


def resolve_plan_hdf(
    project_stem: str,
    plan_key: str,
    *,
    explicit: Path | None = None,
    stage_dir: Path | None = None,
    repo_dir: Path | None = None,
) -> Path | None:
    for candidate in plan_hdf_candidates(
        project_stem,
        plan_key,
        explicit=explicit,
        stage_dir=stage_dir,
        repo_dir=repo_dir,
    ):
        if candidate.is_file():
            return candidate.resolve()
    return None


def format_hdf_search_report(
    project_stem: str,
    plan_key: str,
    *,
    explicit: Path | None = None,
    stage_dir: Path | None = None,
    repo_dir: Path | None = None,
) -> str:
    plan_key = plan_key.zfill(2)
    lines = [
        f"No plan HDF found for {project_stem} Plan {plan_key}.",
        "",
        "Searched:",
    ]
    for candidate in plan_hdf_candidates(
        project_stem,
        plan_key,
        explicit=explicit,
        stage_dir=stage_dir,
        repo_dir=repo_dir,
    ):
        status = "FOUND" if candidate.is_file() else "missing"
        lines.append(f"  [{status}] {candidate}")
    lines.extend(
        [
            "",
            "HEC-RAS GUI checklist:",
            f"  1. py -3 verification\\oracle\\scripts\\chunk1_simple_channel_rating_prep.py --open-ras",
            f"  2. File > Open Project > ...\\hecras_testing\\simple_channel\\simple_channel.prj",
            f"  3. Plans > Plan 03  (not Plan 01)",
            f"  4. Unsteady Flow Editor > RM 0.0 should show Rating Curve",
            f"  5. Run > Compute Plan 03 — wait until complete",
            f"  6. Confirm file exists: ...\\simple_channel\\{project_stem}.p{plan_key}.hdf",
            f"  7. py -3 verification\\oracle\\scripts\\chunk1_simple_channel_rating_capture.py",
            "",
            "Or run headless (Windows):",
            f"  py -3 verification\\oracle\\scripts\\chunk1_simple_channel_rating_capture.py --run-ras",
        ]
    )
    return "\n".join(lines)
