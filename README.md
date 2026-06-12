# STREAM-1D

1D open-channel hydraulics solver (Rust). Steady gradually varied flow (Standard Step) and unsteady Saint-Venant routing on single reaches. Optional culverts, bridges, and one steady tributary junction.

Primary interface: Python extension (`stream1d`). Also compiles to WebAssembly. Stateless API: geometry and boundary inputs in, profile arrays out.

This repository is the solver only. It does not include a GUI, project database, or HEC-RAS file importer. [stream1d.com](https://stream1d.com) is a separate hosted application built on this engine (see [License](#license)).

**Verification** — Golden benchmarks vs HEC-RAS exports and hand-calibrated references live in [`verification/`](verification/) ([README](verification/README.md), [`fixtures/`](verification/fixtures/)). Run: `bash verification/run.sh`.

## Capabilities

| Analysis | Structures |
|----------|------------|
| Steady GVF (subcritical, supercritical, mixed) | Culverts (FHWA inlet/outlet), bridges (HEC-RAS Class A/B/C, pressure, weir, tapered piers) |
| Unsteady routing (single reach) | Inline culverts and bridges |

## Limitations (read before comparing to HEC-RAS)

| Topic | In this engine |
|-------|----------------|
| Topology | Single reach; one tributary junction (steady, subcritical) |
| Unsteady | One reach; upstream *Q*(*t*) and downstream WSEL(*t*); no multi-reach networks |
| Unsteady structures | Default (`unsteady_structure_coupling_mode: 0`): explicit post-step only. Mode `2`: hybrid implicit culvert inlet + subcritical low-flow bridge in Preissmann Jacobian; high-flow/outlet/overtopping still explicit. Mode `1` reserved. Per-step diagnostics when structures present (API v34). |
| Reach geometry | `blocked_obstructions`; `ineffective_flow_areas` on any cross section (steady and unsteady) |
| Reach densification | `max_spacing` inserts interior nodes; set `densify_reach_modifier_policy: 1` when reach ineffective or blocked must apply between user sections (default `0` = table blend only) |
| Bridge cuts | `guide_banks`, `bridge_ineffective_*`, approach/departure ineffective on explicit cuts; interpolated BU/BD inherit bridge ineffective, not reach modifiers |
| Reverse flow (v31) | Bridge rating (`q_values` ±), steady `flow_rate < 0`, unsteady bridge coupling when `Q < 0`. **Not supported:** culvert reversal, network/junction reversal, inferring direction from stages alone. See [`bridge_reverse_flow_rating.md`](docs/development/bridge_reverse_flow_rating.md) |

Modifier semantics: [`docs/reference/equations.md`](docs/reference/equations.md) §H0. **Densified-node inheritance:** §H1. Full HEC-RAS gap table: [`docs/reference/hecras_parity.md`](docs/reference/hecras_parity.md).

## Python

### Install

```bash
pip install stream1d
```

Requires Python ≥ 3.7. Wheels are published for Linux, macOS, and Windows (see [PyPI](https://pypi.org/project/stream1d/)).

### Install from source

For unreleased changes, clone the repository and build with Rust + maturin:

```bash
python -m venv .venv
source .venv/bin/activate   # Windows: .venv\Scripts\activate
pip install maturin pytest
maturin develop --features python
```

Rebuild with `maturin develop --features python` after pulling solver changes. Release process: [`docs/development/publishing.md`](docs/development/publishing.md).

### Steady profile

```python
import stream1d as st

xs_us = st.CrossSection(
    station=1000.0,
    x=[0.0, 0.0, 10.0, 10.0],
    y=[6.0, 1.0, 1.0, 6.0],
    n_stations=[0.0],
    n_values=[0.025],
    unit_system="Metric",
)
xs_ds = st.CrossSection(
    station=0.0,
    x=[0.0, 0.0, 10.0, 10.0],
    y=[5.0, 0.0, 0.0, 5.0],
    n_stations=[0.0],
    n_values=[0.025],
    unit_system="Metric",
)

result = st.solve_steady(st.SteadyInputs(
    cross_sections=[xs_us, xs_ds],
    flow_rate=15.0,
    downstream_wsel=1.5,
    regime=0,
))
print(result["wsel"])
print(result["velocity"])
```

### Unsteady routing

```python
result = st.solve_unsteady(st.UnsteadyInputs(
    cross_sections=[xs_us, xs_ds],
    initial_wsel=[2.0, 1.5],
    initial_q=[14.0, 14.0],
    dt=60.0,
    num_steps=5,
    upstream_q_hydrograph=[14.0] * 5,
    downstream_wsel_hydrograph=[1.0] * 5,
))
print(result["wsel"][-1])
```

### JSON fixtures

Load geometry from JSON with `stream1d.import_utils.cross_section_from_dict`. Example fixtures: [`tests/fixtures/wasm_steady_culvert_tier1.json`](tests/fixtures/wasm_steady_culvert_tier1.json), [`tests/fixtures/wasm_steady_bridge_bu_bd_v22.json`](tests/fixtures/wasm_steady_bridge_bu_bd_v22.json).

Culvert, bridge, junction, and rating-curve examples: [`docs/python/getting_started.md`](docs/python/getting_started.md).

### Interactive notebook

[![Binder](https://mybinder.org/badge_logo.svg)](https://mybinder.org/v2/gh/jlillywh/STREAM-1D/main?filepath=python%2Fstream1d_verification.ipynb)

[`python/stream1d_verification.ipynb`](python/stream1d_verification.ipynb) — profiles and HEC-RAS comparison plots. First Binder build may take several minutes.

## Inputs and outputs

**Cross sections** — river station; (*x*, *y*) polyline; Manning *n* zones; optional `is_overbank`, `blocked_obstructions`, `ineffective_flow_areas`, `guide_banks`. Modifier semantics: [`docs/reference/equations.md`](docs/reference/equations.md) §H0.

**Steady** — `flow_rate`, `regime` (0 subcritical, 1 supercritical, 2 mixed), downstream boundary (`downstream_wsel`, normal depth, rating curve, etc.), optional `max_spacing` and `densify_reach_modifier_policy` (0 none, 1 upstream, 2 downstream, 3 nearest). Structure fields: `culvert_*`, `bridge_*`. Tapered piers: legacy `bridge_pier_widths`, or `bridge_pier_top_widths` / `bridge_pier_bottom_widths` per pier, or piecewise `bridge_pier_width_elevations` / `bridge_pier_width_values` — see [`docs/development/pier_tapered_width.md`](docs/development/pier_tapered_width.md). Pier footings and nosing (API v28): `bridge_pier_footing_*`, `bridge_pier_nosing_*` — see [`docs/development/pier_footings_nosing.md`](docs/development/pier_footings_nosing.md). Deck vents (API v29): `bridge_deck_vent_*` — see [Bridge high flow](#bridge-high-flow) and [`docs/development/deck_vents_slotted_openings.md`](docs/development/deck_vents_slotted_openings.md).

**Unsteady** — `initial_wsel`, `initial_q`, `dt`, `num_steps`, `upstream_q_hydrograph`, `downstream_wsel_hydrograph`, same `max_spacing` / `densify_reach_modifier_policy` as steady. Same structure fields as steady.

**Results** — `wsel`, `velocity`, `area`, `froude_number`, `critical_wsel`, `energy_grade_slope`. With culverts: control type, inlet/outlet HW, barrel and weir discharge. With bridges: flow regime, head loss. Unsteady structure outputs are `[time_step][structure_index]`.

Field reference: [`python/stream1d/__init__.py`](python/stream1d/__init__.py), [`docs/wasm_api.types.ts`](docs/wasm_api.types.ts). Equations: [`docs/reference/equations.md`](docs/reference/equations.md).

## Bridge high flow

When upstream energy exceeds the **low chord** (`bridge_low_chords` or piecewise `bridge_deck_low_elevations`), the bridge solver leaves low-flow Class A/B/C and evaluates **pressure flow** through the net opening under the deck (piers and abutments subtracted). When energy exceeds the **high chord**, **roadway weir** overtopping is added. Low-flow and high-flow answers are compared; the governing headwater is the higher of the two.

| Regime | Typical condition | Discharge model |
|--------|-------------------|-----------------|
| Pressure only | Low chord &lt; EGL ≤ high chord | Main opening + optional deck vents |
| Combined overtopping | EGL &gt; high chord | *Q* = *Q*<sub>opening</sub> + *Q*<sub>vents</sub> + *Q*<sub>weir</sub> |

**Main opening** — Trapezoidal area under the deck low chord. If downstream tailwater is **below the maximum low chord** (deck soffit high point), FHWA **sluice-gate** form applies (`bridge_pressure_flow_coeffs_inlet` optional). If tailwater is **at or above** that elevation, **submerged orifice**: *Q* = *C* *A*<sub>net</sub> √(2*g*(*E*<sub>up</sub> − TW)) with `bridge_orifice_coeffs` (typical 0.8).

**Deck vents** (optional) — Localized grate/slot segments **above** the main soffit but **below** the roadway crest. Per-segment invert, soffit, width, and discharge coefficient (`bridge_deck_vent_*`). Submerged vent area grows with WSEL up to each segment soffit; vent flow uses the same drive head as the main submerged orifice and is summed in parallel. Type `0` = orifice (default); type `1` = slotted weir until the slot is submerged, then full-slot orifice. HEC-RAS 1D has no separate vent fields — this is a STREAM-1D extension. Full API: [`docs/development/deck_vents_slotted_openings.md`](docs/development/deck_vents_slotted_openings.md).

**Weir overtopping** — Bradley (1978) form on each deck segment whose crest is cleared by upstream EGL, with segment submergence factor from downstream tailwater. If the maximum segment submergence exceeds `bridge_max_weir_submergence` (default 0.98), the solver falls back to the **energy** method through the opening (no explicit weir or deck vents on that branch).

**Method selection** — `bridge_high_flow_methods` per bridge:

| Value | Behavior |
|-------|----------|
| `0` (default) | Pressure + weir; energy only on high weir submergence |
| `1` | Always energy balance through the obstructed opening |

Piecewise deck profiles (`bridge_deck_stations`, `bridge_deck_low_elevations`, `bridge_deck_high_elevations`) set local low/high chords for opening area, weir length, and vent placement. Equations: [`docs/reference/equations.md`](docs/reference/equations.md) §E–§F.

**HEC-RAS parity** — Phase 4.2 aligns iteration order, segment weir onset, and submergence caps. **Intentional remaining deltas** (deck vents, energy path without vents, opening-area approximation, explicit unsteady coupling): [`docs/development/pressure_weir_combined_flow_audit.md#intentional-remaining-deltas`](docs/development/pressure_weir_combined_flow_audit.md#intentional-remaining-deltas).

## Verification

External-source regression suites: **[`verification/`](verification/)** (catalog, fixtures, `bash verification/run.sh`).

| Case | Reference | Tolerance |
|------|-----------|-----------|
| ConSpan culvert 5/25/50 yr profiles | [`verification/fixtures/hecras_conspan_profiles.json`](verification/fixtures/hecras_conspan_profiles.json) | ±0.04 ft WSEL |
| Bridge abutments (WSPRO) | [`verification/fixtures/bridge_abutment_hecras.json`](verification/fixtures/bridge_abutment_hecras.json) | ±2 mm HW |
| Bridge BU/BD faces | [`verification/fixtures/bridge_bu_bd_hecras.json`](verification/fixtures/bridge_bu_bd_hecras.json) | ±2 mm HW |
| Bridge high flow (pressure / weir / energy) | [`verification/fixtures/bridge_high_flow_hecras.json`](verification/fixtures/bridge_high_flow_hecras.json) | ±2 mm HW — 6 cases |
| Bridge opening ↔ reach alignment (skew + offset origin) | [`tests/bridge_opening_alignment_verification.rs`](tests/bridge_opening_alignment_verification.rs) | preprocessor + validation |
| Roadway embankment fill (unified API) | [`verification/fixtures/bridge_roadway_embankment.json`](verification/fixtures/bridge_roadway_embankment.json) | ±2 mm WSEL |

```bash
bash verification/run.sh
PYTHONPATH=python python python/test_hecras_culvert_verification.py
```

Test commands: [`docs/development/testing.md`](docs/development/testing.md).

## Build targets

| Target | Command | Notes |
|--------|---------|-------|
| Python | `maturin develop --features python` | Default for research and scripting |
| WebAssembly | `bash build_wasm.sh` | Browser (`pkg/`) and Node (`pkg-node/`) |

WASM API: [`docs/web/wasm_integration.md`](docs/web/wasm_integration.md).

## Documentation

| Document | Contents |
|----------|----------|
| [`docs/README.md`](docs/README.md) | Index — canonical source per topic |
| [`docs/python/getting_started.md`](docs/python/getting_started.md) | Culvert, bridge, unsteady Python examples |
| [`docs/reference/equations.md`](docs/reference/equations.md) | GVF, Saint-Venant, culvert and bridge theory |
| [`docs/reference/hecras_parity.md`](docs/reference/hecras_parity.md) | Scope vs HEC-RAS (bridge pier editor field map) |
| [`docs/reference/api_changelog.md`](docs/reference/api_changelog.md) | Input schema versions |
| [`docs/development/pier_tapered_width.md`](docs/development/pier_tapered_width.md) | Tapered pier width API (v27) |
| [`docs/development/pier_footings_nosing.md`](docs/development/pier_footings_nosing.md) | Pier footings, nosing (API v28); plan polygons and wing walls (design) |
| [`docs/development/deck_vents_slotted_openings.md`](docs/development/deck_vents_slotted_openings.md) | Deck vents & slotted openings (API design, v29) |
| [`docs/development/extended_pier_shape_catalog.md`](docs/development/extended_pier_shape_catalog.md) | Extended pier shape catalog — `bridge_pier_shapes` 0–11 (API v29) |
| [`verification/`](verification/) | Golden benchmarks vs HEC-RAS / hand references — [`README`](verification/README.md), [`fixtures/`](verification/fixtures/), `bash verification/run.sh` |
| [`docs/development/pressure_weir_combined_flow_audit.md`](docs/development/pressure_weir_combined_flow_audit.md) | High-flow audit (Phase 4), intentional remaining deltas vs HEC-RAS |
| [`docs/BRIDGE_INTERIOR_SECTIONS_API.md`](docs/BRIDGE_INTERIOR_SECTIONS_API.md) | Advanced: BU/BD, opening alignment, guide banks |
| [`docs/web/wasm_integration.md`](docs/web/wasm_integration.md) | WASM build and JavaScript usage |
| [`docs/development/testing.md`](docs/development/testing.md) | Test suites and CI |
| [`docs/development/publishing.md`](docs/development/publishing.md) | PyPI trusted publishing and releases |
| [`docs/development/tech_spec.md`](docs/development/tech_spec.md) | Host-application architecture |

## Repository layout

```
src/solvers/     steady, unsteady, culvert, bridge, junction
python/          stream1d bindings, notebook
verification/    golden fixtures vs HEC-RAS / hand references (see README)
docs/            reference manuals and WASM types
tests/           Rust integration tests and JSON fixtures
examples/wasm/   Node smoke tests and sample payloads
```

## License

Engine (this repository): [MIT License](LICENSE).

[stream1d.com](https://stream1d.com) web application: separate proprietary product (Lillywhite Water Solutions LLC), not covered by this license.
