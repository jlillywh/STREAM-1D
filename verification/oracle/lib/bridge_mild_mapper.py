"""Synthetic subcritical bridge reach for Chunk 6 implicit coupling gates."""

from __future__ import annotations

import json
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

import stream1d as st

from .unsteady_bc import steady_initial_wsel

ORACLE = Path(__file__).resolve().parents[1]
FIXTURE = ORACLE.parent / "fixtures" / "bridge_bu_bd_hecras.json"

# Mild subcritical pulse (m³/s); TW fixed below low chord (3.0 m vs 5.0 m).
MILD_PULSE_CMS = [15.0, 15.5, 16.0, 15.5, 15.0, 14.5, 15.0, 15.0]
# Stiff subcritical step (Chunk 7 face-lag probe): ±50% Q in one interval, short dt.
STIFF_PULSE_CMS = [12.0, 12.0, 22.0, 22.0, 12.0, 12.0, 15.0, 15.0]
DT_SECONDS = 300.0
STIFF_DT_SECONDS = 60.0
DOWNSTREAM_WSEL_M = 3.0

# Face-lag gate (ft): mode 0 vs mode 2 max |ΔWSEL| at BU/BD over all steps.
FACE_LAG_MODE2_SUFFICIENT_FT = 0.02


@dataclass
class BridgeMildFlowStub:
    upstream_q_cfs: list[float] = field(default_factory=list)
    observed_hwm: dict[float, float] = field(default_factory=dict)
    interval_seconds: float = DT_SECONDS
    initial_flow_cfs: float = 15.0
    downstream_rm: float | None = 0.0


def _channel_xs(station: float, bed: float, width: float) -> st.CrossSection:
    return st.CrossSection(
        station=station,
        x=[0.0, 0.0, width, width],
        y=[bed + 10.0, bed, bed, bed + 10.0],
        n_stations=[0.0],
        n_values=[0.03],
        unit_system="Metric",
    )


def _case_params(case: str) -> dict[str, Any]:
    data = json.loads(FIXTURE.read_text(encoding="utf-8"))
    for entry in data["cases"]:
        if case == "yarnell" and entry["name"] == "yarnell_explicit_bu_bd":
            return entry
        if case == "wspro" and entry["name"] == "wspro_narrow_opening_bu_bd":
            return entry
    raise ValueError(f"unknown bridge mild case: {case!r}")


def build_bridge_mild_unsteady_inputs(
    _project_dir: Path,
    *,
    case: str = "yarnell",
    coupling_mode: int = 0,
    pulse_profile: str = "mild",
) -> tuple[dict[str, Any], BridgeMildFlowStub]:
    """
    Build a synthetic metric reach with explicit BU/BD faces and optional mode-2 coupling.

    Checkpoints use reach station (m) as the oracle RM key: 52 (BU), 48 (BD), 25, 0 (DS).
    """
    cfg = _case_params(case)
    bed = 0.0
    channel_w = float(cfg["channel_width_m"])
    opening_w = float(cfg.get("opening_width_m") or channel_w)
    bu_sta = float(cfg["bu_station_m"])
    bd_sta = float(cfg["bd_station_m"])
    bridge_sta = float(cfg["bridge_center_station_m"])
    approach_len = bu_sta - bd_sta

    xs_us = _channel_xs(100.0, bed, channel_w)
    xs_bu = _channel_xs(bu_sta, bed, opening_w)
    xs_bd = _channel_xs(bd_sta, bed, opening_w)
    xs_mid = _channel_xs(25.0, bed, channel_w)
    xs_ds = _channel_xs(0.0, bed, channel_w)
    cross_sections = [xs_us, xs_bu, xs_bd, xs_mid, xs_ds]

    bridge_fields: dict[str, Any] = {
        "bridge_stations": [bridge_sta],
        "bridge_low_chords": [float(cfg["low_chord_m"])],
        "bridge_high_chords": [float(cfg["high_chord_m"])],
        "bridge_pier_widths": [float(cfg["pier_width_m"])],
        "bridge_num_piers": [int(cfg["num_piers"])],
        "bridge_pier_shapes": [0],
        "bridge_weir_coeffs": [1.44],
        "bridge_orifice_coeffs": [0.5],
        "bridge_low_flow_methods": [int(cfg["low_flow_method"])],
        "bridge_lengths": [approach_len],
        "bridge_friction_weighting": [1],
        "bridge_approach_friction_lengths": [approach_len],
        "bridge_departure_friction_lengths": [approach_len],
        "bridge_upstream_cross_sections": [xs_bu],
        "bridge_downstream_cross_sections": [xs_bd],
    }
    if int(cfg["low_flow_method"]) == 4:
        bridge_fields["bridge_wspro_coeffs"] = [0.8]

    if pulse_profile == "stiff":
        hydrograph = list(STIFF_PULSE_CMS)
        dt_seconds = STIFF_DT_SECONDS
    elif pulse_profile == "mild":
        hydrograph = list(MILD_PULSE_CMS)
        dt_seconds = DT_SECONDS
    else:
        raise ValueError(f"unknown pulse_profile: {pulse_profile!r}")

    base_q_cms = hydrograph[0]
    initial_wsel = steady_initial_wsel(
        cross_sections,
        base_q_cms,
        0,
        None,
        DOWNSTREAM_WSEL_M,
        max_spacing=50.0,
        num_slices=50,
        structure_fields=bridge_fields,
    )

    inputs = st.UnsteadyInputs(
        cross_sections=cross_sections,
        initial_wsel=initial_wsel,
        initial_q=[base_q_cms] * len(cross_sections),
        dt=dt_seconds,
        num_steps=len(hydrograph),
        upstream_q_hydrograph=hydrograph,
        downstream_wsel_hydrograph=[DOWNSTREAM_WSEL_M] * len(hydrograph),
        downstream_bc_type=0,
        theta=0.6,
        num_slices=50,
        max_spacing=50.0,
        coeff_contraction=0.3,
        coeff_expansion=0.1,
        structure_coupling_order=0,
        unsteady_structure_coupling_mode=coupling_mode,
        **bridge_fields,
    )

    flow = BridgeMildFlowStub(
        upstream_q_cfs=list(hydrograph),
        initial_flow_cfs=base_q_cms,
        interval_seconds=dt_seconds,
    )
    return inputs.to_dict(), flow


def station_to_payload_index(station_m: float, payload: dict[str, Any]) -> int | None:
    stations = [xs["station"] for xs in payload["cross_sections"]]
    best_idx = None
    best_dist = float("inf")
    for idx, sta in enumerate(stations):
        dist = abs(sta - station_m)
        if dist < best_dist:
            best_dist = dist
            best_idx = idx
    return best_idx if best_idx is not None and best_dist <= 0.05 else None
