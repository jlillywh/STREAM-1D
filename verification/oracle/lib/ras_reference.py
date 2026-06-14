"""Load linked HEC-RAS reference output (export CSV or optional live run)."""

from __future__ import annotations

import csv
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from .scenario import LinkedScenario


@dataclass(frozen=True)
class RasProfilePoint:
    station: float
    profile: str
    wsel_ft: float


def parse_conspan_csv(csv_path: Path) -> dict[tuple[float, str], float]:
    """
    Parse HEC-RAS profile export CSV (ConSpan format).

    Returns {(river_station, profile_name): wsel_ft}.
    """
    table: dict[tuple[float, str], float] = {}
    current_station: float | None = None

    with csv_path.open("r", encoding="utf-8", newline="") as fh:
        reader = csv.reader(fh)
        next(reader, None)  # header row 1
        next(reader, None)  # units row

        for row in reader:
            if not row or all(not cell.strip() for cell in row):
                continue
            if row[0].strip():
                try:
                    current_station = float(row[0].strip())
                except ValueError:
                    continue
            if current_station is None:
                continue
            profile = row[1].strip() if len(row) > 1 else ""
            if not profile:
                continue
            try:
                wsel = float(row[4].strip())
            except (ValueError, IndexError):
                continue
            table[(current_station, profile)] = wsel
    return table


def load_linked_export(scenario: LinkedScenario) -> dict[tuple[float, str], float]:
    ref = scenario.raw["reference"]
    csv_path = scenario.resolve(ref["csv"])
    if not csv_path.is_file():
        raise FileNotFoundError(f"Linked RAS export not found: {csv_path}")
    return parse_conspan_csv(csv_path)


def try_live_ras_run(scenario: LinkedScenario) -> tuple[dict[tuple[float, str], float] | None, str]:
    """
    Attempt to re-run the linked HEC-RAS plan via ras-commander.

    Returns (profile_table_or_none, status_message).
    """
    ref = scenario.raw.get("reference", {})
    if not ref.get("live_ras_optional", False):
        return None, "live RAS disabled for this scenario"

    try:
        from ras_commander import RasCmdr, RasPrj  # type: ignore[import-not-found]
    except ImportError:
        return None, "ras-commander not installed (pip install ras-commander)"

    linked = scenario.raw["linked_project"]
    project_dir = scenario.linked_project_dir()
    files = scenario.linked_files()
    missing = [name for name, path in files.items() if not path.is_file()]
    if missing:
        return None, f"linked project files missing: {', '.join(missing)}"

    plan_number = str(linked.get("plan_number", "01"))
    try:
        RasPrj(project_dir)
        RasCmdr.compute_plan(plan_number)
    except Exception as exc:  # pragma: no cover - depends on local HEC-RAS install
        return None, f"HEC-RAS live run failed: {exc}"

    # Extraction from live output is version-specific; callers fall back to CSV export.
    return None, (
        f"HEC-RAS plan {plan_number} executed in {project_dir}; "
        "profile extraction from live output not yet implemented — using linked CSV export"
    )


def reference_for_profile(
    export_table: dict[tuple[float, str], float],
    station: float,
    profile_name: str,
) -> float | None:
    return export_table.get((station, profile_name))


def load_unsteady_peak_reference(
    scenario: LinkedScenario,
    flow_observed_hwm: dict[float, float] | None = None,
) -> tuple[dict[float, float], str]:
    """
    Load {river_mile: max_wsel_ft} reference for unsteady linked verify.

    Supports `linked_json_peaks`, `linked_u02_observed_hwm`, with optional fallback
    from parsed .u02 Observed HWM lines.
    """
    ref = scenario.raw.get("reference", {})
    source_kind = str(ref.get("source", "linked_u02_observed_hwm"))
    peaks: dict[float, float] = {}
    source_label = source_kind

    if source_kind == "linked_conspan_profile":
        from .conspan_reference import peak_wsel_by_rm

        profile = str(ref.get("profile", "50 yr"))
        peaks = peak_wsel_by_rm(profile)
        source_label = f"ConSpan steady profile {profile!r} (fixtures/hecras_conspan_profiles.json)"

    if source_kind == "linked_json_peaks" and ref.get("file"):
        json_path = (scenario.oracle_root / ref["file"]).resolve()
        if json_path.is_file():
            import json

            data: dict[str, Any] = json.loads(json_path.read_text(encoding="utf-8"))
            for item in data.get("checkpoints", []):
                peaks[float(item["rm"])] = float(item["max_wsel_ft"])
            source_label = str(data.get("source", json_path.name))
        elif not ref.get("fallback_conspan_profile") and not ref.get("fallback_u02_observed_hwm"):
            raise FileNotFoundError(f"Peak WSEL reference not found: {json_path}")

    if not peaks and ref.get("fallback_conspan_profile"):
        from .conspan_reference import peak_wsel_by_rm

        profile = str(ref.get("profile", "50 yr"))
        peaks = peak_wsel_by_rm(profile)
        source_label = f"ConSpan steady profile {profile!r} (fallback)"

    if not peaks and source_kind == "linked_u02_observed_hwm":
        if flow_observed_hwm:
            peaks = dict(flow_observed_hwm)
            source_label = str(ref.get("notes", "linked_u02_observed_hwm"))

    if not peaks and ref.get("fallback_u02_observed_hwm") and flow_observed_hwm:
        peaks = dict(flow_observed_hwm)
        source_label = f"{source_label} (u02 Observed HWM fallback)"

    if not peaks:
        raise ValueError(
            f"No unsteady peak reference loaded for scenario {scenario.id} "
            f"(source={source_kind})"
        )
    return peaks, source_label


def load_unsteady_timeseries_reference(
    scenario: LinkedScenario,
) -> tuple[dict[float, dict[float, float]], str]:
    """
    Load {river_mile: {hour: wsel_ft}} from linked JSON reference.

    Requires reference JSON with ``time_checkpoints_hr`` and per-RM ``wsel_ft_by_hour``.
    """
    ref = scenario.raw.get("reference", {})
    json_rel = ref.get("file")
    if not json_rel:
        raise ValueError(f"No reference.file for timeseries scenario {scenario.id}")

    json_path = (scenario.oracle_root / json_rel).resolve()
    if not json_path.is_file():
        raise FileNotFoundError(f"Timeseries WSEL reference not found: {json_path}")

    import json

    data: dict[str, Any] = json.loads(json_path.read_text(encoding="utf-8"))
    if not data.get("time_checkpoints_hr"):
        raise ValueError(
            f"{json_path.name}: missing time_checkpoints_hr "
            "(re-extract with run_ras_reference.py or chunk1 capture)"
        )

    series: dict[float, dict[float, float]] = {}
    for item in data.get("checkpoints", []):
        rm = float(item["rm"])
        by_hour = item.get("wsel_ft_by_hour") or {}
        if not by_hour:
            raise ValueError(f"{json_path.name}: RM {rm} missing wsel_ft_by_hour")
        series[rm] = {float(k): float(v) for k, v in by_hour.items()}

    if not series:
        raise ValueError(f"No timeseries checkpoints in {json_path}")

    source_label = str(data.get("source", json_path.name))
    return series, source_label
