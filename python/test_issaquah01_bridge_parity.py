#!/usr/bin/env python3
"""
Issaquah01 bridge reach — steady WSE/EGL parity vs HEC-RAS or Stream1D reference.

Uses the geometry export at examples/Issaquah01_stream1d.json and profile settings in
examples/Issaquah01_profiles.json.

Workflow:
  1. Set flow_rate_cfs, downstream_wsel_ft, and other solver settings in the profiles file
     to match your Stream1D workspace sliders.
  2. Run with --print to see computed WSE/EGL at each river station.
  3. Paste HEC-RAS (or Stream1D UI) values into expected_wsel_ft / expected_egl_ft, then
     re-run without --print to verify parity.

Examples:
  PYTHONPATH=python python3 python/test_issaquah01_bridge_parity.py --print
  PYTHONPATH=python python3 python/test_issaquah01_bridge_parity.py
  PYTHONPATH=python python3 python/test_issaquah01_bridge_parity.py --record
"""

from __future__ import annotations

import argparse
import copy
import json
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
ORACLE_LIB = REPO / "verification" / "oracle"
sys.path.insert(0, str(REPO / "python"))
sys.path.insert(0, str(ORACLE_LIB))

from lib.geometry_fixture_mapper import (  # noqa: E402
    load_geometry_fixture,
    run_profile,
)


def _load_profiles(path: Path) -> dict:
    with path.open("r", encoding="utf-8") as fh:
        return json.load(fh)


def _prepare_fixture(fixture: dict, profiles_doc: dict) -> dict:
    if not profiles_doc.get("strip_deck_vents"):
        return fixture
    prepared = copy.deepcopy(fixture)
    for bridge in prepared.get("bridge_data", []):
        bridge["deck_vents"] = []
        bridge.pop("deck_vent_pattern", None)
    return prepared


def _station_key(rs: float) -> str:
    return str(int(rs)) if abs(rs - round(rs)) < 1e-3 else f"{rs:.3f}"


def _print_profile_table(run: dict) -> None:
    bc = run.get("downstream_bc_type", 0)
    if bc == 2:
        bc_label = f"DS BC: normal depth, Sf={run.get('downstream_bc_slope', 0.005)}"
    elif bc == 1:
        bc_label = "DS BC: critical depth"
    elif bc == 0:
        bc_label = f"DS TW={run.get('downstream_wsel_ft', 0):.2f} ft"
    else:
        bc_label = f"DS BC type {bc}"
    print(f"\nProfile: {run['profile']}  Q={run['flow_rate_cfs']:.0f} cfs  {bc_label}")
    print(f"{'STA (ft)':>10}  {'WSEL':>8}  {'Crit':>8}  {'EGL':>8}  {'Vel':>7}  {'Area':>8}  {'Fr':>6}")
    print("-" * 66)
    for row in run["stations"]:
        sta = row["computational_station"]
        print(
            f"{sta:10.1f}  "
            f"{row['wsel_ft']:8.3f}  "
            f"{row['critical_wsel_ft']:8.3f}  "
            f"{row['egl_ft']:8.3f}  "
            f"{row['velocity_fps']:7.2f}  "
            f"{row['area_ft2']:8.1f}  "
            f"{row['froude']:6.3f}"
        )


def _station_lookup(run: dict, profiles_doc: dict) -> dict[float, dict]:
    key = profiles_doc.get("station_key", "river")
    by_key: dict[float, dict] = {}
    for row in run["stations"]:
        if key == "computational":
            by_key[round(float(row["computational_station"]), 1)] = row
        else:
            by_key[float(row["river_station"])] = row
    return by_key


def _compare_profile(
    run: dict,
    profile: dict,
    profiles_doc: dict,
    tol_wsel: float,
    tol_egl: float,
    tol_crit: float,
) -> tuple[bool, list[str]]:
    expected_wsel = {float(k): float(v) for k, v in (profile.get("expected_wsel_ft") or {}).items()}
    expected_egl = {float(k): float(v) for k, v in (profile.get("expected_egl_ft") or {}).items()}
    expected_crit = {float(k): float(v) for k, v in (profile.get("expected_crit_wsel_ft") or {}).items()}
    if not expected_wsel and not expected_egl and not expected_crit:
        return True, ["  (no expected reference values — use --print or --record)"]

    lines: list[str] = []
    passed = True
    by_sta = _station_lookup(run, profiles_doc)
    sta_label = "STA" if profiles_doc.get("station_key") == "computational" else "RS"

    def check(label: str, expected: dict[float, float], actual_key: str, tol: float) -> None:
        nonlocal passed
        for sta, exp in sorted(expected.items(), reverse=True):
            row = by_sta.get(round(sta, 1))
            if row is None:
                nearest = min(by_sta.keys(), key=lambda k: abs(k - sta))
                if abs(nearest - sta) > 2.0:
                    lines.append(f"  FAIL {label} {sta_label} {sta:.1f}: station not in model")
                    passed = False
                    continue
                row = by_sta[nearest]
                sta_text = f"{sta:.1f} (≈{nearest:.1f})"
            else:
                sta_text = f"{sta:.1f}"
            act = float(row[actual_key])
            delta = act - exp
            ok = abs(delta) <= tol
            status = "PASS" if ok else "FAIL"
            lines.append(f"  {status} {label} {sta_label} {sta_text}: calc={act:.3f} ref={exp:.3f} Δ={delta:+.3f}")
            if not ok:
                passed = False

    check("WSEL", expected_wsel, "wsel_ft", tol_wsel)
    check("Crit WSEL", expected_crit, "critical_wsel_ft", tol_crit)
    check("EGL", expected_egl, "egl_ft", tol_egl)
    return passed, lines


def _record_expected(run: dict, profile: dict, profiles_doc: dict) -> None:
    wsel: dict[str, float] = {}
    crit: dict[str, float] = {}
    egl: dict[str, float] = {}
    use_comp = profiles_doc.get("station_key") == "computational"
    for row in run["stations"]:
        key = _station_key(row["computational_station"] if use_comp else row["river_station"])
        wsel[key] = round(row["wsel_ft"], 3)
        crit[key] = round(row["critical_wsel_ft"], 3)
        egl[key] = round(row["egl_ft"], 3)
    profile["expected_wsel_ft"] = wsel
    profile["expected_crit_wsel_ft"] = crit
    profile["expected_egl_ft"] = egl


def main() -> int:
    parser = argparse.ArgumentParser(description="Issaquah01 bridge WSE/EGL parity test")
    parser.add_argument(
        "--geometry",
        type=Path,
        default=REPO / "examples" / "Issaquah01_stream1d.json",
        help="streams1d_geometry JSON export",
    )
    parser.add_argument(
        "--profiles",
        type=Path,
        default=REPO / "examples" / "Issaquah01_profiles.json",
        help="Profile + reference JSON",
    )
    parser.add_argument("--print", dest="print_only", action="store_true", help="Print computed table only")
    parser.add_argument("--record", action="store_true", help="Write computed WSE/EGL into profiles file as reference")
    args = parser.parse_args()

    profiles_doc = _load_profiles(args.profiles)
    fixture = _prepare_fixture(load_geometry_fixture(args.geometry), profiles_doc)
    tol_wsel = float(profiles_doc.get("tolerance_wsel_ft", 0.05))
    tol_egl = float(profiles_doc.get("tolerance_egl_ft", 0.05))
    tol_crit = float(profiles_doc.get("tolerance_crit_wsel_ft", 0.05))

    all_passed = True
    for profile in profiles_doc.get("profiles", []):
        run = run_profile(fixture, profile)
        if args.print_only:
            _print_profile_table(run)
            continue
        if args.record:
            _record_expected(run, profile, profiles_doc)
            _print_profile_table(run)
            continue
        ok, lines = _compare_profile(run, profile, profiles_doc, tol_wsel, tol_egl, tol_crit)
        print(f"\n=== {profile.get('name', 'profile')} ===")
        for line in lines:
            print(line)
        all_passed = all_passed and ok

    if args.record:
        with args.profiles.open("w", encoding="utf-8") as fh:
            json.dump(profiles_doc, fh, indent=2)
            fh.write("\n")
        print(f"\nRecorded reference values to {args.profiles}")
        return 0

    if args.print_only:
        return 0

    if all_passed:
        print("\nSUCCESS: all WSE/EGL/critical checks within tolerance.")
        return 0
    print("\nFAILURE: one or more stations exceeded tolerance.")
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
