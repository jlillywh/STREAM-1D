"""Canonical parity-case model — single source for HEC-RAS emit + STREAM-1D inputs."""

from __future__ import annotations

import json
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any


@dataclass
class CrossSectionSpec:
    rm: float
    lob: float
    ch: float
    rob: float
    x: list[float]
    y: list[float]
    n_stations: list[float]
    n_values: list[float]
    description: str = ""
    coeff_expansion: float = 0.3
    coeff_contraction: float = 0.1
    ineff_blocks: list[tuple[float, float, float]] = field(default_factory=list)
    bank_left: float | None = None
    bank_right: float | None = None


@dataclass
class CulvertSpec:
    rm: float
    hec_shape: int
    rise: float
    span: float
    length: float
    roughness_n: float
    entrance_loss_coeff: float
    exit_loss_coeff: float
    chart: int
    scale: int
    z_up: float
    z_down: float
    num_barrels: int = 1
    roughness_n_bottom: float | None = None
    depth_bottom_n: float = 0.0
    depth_blocked: float = 0.0
    crest_elev: float | None = None
    name: str = "Culvert # 1"
    barrel_centerlines: list[tuple[float, float]] = field(default_factory=list)


@dataclass
class ParityCase:
    raw: dict[str, Any]
    path: Path

    @property
    def id(self) -> str:
        return str(self.raw["id"])

    @property
    def title(self) -> str:
        return str(self.raw.get("title", self.id))

    @property
    def river(self) -> str:
        return str(self.raw.get("river", "Default River"))

    @property
    def reach(self) -> str:
        return str(self.raw.get("reach", "Default Reach"))

    @property
    def tolerance_ft(self) -> float:
        return float(self.raw.get("tolerance_ft", 0.5))

    def stream1d_cfg(self) -> dict[str, Any]:
        return dict(self.raw.get("stream1d", {}))

    def compare_cfg(self) -> dict[str, Any]:
        return dict(self.raw.get("compare", {}))

    def plan_cfg(self) -> dict[str, Any]:
        return dict(self.raw.get("plan", {}))

    def unsteady_cfg(self) -> dict[str, Any]:
        return dict(self.raw["unsteady"])


def _repo_root_from_case(path: Path) -> Path:
    oracle = path.parent.parent if path.parent.name == "cases" else path.parent
    return oracle.parents[1]


def _resolve_fixture_path(case_path: Path, rel: str) -> Path:
    root = _repo_root_from_case(case_path)
    candidate = (root / rel).resolve()
    if not candidate.is_file():
        raise FileNotFoundError(f"parity case fixture not found: {candidate}")
    return candidate


def _xs_from_fixture_row(row: dict[str, Any]) -> CrossSectionSpec:
    ch = float(row.get("ch", row.get("channel_length", 100.0)))
    return CrossSectionSpec(
        rm=float(row["rm"]),
        lob=float(row.get("lob", ch)),
        ch=ch,
        rob=float(row.get("rob", ch)),
        x=[float(v) for v in row["x"]],
        y=[float(v) for v in row["y"]],
        n_stations=[float(v) for v in row["n_stations"]],
        n_values=[float(v) for v in row["n_values"]],
        description=str(row.get("description", f"River mile {row['rm']}")),
        coeff_expansion=float(row.get("coeff_expansion", 0.3)),
        coeff_contraction=float(row.get("coeff_contraction", 0.1)),
        ineff_blocks=[
            (float(b[0]), float(b[1]), float(b[2]))
            for b in row.get("ineff_blocks", [])
        ],
        bank_left=float(row["bank_left"]) if "bank_left" in row else None,
        bank_right=float(row["bank_right"]) if "bank_right" in row else None,
    )


def _culvert_from_fixture_row(row: dict[str, Any], *, hec_shape: int | None = None) -> CulvertSpec:
    shape_map = {0: 1, 1: 2, 2: 3, 3: 9, 4: 4, 5: 5, 6: 6}
    stream_shape = int(row.get("shape_type", 0))
    hec = hec_shape if hec_shape is not None else shape_map.get(stream_shape, 9)
    return CulvertSpec(
        rm=float(row["rm"]),
        hec_shape=hec,
        rise=float(row["rise"]),
        span=float(row["span"]),
        length=float(row["length"]),
        roughness_n=float(row["roughness_n"]),
        entrance_loss_coeff=float(row["entrance_loss_coeff"]),
        exit_loss_coeff=float(row["exit_loss_coeff"]),
        chart=int(row.get("chart", 61)),
        scale=int(row.get("scale", 3)),
        z_up=float(row["z_up"]),
        z_down=float(row["z_down"]),
        num_barrels=int(row.get("num_barrels", 1)),
        roughness_n_bottom=float(row.get("roughness_n_bottom", row["roughness_n"])),
        depth_bottom_n=float(row.get("depth_bottom_n", 0.0)),
        depth_blocked=float(row.get("depth_blocked", 0.0)),
        crest_elev=float(row["crest_elev"]) if row.get("crest_elev") is not None else None,
        name=str(row.get("name", "Culvert # 1")),
        barrel_centerlines=_barrel_centerlines_from_raw(row),
    )


def _barrel_centerlines_from_raw(raw: dict[str, Any]) -> list[tuple[float, float]]:
    pairs = raw.get("barrel_centerlines")
    if not pairs:
        return []
    out: list[tuple[float, float]] = []
    for pair in pairs:
        if len(pair) != 2:
            raise ValueError(f"barrel_centerlines entries must be [us_sta, ds_sta], got {pair!r}")
        out.append((float(pair[0]), float(pair[1])))
    return out


def _xs_from_inline(raw: dict[str, Any]) -> CrossSectionSpec:
    ch = float(raw.get("ch", 100.0))
    return CrossSectionSpec(
        rm=float(raw["rm"]),
        lob=float(raw.get("lob", ch)),
        ch=ch,
        rob=float(raw.get("rob", ch)),
        x=[float(v) for v in raw["x"]],
        y=[float(v) for v in raw["y"]],
        n_stations=[float(v) for v in raw["n_stations"]],
        n_values=[float(v) for v in raw["n_values"]],
        description=str(raw.get("description", f"River mile {raw['rm']}")),
        coeff_expansion=float(raw.get("coeff_expansion", 0.3)),
        coeff_contraction=float(raw.get("coeff_contraction", 0.1)),
        ineff_blocks=[
            (float(b[0]), float(b[1]), float(b[2]))
            for b in raw.get("ineff_blocks", [])
        ],
        bank_left=float(raw["bank_left"]) if "bank_left" in raw else None,
        bank_right=float(raw["bank_right"]) if "bank_right" in raw else None,
    )


def _culvert_from_inline(raw: dict[str, Any]) -> CulvertSpec:
    return CulvertSpec(
        rm=float(raw["rm"]),
        hec_shape=int(raw["hec_shape"]),
        rise=float(raw["rise"]),
        span=float(raw["span"]),
        length=float(raw["length"]),
        roughness_n=float(raw["roughness_n"]),
        entrance_loss_coeff=float(raw["entrance_loss_coeff"]),
        exit_loss_coeff=float(raw["exit_loss_coeff"]),
        chart=int(raw.get("chart", 61)),
        scale=int(raw.get("scale", 3)),
        z_up=float(raw["z_up"]),
        z_down=float(raw["z_down"]),
        num_barrels=int(raw.get("num_barrels", 1)),
        roughness_n_bottom=float(raw.get("roughness_n_bottom", raw["roughness_n"])),
        depth_bottom_n=float(raw.get("depth_bottom_n", 0.0)),
        depth_blocked=float(raw.get("depth_blocked", 0.0)),
        crest_elev=float(raw["crest_elev"]) if raw.get("crest_elev") is not None else None,
        name=str(raw.get("name", "Culvert # 1")),
        barrel_centerlines=_barrel_centerlines_from_raw(raw),
    )


def load_parity_case(path: Path) -> ParityCase:
    with path.open("r", encoding="utf-8") as fh:
        raw = json.load(fh)
    if "parity_case_version" not in raw:
        raise ValueError(f"{path}: missing parity_case_version")
    if "id" not in raw or "unsteady" not in raw:
        raise ValueError(f"{path}: requires id and unsteady blocks")
    return ParityCase(raw=raw, path=path.resolve())


def resolve_cross_sections(case: ParityCase) -> list[CrossSectionSpec]:
    geom = case.raw.get("geometry")
    if geom and geom.get("cross_sections"):
        rows = [_xs_from_inline(row) for row in geom["cross_sections"]]
        return sorted(rows, key=lambda xs: xs.rm, reverse=True)

    ref = case.raw.get("geometry_ref")
    if not ref:
        raise ValueError(f"case {case.id}: provide geometry.cross_sections or geometry_ref")

    fixture_path = _resolve_fixture_path(case.path, str(ref["path"]))
    with fixture_path.open("r", encoding="utf-8") as fh:
        fixture = json.load(fh)

    rm_filter = ref.get("cross_section_rms")
    rows: list[CrossSectionSpec] = []
    for row in fixture.get("geometry_data", []):
        rm = float(row["rm"])
        if rm_filter and rm not in {float(v) for v in rm_filter}:
            continue
        rows.append(_xs_from_fixture_row(row))
    if not rows:
        raise ValueError(f"case {case.id}: no cross sections resolved from {fixture_path}")
    return sorted(rows, key=lambda xs: xs.rm, reverse=True)


def resolve_culverts(case: ParityCase) -> list[CulvertSpec]:
    geom = case.raw.get("geometry", {})
    if geom.get("culverts"):
        return sorted(
            [_culvert_from_inline(row) for row in geom["culverts"]],
            key=lambda c: c.rm,
            reverse=True,
        )

    ref = case.raw.get("geometry_ref", {})
    if not ref.get("include_culvert_from_fixture"):
        return []

    fixture_path = _resolve_fixture_path(case.path, str(ref["path"]))
    with fixture_path.open("r", encoding="utf-8") as fh:
        fixture = json.load(fh)

    culvert_rows = fixture.get("culvert_stations", [])
    if not culvert_rows:
        return []
    station_to_rm = {float(r["station"]): float(r["rm"]) for r in fixture.get("geometry_data", [])}
    default_rm = float(ref.get("culvert_rm", 20.237))
    out: list[CulvertSpec] = []
    for row in culvert_rows:
        merged = dict(row)
        if "rm" not in merged:
            merged["rm"] = station_to_rm.get(float(merged.get("station", 0)), default_rm)
        out.append(_culvert_from_fixture_row(merged))
    return sorted(out, key=lambda c: c.rm, reverse=True)


def upstream_q_series(case: ParityCase) -> tuple[list[float], float]:
    """Return (Q cfs series, interval seconds)."""
    u = case.unsteady_cfg()
    interval = _interval_to_seconds(str(u.get("interval", "1HOUR")))

    if "upstream_q_cfs" in u:
        return [float(v) for v in u["upstream_q_cfs"]], interval

    constant = float(u.get("constant_q_cfs", u.get("initial_flow_cfs", 1000.0)))
    duration_hr = float(u.get("duration_hours", 48.0))
    n_steps = max(2, int(round(duration_hr * 3600.0 / interval)) + 1)

    ramp = u.get("ramp")
    if ramp:
        start_q = float(ramp.get("start_q_cfs", constant))
        end_q = float(ramp.get("end_q_cfs", constant))
        ramp_hr = float(ramp.get("ramp_hours", duration_hr * 0.5))
        ramp_steps = max(1, int(round(ramp_hr * 3600.0 / interval)))
        series: list[float] = []
        for i in range(n_steps):
            if i <= ramp_steps:
                frac = i / max(ramp_steps, 1)
                series.append(start_q + frac * (end_q - start_q))
            else:
                series.append(end_q)
        return series, interval

    if u.get("ramp_q_cfs"):
        ramp = [float(v) for v in u["ramp_q_cfs"]]
        if len(ramp) < n_steps:
            ramp = ramp + [ramp[-1]] * (n_steps - len(ramp))
        return ramp[:n_steps], interval
    return [constant] * n_steps, interval


def _interval_to_seconds(token: str) -> float:
    token = token.strip().upper()
    if token.endswith("HOUR"):
        return float(token.replace("HOUR", "").strip() or "1") * 3600.0
    if token.endswith("MIN"):
        return float(token.replace("MIN", "").strip() or "1") * 60.0
    if token.endswith("SEC"):
        return float(token.replace("SEC", "").strip() or "1")
    return float(token)


def plan_interval_seconds(case: ParityCase) -> float:
    plan = case.plan_cfg()
    return _interval_to_seconds(str(plan.get("computation_interval", plan.get("interval", "1HOUR"))))
