# STREAM-1D

1D open-channel hydraulics solver (Rust). Steady gradually varied flow (Standard Step) and unsteady Saint-Venant routing on single reaches. Optional culverts, bridges, and one steady tributary junction.

Primary interface: Python extension (`stream1d`). Also compiles to WebAssembly. Stateless API: geometry and boundary inputs in, profile arrays out.

This repository is the solver only. It does not include a GUI, project database, or HEC-RAS file importer. [stream1d.com](https://stream1d.com) is a separate hosted application built on this engine (see [License](#license)).

**Verification** — Golden benchmarks vs HEC-RAS exports and hand-calibrated references live in [`verification/`](verification/) ([README](verification/README.md), [`fixtures/`](verification/fixtures/)). Run: `bash verification/run.sh`.

## Capabilities

| Analysis | Structures |
|----------|------------|
| Steady GVF (subcritical, supercritical, mixed) | Culverts (FHWA inlet/outlet), bridges (HEC-RAS Class A/B/C, pressure, weir, tapered piers) |
| Unsteady routing (single reach, Preissmann θ-scheme) | Inline culverts and bridges; coupling modes 0–4 (mode **4** for culvert backwater) |

## Limitations (read before comparing to HEC-RAS)

| Topic | In this engine |
|-------|----------------|
| Topology | Single reach; one tributary junction (steady, subcritical) |
| Unsteady | One reach; upstream *Q*(*t*) and downstream WSEL(*t*); no multi-reach networks |
| Unsteady structures | `unsteady_structure_coupling_mode`: **`0`** post-step only (default); **`1`** reserved; **`2`** hybrid implicit (culvert inlet + subcritical bridge in Preissmann Jacobian; overtopping/outlet/high-flow explicit fallback); **`3`** monolithic Newton (experimental); **`4`** quasi-steady particular + perturbation (**recommended for culvert backwater** — uses mode `2` physics inside Preissmann). Per-step structure diagnostics (API v34). See [Unsteady flow](#unsteady-flow-and-water-surface-elevation). |
| Reach geometry | `blocked_obstructions`; `ineffective_flow_areas` on any cross section (steady and unsteady) |
| Reach densification | `max_spacing` inserts interior nodes; set `densify_reach_modifier_policy: 1` when reach ineffective or blocked must apply between user sections (default `0` = table blend only) |
| Bridge cuts | `guide_banks`, `bridge_ineffective_*`, approach/departure ineffective on explicit cuts; interpolated BU/BD inherit bridge ineffective, not reach modifiers |
| Reverse flow (v31) | Bridge rating (`q_values` ±), steady `flow_rate < 0`, unsteady bridge coupling when `Q < 0`. **Not supported:** culvert reversal, network/junction reversal, inferring direction from stages alone. See [`bridge_extensions.md`](docs/development/bridge_extensions.md) |

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
    unsteady_structure_coupling_mode=0,  # 4 for culvert backwater on Q ramps
))
print(result["wsel"][-1])
```

See [Unsteady flow and water surface elevation](#unsteady-flow-and-water-surface-elevation) for the full time-step pipeline.

### JSON fixtures

Load geometry from JSON with `stream1d.import_utils.cross_section_from_dict`. Example fixtures: [`tests/fixtures/wasm_steady_culvert_tier1.json`](tests/fixtures/wasm_steady_culvert_tier1.json), [`tests/fixtures/wasm_steady_bridge_bu_bd_v22.json`](tests/fixtures/wasm_steady_bridge_bu_bd_v22.json).

Culvert, bridge, junction, and rating-curve examples: [`docs/python/getting_started.md`](docs/python/getting_started.md).

### Interactive notebook

[![Binder](https://mybinder.org/badge_logo.svg)](https://mybinder.org/v2/gh/jlillywh/STREAM-1D/main?filepath=python%2Fstream1d_verification.ipynb)

[`python/stream1d_verification.ipynb`](python/stream1d_verification.ipynb) — ConSpan culvert and **Issaquah01 bridge** steady profiles with HEC-RAS comparison tables and plots. First Binder build may take several minutes.

**Local run:** run from the **repository root** (not `python/`). `.venv`, `requirements.txt`, and `Cargo.toml` all live there.

```bash
cd ~/Lillywhite_Consulting/lillywhite_engine/STREAM-1D   # repo root
python3 scripts/run_verification_notebook.py             # headless (matches CI)
python3 scripts/run_verification_notebook.py --serve     # Jupyter UI (--no-browser on WSL)
```

On **WSL**, `jupyter notebook` keeps the terminal busy (that is normal). Copy the `http://127.0.0.1:8888/...` URL into your **Windows** browser. Stop the server with **Ctrl+C** (twice). Prefer opening [`python/stream1d_verification.ipynb`](python/stream1d_verification.ipynb) in **Cursor/VS Code** with the `.venv` Python kernel instead.

Manual equivalent:

```bash
cd ~/Lillywhite_Consulting/lillywhite_engine/STREAM-1D   # must be repo root
python3 -m venv .venv && source .venv/bin/activate
pip install -r requirements.txt
maturin develop --features python --release
cd python && jupyter notebook --no-browser stream1d_verification.ipynb
```

If your shell is already in `python/`, run `cd ..` first. CI executes this notebook headlessly on every PR (`verification-notebook` job in [`.github/workflows/ci.yml`](.github/workflows/ci.yml)).

## Inputs and outputs

**Cross sections** — river station; (*x*, *y*) polyline; Manning *n* zones; optional `is_overbank`, `blocked_obstructions`, `ineffective_flow_areas`, `guide_banks`. Modifier semantics: [`docs/reference/equations.md`](docs/reference/equations.md) §H0.

**Steady** — `flow_rate`, `regime` (0 subcritical, 1 supercritical, 2 mixed), downstream boundary (`downstream_wsel`, normal depth, rating curve, etc.), optional `max_spacing` and `densify_reach_modifier_policy` (0 none, 1 upstream, 2 downstream, 3 nearest). Structure fields: `culvert_*`, `bridge_*`. Pier, deck vent, ice, reverse-flow extensions: [`docs/development/bridge_extensions.md`](docs/development/bridge_extensions.md).

**Unsteady** — `initial_wsel`, `initial_q`, `dt`, `num_steps`, `upstream_q_hydrograph`, downstream boundary (`downstream_wsel_hydrograph` or `downstream_bc_type` / `downstream_bc_slope` / rating curve), `theta` (Preissmann weight, default 0.6), `unsteady_structure_coupling_mode` (0–4), same `max_spacing` / `densify_reach_modifier_policy` as steady. Same structure fields as steady.

**Results** — `wsel`, `q`, `velocity` as `[time_step][cross_section_index]`. With culverts: control type, inlet/outlet HW, barrel and weir discharge. With bridges: flow regime, head loss. Structure diagnostics are `[time_step][structure_index]`. Optional Courant / recommended-`dt` hints.

Field reference: [`python/stream1d/__init__.py`](python/stream1d/__init__.py), [`docs/wasm_api.types.ts`](docs/wasm_api.types.ts). Equations: [`docs/reference/equations.md`](docs/reference/equations.md).

## Unsteady flow and water surface elevation

`solve_unsteady` routes one reach in time using a **Preissmann θ-scheme** discretization of the 1D Saint-Venant equations on a **densified computational grid**, then optionally reconciles **inline culverts and bridges**. Water surface elevation (WSEL) at each user cross section is read from the densified solution at the end of every time step.

Theory: [`docs/reference/equations.md`](docs/reference/equations.md) §4. Implementation: `src/solvers/unsteady.rs`, `src/solvers/unsteady/preissmann.rs`.

### Governing equations

Continuity and momentum are solved in metric internally:

$$\frac{\partial A}{\partial t} + \frac{\partial Q}{\partial x} = 0$$

$$\frac{\partial Q}{\partial t} + \frac{\partial}{\partial x}\left(\frac{Q^2}{A}\right) + gA\left(\frac{\partial y}{\partial x} - S_0 + S_f\right) = 0$$

where $y$ is water surface elevation, $A$ is storage area from the cross-section lookup table, and friction slope $S_f = (Q/\bar K)^2$ uses the average conveyance $\bar K$ between adjacent nodes (compound channel: sum of left overbank, channel, and right overbank conveyances when `is_overbank` is set). Momentum uses **flow area** (`active_area` when ineffective or guide banks clip conveyance) for velocity $V = Q/A$.

### Computational grid

1. **User cross sections** — sorted upstream → downstream; each gets an elevation→hydraulics lookup table (`num_slices`, default 80).
2. **Densification** — when adjacent user stations exceed `max_spacing`, interior nodes are inserted (linear interpolation of geometry; optional copy of reach modifiers via `densify_reach_modifier_policy`).
3. **Bridge layout** — explicit BU/BD (and optional interior) cuts add nodes at bridge faces.
4. **Structure intervals** — culverts and bridges occupy the reach interval between the upstream and downstream node that bracket each `culvert_stations[i]` / `bridge_stations[i]`.

All time stepping runs on this **densified grid**; results are mapped back to the user cross sections for output.

### Initial conditions

When `initial_wsel` is supplied, the engine **replaces** it with a **steady subcritical profile** (`solve_steady`) at `initial_q[0]` and the first-step downstream boundary — structures included — so the unsteady run starts on a consistent backwater surface. `initial_q` sets discharge at every node (metric internally). Stages are clamped above bed + 0.05 m.

### One time step (default mode `0`; modes `2`–`4` add steps below)

```text
For each hydrograph index:
  Q_up ← upstream_q_hydrograph[step]
  TW   ← downstream boundary (see below)

  [Mode 4 only]  y_qs ← steady profile at (Q_up, TW)
                 re-anchor: y ← y_qs + (y − y_qs_prev)

  Preissmann step on densified grid:
    • Upstream BC: Q[0] = Q_up (continuity row)
    • Downstream BC: WSEL[N−1] = TW (or coupled — see below)
    • Assemble block-tridiagonal system (θ-weighted continuity + momentum per interval)
    • Friction S_f from conveyance; optional contraction/expansion terms (reach Exp/Cntr)
    • [Modes 2, 4] Replace reach momentum rows at eligible structure intervals with
      implicit culvert / bridge residuals; add swell-head friction patches on culvert approach

  Post-step structure coupling (up to 5 passes, downstream-first):
    • Culvert: solve FHWA culvert rating → set upstream face WSEL to required headwater
    • Bridge: Class A/B/C / pressure / weir coupling at bridge interval
    • [Mode 2, 4] Skip explicit culvert pass when implicit residual already satisfied
    • Optional chained standard-step backwater on approach reach cells when Q is changing

  [Mode 4 only]  Blend perturbation η = y − y_qs toward zero (stronger when |dQ/dt| is small);
                 at constant Q, snap y ← y_qs; partial culvert face refresh if HW residual remains

  Clamp min depth; enforce Q[0] = Q_up; re-apply downstream WSEL

  Record WSEL, Q, velocity at user cross sections → wsel[step], q[step], velocity[step]
```

**Downstream boundary** (`downstream_bc_type`):

| Type | Behavior |
|------|----------|
| `0` (default) | Fixed stage from `downstream_wsel_hydrograph[step]` |
| `1` | Critical depth from downstream section geometry and local $Q$ |
| `2` | Normal depth from `downstream_bc_slope` (iterated with Preissmann when $Q$ couples) |
| `3` | Rating curve (`downstream_bc_rating_q` / `downstream_bc_rating_wsel`) |

Types `1`–`3` outer-iterate Preissmann with updated downstream stage until $Q$–stage is consistent (up to 12 passes).

### Structure coupling modes

Set `unsteady_structure_coupling_mode` on `UnsteadyInputs`:

| Mode | Name | When to use |
|------|------|-------------|
| **0** | Post-step only | Reach-only routing, or legacy explicit structure updates |
| **1** | Reach–structure–reach | Reserved (not implemented) |
| **2** | Hybrid implicit | Culvert inlet + subcritical bridge in Preissmann; explicit fallback for overtopping, outlet control, high-flow bridge |
| **3** | Monolithic Newton | Experimental — outer Newton on full Preissmann + culvert HW each step |
| **4** | Quasi-steady particular | **Recommended for culvert approach pools** — decomposes $y = y_{qs}(Q,TW) + \eta$; re-anchors to a fresh steady profile each step; Preissmann and post-step coupling use **mode 2** physics; prevents slow backwater drift on long $Q$ ramps while retaining transient $\eta$ when $|dQ/dt|$ is large |

Mode **4** is still full implicit Saint-Venant routing: the quasi-steady profile tracks the slowly varying backwater; $\eta$ carries the transient correction. It is **not** a replacement for unsteady mass/momentum — it stabilizes operator splitting between reach friction and culvert headwater coupling.

### What WSEL means in the output

`result["wsel"][step][i]` is the **water surface elevation** at user cross section `i` after the complete step (Preissmann + structure coupling + mode-4 reconcile + boundary enforcement). Velocity uses the same stage and local flow area. Culvert and bridge diagnostic arrays report structure-specific headwaters and losses at the same time index.

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
| `reach_mild_unsteady_linked.json` | Open channel, no structures | Linked verify vs committed reference |
| `conspan_unsteady_ramp_matrix_mode4.json` | ConSpan arch culvert, Q ramp | Overall max \|Δ\| ≤ **0.12 ft** vs HEC (mode **4**) |
| `conspan_steady_linked.json` | ConSpan steady | ±0.04 ft profile |

Diagnostic (no pass/fail): `conspan_unsteady_ramp_matrix.json` (mode 2). Details: [`verification/oracle/README.md`](verification/oracle/README.md).

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
| [`docs/README.md`](docs/README.md) | Documentation index |
| [`docs/python/getting_started.md`](docs/python/getting_started.md) | Python examples |
| [`docs/reference/equations.md`](docs/reference/equations.md) | GVF, Saint-Venant, culvert and bridge theory |
| [`docs/reference/hecras_parity.md`](docs/reference/hecras_parity.md) | Scope vs HEC-RAS |
| [`docs/reference/api_changelog.md`](docs/reference/api_changelog.md) | Input schema versions |
| [`docs/development/bridge_extensions.md`](docs/development/bridge_extensions.md) | Pier, deck vent, ice, reverse-flow fields |
| [`docs/development/unsteady_structure_coupling.md`](docs/development/unsteady_structure_coupling.md) | Unsteady coupling modes 0–4 |
| [`docs/development/pressure_weir_combined_flow_audit.md`](docs/development/pressure_weir_combined_flow_audit.md) | High-flow intentional deltas vs HEC |
| [`docs/BRIDGE_INTERIOR_SECTIONS_API.md`](docs/BRIDGE_INTERIOR_SECTIONS_API.md) | BU/BD, opening alignment |
| [`docs/web/wasm_integration.md`](docs/web/wasm_integration.md) | WASM build and JavaScript |
| [`docs/development/testing.md`](docs/development/testing.md) | Test suites and CI |
| [`docs/development/publishing.md`](docs/development/publishing.md) | PyPI releases |
| [`docs/development/tech_spec.md`](docs/development/tech_spec.md) | Host-app architecture |
| [`verification/`](verification/) | Golden fixtures + linked oracle |

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
