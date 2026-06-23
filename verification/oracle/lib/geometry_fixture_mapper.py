"""
Map streams1d_geometry JSON exports to steady solver payloads.

Mirrors workspace_payloads.js (compileBridgePayload, compileCulvertPayload, XsStation).
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any, Mapping

from stream1d.import_utils import cross_section_from_dict

STATION_MATCH_TOLERANCE = 0.02
G_US = 32.174  # ft/s²


def load_geometry_fixture(path: Path | str) -> dict[str, Any]:
    with Path(path).open("r", encoding="utf-8") as fh:
        data = json.load(fh)
    if data.get("format") != "streams1d_geometry":
        raise ValueError(f"expected streams1d_geometry export, got {data.get('format')!r}")
    return data


def get_river_station(xs: Mapping[str, Any]) -> float:
    if xs.get("river_station") is not None:
        return float(xs["river_station"])
    if xs.get("original_station") is not None:
        return float(xs["original_station"])
    if xs.get("station") is not None:
        return float(xs["station"])
    if xs.get("rm") is not None:
        return float(xs["rm"])
    return 0.0


def river_station_to_computational(river_station: float, cross_sections: list[Mapping[str, Any]]) -> float:
    if not cross_sections:
        return float(river_station)
    sorted_xs = sorted(cross_sections, key=lambda xs: get_river_station(xs), reverse=True)
    rs = float(river_station)
    max_rs = get_river_station(sorted_xs[0])
    min_rs = get_river_station(sorted_xs[-1])
    if rs >= max_rs:
        return float(sorted_xs[0]["station"])
    if rs <= min_rs:
        return float(sorted_xs[-1]["station"])
    for i in range(len(sorted_xs) - 1):
        us = sorted_xs[i]
        ds = sorted_xs[i + 1]
        us_rs = get_river_station(us)
        ds_rs = get_river_station(ds)
        if rs <= us_rs and rs >= ds_rs:
            dx = us_rs - ds_rs
            if dx <= 1e-6:
                return float(us["station"])
            t = (us_rs - rs) / dx
            return float(us["station"]) + t * (float(ds["station"]) - float(us["station"]))
    return rs


def find_xs_by_river_station(
    river_station: float,
    cross_sections: list[Mapping[str, Any]],
    tolerance: float = 0.5,
) -> Mapping[str, Any] | None:
    best = None
    min_diff = float("inf")
    for xs in cross_sections:
        diff = abs(get_river_station(xs) - float(river_station))
        if diff < min_diff:
            min_diff = diff
            best = xs
    return best if min_diff <= tolerance else None


def resolve_default_bounding_stations(
    structure_river_st: float,
    cross_sections: list[Mapping[str, Any]],
) -> tuple[float, float]:
    st = float(structure_river_st)
    approach: float | None = None
    departure: float | None = None
    for xs in cross_sections:
        rs = get_river_station(xs)
        if rs > st + 0.05:
            approach = rs if approach is None or rs < approach else approach
        if rs < st - 0.05:
            departure = rs if departure is None or rs > departure else departure
    at_struct = find_xs_by_river_station(st, cross_sections, tolerance=0.05)
    at_rs = get_river_station(at_struct) if at_struct else st
    if approach is None:
        approach = at_rs
    if departure is None:
        departure = at_rs
    return approach, departure


def resolve_bounding_stations(
    struct: Mapping[str, Any],
    cross_sections: list[Mapping[str, Any]],
) -> tuple[float, float]:
    st = float(struct["station"])
    approach_default, departure_default = resolve_default_bounding_stations(st, cross_sections)

    def pick(explicit_flag: str | None, reach_key: str, default: float) -> float:
        if explicit_flag is not None:
            if struct.get(explicit_flag) and _is_set(struct.get(reach_key)):
                return float(struct[reach_key])
            return default
        if _is_set(struct.get(reach_key)):
            return float(struct[reach_key])
        return default

    approach = pick("use_explicit_approach", "approach_reach_station", approach_default)
    departure = pick("use_explicit_departure", "departure_reach_station", departure_default)
    return approach, departure


def _is_set(value: Any) -> bool:
    if value is None or value == "":
        return False
    try:
        float(value)
        return True
    except (TypeError, ValueError):
        return False


def _guide_banks_obj(gb: Mapping[str, Any] | None) -> dict[str, Any] | None:
    if not gb:
        return None
    left = gb.get("left_toe") or {}
    right = gb.get("right_toe") or {}
    ret: dict[str, Any] = {}
    if _is_set(left.get("station")):
        ret["left_toe"] = {
            "station": float(left["station"]),
            "elevation": float(left.get("elevation", 0.0)),
        }
    if _is_set(right.get("station")):
        ret["right_toe"] = {
            "station": float(right["station"]),
            "elevation": float(right.get("elevation", 0.0)),
        }
    return ret or None


def compile_bridge_payload(
    bridges: list[Mapping[str, Any]],
    cross_sections: list[Mapping[str, Any]],
) -> dict[str, Any]:
    if not bridges:
        return {}

    to_comp = lambda rs: river_station_to_computational(float(rs), cross_sections)

    bridge_stations = [to_comp(b["station"]) for b in bridges]
    bridge_low_chords = [float(b["low_chord"]) for b in bridges]
    bridge_high_chords = [float(b["high_chord"]) for b in bridges]
    bridge_pier_widths = [float(b["pier_width"]) for b in bridges]
    bridge_num_piers = [int(b["num_piers"]) for b in bridges]
    bridge_pier_shapes = [int(b["pier_shape"]) for b in bridges]
    bridge_weir_coeffs = [float(b["weir_coeff"]) for b in bridges]
    bridge_orifice_coeffs = [float(b["orifice_coeff"]) for b in bridges]

    deck_vent_left: list[list[float]] = []
    deck_vent_right: list[list[float]] = []
    deck_vent_centers: list[list[float]] = []
    deck_vent_widths: list[list[float]] = []
    deck_vent_inverts: list[list[float]] = []
    deck_vent_soffits: list[list[float]] = []
    deck_vent_cds: list[list[float]] = []
    deck_vent_types: list[list[int]] = []

    deck_stations: list[list[float]] = []
    deck_lows: list[list[float]] = []
    deck_highs: list[list[float]] = []
    pier_stations: list[list[float]] = []

    abut_l_w: list[float] = []
    abut_r_w: list[float] = []
    abut_l_st: list[float] = []
    abut_r_st: list[float] = []
    abut_l_top: list[float] = []
    abut_r_top: list[float] = []
    abut_l_prof_st: list[list[float]] = []
    abut_l_prof_el: list[list[float]] = []
    abut_r_prof_st: list[list[float]] = []
    abut_r_prof_el: list[list[float]] = []

    approach_reach: list[float] = []
    departure_reach: list[float] = []
    approach_guide: list[dict[str, Any] | None] = []
    departure_guide: list[dict[str, Any] | None] = []

    for b in bridges:
        xs = (
            b.get("xs_up")
            if b.get("use_explicit_cuts") and b.get("xs_up")
            else find_xs_by_river_station(b["station"], cross_sections)
        )
        min_x = min(xs["x"]) if xs else 0.0
        origin = (
            float(b["opening_reach_station_origin"])
            if _is_set(b.get("opening_reach_station_origin"))
            else min_x
        )

        v_left: list[float] = []
        v_right: list[float] = []
        v_center: list[float] = []
        v_width: list[float] = []
        v_inv: list[float] = []
        v_soff: list[float] = []
        v_cd: list[float] = []
        v_type: list[int] = []
        for v in b.get("deck_vents") or []:
            left = float(v["left"]) - origin
            right = float(v["right"]) - origin
            v_left.append(left)
            v_right.append(right)
            v_center.append(0.5 * (left + right))
            v_width.append(right - left)
            v_inv.append(float(v["invert"]))
            v_soff.append(float(v["soffit"]))
            v_cd.append(float(v.get("cd", 0.8)))
            v_type.append(int(v.get("type", 0)))
        deck_vent_left.append(v_left)
        deck_vent_right.append(v_right)
        deck_vent_centers.append(v_center)
        deck_vent_widths.append(v_width)
        deck_vent_inverts.append(v_inv)
        deck_vent_soffits.append(v_soff)
        deck_vent_cds.append(v_cd)
        deck_vent_types.append(v_type)

        if b.get("deck_points_us"):
            deck_stations.append([float(pt["x"]) - origin for pt in b["deck_points_us"]])
            deck_lows.append([float(pt.get("low_chord", b["low_chord"])) for pt in b["deck_points_us"]])
            deck_highs.append([float(pt.get("high_chord", b["high_chord"])) for pt in b["deck_points_us"]])
        else:
            deck_stations.append([])
            deck_lows.append([])
            deck_highs.append([])

        if b.get("pier_stations"):
            pier_stations.append([float(s) for s in b["pier_stations"]])
        else:
            pier_stations.append([])

        if b.get("abutment_left_enabled"):
            abut_l_w.append(float(b.get("abutment_left_width", 0.0)))
            abut_l_st.append(float(b.get("abutment_left_station", min_x)) - origin)
            if b.get("abutment_left_use_profile"):
                abut_l_top.append(float(b.get("high_chord", 15.0)))
                abut_l_prof_st.append(
                    [float(s) - origin for s in (b.get("abutment_left_profile_stations") or [])]
                )
                abut_l_prof_el.append([float(e) for e in (b.get("abutment_left_profile_elevations") or [])])
            else:
                abut_l_top.append(
                    float(b["abutment_left_top_elevation"])
                    if _is_set(b.get("abutment_left_top_elevation"))
                    else float(b.get("high_chord", 15.0))
                )
                abut_l_prof_st.append([])
                abut_l_prof_el.append([])
        else:
            abut_l_w.append(0.0)
            abut_l_st.append(0.0)
            abut_l_top.append(float(b.get("high_chord", 15.0)))
            abut_l_prof_st.append([])
            abut_l_prof_el.append([])

        if b.get("abutment_right_enabled"):
            abut_r_w.append(float(b.get("abutment_right_width", 0.0)))
            abut_r_st.append(
                float(b["abutment_right_station"]) - origin
                if _is_set(b.get("abutment_right_station"))
                else 0.0
            )
            if b.get("abutment_right_use_profile"):
                abut_r_top.append(float(b.get("high_chord", 15.0)))
                abut_r_prof_st.append(
                    [float(s) - origin for s in (b.get("abutment_right_profile_stations") or [])]
                )
                abut_r_prof_el.append([float(e) for e in (b.get("abutment_right_profile_elevations") or [])])
            else:
                abut_r_top.append(
                    float(b["abutment_right_top_elevation"])
                    if _is_set(b.get("abutment_right_top_elevation"))
                    else float(b.get("high_chord", 15.0))
                )
                abut_r_prof_st.append([])
                abut_r_prof_el.append([])
        else:
            abut_r_w.append(0.0)
            abut_r_st.append(0.0)
            abut_r_top.append(float(b.get("high_chord", 15.0)))
            abut_r_prof_st.append([])
            abut_r_prof_el.append([])

        app_rs, dep_rs = resolve_bounding_stations(b, cross_sections)
        approach_reach.append(app_rs)
        departure_reach.append(dep_rs)
        approach_guide.append(_guide_banks_obj(b.get("approach_guide_banks")))
        departure_guide.append(_guide_banks_obj(b.get("departure_guide_banks")))

    payload: dict[str, Any] = {
        "bridge_stations": bridge_stations,
        "bridge_low_chords": bridge_low_chords,
        "bridge_high_chords": bridge_high_chords,
        "bridge_pier_widths": bridge_pier_widths,
        "bridge_num_piers": bridge_num_piers,
        "bridge_pier_shapes": bridge_pier_shapes,
        "bridge_weir_coeffs": bridge_weir_coeffs,
        "bridge_orifice_coeffs": bridge_orifice_coeffs,
        "bridge_deck_stations": deck_stations,
        "bridge_deck_low_elevations": deck_lows,
        "bridge_deck_high_elevations": deck_highs,
        "bridge_deck_vent_left_stations": deck_vent_left,
        "bridge_deck_vent_right_stations": deck_vent_right,
        "bridge_deck_vent_stations": deck_vent_centers,
        "bridge_deck_vent_widths": deck_vent_widths,
        "bridge_deck_vent_invert_elevations": deck_vent_inverts,
        "bridge_deck_vent_soffit_elevations": deck_vent_soffits,
        "bridge_deck_vent_discharge_coefficients": deck_vent_cds,
        "bridge_deck_vent_types": deck_vent_types,
        "bridge_abutment_left_widths": abut_l_w,
        "bridge_abutment_right_widths": abut_r_w,
        "bridge_abutment_left_stations": abut_l_st,
        "bridge_abutment_right_stations": abut_r_st,
        "bridge_abutment_left_top_elevations": abut_l_top,
        "bridge_abutment_right_top_elevations": abut_r_top,
        "bridge_abutment_left_top_profile_stations": abut_l_prof_st,
        "bridge_abutment_left_top_profile_elevations": abut_l_prof_el,
        "bridge_abutment_right_top_profile_stations": abut_r_prof_st,
        "bridge_abutment_right_top_profile_elevations": abut_r_prof_el,
        "bridge_approach_reach_stations": approach_reach,
        "bridge_departure_reach_stations": departure_reach,
    }
    if any(pier_stations):
        payload["bridge_pier_stations"] = pier_stations
    if any(g is not None for g in approach_guide):
        payload["bridge_approach_guide_banks"] = approach_guide
    if any(g is not None for g in departure_guide):
        payload["bridge_departure_guide_banks"] = departure_guide
    return payload


def build_cross_sections(geometry_rows: list[Mapping[str, Any]]) -> list:
    return [cross_section_from_dict(row) for row in geometry_rows]


def build_steady_payload(
    fixture: Mapping[str, Any],
    profile: Mapping[str, Any],
) -> dict[str, Any]:
    """Assemble a steady JSON payload matching the Stream1D web worker."""
    geometry = list(fixture["geometry_data"])
    params = fixture.get("parameters") or {}
    bridges = fixture.get("bridge_data") or []

    cross_sections = build_cross_sections(geometry)
    ds_bc_type = int(profile.get("downstream_bc_type", 0))
    us_bc_type = int(profile.get("upstream_bc_type", 0))
    payload: dict[str, Any] = {
        "cross_sections": [xs.to_dict() for xs in cross_sections],
        "flow_rate": float(profile["flow_rate_cfs"]),
        "num_slices": int(profile.get("num_slices", params.get("vertical_slices", 10))),
        "regime": int(profile.get("regime", params.get("flow_regime", 0))),
        "coeff_contraction": float(profile.get("coeff_contraction", 0.1)),
        "coeff_expansion": float(profile.get("coeff_expansion", 0.3)),
        "max_spacing": float(profile.get("max_spacing", params.get("max_spacing", 100.0))),
        "downstream_bc_type": ds_bc_type,
        "upstream_bc_type": us_bc_type,
    }
    if ds_bc_type == 0:
        payload["downstream_wsel"] = float(profile["downstream_wsel_ft"])
    elif ds_bc_type == 2:
        payload["downstream_bc_slope"] = float(profile.get("downstream_bc_slope", 0.01))
    if us_bc_type == 0 and profile.get("upstream_wsel_ft") is not None:
        payload["upstream_wsel"] = float(profile["upstream_wsel_ft"])
    elif us_bc_type == 2:
        payload["upstream_bc_slope"] = float(profile.get("upstream_bc_slope", 0.01))
    payload.update(compile_bridge_payload(bridges, geometry))
    return payload


def river_stations_upstream_first(geometry_rows: list[Mapping[str, Any]]) -> list[float]:
    return sorted((get_river_station(xs) for xs in geometry_rows), reverse=True)


def compute_egl(wsel_ft: float, velocity_fps: float) -> float:
    return wsel_ft + (velocity_fps * velocity_fps) / (2.0 * G_US)


def run_profile(fixture: Mapping[str, Any], profile: Mapping[str, Any]) -> dict[str, Any]:
    import stream1d as st

    payload = build_steady_payload(fixture, profile)
    result = st.solve_steady(payload)
    geometry = list(fixture["geometry_data"])
    rows: list[dict[str, Any]] = []
    for i, xs in enumerate(geometry):
        rs = get_river_station(xs)
        wsel = float(result["wsel"][i])
        vel = float(result["velocity"][i])
        rows.append(
            {
                "river_station": rs,
                "computational_station": float(xs["station"]),
                "wsel_ft": wsel,
                "critical_wsel_ft": float(result["critical_wsel"][i]),
                "velocity_fps": vel,
                "egl_ft": compute_egl(wsel, vel),
                "area_ft2": float(result["area"][i]),
                "top_width_ft": float(result["top_width"][i]),
                "froude": float(result["froude"][i]),
            }
        )
    return {
        "profile": profile.get("name", "profile"),
        "flow_rate_cfs": float(profile["flow_rate_cfs"]),
        "downstream_bc_type": int(profile.get("downstream_bc_type", 0)),
        "downstream_bc_slope": profile.get("downstream_bc_slope"),
        "downstream_wsel_ft": profile.get("downstream_wsel_ft"),
        "stations": rows,
    }
