"""Emit HEC-RAS project files (.g01, .u02, .pXX, .prj) from a parity case."""

from __future__ import annotations

from datetime import datetime, timedelta
from pathlib import Path
from typing import Any

from .parity_case import (
    CulvertSpec,
    CrossSectionSpec,
    ParityCase,
    plan_interval_seconds,
    resolve_cross_sections,
    resolve_culverts,
    upstream_q_series,
)


def _fmt_rm(rm: float) -> str:
    return f"{rm:.3f}".rstrip("0").rstrip(".") if not rm.is_integer() else f"{int(rm)}"


def _chunk_floats(values: list[float], per_line: int = 5) -> list[str]:
    lines: list[str] = []
    for i in range(0, len(values), per_line):
        chunk = values[i : i + per_line]
        lines.append(" ".join(f"{v:8g}" if abs(v) >= 1 else f"{v:8.4f}" for v in chunk))
    return lines


def _sta_elev_lines(xs: CrossSectionSpec) -> list[str]:
    pairs: list[float] = []
    for x, y in zip(xs.x, xs.y):
        pairs.extend([x, y])
    out = [f"#Sta/Elev= {len(xs.x)} "]
    out.extend(_chunk_floats(pairs, 5))
    return out


def _mann_lines(xs: CrossSectionSpec) -> list[str]:
    out = [f"#Mann= {len(xs.n_stations)} , 0 , 0 "]
    rows: list[float] = []
    for sta, n in zip(xs.n_stations, xs.n_values):
        rows.extend([sta, n, 0.0])
    out.extend(_chunk_floats(rows, 3))
    return out


def _ineff_lines(xs: CrossSectionSpec) -> list[str]:
    if not xs.ineff_blocks:
        return []
    out = [f"#XS Ineff= {len(xs.ineff_blocks)} , 0 "]
    rows: list[float] = []
    for lo, hi, elev in xs.ineff_blocks:
        rows.extend([lo, hi, elev])
    out.extend(_chunk_floats(rows, 3))
    out.append("Permanent Ineff=")
    out.append("       F       F")
    return out


def _emit_cross_section(xs: CrossSectionSpec, river: str, reach: str) -> list[str]:
    rm_s = _fmt_rm(xs.rm)
    lines = [
        f"Type RM Length L Ch R = 1 ,{rm_s}  ,{int(xs.lob)},{int(xs.ch)},{int(xs.rob)}",
        "BEGIN DESCRIPTION:",
        xs.description or f"River mile {rm_s}",
        "END DESCRIPTION:",
        *_sta_elev_lines(xs),
        *_mann_lines(xs),
    ]
    if xs.ineff_blocks:
        lines.extend(_ineff_lines(xs))
    if xs.bank_left is not None and xs.bank_right is not None:
        lines.append(f"Bank Sta={xs.bank_left:g},{xs.bank_right:g}")
    lines.extend(
        [
            "XS Rating Curve= 0 ,0",
            "XS HTab Horizontal Distribution= 5 , 5 , 5 ",
            f"Exp/Cntr={xs.coeff_expansion},{xs.coeff_contraction}",
            "",
        ]
    )
    return lines


def _deck_profile_for_culvert(
    culvert: CulvertSpec,
    cross_sections: list[CrossSectionSpec],
) -> tuple[list[float], list[float], list[float]]:
    """Build deck station/elevation arrays from adjacent embankment XS."""
    ordered = sorted(cross_sections, key=lambda xs: xs.rm, reverse=True)
    us_xs = next((xs for xs in ordered if xs.rm > culvert.rm), ordered[0])
    ds_xs = next((xs for xs in reversed(ordered) if xs.rm < culvert.rm), ordered[-1])
    stations = list(us_xs.x)
    crest = culvert.crest_elev
    if crest is None:
        crest = max(max(us_xs.y), max(ds_xs.y)) - 0.5
    low_elevs = []
    high_elevs = []
    for sta in stations:
        y_us = us_xs.y[us_xs.x.index(sta)] if sta in us_xs.x else us_xs.y[0]
        y_ds = ds_xs.y[ds_xs.x.index(sta)] if sta in ds_xs.x else ds_xs.y[0]
        low = min(y_us, y_ds, crest - 0.1)
        low_elevs.append(low)
        high_elevs.append(crest)
    return stations, low_elevs, high_elevs


def _default_barrel_centerlines(culvert: CulvertSpec) -> list[tuple[float, float]]:
    """HEC-RAS centerline station pairs (upstream, downstream) per barrel."""
    if culvert.barrel_centerlines:
        return culvert.barrel_centerlines
    if culvert.num_barrels <= 1:
        return [(1000.0, 1000.0)]
    # HEC Example 4 twin-barrel spacing in a ~1000-ft channel crossing.
    if culvert.num_barrels == 2:
        return [(988.5, 988.5), (1011.5, 1011.5)]
    spacing = 11.0
    center = 1000.0
    half_span = (culvert.num_barrels - 1) * spacing / 2.0
    return [
        (center - half_span + i * spacing, center - half_span + i * spacing)
        for i in range(culvert.num_barrels)
    ]


def _format_barrel_station_line(centerlines: list[tuple[float, float]]) -> str:
    values: list[float] = []
    for us_sta, ds_sta in centerlines:
        values.extend([us_sta, ds_sta])
    parts = [f"{v:8.2f}" for v in values]
    return "".join(parts)


def _emit_culvert(
    culvert: CulvertSpec,
    cross_sections: list[CrossSectionSpec],
) -> list[str]:
    rm_s = _fmt_rm(culvert.rm)
    stations, low_elevs, high_elevs = _deck_profile_for_culvert(culvert, cross_sections)
    crest = culvert.crest_elev or max(high_elevs)
    centerlines = _default_barrel_centerlines(culvert)
    us_distance = 5.0

    if culvert.num_barrels > 1:
        culvert_line = (
            f"Multiple Barrel Culv={culvert.hec_shape},{culvert.rise:g},{culvert.span:g},"
            f"{culvert.length:g},{culvert.roughness_n:g},{culvert.entrance_loss_coeff:g},"
            f"{culvert.exit_loss_coeff:g},{culvert.chart},{culvert.scale},{culvert.z_up:g},"
            f"{culvert.z_down:g},{culvert.num_barrels},{culvert.name} , 0 ,{us_distance:g}"
        )
        culvert_lines = [culvert_line, _format_barrel_station_line(centerlines)]
    else:
        us_sta, ds_sta = centerlines[0]
        culvert_payload = (
            f"{culvert.hec_shape},{culvert.rise:g},{culvert.span:g},{culvert.length:g},"
            f"{culvert.roughness_n:g},{culvert.entrance_loss_coeff:g},{culvert.exit_loss_coeff:g},"
            f"{culvert.chart},{culvert.scale},{culvert.z_up:g},{us_sta:g},{culvert.z_down:g},"
            f"{ds_sta:g},{culvert.name} , 0 ,{us_distance:g}"
        )
        culvert_lines = [f"Culvert={culvert_payload}", _format_barrel_station_line(centerlines)]

    lines = [
        f"Type RM Length L Ch R = 2 ,{rm_s}  ,,,",
        "BEGIN DESCRIPTION:",
        culvert.name,
        "END DESCRIPTION:",
        "Bridge Culvert-0,0,1,-1, 0 ",
        "Deck Dist Width WeirC Skew NumUp NumDn MinLoCord MaxHiCord MaxSubmerge Is_Ogee",
        f"10,40,2.6,0, 8, 8, {crest:g}, , 0.95, 0, 2,2,,",
        "     " + "     ".join(f"{s:6g}" for s in stations),
        "    " + "    ".join(f"{e:5g}" for e in low_elevs),
        "                                                                ",
        "     " + "     ".join(f"{s:6g}" for s in stations),
        "    " + "    ".join(f"{e:5g}" for e in high_elevs),
        "                                                                ",
        *culvert_lines,
    ]
    if culvert.roughness_n_bottom is not None:
        lines.append(f"Culvert Bottom n={culvert.roughness_n_bottom:g}")
    if culvert.depth_bottom_n > 0:
        lines.append(f"Culvert Bottom Depth={culvert.depth_bottom_n:g}")
    lines.extend(
        [
            "BC Design=,, 0 ,, 0 ,,,,,,",
            f"BC HTab HWMax={max(high_elevs):g}",
            "BC Use User HTab Curves=0",
            "BC User HTab FreeFlow(D)= 0 ",
            "",
        ]
    )
    return lines


def _merge_reach_features(
    cross_sections: list[CrossSectionSpec],
    culverts: list[CulvertSpec],
) -> list[tuple[str, Any]]:
    """Ordered upstream→downstream: ('xs', spec) and ('culvert', spec)."""
    items: list[tuple[float, int, str, Any]] = []
    for xs in cross_sections:
        items.append((xs.rm, 0, "xs", xs))
    for culvert in culverts:
        items.append((culvert.rm, 1, "culvert", culvert))
    items.sort(key=lambda row: (-row[0], row[1]))
    return [(kind, payload) for _, _, kind, payload in items]


def emit_g01(case: ParityCase, path: Path) -> None:
    cross_sections = resolve_cross_sections(case)
    culverts = resolve_culverts(case)
    river = case.river
    reach = case.reach

    lines = [
        f"Geom Title={case.title}",
        "Program Version=5.00",
        "Viewing Rectangle= 0.0 , 1.0 , 1.0 , 0.0 ",
        "",
        f"River Reach={river:<16},{reach:<16}",
        "Reach XY= 2 ",
        "       0.0       0.0       1.0       0.0",
        "       1.0       1.0",
        "Rch Text X Y=0.5,0.5",
        "Reverse River Text= 0 ",
        "",
    ]

    for kind, payload in _merge_reach_features(cross_sections, culverts):
        if kind == "xs":
            lines.extend(_emit_cross_section(payload, river, reach))
        else:
            lines.extend(_emit_culvert(payload, cross_sections))

    lines.extend(
        [
            "LCMann Time=Dec/30/1899 00:00:00",
            "LCMann Region Time=Dec/30/1899 00:00:00",
            "LCMann Table=0",
            "Chan Stop Cuts=-1 ",
            "",
            "Use User Specified Reach Order=0",
            "GIS Ratio Cuts To Invert=-1",
            "GIS Limit At Bridges=0",
            "Composite Channel Slope=5",
            "",
        ]
    )
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("\n".join(lines), encoding="utf-8")


def _interval_label(seconds: float) -> str:
    if abs(seconds - round(seconds / 3600) * 3600) < 1e-6:
        hours = int(round(seconds / 3600))
        return "1HOUR" if hours == 1 else f"{hours}HOUR"
    if abs(seconds - round(seconds / 60) * 60) < 1e-6:
        mins = int(round(seconds / 60))
        return "1MIN" if mins == 1 else f"{mins}MIN"
    return "1SEC"


def emit_u02(case: ParityCase, path: Path) -> None:
    u = case.unsteady_cfg()
    river = case.river
    reach = case.reach
    q_series, interval = upstream_q_series(case)
    initial_q = float(u.get("initial_flow_cfs", q_series[0]))
    us_rm = float(u.get("upstream_rm", resolve_cross_sections(case)[0].rm))
    ds_rm = float(u.get("downstream_rm", resolve_cross_sections(case)[-1].rm))

    lines = [
        f"Flow Title={case.title} unsteady",
        "Program Version=5.00",
        "Use Restart= 0 ",
        f"Initial Flow Loc={river:<16},{reach:<16},{us_rm:g}    ,{initial_q:g}",
    ]

    lines.append(
        f"Boundary Location={river:<16},{reach:<16},{us_rm:g}    ,        ,                ,                ,                "
    )
    lines.append(f"Interval={_interval_label(interval)}")
    lines.append(f"Flow Hydrograph= {len(q_series)} ")
    lines.extend(_chunk_floats(q_series, 10))

    ds = u.get("downstream", {})
    ds_type = str(ds.get("type", "stage")).lower()
    lines.append(
        f"Boundary Location={river:<16},{reach:<16},{ds_rm:g}     ,        ,                ,                ,                "
    )
    if ds_type == "friction_slope":
        slope = float(ds.get("slope", 0.001))
        lines.append(f"Friction Slope={slope:g}")
    elif ds_type == "rating":
        rq = [float(v) for v in ds["rating_q_cfs"]]
        rw = [float(v) for v in ds["rating_wsel_ft"]]
        lines.append(f"Rating Curve= {len(rq)} ")
        for q, w in zip(rq, rw):
            lines.append(f" {q:g} {w:g}")
    else:
        if "wsel_hydrograph_ft" in ds:
            stages = [float(v) for v in ds["wsel_hydrograph_ft"]]
            if len(stages) != len(q_series):
                from .unsteady_bc import resample_series

                stages = resample_series(stages, len(q_series))
            wsel = stages[0]
            lines.append(f"Stage Hydrograph= {len(stages)} ")
            lines.extend(_chunk_floats(stages, 10))
        else:
            wsel = float(ds.get("wsel_ft", 30.51))
            lines.append(f"Stage Hydrograph= {len(q_series)} ")
            lines.extend(_chunk_floats([wsel] * len(q_series), 10))

    lines.append("DSS File=dss")
    lines.append("Use DSS=False")
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


_HEC_MONTHS = ("JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV", "DEC")


def _fmt_hec_date(dt: datetime) -> str:
    return f"{dt.day:02d}{_HEC_MONTHS[dt.month - 1]}{dt.year}"


def _fmt_hec_time(dt: datetime) -> str:
    return f"{dt.hour:02d}{dt.minute:02d}"


def _simulation_date_line(case: ParityCase) -> str:
    """HEC-RAS plan Simulation Date=startDate,startTime,endDate,endTime (required for unsteady)."""
    duration_hr = float(case.unsteady_cfg().get("duration_hours", 48.0))
    start = datetime(2000, 1, 1, 0, 0)
    end = start + timedelta(hours=duration_hr)
    return (
        f"Simulation Date={_fmt_hec_date(start)},{_fmt_hec_time(start)},"
        f"{_fmt_hec_date(end)},{_fmt_hec_time(end)}"
    )


def emit_plan(case: ParityCase, path: Path, *, plan_number: str = "02") -> None:
    plan = case.plan_cfg()
    comp = plan_interval_seconds(case)
    out = plan.get("output_interval", plan.get("computation_interval", "1HOUR"))
    if isinstance(out, (int, float)):
        out_seconds = float(out)
        out_label = _interval_label(out_seconds)
    else:
        out_label = str(out)

    theta = float(plan.get("theta", 1.0))
    friction_method = int(plan.get("unsteady_friction_slope_method", 2))
    stem = path.parent.name

    lines = [
        f"Plan Title={case.title}",
        "Program Version=5.00",
        f"Short Identifier={case.id[:12]:<12}",
        _simulation_date_line(case),
        "Geom File=g01",
        "Flow File=u02",
        "Subcritical Flow",
        "K Sum by GR= 0 ",
        f"Computation Interval={_interval_label(comp)}",
        f"Output Interval={out_label}",
        "Run HTab= 1 ",
        "Run UNet= 1 ",
        "Run PostProcess= 1 ",
        f"UNET Theta= {theta:g} ",
        f"Unsteady Friction Slope Method= {friction_method} ",
        "UNET 1D Methodology=Finite Difference",
        "UNET Froude Reduction=False",
        "UNET MxIter= 20 ",
        "UNET ZTol= 0.02 ",
        "",
    ]
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("\n".join(lines), encoding="utf-8")


def emit_prj(case: ParityCase, path: Path) -> None:
    """Write .prj using HEC-RAS short file refs (g01/u02/p02), not stem-prefixed names."""
    lines = [
        f"Proj Title={case.title}",
        "Program Version=5.00",
        "Current Plan=p02",
        f"Default Exp/Contr={resolve_cross_sections(case)[0].coeff_expansion:g},"
        f"{resolve_cross_sections(case)[0].coeff_contraction:g}",
        "Geom File=g01",
        "Unsteady File=u02",
        "Plan File=p02",
        "",
    ]
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("\n".join(lines), encoding="utf-8")


def emit_hecras_project(case: ParityCase, project_dir: Path) -> dict[str, Path]:
    """Write a complete HEC-RAS project directory for the case."""
    stem = case.id
    project_dir.mkdir(parents=True, exist_ok=True)
    files = {
        "geometry": project_dir / f"{stem}.g01",
        "unsteady_flow": project_dir / f"{stem}.u02",
        "plan": project_dir / f"{stem}.p02",
        "project": project_dir / f"{stem}.prj",
    }
    emit_g01(case, files["geometry"])
    emit_u02(case, files["unsteady_flow"])
    emit_plan(case, files["plan"])
    emit_prj(case, files["project"])
    return files


def emit_linked_scenario(case: ParityCase, oracle_root: Path, project_dir: Path) -> dict[str, Any]:
    """Build a linked-verify scenario manifest for the emitted project."""
    rel_dir = project_dir.relative_to(oracle_root)
    stem = case.id
    compare = case.compare_cfg()
    stream1d = case.stream1d_cfg()
    ref_rel = str(rel_dir / "reference_wsel.json")

    return {
        "schema_version": 1,
        "id": f"{case.id}_parity",
        "title": f"{case.title} (parity case)",
        "mode": "unsteady",
        "parity_program": {
            "chunk": 5,
            "certification": "diagnostic",
            "notes": f"Auto-generated from cases/{case.id}.json",
        },
        "linked_project": {
            "directory": str(rel_dir).replace("\\", "/"),
            "geometry": f"{stem}.g01",
            "unsteady_flow": f"{stem}.u02",
            "plan": f"{stem}.p02",
            "plan_number": "02",
            "validate_files": True,
        },
        "stream1d": {
            "mapper": "generic_unsteady_mapper.build_generic_unsteady_inputs",
            "coupling_mode": int(stream1d.get("coupling_mode", 0)),
            "unsteady_friction_slope_method": stream1d.get("unsteady_friction_slope_method"),
            "num_slices": stream1d.get("num_slices"),
            "max_spacing": stream1d.get("max_spacing"),
        },
        "reference": {
            "source": "linked_json_timeseries",
            "file": ref_rel.replace("\\", "/"),
            "live_ras_optional": True,
        },
        "compare": {
            "quantity": compare.get("quantity", "wsel_timeseries"),
            "match_by": "river_mile",
            "checkpoints_rm": compare.get("checkpoints_rm", []),
            "time_checkpoints_hr": compare.get("time_checkpoints_hr", [0, 48]),
        },
        "tolerance_ft": case.tolerance_ft,
        "quantity": compare.get("quantity", "wsel_timeseries"),
    }
