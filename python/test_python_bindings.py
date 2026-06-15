import os
import json
import sys
import stream1d as st

def main():
    print("=========================================================")
    print("STREAM-1D: Python Bindings HEC-RAS Verification Test")
    print("=========================================================")

    # 1. Path to test project
    project_json_path = os.path.join(
        os.path.dirname(os.path.abspath(__file__)),
        "..",
        "verification",
        "fixtures",
        "conspan_project_12.json",
    )
    if not os.path.exists(project_json_path):
        print(f"ERROR: Verification dataset not found at {project_json_path}")
        sys.exit(1)

    with open(project_json_path, "r") as f:
        project = json.load(f)

    # 2. Extract geometries and parameters
    xs_raw = project["geometry_data"]
    cross_sections = []
    for xs in xs_raw:
        cross_sections.append(
            st.CrossSection(
                station=float(xs["station"]),
                x=[float(val) for val in xs["x"]],
                y=[float(val) for val in xs["y"]],
                n_stations=[float(val) for val in xs["n_stations"]],
                n_values=[float(val) for val in xs["n_values"]],
                unit_system=xs.get("unit_system", "Metric"),
                is_overbank=xs.get("is_overbank")
            )
        )

    culverts = project.get("culvert_stations", [])
    culvert_stations = [float(c["station"]) for c in culverts]
    culvert_shape_types = [int(c["shape_type"]) for c in culverts]
    culvert_spans = [float(c["span"]) for c in culverts]
    culvert_rises = [float(c["rise"]) for c in culverts]
    culvert_roughness_ns = [float(c["roughness_n"]) for c in culverts]
    culvert_lengths = [float(c["length"]) for c in culverts]
    culvert_entrance_loss_coeffs = [float(c["entrance_loss_coeff"]) for c in culverts]
    culvert_exit_loss_coeffs = [float(c["exit_loss_coeff"]) for c in culverts]
    culvert_barrels = [int(c.get("num_barrels", 1)) for c in culverts]
    culvert_roughness_n_bottoms = [float(c.get("roughness_n_bottom", c["roughness_n"])) for c in culverts]
    culvert_depth_bottom_ns = [float(c.get("depth_bottom_n", 0.0)) for c in culverts]
    culvert_depth_blockeds = [float(c.get("depth_blocked", 0.0)) for c in culverts]

    # Q = 1000 cfs High Flow profile inputs
    inputs = st.SteadyInputs(
        cross_sections=cross_sections,
        flow_rate=1000.0,
        num_slices=int(project["parameters"].get("vertical_slices", 100)),
        coeff_contraction=0.1,
        coeff_expansion=0.3,
        regime=int(project["parameters"].get("flow_regime", 0)),
        downstream_wsel=30.51,
        max_spacing=float(project["parameters"].get("max_spacing", 100.0)),
        # Boundary conditions
        downstream_bc_type=0, # Known WSEL
        downstream_bc_slope=float(project["parameters"].get("downstream_bc_slope", 0.001)),
        downstream_bc_rating_q=[],
        downstream_bc_rating_wsel=[],
        upstream_bc_type=0,
        upstream_bc_slope=float(project["parameters"].get("upstream_bc_slope", 0.01)),
        upstream_bc_rating_q=[],
        upstream_bc_rating_wsel=[],
        upstream_wsel=float(project["parameters"].get("upstream_wsel", 15.0)),
        # Culvert parameters
        culvert_stations=culvert_stations,
        culvert_shape_types=culvert_shape_types,
        culvert_spans=culvert_spans,
        culvert_rises=culvert_rises,
        culvert_roughness_ns=culvert_roughness_ns,
        culvert_lengths=culvert_lengths,
        culvert_entrance_loss_coeffs=culvert_entrance_loss_coeffs,
        culvert_exit_loss_coeffs=culvert_exit_loss_coeffs,
        culvert_barrels=culvert_barrels,
        culvert_roughness_n_bottoms=culvert_roughness_n_bottoms,
        culvert_depth_bottom_ns=culvert_depth_bottom_ns,
        culvert_depth_blockeds=culvert_depth_blockeds,
    )

    # 3. Solve profile
    print("\nRunning solver (Q = 1000 cfs, Downstream WSEL = 30.51 ft)...")
    results = st.solve_steady(inputs)

    # 4. Define expected verification checks for Q = 1000 cfs profile
    expected_wsel = {
        2827.0: 33.720,
        1257.0: 32.920,
        0.0: 30.510
    }
    tolerance = 0.04

    print("\nSTA\tCalculated\tHEC-RAS\tDiff (ft)\tStatus")
    print("---------------------------------------------------------")

    all_passed = True
    station_list = [float(xs["station"]) for xs in xs_raw]
    for station, expected_val in expected_wsel.items():
        # Find index in geometry stations
        if station in station_list:
            idx = station_list.index(station)
            calc_val = results["wsel"][idx]
            diff = calc_val - expected_val
            abs_diff = abs(diff)
            status = "PASS" if abs_diff <= tolerance else "FAIL"
            print(f"{int(station)}\t{calc_val:.3f}\t\t{expected_val:.3f}\t{diff:+.3f}\t\t[{status}]")
            if status == "FAIL":
                all_passed = False
        else:
            print(f"ERROR: Station {station} not found in model geometry!")
            all_passed = False

    # 5. Culvert control type reporting (Tier 1)
    if culvert_stations:
        control_types = results.get("culvert_control_types")
        if not control_types or len(control_types) != len(culvert_stations):
            print("FAIL: culvert_control_types missing or wrong length")
            all_passed = False
        elif control_types[0] not in ("inlet", "outlet", "overtopping"):
            print(f"FAIL: unexpected culvert control type: {control_types[0]}")
            all_passed = False
        else:
            print(f"Culvert control type at station {culvert_stations[0]}: {control_types[0]} [PASS]")

    print("---------------------------------------------------------")
    if all_passed:
        print("✅ SUCCESS: Python bindings verified successfully!")
        sys.exit(0)
    else:
        print("❌ FAILURE: Python solver results do not match HEC-RAS within tolerance!")
        sys.exit(1)

if __name__ == "__main__":
    main()
