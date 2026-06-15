"""ConSpan steady profile reference keyed by river mile (linked oracle)."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

import stream1d as st

ORACLE_ROOT = Path(__file__).resolve().parents[1]
ENGINE_VERIFICATION = ORACLE_ROOT.parent
CONSPAN_PROJECT_PATH = ENGINE_VERIFICATION / "fixtures" / "conspan_project_12.json"


def _load_conspan_project() -> dict[str, Any]:
    with CONSPAN_PROJECT_PATH.open("r", encoding="utf-8") as fh:
        return json.load(fh)


def _rm_matches(a: float, b: float, tol: float = 1e-3) -> bool:
    return abs(float(a) - float(b)) <= tol


def rm_to_conspan_station(rm: float) -> float | None:
    """Map river mile to ConSpan fixture river station."""
    for xs in _load_conspan_project()["geometry_data"]:
        if _rm_matches(float(xs["rm"]), rm):
            return float(xs["station"])
    return None


def cross_section_from_fixture_row(row: dict[str, Any]) -> st.CrossSection:
    kwargs: dict[str, Any] = {
        "station": float(row["station"]),
        "x": [float(v) for v in row["x"]],
        "y": [float(v) for v in row["y"]],
        "n_stations": [float(v) for v in row["n_stations"]],
        "n_values": [float(v) for v in row["n_values"]],
        "unit_system": row.get("unit_system", "USCustomary"),
        "is_overbank": row.get("is_overbank"),
    }
    if "coeff_contraction" in row:
        kwargs["coeff_contraction"] = float(row["coeff_contraction"])
    if "coeff_expansion" in row:
        kwargs["coeff_expansion"] = float(row["coeff_expansion"])
    return st.CrossSection(**kwargs)


def load_conspan_cross_sections_for_rms(rms: list[float]) -> list[st.CrossSection]:
    """Load fixture XS for each requested RM in order."""
    by_rm = {float(xs["rm"]): xs for xs in _load_conspan_project()["geometry_data"]}
    out: list[st.CrossSection] = []
    for rm in rms:
        row = None
        for key, xs in by_rm.items():
            if _rm_matches(key, rm):
                row = xs
                break
        if row is None:
            raise ValueError(f"No ConSpan fixture XS for RM {rm}")
        out.append(cross_section_from_fixture_row(row))
    return out


def conspan_solver_params() -> dict[str, Any]:
    params = _load_conspan_project().get("parameters", {})
    return {
        "num_slices": int(params.get("vertical_slices", 100)),
        "max_spacing": float(params.get("max_spacing", 100.0)),
    }


def load_all_conspan_cross_sections() -> list[st.CrossSection]:
    """Full ConSpan reach geometry (upstream → downstream by station)."""
    rows = sorted(
        _load_conspan_project()["geometry_data"],
        key=lambda xs: float(xs["station"]),
        reverse=True,
    )
    return [cross_section_from_fixture_row(row) for row in rows]


def conspan_culvert_fields() -> dict[str, Any]:
    """Culvert arrays from verified ConSpan project fixture."""
    from .culvert_mapper import culvert_fields_from_fixture_rows

    return culvert_fields_from_fixture_rows(_load_conspan_project().get("culvert_stations", []))


def conspan_geometry_rms_upstream_first() -> list[float]:
    """River miles in payload index order (upstream first)."""
    rows = sorted(
        _load_conspan_project()["geometry_data"],
        key=lambda xs: float(xs["station"]),
        reverse=True,
    )
    return [float(row["rm"]) for row in rows]


def rm_to_conspan_payload_index(rm: float) -> int | None:
    for idx, row_rm in enumerate(conspan_geometry_rms_upstream_first()):
        if _rm_matches(row_rm, rm):
            return idx
    return None


def peak_wsel_by_rm(profile_name: str = "50 yr") -> dict[float, float]:
    profiles_path = ENGINE_VERIFICATION / "fixtures" / "hecras_conspan_profiles.json"
    with profiles_path.open("r", encoding="utf-8") as fh:
        profiles_file = json.load(fh)
    profile = next(
        p for p in profiles_file["profiles"] if str(p["name"]).lower() == profile_name.lower()
    )
    station_wsel = {float(k): float(v) for k, v in profile["expected_wsel_ft"].items()}
    out: dict[float, float] = {}
    for xs in _load_conspan_project()["geometry_data"]:
        sta = float(xs["station"])
        if sta in station_wsel:
            out[float(xs["rm"])] = station_wsel[sta]
    return out
