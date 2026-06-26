# Python usage

Install: see [README](../README.md#python). `SteadyInputs` field names match [`docs/wasm_api.types.ts`](../wasm_api.types.ts).

## Steady example

```python
import stream1d as st

# 1. Define cross-sections
xs1000 = st.CrossSection(
    station=1000.0,
    x=[0.0, 0.0, 10.0, 10.0],
    y=[6.0, 1.0, 1.0, 6.0],
    n_stations=[0.0],
    n_values=[0.025],
    unit_system="Metric"
)
xs500 = st.CrossSection(
    station=500.0,
    x=[0.0, 0.0, 10.0, 10.0],
    y=[5.5, 0.5, 0.5, 5.5],
    n_stations=[0.0],
    n_values=[0.025],
    unit_system="Metric"
)
xs0 = st.CrossSection(
    station=0.0,
    x=[0.0, 0.0, 10.0, 10.0],
    y=[5.0, 0.0, 0.0, 5.0],
    n_stations=[0.0],
    n_values=[0.025],
    unit_system="Metric"
)

# 2. Configure steady inputs
inputs = st.SteadyInputs(
    cross_sections=[xs1000, xs500, xs0],
    flow_rate=15.0,            # 15 cms
    num_slices=100,
    regime=0,                  # Subcritical
    downstream_wsel=1.5,       # Tailwater boundary elevation
    downstream_bc_type=0,      # Known WSEL
    coeff_contraction=0.1,
    coeff_expansion=0.3
)

# 3. Solve steady profile
steady_results = st.solve_steady(inputs)
print("Steady WSELs:", steady_results["wsel"])
```

## Culvert example

```python
inputs = st.SteadyInputs(
    cross_sections=[xs1000, xs500, xs0],
    flow_rate=100.0,
    regime=0,
    downstream_wsel=3.0,
    culvert_stations=[50.0],
    culvert_shape_types=[0],
    culvert_spans=[5.0],
    culvert_rises=[5.0],
    culvert_roughness_ns=[0.012],
    culvert_lengths=[100.0],
    culvert_entrance_loss_coeffs=[0.5],
    culvert_exit_loss_coeffs=[1.0],
    culvert_inlet_types=[1],
    culvert_crest_elevs=[14.0],
    culvert_weir_lengths=[20.0],
    culvert_barrels=[2],
    culvert_active_barrels=[2],
    culvert_skew_angles=[15.0],
    culvert_barrel_spans=[[8.0, 6.0]],
    culvert_barrel_rises=[[6.0, 6.0]],
)
results = st.solve_steady(inputs)
print("Culvert control:", results.get("culvert_control_types"))
print("Diagnostics:", results.get("culvert_wsel_inlet"), results.get("culvert_q_barrels"))
```

`SteadyInputs` uses the same culvert field names as the JSON schema. Shape constants: `st.CULVERT_SHAPE_CIRCULAR` (0) through `st.CULVERT_SHAPE_HORSESHOE` (6).

## Bridge abutments

Per-side abutment fields (`bridge_abutment_left_*`, `bridge_abutment_right_*`). See [`docs/reference/equations.md`](../reference/equations.md) §6.D.

```python
inputs = st.SteadyInputs(
    cross_sections=[xs1000, xs500, xs0],
    flow_rate=15.0,
    downstream_wsel=1.5,
    bridge_stations=[500.0],
    bridge_low_chords=[5.0],
    bridge_high_chords=[7.0],
    bridge_low_flow_methods=[4],  # WSPRO
    bridge_abutment_left_widths=[1.0],
    bridge_abutment_right_widths=[4.0],
    bridge_abutment_right_top_elevations=[2.5],
)
result = st.solve_steady(inputs)
```

Rating curve with flattened abutment keys:

```python
curve = st.compute_bridge_rating_curve({
    "q_values": [15.0, 25.0],
    "low_chord": 5.0,
    "high_chord": 7.0,
    "z_up": 0.0,
    "z_down": 0.0,
    "tw_wsel": 2.5,
    "units": "Metric",
    "low_flow_method": 4,
    "channel_width": 10.0,
    "manning_n": 0.03,
    "abutment_left_width": 1.0,
    "abutment_right_width": 4.0,
    "abutment_right_top_elevation": 2.5,
})
```
