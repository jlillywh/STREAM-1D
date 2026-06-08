"""
Helpers for importing HEC-RAS-style junction projects into STREAM-1D's two-branch API.

When a confluence is stored as three reaches (upper main, lower main, tributary),
call `prepare_junction_import()` to merge the two main reaches automatically.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any, Dict, Iterable, List, Mapping, Optional, Sequence, Union

from . import CrossSection

STATION_TOL = 1e-4

ReachRole = str  # "main_upper" | "main_lower" | "tributary"


def _station(xs: Union[CrossSection, Mapping[str, Any]]) -> float:
    return float(xs.station if isinstance(xs, CrossSection) else xs["station"])


def _stations(sections: Sequence[Union[CrossSection, Mapping[str, Any]]]) -> List[float]:
    return [_station(xs) for xs in sections]


def stations_match(a: float, b: float, tol: float = STATION_TOL) -> bool:
    return abs(a - b) <= tol


def cross_section_from_dict(data: Mapping[str, Any]) -> CrossSection:
    return CrossSection(
        station=float(data["station"]),
        x=[float(v) for v in data["x"]],
        y=[float(v) for v in data["y"]],
        n_stations=[float(v) for v in data["n_stations"]],
        n_values=[float(v) for v in data["n_values"]],
        unit_system=data.get("unit_system", "Metric"),
        is_overbank=data.get("is_overbank"),
    )


def cross_sections_from_dicts(rows: Iterable[Mapping[str, Any]]) -> List[CrossSection]:
    return [cross_section_from_dict(row) for row in rows]


@dataclass
class ReachImport:
    """One imported HEC-RAS reach before junction flattening."""

    name: str
    cross_sections: List[CrossSection]
    role: ReachRole = ""
    reach_id: Optional[str] = None


@dataclass
class MainStemMergeResult:
    cross_sections: List[CrossSection]
    junction_main_station: float
    display_segments: List[Dict[str, Any]] = field(default_factory=list)
    warnings: List[str] = field(default_factory=list)


@dataclass
class JunctionImportResult:
    """Flattened geometry ready for `SteadyInputs` / WASM `solveSteady`."""

    cross_sections: List[CrossSection]
    tributary_cross_sections: List[CrossSection]
    junction_main_station: float
    display_segments: List[Dict[str, Any]] = field(default_factory=list)
    warnings: List[str] = field(default_factory=list)

    def to_steady_geometry(self) -> Dict[str, Any]:
        return {
            "cross_sections": [xs.to_dict() for xs in self.cross_sections],
            "tributary_cross_sections": [xs.to_dict() for xs in self.tributary_cross_sections],
            "junction_main_station": self.junction_main_station,
        }


def detect_junction_station(
    upper_main: Sequence[Union[CrossSection, Mapping[str, Any]]],
    lower_main: Sequence[Union[CrossSection, Mapping[str, Any]]],
    junction_main_station: Optional[float] = None,
) -> float:
    """
    Resolve the main-channel junction station.

    Uses `junction_main_station` when provided; otherwise expects the downstream
    end of the upper reach to meet the upstream end of the lower reach.
    """
    if junction_main_station is not None:
        j = float(junction_main_station)
        upper_hits = any(stations_match(_station(xs), j) for xs in upper_main)
        lower_hits = any(stations_match(_station(xs), j) for xs in lower_main)
        if not upper_hits or not lower_hits:
            raise ValueError(
                f"junction_main_station {j} must appear on both upper and lower main reaches"
            )
        return j

    upper_stations = _stations(upper_main)
    lower_stations = _stations(lower_main)
    if not upper_stations or not lower_stations:
        raise ValueError("upper and lower main reaches must each contain cross-sections")

    upper_ds = min(upper_stations)
    lower_us = max(lower_stations)
    if stations_match(upper_ds, lower_us):
        return upper_ds

    shared = []
    for s in upper_stations:
        for t in lower_stations:
            if stations_match(s, t):
                shared.append(s)
    if len(shared) == 1:
        return shared[0]
    if len(shared) > 1:
        raise ValueError(
            f"ambiguous junction: multiple shared stations {shared}; pass junction_main_station explicitly"
        )

    raise ValueError(
        "could not detect junction station: upper main downstream end "
        f"({upper_ds}) does not match lower main upstream end ({lower_us})"
    )


def merge_main_stem_reaches(
    upper_main: Sequence[Union[CrossSection, Mapping[str, Any]]],
    lower_main: Sequence[Union[CrossSection, Mapping[str, Any]]],
    junction_main_station: Optional[float] = None,
    upper_name: str = "Main upper",
    lower_name: str = "Main lower",
) -> MainStemMergeResult:
    """
    Merge upper and lower main reaches into one continuous main-stem cross-section list.

    Preserves HEC-RAS station values. The junction cross-section appears once.
    Output order matches the solver convention: descending station (upstream first).
    """
    upper = [
        xs if isinstance(xs, CrossSection) else cross_section_from_dict(xs)
        for xs in upper_main
    ]
    lower = [
        xs if isinstance(xs, CrossSection) else cross_section_from_dict(xs)
        for xs in lower_main
    ]

    junction = detect_junction_station(upper, lower, junction_main_station)
    warnings: List[str] = []

    merged: Dict[float, CrossSection] = {}
    for xs in upper:
        if _station(xs) + STATION_TOL >= junction:
            merged[_station(xs)] = xs
    for xs in lower:
        st = _station(xs)
        if st - STATION_TOL <= junction:
            existing = next((k for k in merged if stations_match(k, st)), None)
            if existing is not None and existing != st:
                warnings.append(
                    f"duplicate junction cross-section at {st}; keeping upper-main definition"
                )
            elif existing is None:
                merged[st] = xs

    if len([st for st in merged if st + STATION_TOL >= junction]) < 1:
        raise ValueError("merged main stem has no cross-sections at/above the junction")
    if len([st for st in merged if st - STATION_TOL <= junction]) < 2:
        raise ValueError(
            "merged main stem needs at least two cross-sections at/below the junction"
        )

    cross_sections = sorted(merged.values(), key=_station, reverse=True)
    upper_ids = [_station(xs) for xs in upper if _station(xs) + STATION_TOL >= junction]
    lower_ids = [_station(xs) for xs in lower if _station(xs) - STATION_TOL <= junction]

    display_segments = [
        {
            "name": upper_name,
            "role": "main_upper",
            "station_min": min(upper_ids) if upper_ids else junction,
            "station_max": max(upper_ids) if upper_ids else junction,
        },
        {
            "name": lower_name,
            "role": "main_lower",
            "station_min": min(lower_ids) if lower_ids else junction,
            "station_max": max(lower_ids) if lower_ids else junction,
        },
        {
            "name": f"{upper_name} + {lower_name}",
            "role": "main_merged",
            "station_min": min(_stations(cross_sections)),
            "station_max": max(_stations(cross_sections)),
        },
    ]

    return MainStemMergeResult(
        cross_sections=cross_sections,
        junction_main_station=junction,
        display_segments=display_segments,
        warnings=warnings,
    )


def _reach_from_mapping(data: Mapping[str, Any]) -> ReachImport:
    geometry = data.get("geometry_data") or data.get("cross_sections") or []
    return ReachImport(
        name=str(data.get("name") or data.get("reach_name") or "Unnamed reach"),
        cross_sections=cross_sections_from_dicts(geometry),
        role=str(data.get("role") or ""),
        reach_id=data.get("reach_id") or data.get("id"),
    )


def _pick_reach_by_role(
    reaches: Sequence[ReachImport], role: str, fallback_name: Optional[str] = None
) -> ReachImport:
    matches = [r for r in reaches if r.role == role]
    if len(matches) == 1:
        return matches[0]
    if len(matches) > 1:
        raise ValueError(f"multiple reaches tagged role={role!r}")

    if fallback_name:
        for reach in reaches:
            if reach.name == fallback_name:
                return reach

    raise ValueError(f"no reach found with role={role!r}")


def prepare_junction_import(
    upper_main: Union[ReachImport, Sequence[Union[CrossSection, Mapping[str, Any]]], Mapping[str, Any]],
    lower_main: Union[ReachImport, Sequence[Union[CrossSection, Mapping[str, Any]]], Mapping[str, Any]],
    tributary: Union[ReachImport, Sequence[Union[CrossSection, Mapping[str, Any]]], Mapping[str, Any]],
    junction_main_station: Optional[float] = None,
) -> JunctionImportResult:
    """
    Convert a three-reach HEC-RAS junction layout into STREAM-1D's two-branch geometry.

    Returns merged main stem cross-sections, tributary cross-sections, junction station,
    and display segment metadata for Plan View styling.
    """
    def normalize(
        reach: Union[ReachImport, Sequence[Union[CrossSection, Mapping[str, Any]]], Mapping[str, Any]]
    ) -> ReachImport:
        if isinstance(reach, ReachImport):
            return reach
        if isinstance(reach, Mapping) and ("geometry_data" in reach or "cross_sections" in reach):
            return _reach_from_mapping(reach)
        return ReachImport(name="Reach", cross_sections=cross_sections_from_dicts(reach))  # type: ignore[arg-type]

    upper = normalize(upper_main)
    lower = normalize(lower_main)
    trib = normalize(tributary)

    if not trib.cross_sections:
        raise ValueError("tributary reach must contain cross-sections")

    merge = merge_main_stem_reaches(
        upper.cross_sections,
        lower.cross_sections,
        junction_main_station=junction_main_station,
        upper_name=upper.name,
        lower_name=lower.name,
    )

    trib_sorted = sorted(trib.cross_sections, key=_station, reverse=True)
    display_segments = merge.display_segments + [
        {
            "name": trib.name,
            "role": "tributary",
            "station_min": min(_stations(trib_sorted)),
            "station_max": max(_stations(trib_sorted)),
            "reach_id": trib.reach_id,
        }
    ]

    return JunctionImportResult(
        cross_sections=merge.cross_sections,
        tributary_cross_sections=trib_sorted,
        junction_main_station=merge.junction_main_station,
        display_segments=display_segments,
        warnings=list(merge.warnings),
    )


def import_junction_project(project: Mapping[str, Any]) -> JunctionImportResult:
    """
    Import a junction project JSON object with three reaches.

    Expected shape:
    {
      "reaches": [
        {"name": "...", "role": "main_upper", "geometry_data": [...]},
        {"name": "...", "role": "main_lower", "geometry_data": [...]},
        {"name": "...", "role": "tributary", "geometry_data": [...]}
      ],
      "junction_main_station": 8500.0   // optional
    }

    Roles may be omitted if reach names are passed via `main_upper_reach`,
    `main_lower_reach`, and `tributary_reach`.
    """
    reaches_raw = project.get("reaches")
    if not isinstance(reaches_raw, list):
        raise ValueError("project.reaches must be a list of three reaches")

    reaches = [_reach_from_mapping(item) for item in reaches_raw]
    junction_station = project.get("junction_main_station")

    upper = _pick_reach_by_role(
        reaches,
        "main_upper",
        fallback_name=project.get("main_upper_reach"),
    )
    lower = _pick_reach_by_role(
        reaches,
        "main_lower",
        fallback_name=project.get("main_lower_reach"),
    )
    trib = _pick_reach_by_role(
        reaches,
        "tributary",
        fallback_name=project.get("tributary_reach"),
    )

    return prepare_junction_import(
        upper,
        lower,
        trib,
        junction_main_station=junction_station,
    )
