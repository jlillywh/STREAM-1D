"""HEC-RAS culvert verification: ConSpan reach profiles + point culvert benchmarks."""

import json
import os
import sys

import streams1d as st


def load_conspan_project(verification_dir: str) -> dict:
    path = os.path.join(verification_dir, "conspan_project_12.json")
    with open(path, "r", encoding="utf-8") as f:
        return json.load(f)


def build_cross_sections(project: dict) -> list:
    cross_sections = []
    for xs in project["geometry_data"]:
        cross_sections.append(
            st.CrossSection(
                station=float(xs["station"]),
                x=[float(v) for v in xs["x"]],
                y=[float(v) for v in xs["y"]],
                n_stations=[float(v) for v in xs["n_stations"]],
                n_values=[float(v) for v in xs["n_values"]],
                unit_system=xs.get("unit_system", "Metric"),
                is_overbank=xs.get("is_overbank"),
            )
        )
    return cross_sections


def build_culvert_arrays(project: dict) -> dict:
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


def verify_conspan_profiles(verification_dir: str) -> bool:
    profiles_path = os.path.join(verification_dir, "hecras_conspan_profiles.json")
    with open(profiles_path, "r", encoding="utf-8") as f:
        profiles_file = json.load(f)

    project = load_conspan_project(verification_dir)
    cross_sections = build_cross_sections(project)
    culvert_fields = build_culvert_arrays(project)
    station_list = [float(xs["station"]) for xs in project["geometry_data"]]
    tolerance = float(profiles_file["tolerance_ft"])
    all_passed = True

    print("\nConSpan HEC-RAS profile verification")
    print("STA\tProfile\tCalc\tHEC-RAS\tDiff\tStatus")
    print("-" * 55)

    for profile in profiles_file["profiles"]:
        inputs = st.SteadyInputs(
            cross_sections=cross_sections,
            flow_rate=float(profile["flow_rate_cfs"]),
            num_slices=int(project["parameters"].get("vertical_slices", 100)),
            regime=int(project["parameters"].get("flow_regime", 0)),
            downstream_wsel=float(profile["downstream_wsel_ft"]),
            max_spacing=float(project["parameters"].get("max_spacing", 100.0)),
            downstream_bc_type=0,
            **culvert_fields,
        )
        results = st.solve_steady(inputs)

        for sta_key, expected in profile["expected_wsel_ft"].items():
            station = float(sta_key)
            if station not in station_list:
                print(f"ERROR: station {station} missing")
                all_passed = False
                continue
            idx = station_list.index(station)
            calc = results["wsel"][idx]
            diff = calc - expected
            status = "PASS" if abs(diff) <= tolerance else "FAIL"
            print(
                f"{int(station)}\t{profile['name']}\t{calc:.3f}\t{expected:.3f}\t{diff:+.3f}\t[{status}]"
            )
            if status == "FAIL":
                all_passed = False

    return all_passed


def main() -> None:
    print("=" * 57)
    print("STREAM-1D: HEC-RAS Culvert Verification")
    print("=" * 57)

    verification_dir = os.path.join(os.path.dirname(os.path.abspath(__file__)), "verification")
    if not os.path.isdir(verification_dir):
        print(f"ERROR: verification dir not found: {verification_dir}")
        sys.exit(1)

    passed = verify_conspan_profiles(verification_dir)
    print("-" * 55)
    if passed:
        print("SUCCESS: All ConSpan HEC-RAS profiles within tolerance.")
        sys.exit(0)
    print("FAILURE: One or more profile checks exceeded tolerance.")
    sys.exit(1)


if __name__ == "__main__":
    main()
