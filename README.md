# STREAM-1D

1D open-channel hydraulics solver (Rust). Steady gradually varied flow (Standard Step) and unsteady Saint-Venant routing on single reaches. Optional culverts, bridges, and one steady tributary junction.

Primary interface: Python extension (`streams1d`). Also compiles to WebAssembly. Stateless API: geometry and boundary inputs in, profile arrays out.

This repository is the solver only. It does not include a GUI, project database, or HEC-RAS file importer. [stream1d.com](https://stream1d.com) is a separate hosted application built on this engine (see [License](#license)).

## Capabilities

| Analysis | Structures | Limits |
|----------|------------|--------|
| Steady GVF (subcritical, supercritical, mixed) | Culverts (FHWA inlet/outlet), bridges (HEC-RAS Class A/B/C, pressure, weir) | Single reach; one tributary junction (steady, subcritical) |
| Unsteady routing (single reach) | Inline culverts and bridges | Upstream *Q*(*t*), downstream WSEL(*t*); no multi-reach networks |

Full HEC-RAS comparison: [`docs/reference/hecras_parity.md`](docs/reference/hecras_parity.md).

## Python

### Install

```bash
pip install streams1d
```

Requires Python ≥ 3.7. Wheels are published for Linux, macOS, and Windows (see [PyPI](https://pypi.org/project/streams1d/)).

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
import streams1d as st

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

Load geometry from JSON with `streams1d.import_utils.cross_section_from_dict`. Example fixtures: [`tests/fixtures/wasm_steady_culvert_tier1.json`](tests/fixtures/wasm_steady_culvert_tier1.json), [`tests/fixtures/wasm_steady_bridge_bu_bd_v22.json`](tests/fixtures/wasm_steady_bridge_bu_bd_v22.json).

Culvert, bridge, junction, and rating-curve examples: [`docs/python/getting_started.md`](docs/python/getting_started.md).

### Interactive notebook

[![Binder](https://mybinder.org/badge_logo.svg)](https://mybinder.org/v2/gh/jlillywh/STREAM-1D/main?filepath=python%2Fstreams1d_verification.ipynb)

[`python/streams1d_verification.ipynb`](python/streams1d_verification.ipynb) — profiles and HEC-RAS comparison plots. First Binder build may take several minutes.

## Inputs and outputs

**Cross sections** — river station; (*x*, *y*) cut polyline; Manning *n* zones (`n_stations`, `n_values`); optional `is_overbank`, `blocked_obstructions`, `ineffective_flow_areas`.

**Steady** — `flow_rate`, `regime` (0 subcritical, 1 supercritical, 2 mixed), downstream boundary (`downstream_wsel`, normal depth, rating curve, etc.). Structure fields: `culvert_*`, `bridge_*`.

**Unsteady** — `initial_wsel`, `initial_q`, `dt`, `num_steps`, `upstream_q_hydrograph`, `downstream_wsel_hydrograph`. Same structure fields as steady.

**Results** — `wsel`, `velocity`, `area`, `froude_number`, `critical_wsel`, `energy_grade_slope`. With culverts: control type, inlet/outlet HW, barrel and weir discharge. With bridges: flow regime, head loss. Unsteady structure outputs are `[time_step][structure_index]`.

Field reference: [`python/streams1d/__init__.py`](python/streams1d/__init__.py), [`docs/wasm_api.types.ts`](docs/wasm_api.types.ts). Equations: [`docs/reference/equations.md`](docs/reference/equations.md).

## Verification

| Case | Reference | Tolerance |
|------|-----------|-----------|
| ConSpan culvert 5/25/50 yr profiles | [`python/verification/hecras_conspan_profiles.json`](python/verification/hecras_conspan_profiles.json) | ±0.04 ft WSEL |
| Bridge abutments (WSPRO) | [`python/verification/bridge_abutment_hecras.json`](python/verification/bridge_abutment_hecras.json) | ±2 mm HW |
| Bridge BU/BD faces | [`python/verification/bridge_bu_bd_hecras.json`](python/verification/bridge_bu_bd_hecras.json) | ±2 mm HW |

```bash
PYTHONPATH=python python python/test_hecras_culvert_verification.py
cargo test --test bridge_bu_bd_hecras_verification
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
| [`docs/python/getting_started.md`](docs/python/getting_started.md) | Culvert, bridge, unsteady Python examples |
| [`docs/reference/equations.md`](docs/reference/equations.md) | GVF, Saint-Venant, culvert and bridge theory |
| [`docs/reference/hecras_parity.md`](docs/reference/hecras_parity.md) | Scope vs HEC-RAS |
| [`docs/reference/api_changelog.md`](docs/reference/api_changelog.md) | Input schema versions |
| [`docs/BRIDGE_INTERIOR_SECTIONS_API.md`](docs/BRIDGE_INTERIOR_SECTIONS_API.md) | BU/BD interior sections |
| [`docs/web/wasm_integration.md`](docs/web/wasm_integration.md) | WASM build and JavaScript usage |
| [`docs/development/testing.md`](docs/development/testing.md) | Test suites and CI |
| [`docs/development/publishing.md`](docs/development/publishing.md) | PyPI trusted publishing and releases |
| [`tech_spec.md`](tech_spec.md) | Host-application architecture |

## Repository layout

```
src/solvers/     steady, unsteady, culvert, bridge, junction
python/          streams1d bindings, verification data, notebook
docs/            reference manuals and WASM types
tests/           Rust integration tests and JSON fixtures
examples/wasm/   Node smoke tests and sample payloads
```

## License

Engine (this repository): [MIT License](LICENSE).

[stream1d.com](https://stream1d.com) web application: separate proprietary product (Lillywhite Water Solutions LLC), not covered by this license.
