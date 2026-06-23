#!/usr/bin/env python3
"""Print culvert head loss / swell-head diagnostics for ConSpan ramp @ 48h."""

from __future__ import annotations

import json
import sys
from pathlib import Path

ORACLE = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ORACLE))
sys.path.insert(0, str(ORACLE.parent.parent / "python"))

import stream1d as st  # noqa: E402
from lib.conspan_mapper import build_conspan_unsteady_inputs  # noqa: E402

PROJECT = ORACLE / "projects" / "conspan"
REF = PROJECT / "reference_wsel_timeseries_ramp_full.json"


def main() -> None:
    payload, _ = build_conspan_unsteady_inputs(
        PROJECT,
        geometry_name="ConSpan.g01",
        flow_name="ConSpan.u07",
        plan_name="ConSpan.p08",
        coupling_mode=2,
    )
    result = st.solve_unsteady(payload)

    step = min(48 * 3600 // int(payload["dt"]), int(payload["num_steps"]) - 1)
    print(f"step={step} t_hr={step * payload['dt'] / 3600:.1f}")

    culvert_inlet = result.get("culvert_wsel_inlet")
    culvert_outlet = result.get("culvert_wsel_outlet")
    if culvert_inlet and culvert_outlet:
        print(
            f"STREAM culvert inlet={culvert_inlet[step][0]:.3f} "
            f"outlet={culvert_outlet[step][0]:.3f}"
        )
        hl = culvert_inlet[step][0] - culvert_outlet[step][0]
        print(f"STREAM hl (inlet-outlet) = {hl:.3f} ft")
        barrel_len = float(payload.get("culvert_lengths", [50])[0])
        sh = hl * 0.3048 / max(barrel_len * 0.3048, 1.0)
        print(f"approx Sh = hl/barrel = {sh:.5f} (cap 0.15)")

    wsel = result["wsel"][step]
    from lib.conspan_reference import conspan_geometry_rms_upstream_first

    rms_list = conspan_geometry_rms_upstream_first()
    rms = [20.535, 20.422, 20.308, 20.251, 20.238, 20.227, 20.095, 20.0]

    print("STREAM WSEL @ step:")
    for rm in rms:
        if rm in rms_list:
            idx = rms_list.index(rm)
            if idx < len(wsel):
                print(f"  RM {rm:.3f}: {wsel[idx]:.3f} ft")

    if REF.exists():
        with REF.open() as fh:
            ref = json.load(fh)
        t_hr = step * payload["dt"] / 3600
        print(f"HEC-RAS reference @ ~{t_hr:.0f} hr:")
        for key, val in ref.items():
            if isinstance(val, dict) and "river_mile" in str(val):
                pass
        # reference_wsel_timeseries_ramp_full.json layout
        entries = ref.get("wsel_by_rm") or ref.get("series") or []
        if isinstance(entries, dict):
            for rm in rms:
                series = entries.get(str(rm)) or entries.get(f"{rm:.3f}")
                if series and isinstance(series, list) and len(series) > step:
                    print(f"  RM {rm:.3f}: {float(series[step]):.3f} ft")
        else:
            for series in entries:
                rm = float(series.get("river_mile", series.get("rm", -1)))
                if rm not in rms:
                    continue
                times = series.get("time_hr", series.get("times_hr", []))
                vals = series.get("wsel_ft", series.get("wsel", []))
                if not times or not vals:
                    continue
                j = min(range(len(times)), key=lambda k: abs(times[k] - t_hr))
                print(f"  RM {rm:.3f}: {vals[j]:.3f} ft (t={times[j]:.1f} hr)")


if __name__ == "__main__":
    main()
