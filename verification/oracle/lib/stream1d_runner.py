"""Build STREAM-1D inputs from linked fixtures and run the solver."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

import stream1d as st

from .scenario import LinkedScenario


def _build_cross_sections(project: dict[str, Any]) -> list[st.CrossSection]:
    rows: list[st.CrossSection] = []
    for xs in project["geometry_data"]:
        rows.append(
            st.CrossSection(
                station=float(xs["station"]),
                x=[float(v) for v in xs["x"]],
                y=[float(v) for v in xs["y"]],
                n_stations=[float(v) for v in xs["n_stations"]],
                n_values=[float(v) for v in xs["n_values"]],
                unit_system=xs.get("unit_system", "USCustomary"),
                is_overbank=xs.get("is_overbank"),
            )
        )
    return rows


def _build_culvert_fields(project: dict[str, Any]) -> dict[str, Any]:
    culverts = project.get("culvert_stations", [])
    return {
        "culvert_stations": [float(c["station"]) for c in culverts],
        "culvert_shape_types": [int(c["shape_type"]) for c in culverts],
        "culvert_spans": [float(c["span"]) for c in culverts],
        "culvert_rises": [float(c["rise"]) for c in culverts],
        "culvert_roughness_ns": [float(c["roughness_n"]) for c in culverts],
        "culvert_lengths": [float(c["length"]) for c in culverts],
        "culvert_entrance_loss_coeffs": [float(c["entrance_loss_coeff"]) for c in culverts],
        "culvert_exit_loss_coeffs": [float(c["exit_loss_coeff"]) for c in culverts],
        "culvert_barrels": [int(c.get("num_barrels", 1)) for c in culverts],
        "culvert_roughness_n_bottoms": [
            float(c.get("roughness_n_bottom", c["roughness_n"])) for c in culverts
        ],
        "culvert_depth_bottom_ns": [float(c.get("depth_bottom_n", 0.0)) for c in culverts],
        "culvert_depth_blockeds": [float(c.get("depth_blocked", 0.0)) for c in culverts],
    }


def load_project_bundle(scenario: LinkedScenario) -> tuple[dict[str, Any], dict[str, Any]]:
    project_path = scenario.resolve(scenario.raw["stream1d"]["project_fixture"])
    profiles_path = scenario.resolve(scenario.raw["stream1d"]["profiles_fixture"])
    with project_path.open("r", encoding="utf-8") as fh:
        project = json.load(fh)
    with profiles_path.open("r", encoding="utf-8") as fh:
        profiles = json.load(fh)
    return project, profiles


def run_steady_profiles(scenario: LinkedScenario) -> list[dict[str, Any]]:
    """Run STREAM-1D steady for each profile in the linked scenario."""
    project, profiles_file = load_project_bundle(scenario)
    cross_sections = _build_cross_sections(project)
    culvert_fields = _build_culvert_fields(project)
    station_list = [float(xs["station"]) for xs in project["geometry_data"]]
    params = project.get("parameters", {})

    runs: list[dict[str, Any]] = []
    for profile in profiles_file["profiles"]:
        inputs = st.SteadyInputs(
            cross_sections=cross_sections,
            flow_rate=float(profile["flow_rate_cfs"]),
            num_slices=int(params.get("vertical_slices", 100)),
            regime=int(params.get("flow_regime", 0)),
            downstream_wsel=float(profile["downstream_wsel_ft"]),
            max_spacing=float(params.get("max_spacing", 100.0)),
            downstream_bc_type=0,
            **culvert_fields,
        )
        result = st.solve_steady(inputs)
        wsel_by_station: dict[float, float] = {}
        for idx, station in enumerate(station_list):
            wsel_by_station[station] = float(result["wsel"][idx])
        runs.append(
            {
                "name": profile["name"],
                "flow_rate_cfs": float(profile["flow_rate_cfs"]),
                "downstream_wsel_ft": float(profile["downstream_wsel_ft"]),
                "wsel_by_station": wsel_by_station,
                "expected_wsel_ft": {
                    float(k): float(v) for k, v in profile["expected_wsel_ft"].items()
                },
            }
        )
    return runs
