"""Compare STREAM-1D results to linked HEC-RAS reference."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any

from .ras_reference import reference_for_profile
from .scenario import LinkedScenario


@dataclass
class CompareRow:
    station: float
    profile: str
    stream1d_wsel_ft: float
    hecras_wsel_ft: float
    delta_ft: float
    passed: bool


@dataclass
class CompareReport:
    scenario_id: str
    title: str
    tolerance_ft: float
    rows: list[CompareRow]
    linked_project_notes: str
    reference_source: str
    live_ras_status: str

    @property
    def passed(self) -> bool:
        return all(row.passed for row in self.rows)

    @property
    def max_abs_delta_ft(self) -> float:
        if not self.rows:
            return 0.0
        return max(abs(row.delta_ft) for row in self.rows)


def compare_steady_linked(
    scenario: LinkedScenario,
    stream1d_runs: list[dict[str, Any]],
    hecras_export: dict[tuple[float, str], float],
    *,
    reference_source: str,
    live_ras_status: str,
) -> CompareReport:
    tolerance = scenario.tolerance_ft
    rows: list[CompareRow] = []

    for run in stream1d_runs:
        profile_name = str(run["name"])
        for station, calc in run["wsel_by_station"].items():
            expected_fixture = run["expected_wsel_ft"].get(station)
            hecras = reference_for_profile(hecras_export, station, profile_name)
            if hecras is None and expected_fixture is not None:
                hecras = expected_fixture
            if hecras is None:
                continue
            delta = calc - hecras
            rows.append(
                CompareRow(
                    station=station,
                    profile=profile_name,
                    stream1d_wsel_ft=calc,
                    hecras_wsel_ft=hecras,
                    delta_ft=delta,
                    passed=abs(delta) <= tolerance,
                )
            )

    linked_notes = str(scenario.raw.get("linked_project", {}).get("notes", ""))
    return CompareReport(
        scenario_id=scenario.id,
        title=scenario.title,
        tolerance_ft=tolerance,
        rows=rows,
        linked_project_notes=linked_notes,
        reference_source=reference_source,
        live_ras_status=live_ras_status,
    )


@dataclass
class UnsteadyCompareRow:
    river_mile: float
    stream1d_max_wsel_ft: float
    hecras_max_wsel_ft: float
    delta_ft: float
    passed: bool


@dataclass
class UnsteadyCompareReport:
    scenario_id: str
    title: str
    tolerance_ft: float
    rows: list[UnsteadyCompareRow]
    mapping_notes: str
    reference_source: str
    num_steps: int
    coupling_mode: int

    @property
    def passed(self) -> bool:
        return all(row.passed for row in self.rows)

    @property
    def max_abs_delta_ft(self) -> float:
        if not self.rows:
            return 0.0
        return max(abs(row.delta_ft) for row in self.rows)


@dataclass
class UnsteadyTimeseriesCompareRow:
    river_mile: float
    hour: float
    stream1d_wsel_ft: float
    hecras_wsel_ft: float
    delta_ft: float
    passed: bool


@dataclass
class UnsteadyTimeseriesCompareReport:
    scenario_id: str
    title: str
    tolerance_ft: float
    rows: list[UnsteadyTimeseriesCompareRow]
    mapping_notes: str
    reference_source: str
    num_steps: int
    coupling_mode: int
    time_checkpoints_hr: list[float]
    overall_max_tolerance_ft: float | None = None

    @property
    def passed(self) -> bool:
        if self.overall_max_tolerance_ft is not None:
            return self.max_abs_delta_ft <= self.overall_max_tolerance_ft
        return all(row.passed for row in self.rows)

    @property
    def max_abs_delta_ft(self) -> float:
        if not self.rows:
            return 0.0
        return max(abs(row.delta_ft) for row in self.rows)


def compare_unsteady_timeseries_wsel(
    *,
    scenario_id: str,
    title: str,
    tolerance_ft: float,
    checkpoints_rm: list[float],
    time_checkpoints_hr: list[float],
    reference_series: dict[float, dict[float, float]],
    wsel_time_series: list[list[float]],
    rm_to_index,
    mapping_notes: str,
    reference_source: str,
    num_steps: int,
    coupling_mode: int,
    dt_seconds: float = 3600.0,
    overall_max_tolerance_ft: float | None = None,
) -> UnsteadyTimeseriesCompareReport:
    rows: list[UnsteadyTimeseriesCompareRow] = []
    for rm in checkpoints_rm:
        ref_by_hour = reference_series.get(rm)
        if ref_by_hour is None and reference_series:
            closest = min(reference_series.keys(), key=lambda k: abs(k - rm))
            if abs(closest - rm) > 0.02:
                continue
            ref_by_hour = reference_series[closest]
        if not ref_by_hour:
            continue
        idx = rm_to_index(rm)
        if idx is None:
            continue
        for hour in time_checkpoints_hr:
            ref = ref_by_hour.get(float(hour))
            if ref is None:
                ref = ref_by_hour.get(float(int(hour)))
            if ref is None:
                continue
            step = int(round(float(hour) * 3600.0 / max(dt_seconds, 1.0)))
            if step < 0 or step >= len(wsel_time_series):
                continue
            series = wsel_time_series[step]
            if idx >= len(series):
                continue
            calc = series[idx]
            delta = calc - ref
            rows.append(
                UnsteadyTimeseriesCompareRow(
                    river_mile=rm,
                    hour=float(hour),
                    stream1d_wsel_ft=calc,
                    hecras_wsel_ft=ref,
                    delta_ft=delta,
                    passed=abs(delta) <= tolerance_ft,
                )
            )
    return UnsteadyTimeseriesCompareReport(
        scenario_id=scenario_id,
        title=title,
        tolerance_ft=tolerance_ft,
        rows=rows,
        mapping_notes=mapping_notes,
        reference_source=reference_source,
        num_steps=num_steps,
        coupling_mode=coupling_mode,
        time_checkpoints_hr=[float(h) for h in time_checkpoints_hr],
        overall_max_tolerance_ft=overall_max_tolerance_ft,
    )


def format_unsteady_timeseries_report(
    report: UnsteadyTimeseriesCompareReport,
    *,
    enforce_tolerance: bool = True,
    compare_notes: str = "",
) -> str:
    hours = ", ".join(f"{h:g}" for h in report.time_checkpoints_hr)
    if enforce_tolerance:
        compare_line = (
            f"Tolerance: ±{report.tolerance_ft:.3f} ft WSEL at hours [{hours}]"
        )
    else:
        compare_line = (
            f"Compare: STREAM − HEC WSEL (ft) at hours [{hours}] — diagnostic, no pass/fail"
        )
    lines = [
        "=" * 72,
        f"Linked verify (unsteady): {report.title}",
        f"Scenario: {report.scenario_id}",
        f"Reference: {report.reference_source}",
        f"Steps: {report.num_steps}  coupling_mode: {report.coupling_mode}",
        compare_line,
    ]
    if report.mapping_notes:
        lines.append(f"Mapping: {report.mapping_notes}")
    if compare_notes:
        lines.append(f"Notes: {compare_notes}")
    if enforce_tolerance:
        lines.extend(
            [
                "-" * 72,
                f"{'RM':>8}  {'Hour':>6}  {'STREAM-1D':>14}  {'HEC-RAS ref':>14}  {'Δ (ft)':>8}  Status",
                "-" * 72,
            ]
        )
    else:
        lines.extend(
            [
                "-" * 72,
                f"{'RM':>8}  {'Hour':>6}  {'STREAM-1D':>14}  {'HEC-RAS ref':>14}  {'Δ (ft)':>8}",
                "-" * 72,
            ]
        )
    for row in sorted(report.rows, key=lambda r: (-r.river_mile, r.hour)):
        if enforce_tolerance:
            status = "PASS" if row.passed else "FAIL"
            lines.append(
                f"{row.river_mile:>8.3f}  {row.hour:>6.0f}  {row.stream1d_wsel_ft:14.3f}  "
                f"{row.hecras_wsel_ft:14.3f}  {row.delta_ft:+8.3f}  [{status}]"
            )
        else:
            lines.append(
                f"{row.river_mile:>8.3f}  {row.hour:>6.0f}  {row.stream1d_wsel_ft:14.3f}  "
                f"{row.hecras_wsel_ft:14.3f}  {row.delta_ft:+8.3f}"
            )
    lines.append("-" * 72)
    lines.append(f"Overall max |Δ| = {report.max_abs_delta_ft:.4f} ft  (STREAM − HEC)")
    if enforce_tolerance:
        lines.append("RESULT: PASS" if report.passed else "RESULT: FAIL")
    lines.append("=" * 72)
    return "\n".join(lines)


def format_unsteady_timeseries_matrix_report(
    report: UnsteadyTimeseriesCompareReport,
    *,
    enforce_tolerance: bool = True,
    compare_notes: str = "",
) -> str:
    """Pivot table: one row per RM, columns are Δ (ft) at each checkpoint hour."""
    hours = sorted({row.hour for row in report.rows})
    rms = sorted({row.river_mile for row in report.rows}, reverse=True)
    if not hours or not rms:
        return format_unsteady_timeseries_report(
            report,
            enforce_tolerance=enforce_tolerance,
            compare_notes=compare_notes,
        )

    by_rm_hour: dict[tuple[float, float], UnsteadyTimeseriesCompareRow] = {
        (row.river_mile, row.hour): row for row in report.rows
    }
    hour_labels = [f"{h:g}h" for h in hours]
    col_w = max(7, max(len(l) for l in hour_labels))
    rm_w = 8

    if enforce_tolerance:
        if report.overall_max_tolerance_ft is not None:
            compare_line = (
                f"Gate: overall max |Δ| ≤ {report.overall_max_tolerance_ft:.3f} ft  "
                f"({len(rms)} RMs × {len(hours)} times)"
            )
        else:
            compare_line = (
                f"Tolerance: ±{report.tolerance_ft:.3f} ft WSEL  "
                f"({len(rms)} RMs × {len(hours)} times)"
            )
    else:
        compare_line = (
            f"Compare: STREAM − HEC ΔWSEL (ft)  ({len(rms)} RMs × {len(hours)} times) "
            f"— diagnostic, no pass/fail"
        )
    lines = [
        "=" * 72,
        f"Linked verify (unsteady matrix): {report.title}",
        f"Scenario: {report.scenario_id}",
        f"Reference: {report.reference_source}",
        f"Steps: {report.num_steps}  coupling_mode: {report.coupling_mode}",
        compare_line,
    ]
    if report.mapping_notes:
        lines.append(f"Mapping: {report.mapping_notes}")
    if compare_notes:
        lines.append(f"Notes: {compare_notes}")
    lines.append("-" * 72)
    lines.append(
        f"{'RM':>{rm_w}}  "
        + "  ".join(f"{lbl:>{col_w}}" for lbl in hour_labels)
        + "  | Max|Δ|"
    )
    lines.append("-" * 72)

    for rm in rms:
        cells: list[str] = []
        max_abs = 0.0
        for hour in hours:
            row = by_rm_hour.get((rm, hour))
            if row is None:
                cells.append(f"{'—':>{col_w}}")
                continue
            max_abs = max(max_abs, abs(row.delta_ft))
            if enforce_tolerance and not row.passed:
                cells.append(f"{row.delta_ft:+{col_w - 1}.2f}*")
            else:
                cells.append(f"{row.delta_ft:+{col_w}.2f}")
        lines.append(
            f"{rm:>{rm_w}.3f}  " + "  ".join(cells) + f"  | {max_abs:6.3f}"
        )

    lines.append("-" * 72)
    if enforce_tolerance:
        lines.append("* = outside tolerance")
    lines.append(
        f"Overall max |Δ| = {report.max_abs_delta_ft:.4f} ft  (STREAM − HEC)"
    )
    if enforce_tolerance and report.overall_max_tolerance_ft is not None:
        lines.append(
            f"Overall gate: ≤ {report.overall_max_tolerance_ft:.3f} ft max |Δ|"
        )
    if enforce_tolerance:
        lines.append("RESULT: PASS" if report.passed else "RESULT: FAIL")
    lines.append("=" * 72)
    return "\n".join(lines)


def compare_unsteady_peak_wsel(
    *,
    scenario_id: str,
    title: str,
    tolerance_ft: float,
    checkpoints_rm: list[float],
    observed_hwm: dict[float, float],
    cross_sections,
    wsel_time_series: list[list[float]],
    rm_to_index,
    mapping_notes: str,
    reference_source: str,
    num_steps: int,
    coupling_mode: int,
    quantity: str = "max_wsel",
) -> UnsteadyCompareReport:
    if not observed_hwm and not checkpoints_rm:
        raise ValueError(
            "No reference peaks or Observed HWM entries for unsteady compare "
            "(provide reference JSON or Observed HWM= lines in .u02)"
        )
    rows: list[UnsteadyCompareRow] = []
    for rm in checkpoints_rm:
        ref = observed_hwm.get(rm)
        if ref is None and observed_hwm:
            closest = min(observed_hwm.keys(), key=lambda k: abs(k - rm))
            if abs(closest - rm) > 0.02:
                continue
            ref = observed_hwm[closest]
        if ref is None:
            continue
        idx = rm_to_index(rm)
        if idx is None:
            continue
        series = [step[idx] for step in wsel_time_series if idx < len(step)]
        if not series:
            continue
        if quantity == "terminal_wsel":
            calc = series[-1]
        else:
            calc = max(series)
        delta = calc - ref
        rows.append(
            UnsteadyCompareRow(
                river_mile=rm,
                stream1d_max_wsel_ft=calc,
                hecras_max_wsel_ft=ref,
                delta_ft=delta,
                passed=abs(delta) <= tolerance_ft,
            )
        )
    return UnsteadyCompareReport(
        scenario_id=scenario_id,
        title=title,
        tolerance_ft=tolerance_ft,
        rows=rows,
        mapping_notes=mapping_notes,
        reference_source=reference_source,
        num_steps=num_steps,
        coupling_mode=coupling_mode,
    )


def format_unsteady_report(report: UnsteadyCompareReport, *, quantity: str = "max_wsel") -> str:
    qty_label = "terminal WSEL" if quantity == "terminal_wsel" else "peak WSEL"
    lines = [
        "=" * 72,
        f"Linked verify (unsteady): {report.title}",
        f"Scenario: {report.scenario_id}",
        f"Reference: {report.reference_source}",
        f"Steps: {report.num_steps}  coupling_mode: {report.coupling_mode}",
        f"Tolerance: ±{report.tolerance_ft:.3f} ft {qty_label}",
    ]
    if report.mapping_notes:
        lines.append(f"Mapping: {report.mapping_notes}")
    lines.extend(
        [
            "-" * 72,
            f"{'RM':>8}  {'STREAM-1D':>14}  {'HEC-RAS ref':>14}  {'Δ (ft)':>8}  Status",
            "-" * 72,
        ]
    )
    for row in report.rows:
        status = "PASS" if row.passed else "FAIL"
        lines.append(
            f"{row.river_mile:>8.3f}  {row.stream1d_max_wsel_ft:14.3f}  "
            f"{row.hecras_max_wsel_ft:14.3f}  {row.delta_ft:+8.3f}  [{status}]"
        )
    lines.extend(
        [
            "-" * 72,
            f"Max |Δ| = {report.max_abs_delta_ft:.4f} ft",
            "RESULT: PASS" if report.passed else "RESULT: FAIL",
            "=" * 72,
        ]
    )
    return "\n".join(lines)


def format_report(report: CompareReport) -> str:
    lines = [
        "=" * 72,
        f"Linked verify: {report.title}",
        f"Scenario: {report.scenario_id}",
        f"Reference: {report.reference_source}",
        f"Tolerance: ±{report.tolerance_ft:.3f} ft WSEL",
    ]
    if report.linked_project_notes:
        lines.append(f"Linked project: {report.linked_project_notes}")
    if report.live_ras_status:
        lines.append(f"Live HEC-RAS: {report.live_ras_status}")
    lines.extend(
        [
            "-" * 72,
            f"{'Station':>8}  {'Profile':<8}  {'STREAM-1D':>10}  {'HEC-RAS':>10}  {'Δ (ft)':>8}  Status",
            "-" * 72,
        ]
    )
    for row in sorted(report.rows, key=lambda r: (r.profile, -r.station)):
        status = "PASS" if row.passed else "FAIL"
        lines.append(
            f"{int(row.station):>8}  {row.profile:<8}  {row.stream1d_wsel_ft:10.3f}  "
            f"{row.hecras_wsel_ft:10.3f}  {row.delta_ft:+8.3f}  [{status}]"
        )
    lines.extend(
        [
            "-" * 72,
            f"Max |Δ| = {report.max_abs_delta_ft:.4f} ft",
            "RESULT: PASS" if report.passed else "RESULT: FAIL",
            "=" * 72,
        ]
    )
    return "\n".join(lines)
