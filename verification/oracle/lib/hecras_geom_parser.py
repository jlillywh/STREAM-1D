"""Minimal HEC-RAS .g01 parser for linked verify (cross sections + inline bridge)."""

from __future__ import annotations

import re
from dataclasses import dataclass, field
from pathlib import Path


@dataclass
class ParsedCrossSection:
    river: str
    reach: str
    rm: float
    ch: float
    x: list[float]
    y: list[float]
    n_stations: list[float]
    n_values: list[float]
    station: float = 0.0
    description: str = ""
    coeff_contraction: float = 0.1
    coeff_expansion: float = 0.3
    bank_left: float | None = None
    bank_right: float | None = None
    ineff_blocks: list[tuple[float, float, float]] = field(default_factory=list)


@dataclass
class ParsedBridge:
    river: str
    reach: str
    rm: float
    deck_low_stations: list[float] = field(default_factory=list)
    deck_low_elevations: list[float] = field(default_factory=list)
    deck_high_elevations: list[float] = field(default_factory=list)
    weir_coeff: float = 2.6
    orifice_coeff: float = 0.5
    pier_stations: list[float] = field(default_factory=list)
    pier_width: float = 1.25
    pier_base_elevations: list[float] = field(default_factory=list)
    pier_top_elevations: list[float] = field(default_factory=list)
    low_flow_method: str = "wspro"
    bridge_length: float = 30.0
    wspro_coeff: float = 0.8
    max_weir_submergence: float = 0.95


@dataclass
class ParsedGeometry:
    cross_sections: list[ParsedCrossSection]
    bridge: ParsedBridge | None
    unit_system: str = "USCustomary"


def _split_floats(line: str) -> list[float]:
    return [float(t) for t in line.split() if t.strip()]


def parse_g01(path: Path) -> ParsedGeometry:
    text = path.read_text(encoding="utf-8", errors="replace")
    lines = text.splitlines()

    current_river = "Default River"
    current_reach = "Default Reach"
    cross_sections: list[ParsedCrossSection] = []
    bridge: ParsedBridge | None = None

    current_xs: ParsedCrossSection | None = None
    parse_state: str | None = None
    expected_points = 0
    expected_mann = 0
    expected_ineff = 0
    temp_numbers: list[float] = []
    description_lines: list[str] = []

    current_bridge: ParsedBridge | None = None
    bridge_section: str | None = None
    pier_block_stations: list[float] = []
    deck_parsed = False
    duplicate_deck_remaining = 0

    i = 0
    while i < len(lines):
        line = lines[i].strip()
        i += 1
        if not line:
            continue

        if line.startswith("River Reach="):
            parts = line.split("=", 1)[1].split(",")
            if len(parts) >= 2:
                current_river = parts[0].strip()
                current_reach = parts[1].strip()
            continue

        if line.startswith("BEGIN DESCRIPTION:"):
            description_lines = []
            parse_state = "description"
            continue
        if parse_state == "description":
            if line.startswith("END DESCRIPTION:"):
                if current_xs is not None:
                    current_xs.description = " ".join(description_lines).strip()
                parse_state = None
            else:
                description_lines.append(line)
            continue

        if line.startswith("Type RM Length L Ch R"):
            if current_xs is not None:
                cross_sections.append(current_xs)
                current_xs = None
            parse_state = None
            description_lines = []

            payload = line.split("=", 1)[1]
            sub = [p.strip() for p in payload.split(",")]
            xs_type = int(sub[0])
            rm = float(re.sub(r"[^0-9.\-]", "", sub[1]))

            if xs_type == 1:
                ch = float(sub[3]) if len(sub) > 3 else 0.0
                current_xs = ParsedCrossSection(
                    river=current_river,
                    reach=current_reach,
                    rm=rm,
                    ch=ch,
                    x=[],
                    y=[],
                    n_stations=[],
                    n_values=[],
                )
            elif xs_type == 3:
                current_bridge = ParsedBridge(
                    river=current_river,
                    reach=current_reach,
                    rm=rm,
                )
                bridge_section = None
                pier_block_stations = []
                deck_parsed = False
            continue

        if current_bridge is not None and current_xs is None:
            if line.startswith("Deck Dist Width"):
                bridge_section = "deck_coeffs"
                continue
            if bridge_section == "deck_coeffs":
                parts = [p.strip() for p in line.split(",")]
                if parts:
                    try:
                        current_bridge.bridge_length = float(parts[0])
                    except ValueError:
                        pass
                if len(parts) >= 3:
                    try:
                        current_bridge.weir_coeff = float(parts[2])
                    except ValueError:
                        pass
                bridge_section = "deck_coeffs_wait_sta"
                continue
            if bridge_section == "deck_coeffs_wait_sta" and line[0].isdigit() and not deck_parsed:
                deck_sta_line = line
                bridge_section = "deck_low_sta"
                continue
            if bridge_section == "deck_low_sta":
                current_bridge.deck_low_stations = _split_floats(deck_sta_line)
                current_bridge.deck_low_elevations = _split_floats(line)
                bridge_section = "deck_high"
                continue
            if bridge_section == "deck_high":
                current_bridge.deck_high_elevations = _split_floats(line)
                bridge_section = None
                deck_parsed = True
                duplicate_deck_remaining = 3
                continue
            if duplicate_deck_remaining > 0:
                duplicate_deck_remaining -= 1
                continue
            if line.startswith("Pier Skew"):
                m = re.search(r",(\d+(?:\.\d+)?),\s*2\s*,", line)
                if m:
                    pier_block_stations.append(float(m.group(1)))
                bridge_section = "pier_data"
                continue
            if bridge_section == "pier_data":
                if line.startswith(("Pier Skew", "BR Coef=", "WSPro=", "Type RM")):
                    bridge_section = None
                    i -= 1
                    continue
                nums = _split_floats(line)
                if nums:
                    if max(nums) < 50.0:
                        current_bridge.pier_width = nums[0]
                    else:
                        for j in range(0, len(nums) - 1, 2):
                            current_bridge.pier_base_elevations.append(nums[j])
                            current_bridge.pier_top_elevations.append(nums[j + 1])
                continue
            if line.startswith("BR Coef="):
                nums = re.findall(r"[\d.]+", line.split("BR Coef=")[1])
                if len(nums) >= 2:
                    current_bridge.orifice_coeff = float(nums[1])
                if pier_block_stations:
                    current_bridge.pier_stations = pier_block_stations
                bridge = current_bridge
                current_bridge = None
                continue
            if line.startswith("WSPro="):
                current_bridge.low_flow_method = "wspro"
                continue
            continue

        if current_xs is None:
            continue

        if line.startswith("#Sta/Elev"):
            expected_points = int(line.split("=")[1].strip())
            parse_state = "sta_elev"
            temp_numbers = []
            continue

        if line.startswith("#Mann"):
            expected_mann = int(line.split("=")[1].split(",")[0].strip())
            parse_state = "mann"
            temp_numbers = []
            continue

        if line.startswith("#XS Ineff"):
            expected_ineff = int(line.split("=")[1].split(",")[0].strip())
            parse_state = "xs_ineff"
            temp_numbers = []
            continue

        if line.startswith("Exp/Cntr="):
            parts = line.split("=")[1].split(",")
            if len(parts) >= 2:
                try:
                    current_xs.coeff_expansion = float(parts[0].strip())
                    current_xs.coeff_contraction = float(parts[1].strip())
                except ValueError:
                    pass
            parse_state = None
            continue

        if line.startswith("Bank Sta="):
            parts = line.split("=", 1)[1].split(",")
            if len(parts) >= 2:
                try:
                    current_xs.bank_left = float(parts[0].strip())
                    current_xs.bank_right = float(parts[1].strip())
                except ValueError:
                    pass
            parse_state = None
            continue

        if line.startswith(("XS Rating Curve", "Permanent Ineff", "XS HTab")):
            parse_state = None

        if parse_state == "sta_elev":
            temp_numbers.extend(_split_floats(line))
            if len(temp_numbers) >= expected_points * 2:
                for j in range(expected_points):
                    current_xs.x.append(temp_numbers[j * 2])
                    current_xs.y.append(temp_numbers[j * 2 + 1])
                parse_state = None
        elif parse_state == "mann":
            temp_numbers.extend(_split_floats(line))
            if len(temp_numbers) >= expected_mann * 3:
                for j in range(expected_mann):
                    current_xs.n_stations.append(temp_numbers[j * 3])
                    current_xs.n_values.append(temp_numbers[j * 3 + 1])
                parse_state = None
        elif parse_state == "xs_ineff":
            temp_numbers.extend(_split_floats(line))
            if len(temp_numbers) >= expected_ineff * 3:
                for j in range(expected_ineff):
                    lo = temp_numbers[j * 3]
                    hi = temp_numbers[j * 3 + 1]
                    elev = temp_numbers[j * 3 + 2]
                    current_xs.ineff_blocks.append((lo, hi, elev))
                parse_state = None

    if current_xs is not None:
        cross_sections.append(current_xs)
    if current_bridge is not None:
        bridge = current_bridge

    if bridge is not None and pier_block_stations and not bridge.pier_stations:
        bridge.pier_stations = pier_block_stations

    _assign_reach_stations(cross_sections)
    return ParsedGeometry(cross_sections=cross_sections, bridge=bridge)


def _assign_reach_stations(cross_sections: list[ParsedCrossSection]) -> None:
    if not cross_sections:
        return
    cross_sections.sort(key=lambda xs: xs.rm, reverse=True)
    cross_sections[-1].station = 0.0
    for idx in range(len(cross_sections) - 2, -1, -1):
        cross_sections[idx].station = cross_sections[idx + 1].station + cross_sections[idx].ch


def cross_section_at_rm(cross_sections: list[ParsedCrossSection], rm: float, tol: float = 0.02) -> ParsedCrossSection | None:
    best: ParsedCrossSection | None = None
    best_dist = float("inf")
    for xs in cross_sections:
        dist = abs(xs.rm - rm)
        if dist < best_dist:
            best_dist = dist
            best = xs
    return best if best is not None and best_dist <= tol else None


def cross_section_by_description(cross_sections: list[ParsedCrossSection], needle: str) -> ParsedCrossSection | None:
    needle_l = needle.lower()
    for xs in cross_sections:
        if needle_l in xs.description.lower():
            return xs
    return None


def rm_to_station(cross_sections: list[ParsedCrossSection], rm: float) -> float:
    if not cross_sections:
        return 0.0
    ordered = sorted(cross_sections, key=lambda xs: xs.rm, reverse=True)
    if rm >= ordered[0].rm:
        return ordered[0].station
    if rm <= ordered[-1].rm:
        return ordered[-1].station
    for us, ds in zip(ordered, ordered[1:]):
        if rm <= us.rm and rm >= ds.rm:
            span = us.rm - ds.rm
            t = (rm - ds.rm) / span if span > 1e-6 else 0.5
            return ds.station + t * (us.station - ds.station)
    return ordered[-1].station


def _is_overbank_flags(xs: ParsedCrossSection) -> list[bool] | None:
    if xs.bank_left is None or xs.bank_right is None:
        return None
    return [xi < xs.bank_left or xi > xs.bank_right for xi in xs.x]


def parsed_xs_to_dict(xs: ParsedCrossSection) -> dict:
    """Convert parsed XS to stream1d CrossSection kwargs."""
    out = {
        "station": xs.station,
        "x": xs.x,
        "y": xs.y,
        "n_stations": xs.n_stations,
        "n_values": xs.n_values,
        "unit_system": "USCustomary",
    }
    overbank = _is_overbank_flags(xs)
    if overbank is not None:
        out["is_overbank"] = overbank
    return out
