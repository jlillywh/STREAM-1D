# STREAM-1D

**STREAM-1D** is a 1D open-channel hydraulics engine for research and scripting. It solves **steady gradually varied flow** (standard step) on a single reach with optional **inline culverts**, **bridges** (Class A/B/C, pressure and weir overtopping, roadway decks and piers), and **one tributary junction** (steady, subcritical).

**Python** is the primary interface (`pip install stream1d` or `maturin develop` from source). You pass cross-section geometry and boundary conditions in; you get water-surface, discharge, and structure diagnostics out. There is no GUI, project database, or HEC-RAS file importer in this repository — [stream1d.com](https://stream1d.com) is a separate hosted product built on this solver ([License](#license)).

**Verification** — Benchmarks against HEC-RAS and hand-calibrated references: [`verification/`](verification/) ([README](verification/README.md)). Try the comparison notebook on [Binder](https://mybinder.org/v2/gh/jlillywh/STREAM-1D/main?filepath=python%2Fstream1d_verification.ipynb) or run `python3 scripts/run_verification_notebook.py` from a clone.

## Capabilities

| Analysis | Structures |
|----------|------------|
| Steady GVF (subcritical, supercritical, mixed) | Culverts (FHWA inlet/outlet), bridges (HEC-RAS Class A/B/C, pressure, weir, tapered piers) |

## Limitations (read before comparing to HEC-RAS)

| Topic | In this engine |
|-------|----------------|
| Topology | Single reach; one tributary junction (steady, subcritical) |
| Reach geometry | `blocked_obstructions`; `ineffective_flow_areas` on any cross section |
| Reach densification | `max_spacing` inserts interior nodes; set `densify_reach_modifier_policy: 1` when reach ineffective or blocked must apply between user sections (default `0` = table blend only) |
| Bridge cuts | `guide_banks`, `bridge_ineffective_*`, approach/departure ineffective on explicit cuts; interpolated BU/BD inherit bridge ineffective, not reach modifiers |
| Reverse flow (v31) | Bridge rating (`q_values` ±), steady `flow_rate < 0`. **Not supported:** culvert reversal, network/junction reversal, inferring direction from stages alone. See [`bridge_extensions.md`](docs/development/bridge_extensions.md) |

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


### JSON fixtures

Load geometry from JSON with `stream1d.import_utils.cross_section_from_dict`. Example payloads: [`tests/fixtures/`](tests/fixtures/) (culvert, bridge, steady). Culvert, bridge, junction, and rating-curve walkthroughs: [`docs/python/getting_started.md`](docs/python/getting_started.md).

### Verification notebook

[![Binder](https://mybinder.org/badge_logo.svg)](https://mybinder.org/v2/gh/jlillywh/STREAM-1D/main?filepath=python%2Fstream1d_verification.ipynb)

[`python/stream1d_verification.ipynb`](python/stream1d_verification.ipynb) compares STREAM-1D to HEC-RAS on ConSpan culvert and Issaquah01 bridge. **Binder** runs it in the browser with no local setup (first build may take a few minutes).

From a clone, run the same notebook headlessly (matches CI):

```bash
git clone https://github.com/jlillywh/STREAM-1D.git
cd STREAM-1D
python3 scripts/run_verification_notebook.py
```

More verification scenarios: [`verification/README.md`](verification/README.md).

## Inputs and outputs

**Cross sections** — river station; (*x*, *y*) polyline; Manning *n* zones; optional `is_overbank`, `blocked_obstructions`, `ineffective_flow_areas`, `guide_banks`. Modifier semantics: [`docs/reference/equations.md`](docs/reference/equations.md) §H0.

**Steady** — `flow_rate`, `regime` (0 subcritical, 1 supercritical, 2 mixed), downstream boundary (`downstream_wsel`, normal depth, rating curve, etc.), optional `max_spacing` and `densify_reach_modifier_policy` (0 none, 1 upstream, 2 downstream, 3 nearest). Structure fields: `culvert_*`, `bridge_*`. Pier, deck vent, ice, reverse-flow extensions: [`docs/development/bridge_extensions.md`](docs/development/bridge_extensions.md).

**Results** — `wsel`, `velocity` as lists of floats (one value per cross section). With culverts: control type, inlet/outlet HW, barrel and weir discharge. With bridges: flow regime, head loss.

Field reference: [`python/stream1d/__init__.py`](python/stream1d/__init__.py), [`docs/reference/api_changelog.md`](docs/reference/api_changelog.md). Equations: [`docs/reference/equations.md`](docs/reference/equations.md).


## Bridge high flow

When upstream energy exceeds the **low chord** (`bridge_low_chords` or piecewise `bridge_deck_low_elevations`), the bridge solver leaves low-flow Class A/B/C and evaluates **pressure flow** through the net opening under the deck (piers and abutments subtracted). When energy exceeds the **high chord**, **roadway weir** overtopping is added. Low-flow and high-flow answers are compared; the governing headwater is the higher of the two.

| Regime | Typical condition | Discharge model |
|--------|-------------------|-----------------|
| Pressure only | Low chord &lt; EGL ≤ high chord | Main opening + optional deck vents |
| Combined overtopping | EGL &gt; high chord | *Q* = *Q*<sub>opening</sub> + *Q*<sub>vents</sub> + *Q*<sub>weir</sub> |

**Main opening** — Trapezoidal area under the deck low chord. If downstream tailwater is **below the maximum low chord** (deck soffit high point), FHWA **sluice-gate** form applies (`bridge_pressure_flow_coeffs_inlet` optional). If tailwater is **at or above** that elevation, **submerged orifice**: *Q* = *C* *A*<sub>net</sub> √(2*g*(*E*<sub>up</sub> − TW)) with `bridge_orifice_coeffs` (typical 0.8).

**Deck vents** (optional) — Localized grate/slot segments **above** the main soffit but **below** the roadway crest. Per-segment invert, soffit, width, and discharge coefficient (`bridge_deck_vent_*`). Submerged vent area grows with WSEL up to each segment soffit; vent flow uses the same drive head as the main submerged orifice and is summed in parallel. Type `0` = orifice (default); type `1` = slotted weir until the slot is submerged, then full-slot orifice. HEC-RAS 1D has no separate vent fields — STREAM-1D extension. See [`docs/development/bridge_extensions.md`](docs/development/bridge_extensions.md).

**Weir overtopping** — Bradley (1978) form on each deck segment whose crest is cleared by upstream EGL, with segment submergence factor from downstream tailwater. If the maximum segment submergence exceeds `bridge_max_weir_submergence` (default 0.98), the solver falls back to the **energy** method through the opening (no explicit weir or deck vents on that branch).

**Method selection** — `bridge_high_flow_methods` per bridge:

| Value | Behavior |
|-------|----------|
| `0` (default) | Pressure + weir; energy only on high weir submergence |
| `1` | Always energy balance through the obstructed opening |

Piecewise deck profiles (`bridge_deck_stations`, `bridge_deck_low_elevations`, `bridge_deck_high_elevations`) set local low/high chords for opening area, weir length, and vent placement. Equations: [`docs/reference/equations.md`](docs/reference/equations.md) §E–§F.

**HEC-RAS parity** — Phase 4.2 aligns iteration order, segment weir onset, and submergence caps. **Intentional remaining deltas** (deck vents, energy path without vents, opening-area approximation, explicit unsteady coupling): [`docs/development/pressure_weir_combined_flow_audit.md#intentional-remaining-deltas`](docs/development/pressure_weir_combined_flow_audit.md#intentional-remaining-deltas).

## Verification

Two layers: **frozen JSON fixtures** under [`verification/fixtures/`](verification/fixtures/) and the **linked HEC-RAS oracle** under [`verification/oracle/`](verification/oracle/) (bundled `.g01` projects + committed reference WSEL). Overview: [`verification/README.md`](verification/README.md).

### Frozen fixture suites

```bash
bash verification/run.sh
PYTHONPATH=python python python/test_hecras_culvert_verification.py
```

| Case | Reference | Tolerance |
|------|-----------|-----------|
| ConSpan culvert 5/25/50 yr profiles (steady) | [`verification/fixtures/hecras_conspan_profiles.json`](verification/fixtures/hecras_conspan_profiles.json) | ±0.04 ft WSEL |
| Bridge abutments (WSPRO) | [`verification/fixtures/bridge_abutment_hecras.json`](verification/fixtures/bridge_abutment_hecras.json) | ±2 mm HW |
| Bridge BU/BD faces | [`verification/fixtures/bridge_bu_bd_hecras.json`](verification/fixtures/bridge_bu_bd_hecras.json) | ±2 mm HW |
| Bridge high flow (pressure / weir / energy) | [`verification/fixtures/bridge_high_flow_hecras.json`](verification/fixtures/bridge_high_flow_hecras.json) | ±2 mm HW — 6 cases |
| Bridge opening ↔ reach alignment (skew + offset origin) | [`tests/bridge_opening_alignment_verification.rs`](tests/bridge_opening_alignment_verification.rs) | preprocessor + validation |
| Roadway embankment fill (unified API) | [`verification/fixtures/bridge_roadway_embankment.json`](verification/fixtures/bridge_roadway_embankment.json) | ±2 mm WSEL |

### Linked oracle (CI)

Maps STREAM-1D inputs from the same HEC-RAS geometry as bundled projects; compares WSEL without requiring a local HEC install (committed references).

```bash
maturin develop --features python --release
python3 verification/oracle/scripts/run_oracle_ci.py
```

| Scenario | Project | Gate |
|----------|---------|------|
| `conspan_steady_linked.json` | ConSpan steady | ±0.04 ft profile |

Details: [`verification/oracle/README.md`](verification/oracle/README.md).

Test commands: [`docs/development/testing.md`](docs/development/testing.md).

## Building from source

Researchers extending the solver need Rust and [maturin](https://www.maturin.rs/):

```bash
maturin develop --features python --release
```

Browser/WebAssembly builds are maintained separately — see [`docs/web/wasm_integration.md`](docs/web/wasm_integration.md).

## Documentation

| Document | Contents |
|----------|----------|
| [`docs/README.md`](docs/README.md) | Documentation index |
| [`docs/python/getting_started.md`](docs/python/getting_started.md) | Python examples |
| [`docs/reference/equations.md`](docs/reference/equations.md) | GVF, Saint-Venant, culvert and bridge theory |
| [`docs/reference/hecras_parity.md`](docs/reference/hecras_parity.md) | Scope vs HEC-RAS |
| [`docs/reference/api_changelog.md`](docs/reference/api_changelog.md) | Input schema versions |
| [`docs/development/bridge_extensions.md`](docs/development/bridge_extensions.md) | Pier, deck vent, ice, reverse-flow fields |

| [`docs/development/pressure_weir_combined_flow_audit.md`](docs/development/pressure_weir_combined_flow_audit.md) | High-flow intentional deltas vs HEC |
| [`docs/BRIDGE_INTERIOR_SECTIONS_API.md`](docs/BRIDGE_INTERIOR_SECTIONS_API.md) | BU/BD, opening alignment |
| [`docs/development/testing.md`](docs/development/testing.md) | Test suites and CI |
| [`verification/`](verification/) | Golden fixtures + linked oracle |

## Repository layout

```
src/solvers/     steady, culvert, bridge, junction
python/          stream1d package and verification notebook
verification/    HEC-RAS parity benchmarks (see README)
docs/            reference manuals
tests/           Rust tests and JSON example payloads
```

## License

Engine (this repository): [MIT License](LICENSE).

[stream1d.com](https://stream1d.com) web application: separate proprietary product (Lillywhite Water Solutions LLC), not covered by this license.
