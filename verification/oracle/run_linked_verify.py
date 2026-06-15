#!/usr/bin/env python3
"""
Linked HEC-RAS oracle — compare STREAM-1D to a bundled HEC-RAS project.

Workflow (linked verify):
  1. User supplies or uses a bundled HEC-RAS project (.g01 + .uXX / .pXX + .fXX).
  2. STREAM-1D inputs are mapped from the same geometry.
  3. HEC-RAS reference comes from a linked export (CSV, JSON peaks, Observed HWM, HDF).
  4. This script runs STREAM-1D and prints a side-by-side diff report.

Requires: stream1d Python extension (maturin develop --features python).

Usage:
  bash verification/oracle/run_oracle.sh
  bash verification/oracle/run_oracle.sh \\
    --scenario verification/oracle/scenarios/reach_mild_unsteady_linked.json

  PYTHONPATH=python python3 verification/oracle/run_linked_verify.py \\
    --scenario verification/oracle/scenarios/reach_mild_unsteady_linked.json
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path
from typing import Any

ORACLE_ROOT = Path(__file__).resolve().parent
REPO_ROOT = ORACLE_ROOT.parents[1]
_PYTHON_PKG = REPO_ROOT / "python"

if _PYTHON_PKG.is_dir() and str(_PYTHON_PKG) not in sys.path:
    sys.path.insert(0, str(_PYTHON_PKG))

if str(ORACLE_ROOT) not in sys.path:
    sys.path.insert(0, str(ORACLE_ROOT))

import stream1d as st  # noqa: E402

from lib.beaver_mapper import build_beaver_unsteady_inputs  # noqa: E402
from lib.compare import (  # noqa: E402
    compare_steady_linked,
    compare_unsteady_peak_wsel,
    compare_unsteady_timeseries_wsel,
    format_report,
    format_unsteady_report,
    format_unsteady_timeseries_matrix_report,
    format_unsteady_timeseries_report,
)
from lib.hecras_geom_parser import parse_g01  # noqa: E402
from lib.hecras_unsteady_parser import ParsedUnsteadyFlow  # noqa: E402
from lib.ras_reference import (  # noqa: E402
    load_linked_export,
    load_unsteady_peak_reference,
    load_unsteady_timeseries_reference,
    try_live_ras_run,
)
from lib.conspan_mapper import build_conspan_unsteady_inputs  # noqa: E402
from lib.conspan_reference import (  # noqa: E402
    conspan_geometry_rms_upstream_first,
    load_all_conspan_cross_sections,
    load_conspan_cross_sections_for_rms,
    rm_to_conspan_payload_index,
)
from lib.generic_unsteady_mapper import build_generic_unsteady_inputs  # noqa: E402
from lib.reach_mapper import build_reach_unsteady_inputs  # noqa: E402
from lib.scenario import LinkedScenario, load_scenario  # noqa: E402
from lib.stream1d_runner import run_steady_profiles  # noqa: E402


def _default_scenario() -> Path:
    return ORACLE_ROOT / "scenarios" / "conspan_steady_linked.json"


def _validate_linked_files(scenario: LinkedScenario) -> list[str]:
    errors: list[str] = []
    linked = scenario.raw.get("linked_project", {})
    if linked.get("validate_files") is False:
        return errors
    for label, path in scenario.linked_files().items():
        if not path.is_file():
            errors.append(f"missing linked {label}: {path}")
    ref = scenario.raw.get("reference", {})
    ref_sources_needing_json = {"linked_json_peaks", "linked_json_timeseries", "hdf_timeseries"}
    if ref.get("source") in ref_sources_needing_json and ref.get("file"):
        json_path = (scenario.oracle_root / ref["file"]).resolve()
        if not json_path.is_file():
            errors.append(f"missing reference JSON: {json_path}")
    return errors


def _build_unsteady_payload(
    scenario: LinkedScenario,
) -> tuple[dict[str, Any], ParsedUnsteadyFlow, list, list]:
    linked = scenario.raw["linked_project"]
    project_dir = scenario.linked_project_dir()
    mapper = str(scenario.raw.get("stream1d", {}).get("mapper", ""))

    if mapper == "beaver_mapper.build_beaver_unsteady_inputs":
        payload, flow = build_beaver_unsteady_inputs(project_dir)
        geom = parse_g01(project_dir / linked["geometry"])
        xs_list = geom.cross_sections
        parsed_xs = geom.cross_sections
    elif mapper == "conspan_mapper.build_conspan_unsteady_inputs":
        coupling = int(scenario.raw.get("stream1d", {}).get("coupling_mode", 0))
        friction_override = scenario.raw.get("stream1d", {}).get("unsteady_friction_slope_method")
        payload, flow = build_conspan_unsteady_inputs(
            project_dir,
            geometry_name=linked["geometry"],
            flow_name=linked["unsteady_flow"],
            plan_name=linked.get("plan"),
            coupling_mode=coupling,
            unsteady_friction_slope_method=(
                int(friction_override) if friction_override is not None else None
            ),
        )
        geom = parse_g01(project_dir / linked["geometry"])
        xs_list = load_all_conspan_cross_sections()

        class _RmTag:
            __slots__ = ("rm",)

            def __init__(self, rm: float) -> None:
                self.rm = rm

        parsed_xs = [_RmTag(rm) for rm in conspan_geometry_rms_upstream_first()]
    elif mapper == "reach_mapper.build_reach_unsteady_inputs":
        payload, flow = build_reach_unsteady_inputs(
            project_dir,
            geometry_name=linked["geometry"],
            flow_name=linked["unsteady_flow"],
        )
        geom = parse_g01(project_dir / linked["geometry"])
        xs_list = load_conspan_cross_sections_for_rms([xs.rm for xs in geom.cross_sections])
        parsed_xs = geom.cross_sections
    elif mapper == "simple_channel_mapper.build_simple_channel_unsteady_inputs":
        from lib.simple_channel_mapper import build_simple_channel_unsteady_inputs

        payload, flow = build_simple_channel_unsteady_inputs(
            project_dir,
            geometry_name=linked["geometry"],
            flow_name=linked["unsteady_flow"],
        )
        geom = parse_g01(project_dir / linked["geometry"])
        xs_list = geom.cross_sections
        parsed_xs = geom.cross_sections
    elif mapper == "bridge_mild_mapper.build_bridge_mild_unsteady_inputs":
        from lib.bridge_mild_mapper import build_bridge_mild_unsteady_inputs

        stream_cfg = scenario.raw.get("stream1d", {})
        case = str(stream_cfg.get("case", "yarnell"))
        coupling = int(stream_cfg.get("coupling_mode", 0))
        payload, flow = build_bridge_mild_unsteady_inputs(
            project_dir,
            case=case,
            coupling_mode=coupling,
        )
        xs_list = payload["cross_sections"]

        class _StaTag:
            __slots__ = ("rm",)

            def __init__(self, rm: float) -> None:
                self.rm = rm

        parsed_xs = [_StaTag(float(xs["station"])) for xs in xs_list]
    elif mapper == "generic_unsteady_mapper.build_generic_unsteady_inputs":
        stream_cfg = scenario.raw.get("stream1d", {})
        friction_override = stream_cfg.get("unsteady_friction_slope_method")
        payload, flow = build_generic_unsteady_inputs(
            project_dir,
            geometry_name=linked["geometry"],
            flow_name=linked["unsteady_flow"],
            plan_name=linked.get("plan"),
            coupling_mode=int(stream_cfg.get("coupling_mode", 0)),
            unsteady_friction_slope_method=(
                int(friction_override) if friction_override is not None else None
            ),
            num_slices=stream_cfg.get("num_slices"),
            max_spacing=stream_cfg.get("max_spacing"),
        )
        geom = parse_g01(project_dir / linked["geometry"])
        xs_list = geom.cross_sections
        parsed_xs = geom.cross_sections
    else:
        raise ValueError(f"Unsupported unsteady mapper: {mapper!r}")

    return payload, flow, xs_list, parsed_xs


def _run_steady(scenario: LinkedScenario, args) -> int:
    live_status = "skipped (use --live-ras to attempt)"
    reference_source = f"linked export: {scenario.raw['reference']['csv']}"
    if args.live_ras:
        _, live_status = try_live_ras_run(scenario)

    hecras_export = load_linked_export(scenario)
    stream1d_runs = run_steady_profiles(scenario)
    report = compare_steady_linked(
        scenario,
        stream1d_runs,
        hecras_export,
        reference_source=reference_source,
        live_ras_status=live_status,
    )
    print(format_report(report))
    return 0 if report.passed else 1


def _run_unsteady(scenario: LinkedScenario, args) -> int:
    live_status = "skipped (use --live-ras to attempt)"
    if args.live_ras:
        _, live_status = try_live_ras_run(scenario)

    payload, flow, xs_list, parsed_xs = _build_unsteady_payload(scenario)
    stream1d_cfg = scenario.raw.get("stream1d", {})
    mapper = str(stream1d_cfg.get("mapper", ""))
    coupling = int(stream1d_cfg.get("coupling_mode", 0))
    if coupling:
        payload["unsteady_structure_coupling_mode"] = coupling
    order = stream1d_cfg.get("structure_coupling_order")
    if order is not None:
        payload["structure_coupling_order"] = int(order)

    result = st.solve_unsteady(payload)
    wsel_raw = result["wsel"]
    if mapper == "bridge_mild_mapper.build_bridge_mild_unsteady_inputs":
        m_to_ft = 3.280839895
        wsel_time_series = [[v * m_to_ft for v in step] for step in wsel_raw]
    else:
        wsel_time_series = wsel_raw
    reference_peaks, reference_source = load_unsteady_peak_reference(
        scenario, flow.observed_hwm
    )

    def rm_to_index(rm: float):
        if mapper == "conspan_mapper.build_conspan_unsteady_inputs":
            return rm_to_conspan_payload_index(rm)
        if mapper == "bridge_mild_mapper.build_bridge_mild_unsteady_inputs":
            from lib.bridge_mild_mapper import station_to_payload_index

            return station_to_payload_index(rm, payload)
        best_idx = None
        best_dist = float("inf")
        for idx, xs in enumerate(parsed_xs):
            dist = abs(xs.rm - rm)
            if dist < best_dist:
                best_dist = dist
                best_idx = idx
        return best_idx if best_idx is not None and best_dist <= 0.02 else None

    checkpoints = scenario.raw.get("compare", {}).get("checkpoints_rm")
    if not checkpoints:
        checkpoints = sorted(reference_peaks.keys(), reverse=True)
    cli_rms = _parse_float_list(getattr(args, "checkpoints_rm", None))
    if cli_rms is not None:
        checkpoints = cli_rms

    quantity = str(
        scenario.raw.get("compare", {}).get(
            "quantity",
            scenario.raw.get("quantity", "max_wsel"),
        )
    )
    compare_cfg = scenario.raw.get("compare", {})
    time_checkpoints_hr = [
        float(h) for h in compare_cfg.get("time_checkpoints_hr", [])
    ]
    cli_hours = _parse_float_list(getattr(args, "time_checkpoints_hr", None))
    if cli_hours is not None:
        time_checkpoints_hr = cli_hours

    if quantity == "wsel_timeseries":
        if not time_checkpoints_hr:
            print(
                "ERROR: compare.quantity=wsel_timeseries requires compare.time_checkpoints_hr",
                file=sys.stderr,
            )
            return 2
        ref_override = getattr(args, "reference_file", None)
        reference_series, reference_source = load_unsteady_timeseries_reference(
            scenario,
            reference_file=ref_override,
        )
        report = compare_unsteady_timeseries_wsel(
            scenario_id=scenario.id,
            title=scenario.title,
            tolerance_ft=scenario.tolerance_ft,
            checkpoints_rm=[float(rm) for rm in checkpoints],
            time_checkpoints_hr=time_checkpoints_hr,
            reference_series=reference_series,
            wsel_time_series=wsel_time_series,
            rm_to_index=rm_to_index,
            mapping_notes=str(scenario.raw.get("stream1d", {}).get("notes", "")),
            reference_source=reference_source,
            num_steps=int(payload.get("num_steps", len(flow.upstream_q_cfs))),
            coupling_mode=coupling,
            dt_seconds=float(payload.get("dt", flow.interval_seconds)),
            overall_max_tolerance_ft=(
                float(compare_cfg["overall_max_tolerance_ft"])
                if compare_cfg.get("overall_max_tolerance_ft") is not None
                else None
            ),
        )
        report_fmt = getattr(args, "format", "table")
        enforce = scenario.enforce_tolerance
        compare_notes = str(compare_cfg.get("notes", ""))
        if report_fmt in ("table", "both"):
            print(
                format_unsteady_timeseries_report(
                    report,
                    enforce_tolerance=enforce,
                    compare_notes=compare_notes,
                )
            )
        if report_fmt in ("matrix", "both"):
            if report_fmt == "both":
                print()
            print(
                format_unsteady_timeseries_matrix_report(
                    report,
                    enforce_tolerance=enforce,
                    compare_notes=compare_notes,
                )
            )
        if live_status and live_status != "skipped (use --live-ras to attempt)":
            print(f"Live HEC-RAS: {live_status}")
        return 1 if enforce and not report.passed else 0

    report = compare_unsteady_peak_wsel(
        scenario_id=scenario.id,
        title=scenario.title,
        tolerance_ft=scenario.tolerance_ft,
        checkpoints_rm=[float(rm) for rm in checkpoints],
        observed_hwm=reference_peaks,
        cross_sections=xs_list,
        wsel_time_series=wsel_time_series,
        rm_to_index=rm_to_index,
        mapping_notes=str(scenario.raw.get("stream1d", {}).get("notes", "")),
        reference_source=reference_source,
        num_steps=int(payload.get("num_steps", len(flow.upstream_q_cfs))),
        coupling_mode=coupling,
        quantity=quantity,
    )
    print(format_unsteady_report(report, quantity=quantity))
    if live_status and live_status != "skipped (use --live-ras to attempt)":
        print(f"Live HEC-RAS: {live_status}")
    return 0 if report.passed else 1


def _parse_float_list(raw: str | None) -> list[float] | None:
    if raw is None:
        return None
    parts = [p.strip() for p in raw.split(",") if p.strip()]
    if not parts:
        return None
    return [float(p) for p in parts]


def main() -> int:
    parser = argparse.ArgumentParser(description="Linked HEC-RAS vs STREAM-1D verification")
    parser.add_argument(
        "--scenario",
        type=Path,
        default=_default_scenario(),
        help="Scenario manifest JSON",
    )
    parser.add_argument(
        "--live-ras",
        action="store_true",
        help="Attempt live HEC-RAS re-run when plan file is bundled (optional)",
    )
    parser.add_argument(
        "--checkpoints-rm",
        type=str,
        default=None,
        help="Override compare.checkpoints_rm (comma-separated river miles)",
    )
    parser.add_argument(
        "--time-checkpoints-hr",
        type=str,
        default=None,
        help="Override compare.time_checkpoints_hr (comma-separated hours)",
    )
    parser.add_argument(
        "--reference-file",
        type=Path,
        default=None,
        help="Override reference JSON for wsel_timeseries compare",
    )
    parser.add_argument(
        "--format",
        choices=("table", "matrix", "both"),
        default="table",
        help="Report layout for wsel_timeseries compare (default: table)",
    )
    args = parser.parse_args()

    try:
        import stream1d  # noqa: F401
    except ImportError:
        print(
            "ERROR: stream1d Python extension not found.\n"
            "Build with: maturin develop --features python",
            file=sys.stderr,
        )
        return 2

    scenario_path = args.scenario.resolve()
    if not scenario_path.is_file():
        print(f"ERROR: scenario not found: {scenario_path}", file=sys.stderr)
        return 2

    scenario = load_scenario(scenario_path)
    missing = _validate_linked_files(scenario)
    if missing:
        for msg in missing:
            print(f"ERROR: {msg}", file=sys.stderr)
        return 2

    if scenario.mode == "steady":
        return _run_steady(scenario, args)
    if scenario.mode == "unsteady":
        return _run_unsteady(scenario, args)

    print(f"ERROR: unsupported scenario mode: {scenario.mode}", file=sys.stderr)
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
