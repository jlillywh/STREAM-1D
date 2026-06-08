# STREAM-1D

**An open-source 1D open-channel hydraulics engine for the web and Python.**

STREAM-1D is a Rust 1D open-channel hydraulics engine. It provides steady gradually varied flow (Standard Step, including culverts, bridges, and main-stem/tributary junctions) and unsteady Saint-Venant routing on single reaches. The core solver is decoupled from any specific user interface and compiles to two primary targets: WebAssembly (WASM) for client-side execution in the browser, and a native Python extension for automated scripting and batch processing. The API is stateless: structured inputs in, result arrays out.

## Project Goals

* **Embeddable Execution:** Run hydraulic simulations in web dashboards or Python data pipelines without requiring desktop hydraulic software.
* **Structural Hydraulics:** Model inline structures and composite roughness on single reaches or a main stem with one joining tributary (steady)—culverts, bridge piers, roadway overtopping, and multi-zone Manning's *n*.
* **Unsteady Routing:** Dynamic routing with upstream flow and downstream stage hydrographs; stabilization for steep transients and mixed regimes is an active development focus.
* **WebAssembly API:** Browser and Worker integration via `solveSteady` / `solveUnsteady`, `computeCulvertRatingCurve`, metadata discovery (`getWasmApiMetadata`), and input validation (`validateSteadyInputs`). Culvert **Tier 1** (explicit inlet types, invert overrides, roadway overtopping, control reporting) and **Tier 2a** (extended culvert diagnostics, headwater rating curves) ship at **API version 3**.

**Web app integrators:** See [`docs/wasm_integration.md`](docs/wasm_integration.md) for Worker setup and culvert field mapping. Steady tributary junctions use a two-branch API (one main stem array + one tributary array). HEC-RAS projects with three reaches at a confluence must merge or concatenate the upper and lower main stems before calling WASM. See [`docs/web_gui_tributary_junction.md`](docs/web_gui_tributary_junction.md). Culvert GUI handoff spec (Tier 1 + Tier 2a): [`docs/web_gui_culvert_integration.md`](docs/web_gui_culvert_integration.md).

## Architecture

* **Stateless:** No project files, local file administration, or hidden global state inside the engine. Each call is an independent solve.
* **UI-Agnostic:** The library exposes solver functions only; threading, workers, and visualization are the responsibility of the host application.
* **Open-Channel Focus:** Non-linear cross-section lookup tables with composite Manning's *n*, subdivided overbank/channel geometry, and mixed-regime steady profiles.
* **Intermediate Outputs:** Steady results include section-by-section area, top width, velocity, Froude number, and energy grade slope—useful for capacity review without running an unsteady simulation.

## Limitations (read before comparing to HEC-RAS)

STREAM-1D is an **embeddable 1D hydraulics engine** — the Rust/WASM/Python solve core in this repository, not a complete desktop product like HEC-RAS. It exposes a **stateless** API (`cross_sections` and boundary inputs in, profile arrays out). It does **not** ship a user interface, project database, RAS Map, 2D floodplain meshing, or native HEC-RAS Plan/Unsteady solvers.

**Companion web applications** built on this engine (not part of this repository) may provide cross-section editing, HEC-RAS geometry import (e.g. `.g01`), and project persistence. Those features convert imported or edited geometry into `SteadyInputs` / `UnsteadyInputs` before calling WASM. The Python bindings in this repo accept geometry arrays directly — they do not include a HEC-RAS file importer or cross-section editor.

### What the STREAM-1D engine supports

| Area | Supported |
|------|-----------|
| **Steady flow** | Standard Step backwater/drawdown; subcritical, supercritical, and mixed regime (`regime` 0/1/2) |
| **Boundary conditions (steady)** | Known WSEL, critical depth, normal depth, rating curve (upstream and downstream) |
| **Cross-sections** | Arbitrary $(x,y)$ polylines; composite Manning's *n*; optional channel/overbank subdivision (`is_overbank`) |
| **Main stem + tributary (steady)** | One tributary joining one main channel at a shared WSEL node — main stem above/below the junction plus tributary inflow (`tributary_cross_sections`, `tributary_flow_rate`, `junction_main_station`); subcritical only (see [`docs/web_gui_tributary_junction.md`](docs/web_gui_tributary_junction.md)) |
| **Culverts (steady, main stem)** | Circular, box, arch, and ConSpan; FHWA-style inlet/outlet control with signed barrel slope (adverse grade supported): 1. Explicit inlet types, optional invert elevations, roadway overtopping weir, composite bottom roughness, sediment blockage depth, per-culvert control type (`inlet` / `outlet` / `overtopping`), 2. Extended diagnostics per culvert — inlet vs outlet headwater, barrel vs weir discharge split, barrel depth, velocity, and Froude number; standalone `computeCulvertRatingCurve` for headwater vs $Q$ at fixed tailwater |
| **Bridges (steady, main stem)** | Yarnell Class A pier loss; pressure (orifice) flow; roadway weir overtopping |
| **Unsteady flow** | Preissmann Saint-Venant on a **single reach**; upstream $Q(t)$ and downstream WSEL($t$) hydrographs |
| **Outputs** | WSEL, critical WSEL, velocity, area, top width, Froude number, energy grade slope (+ `tributary_wsel`, `tributary_velocity`, `tributary_froude` when a junction is modeled; + `culvert_control_types` and Tier 2a culvert arrays — `culvert_wsel_inlet`, `culvert_wsel_outlet`, `culvert_q_barrels`, `culvert_q_weirs`, `culvert_barrel_depths`, `culvert_barrel_velocities`, `culvert_barrel_froude` — when culverts are modeled) |

### Companion web application features (not in this repository)

These are implemented in the **web GUI** that uses this engine, not in the Rust/WASM/Python solver crate:

| Feature | Description |
|---------|-------------|
| **Cross-section editing** | Interactive editing of reach geometry and Manning's *n* in the browser |
| **HEC-RAS geometry import** | Import HEC-RAS geometry files (e.g. `.g01`) to build reaches, cross-sections, and structures automatically, then map to solver inputs (including merging upper + lower main stem at a junction when needed) |

### Not supported (common HEC-RAS features)

| Category | HEC-RAS capability | STREAM-1D today |
|----------|-------------------|-----------------|
| **Dimensionality** | 1D, 2D, and coupled 1D/2D | **1D only** |
| **River networks** | Dendritic systems, multiple junctions, looped reaches | **One** main stem + **one** tributary (**steady only**); no general network graph |
| **Unsteady scope** | Networks, structures, storage areas, lateral inflows | **Single reach**; **no** inline culverts/bridges in the unsteady sweep |
| **Storage & diversions** | Ponds, reservoirs, split flow, lateral structures, pumps, gates | Not modeled |
| **Inline weirs & dams** | Standalone weirs, inline structures, dam breach | Not modeled (bridge roadway overtopping only) |
| **Bridge hydraulics** | Full low-flow classes, pressure/weir combinations, bridge methods, abutments, deck geometry | Yarnell **Class A pier loss** only; simplified pressure + weir overtopping; no abutment or Class B/C low flow |
| **Culvert hydraulics** | Full HEC-RAS culvert catalog (pipe-arch, horseshoe, etc.), barrel skew, unequal multi-barrel flow, supercritical barrel routing in mixed profiles, culverts in unsteady networks | FHWA nomograph (circular, box, arch, ConSpan) with explicit inlet types; multi-barrel **equal** $Q$ split; invert offsets, roadway overtopping, Tier 2a diagnostics and rating-curve API — **no** skew, extended shape catalog, unequal barrels, supercritical culvert solve in the upstream sweep, or unsteady inline culverts |
| **Ineffective flow** | Roadway embankment blocking, blocked obstructions, storage from ineffective areas | Partial: `channel_area` at structure-adjacent sections when overbanks are subdivided — not full RAS ineffective-flow workflow |
| **Terrain & mapping** | RAS Terrain, TIN/bathymetry authoring, RAS Map | **Not in the engine** — the companion **web app** may edit cross-sections and import HEC-RAS geometry; the solver only receives $(x,y)$ sections and stations |
| **Sediment & morphology** | Mobile bed, sediment transport, scour | Not modeled (optional fixed culvert blockage depth only) |
| **Water quality & ice** | Temperature, water quality, ice jams | Not modeled |
| **Project workflow** | Full HEC-RAS `.prj` with Plan, Geometry, and Unsteady files | **Not in the engine** — no built-in project format; the **web app** may import geometry and manage projects, then call WASM per solve |
| **Regulatory reporting** | FEMA, flood insurance, HEC-RAS report templates | Not included |

### Practical guidance

* **Good fit:** Embedding steady or unsteady 1D solves in a **web dashboard** (with optional HEC-RAS import and cross-section editing in that app) or a **Python pipeline** where you supply geometry arrays directly.
* **Poor fit:** Replacing HEC-RAS for FEMA studies, complex multi-reach unsteady networks, 2D overbank flood routing, or models that rely on RAS-specific structure and ineffective-flow workflows without host-app preprocessing.
* **Junction models:** Import upper + lower main stem as one `cross_sections` array (see [`docs/web_gui_tributary_junction.md`](docs/web_gui_tributary_junction.md)).
* **Active development:** Unsteady stabilization for steep transients and mixed regimes; broader network and structure support may follow — check release notes and open issues.

For host-application architecture (Web Workers, data transfer, GIS integration), see [`tech_spec.md`](tech_spec.md).

## Repository Structure

```
streams1d/
├── Cargo.toml                  # Rust library and WASM dependencies configuration
├── README.md                   # Project documentation and equations
├── tech_spec.md                # Host-app architecture and integration scope
├── build_wasm.sh               # WSL script to build WASM package
├── src/
│   ├── lib.rs                  # WASM entrypoints (solveSteady, getWasmApiMetadata, …)
│   ├── wasm_api.rs             # API metadata & version constants for host apps
│   ├── utils.rs                # Matrix solvers (Thomas, Block Thomas) and unit systems
│   ├── geometry/
│   │   ├── mod.rs              # Geometry module exports
│   │   └── processor.rs        # Cross-section slicing, area, perimeter, and composite roughness
│   └── solvers/
│       ├── mod.rs              # Solvers module exports
│       ├── steady.rs           # Standard Step backwater and critical depth solvers
│       ├── junction.rs         # Steady main-stem + tributary junction solver
│       ├── bridge.rs           # Bridge pier, pressure, and weir hydraulics
│       ├── culvert.rs          # Culvert inlet/outlet control (FHWA-style)
│       └── unsteady.rs         # Saint-Venant dynamic routing solver
├── python/                     # Python bindings, tests, and verification notebook
├── docs/                       # Integration guides for host applications
│   ├── wasm_integration.md     # WASM Worker setup, Tier 1 culvert mapping
│   ├── wasm_api.types.ts       # TypeScript definitions for the web app
│   ├── web_gui_culvert_integration.md # Culvert GUI spec — Tier 1 + Tier 2a (companion web app)
│   ├── web_gui_culvert_tier1.md     # Superseded; see web_gui_culvert_integration.md
│   └── web_gui_tributary_junction.md
├── examples/wasm/              # Worker reference + Node smoke test
├── tests/
│   ├── fixtures/               # JSON payloads for WASM contract tests
│   └── wasm_json_contract.rs   # WASM JSON schema integration tests
└── pkg/                        # Compiled WASM package generated by wasm-pack
```

---

## Mathematical Formulations

### 1. Equivalent Composite Roughness (Horton-Einstein)
When Manning's roughness coefficient ($n$) varies across a cross-section, the composite roughness $n_{composite}$ for a wetted perimeter $P$ composed of $M$ segments is:
$$n_{composite} = \left( \frac{\sum_{j=1}^{M} P_j n_j^{1.5}}{P} \right)^{2/3}$$

For culverts with varying bottom and top roughness, the Horton-Einstein composite Manning's $n$ is evaluated when the water depth exceeds the specified bottom roughness depth ($d_{bottom}$):
$$n_c = \left[ \frac{P_{bottom} n_{bottom}^{1.5} + P_{top} n_{top}^{1.5}}{P_{total}} \right]^{2/3}$$

### 2. Gradually Varied Flow Energy Balance
The Standard Step Method solves the 1D Energy Equation between two adjacent cross-sections:
$$WSEL_2 + \alpha_2 \frac{V_2^2}{2g} = WSEL_1 + \alpha_1 \frac{V_1^2}{2g} + h_f + h_o$$
where:
* $\alpha_1, \alpha_2$ are velocity-head coefficients (implemented as $1.0$ in the Standard Step sweep). Culvert outlet-control energy uses $\alpha \approx 1.3$ on adjacent approach/departure velocities (see Section 5B).
* Friction loss ($h_f$) is calculated using the average conveyance:
  $$h_f = L \bar{S}_f = L \left( \frac{Q}{\bar{K}} \right)^2, \quad \bar{K} = \frac{K_1 + K_2}{2}$$
* Minor expansion/contraction losses are represented by $h_o$:
  $$h_o = C_{c/e} \left| \alpha_2 \frac{V_2^2}{2g} - \alpha_1 \frac{V_1^2}{2g} \right|$$

### 3. Mixed Regime Selection (Specific Force / Momentum)
For mixed regime profiles (`regime = 2`), subcritical and supercritical sweeps are both computed; at each cross-section the result with the **higher specific force** is selected:
$$M = \frac{Q^2}{g A} + A \bar{y}, \quad A \bar{y} = \int_{Y_{min}}^{WSEL} A(y) dy$$

### 4. 1D Saint-Venant Equations (Unsteady Routing)
* **Continuity:** $\frac{\partial A}{\partial t} + \frac{\partial Q}{\partial x} = 0$
* **Momentum:** $\frac{\partial Q}{\partial t} + \frac{\partial}{\partial x} \left(\frac{Q^2}{A}\right) + gA\left(\frac{\partial y}{\partial x} - S_0 + S_f\right) = 0$

### 5. Structure Hydraulics: Culvert Solver
The culvert solver evaluates both inlet and outlet control to determine the controlling upstream water surface elevation:
$$WSEL_{up} = \max(WSEL_{inlet}, WSEL_{outlet})$$

#### A. Inlet Control (FHWA Nomograph Formulations)
Based on Federal Highway Administration (FHWA) standards, the inlet control headwater depth ($HW$) relative to the barrel rise ($D$) is computed for:
* **Unsubmerged Flow ($\frac{Q}{AD^{0.5}} \le 3.0$):**
  $$\frac{HW}{D} = \frac{H_c}{D} + K \left[\frac{Q}{A D^{0.5}}\right]^M - 0.5 S$$
* **Submerged Flow ($\frac{Q}{AD^{0.5}} \ge 4.0$):**
  $$\frac{HW}{D} = c \left[\frac{Q}{A D^{0.5}}\right]^2 + Y - 0.5 S$$
* **Transition Zone ($3.0 < \frac{Q}{AD^{0.5}} < 4.0$):**
  Linear interpolation between unsubmerged and submerged formulas.
* *Note: The shape parameters $K, M, c, Y$ are selected from FHWA nomographs by `culvert_inlet_types` (or legacy $K_e$ threshold when inlet type is 0).*
* **Inlet types:** `culvert_inlet_types` per culvert — Circular: 1 square headwall, 2 groove end, 3 beveled 45°, 4 projecting; Box: 10 square edge, 11 flared wingwalls, 12 beveled top; Arch/ConSpan: 20 projecting, 21 smooth entry; 0 = legacy $K_e$ threshold.
* **Invert overrides:** Optional `culvert_z_ups` / `culvert_z_downs` (defaults to adjacent section bed).
* **Roadway overtopping:** Optional `culvert_crest_elevs` with `culvert_weir_coeffs` (default 2.6 US / 1.44 metric) and `culvert_weir_lengths` (default span × barrels).
* **Control reporting:** Steady results include `culvert_control_types` aligned with `culvert_stations`.
* **Tier 2a diagnostics (API v3):** Steady results also return `culvert_wsel_inlet`, `culvert_wsel_outlet`, `culvert_q_barrels`, `culvert_q_weirs`, `culvert_barrel_depths`, `culvert_barrel_velocities`, and `culvert_barrel_froude` per culvert. Barrel slope $S$ in the inlet nomograph includes adverse grade (upstream invert above downstream).
* **Tier 2a rating curve:** `computeCulvertRatingCurve` samples headwater vs discharge at fixed tailwater for a single culvert (same geometry/loss fields as the steady solver).

#### B. Outlet Control (Energy losses)
The outlet control upstream elevation is computed via energy headwater balance:
$$WSEL_{outlet} = WSEL_{down} + \alpha_{down} \frac{V_{down}^2}{2g} + h_e + h_f + h_o - \alpha_{up} \frac{V_{up}^2}{2g}$$
where $\alpha_{down} = \alpha_{up} \approx 1.3$ on contracted approach/departure channel velocities in outlet control:
* **Entrance Loss:** $h_e = K_e \frac{V_{barrel}^2}{2g}$
* **Exit Loss (Velocity Head Recovery):** $h_o = K_x \max\left(0, \frac{V_{barrel}^2}{2g} - \alpha_{down} \frac{V_{down}^2}{2g}\right)$
* **Friction Loss:** $h_f = L S_f$ (where friction slope $S_f$ utilizes composite Manning's $n_c$ and hydraulic radius $R_{barrel}$ evaluated at the barrel depth $y_{barrel} = \max(y_c, \min(D, y_{down}))$).

#### C. Sediment Blockage (Blocked Depth)
If a sediment/blockage depth ($d_b$) is specified:
* The active flow area is reduced: $A_{effective}(y) = A(y) - A(d_b)$.
* The wetted perimeter is modified to account for the horizontal sediment bed: $P_{effective}(y) = P(y) - P(d_b) + T(d_b)$, where $T(d_b)$ is the top width at the blockage height.
* The physical invert elevation is shifted upward: $z_{invert\_eff} = z_{invert} + d_b$.

---

### 6. Structure Hydraulics: Bridge Solver
The bridge solver evaluates backwater losses through pier obstructions, deck pressure flow, and roadway overtopping:

#### A. Low Flow Pier Loss (Yarnell Equation, HEC-RAS Class A)
For unsubmerged flow through the bridge deck (Class A low flow), the water surface rise from the downstream section to the upstream section is computed with the HEC-RAS Yarnell equation:
$$H_{3-2} = 2K(K + 10\omega - 0.6)(\alpha + 15\alpha^4)\frac{V^2}{2g}$$
where:
* $K$ is the Yarnell pier shape coefficient ($0.90$ semicircular, $0.95$ twin-cylinder with diaphragm, $1.05$ triangular, $1.25$ square).
* $\omega = (V^2/2g) / y$ is the velocity-head-to-depth ratio at the downstream section.
* $\alpha = A_{piers} / (A_{flow} - A_{piers})$ is the pier obstruction ratio over unobstructed flow area.
* $V$ is the mean velocity at the downstream section ($Q / A_{flow}$).

*Limitations:* Yarnell is intended for uniform channel sections without overbank storage, where piers dominate losses. Abutments, deck shape, and Class B/C low flow are not modeled with this method.

#### B. High Flow: Pressure (Orifice) Flow
When the water surface reaches the low chord of the bridge deck, pressure flow governs:
$$Q = C_d A_{net} \sqrt{2g (WSEL_{up} - WSEL_{down})}$$
where $A_{net}$ is the net opening area (gross area minus submerged pier obstruction area) and $C_d$ is the orifice discharge coefficient.

#### C. High Flow: Weir Overtopping (Combined Flow)
When upstream headwater exceeds the high chord of the roadway, flow is split between pressure flow under the deck and weir overtopping:
$$Q_{total} = Q_{pressure} + Q_{weir}$$
$$Q_{weir} = C_w L_{road} (WSEL_{up} - H_{road})^{1.5}$$
The solver uses a bisection search to iteratively converge on the upstream $WSEL_{up}$ that balances $Q_{total}$.

---

### 7. Core Solver Assumptions & Corrections

#### A. Channel vs. Overbank Flow at Structures
When cross-sections are subdivided into channel and overbank zones (`is_overbank`), stagnant overbank storage can inflate total area near structures.
* **Implementation:** Geometry tables include a **`channel_area`** lookup (main channel only). At cross-sections adjacent to bridges and culverts, Standard Step and Yarnell pier losses use `channel_area` instead of total area where subdivision is present.

#### B. Culvert Outlet Velocity Head ($\alpha$)
In culvert **outlet control**, contracted approach/departure velocities use a velocity-head multiplier of $\alpha \approx 1.3$ on the downstream and upstream channel velocities when evaluating exit-loss and energy balance terms. The general Standard Step sweep between cross-sections uses $\alpha = 1.0$.

---

## Compilation and Build

### 1. WebAssembly (WASM) Target
To compile the Rust engine into WebAssembly, make sure you have `cargo` and `wasm-pack` installed. Run the build script in a WSL/Linux environment:

```bash
chmod +x ./build_wasm.sh
./build_wasm.sh
```

This generates the WebAssembly module in the `./pkg` (browser) and `./pkg-node` (Node) directories. The build script also runs WASM JSON contract tests and a Node smoke test.

#### WASM entry points

| Function | Description |
|----------|-------------|
| `init()` | Load the WASM module (generated by wasm-pack) |
| `getEngineVersion()` | Engine semver string |
| `getWasmApiMetadata()` | `api_version`, culvert inlet/shape enums, Tier 1 / Tier 2a field lists |
| `validateSteadyInputs(inputs)` | Parse-check a payload without solving |
| `solveSteady(inputs)` | Steady GVF + structures → `SteadyResult` |
| `solveUnsteady(inputs)` | Unsteady routing → `UnsteadyResult` |
| `computeCulvertRatingCurve(inputs)` | Headwater vs $Q$ at fixed tailwater → `CulvertRatingCurveResult` |

All payloads use **snake_case** field names (same schema as Python JSON). Check `getWasmApiMetadata().api_version` after each engine upgrade; **version 3** adds Tier 2a culvert diagnostics and rating curves (version 2 introduced Tier 1 culvert inputs).

### 2. Python Target
To compile and install the native Python extension locally:
1. Ensure you have `python` (>= 3.7) and a virtual environment set up.
2. Install `maturin` and compile the extension:
   ```bash
   pip install maturin pytest
   maturin develop --features python
   ```
This compiles the Rust solver and installs the package as `streams1d` in the active virtual environment.

---

## Testing & Verification

### 1. HEC-RAS Profile Verification (ConSpan Dataset)
STREAM-1D includes a verification dataset under `python/verification/` extracted from a HEC-RAS model of a channel reach featuring a $28\text{ ft} \times 6\text{ ft}$ ConSpan arch culvert with a composite bottom-roughness layer ($Q = 1000\text{ cfs}$, Downstream WSEL = $30.51\text{ ft}$). Optional sediment blockage depth is supported by the culvert solver but is zero in this benchmark.

The solver's calculated water surface elevations match HEC-RAS within a strict $0.04\text{ ft}$ tolerance:

| Cross-Section Station | Calculated WSEL (ft) | HEC-RAS WSEL (ft) | Difference (ft) | Verification Status |
| :--- | :--- | :--- | :--- | :--- |
| **2827** (Upstream) | 33.712 | 33.720 | -0.008 | **[PASS]** |
| **1257** (Inlet) | 32.919 | 32.920 | -0.001 | **[PASS]** |
| **0** (Downstream) | 30.510 | 30.510 | +0.000 | **[PASS]** |

### 2. Bridge Pier Backwater Validation
The bridge solver implements the HEC-RAS Yarnell equation for Class A low flow. On a 10 m rectangular channel ($Q = 15\text{ cms}$, downstream WSEL $= 3.0\text{ m}$, two $0.5\text{ m}$ square piers), the computed pier head loss is $H_{3-2} \approx 0.00247\text{ m}$, verified by unit tests against the closed-form HEC-RAS formula.

### 3. Culvert Tier 1 & Tier 2a Verification

Culvert **Tier 1** (explicit inlet types, invert overrides, roadway overtopping, `culvert_control_types`) and **Tier 2a** (extended steady diagnostics, adverse barrel slope, `computeCulvertRatingCurve`) are covered by Rust unit/integration tests, WASM JSON contract tests, and Python pytest cases. Example WASM steady fixture: [`tests/fixtures/wasm_steady_culvert_tier1.json`](tests/fixtures/wasm_steady_culvert_tier1.json).

### 4. Running the Test Suites

* **Rust unit and integration tests:**
  ```bash
  cargo test
  cargo test --test wasm_json_contract
  ```
* **WASM build + smoke test:**
  ```bash
  bash build_wasm.sh
  ```
* **Python pytest suite** (requires `maturin develop --features python` in a venv):
  ```bash
  PYTHONPATH=python pytest -c /dev/null python/test_streams1d.py
  ```
* **Python HEC-RAS verification script:**
  ```bash
  PYTHONPATH=python python python/test_python_bindings.py
  ```

---

## Interactive Jupyter Notebook & Binder

To run calculations, view water surface profile charts, and inspect tables interactively on the web without any local installation:

[![Binder](https://mybinder.org/badge_logo.svg)](https://mybinder.org/v2/gh/jlillywh/STREAM-1D/main?filepath=python%2Fstreams1d_verification.ipynb)

* **Interactive Notebook:** [python/streams1d_verification.ipynb](python/streams1d_verification.ipynb)
* Click the **Binder** badge above to launch a sandbox environment in your browser. The first launch compiles Rust and may take **5–10 minutes**; later launches reuse the cached image.

---

## Usage Examples

### 1. JavaScript Usage Example

Below is an example of loading and executing the steady-state solver inside a browser or Web Worker:

```javascript
import init, {
    getEngineVersion,
    getWasmApiMetadata,
    validateSteadyInputs,
    solveSteady,
} from './pkg/streams1d.js';

async function run() {
    // Initialize the WebAssembly module
    await init();
    console.log('STREAM-1D', getEngineVersion(), 'API', getWasmApiMetadata().api_version);

    // Define cross-sections
    const crossSections = [
        {
            station: 1000.0, // Upstream
            x: [0.0, 0.0, 10.0, 10.0],
            y: [6.0, 1.0, 1.0, 6.0], // Bed elevation = 1.0
            n_stations: [0.0],
            n_values: [0.025],
            unit_system: "Metric"
        },
        {
            station: 500.0, // Mid
            x: [0.0, 0.0, 10.0, 10.0],
            y: [5.5, 0.5, 0.5, 5.5], // Bed elevation = 0.5
            n_stations: [0.0],
            n_values: [0.025],
            unit_system: "Metric"
        },
        {
            station: 0.0, // Downstream
            x: [0.0, 0.0, 10.0, 10.0],
            y: [5.0, 0.0, 0.0, 5.0], // Bed elevation = 0.0
            n_stations: [0.0],
            n_values: [0.025],
            unit_system: "Metric"
        }
    ];

    const inputs = {
        cross_sections: crossSections,
        flow_rate: 15.0,            // 15 cms
        num_slices: 100,            // Vertical slicing count
        regime: 0,                  // 0 = Subcritical
        downstream_wsel: 1.5,       // Tailwater boundary condition
        coeff_contraction: 0.1,
        coeff_expansion: 0.3
    };

    // Run the steady solver
    validateSteadyInputs(inputs);
    const results = solveSteady(inputs);

    console.log("Calculated WSELs:", results.wsel);
    console.log("Critical depths:", results.critical_wsel);
    console.log("Velocities:", results.velocity);
    console.log("Culvert controls:", results.culvert_control_types);
}

run();
```

**Web app integrators:** TypeScript types, Worker pattern, and Culvert Tier 1 field mapping are in [`docs/wasm_integration.md`](docs/wasm_integration.md) and [`docs/wasm_api.types.ts`](docs/wasm_api.types.ts).

#### Culvert Tier 1 WASM example

```javascript
const inputs = {
    cross_sections: crossSections,
    flow_rate: 100.0,
    regime: 0,
    downstream_wsel: 3.0,
    culvert_stations: [50.0],
    culvert_shape_types: [3],              // ConSpan
    culvert_spans: [28.0],
    culvert_rises: [6.0],
    culvert_roughness_ns: [0.013],
    culvert_lengths: [50.0],
    culvert_entrance_loss_coeffs: [0.5],
    culvert_exit_loss_coeffs: [1.0],
    culvert_inlet_types: [21],             // Arch/ConSpan smooth entry
    culvert_z_ups: [30.0],                 // optional invert override
    culvert_z_downs: [29.5],
    culvert_crest_elevs: [35.0],           // optional roadway overtopping
    culvert_weir_coeffs: [2.6],
    culvert_weir_lengths: [28.0],
    culvert_barrels: [1],
};

const results = solveSteady(inputs);
console.log(results.culvert_control_types);  // e.g. ["inlet"]
```

### 2. Python Usage Example

Below is an example of executing the steady-state and unsteady solvers using the Python API:

```python
import streams1d as st

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

# 4. Configure and solve unsteady routing
unsteady_inputs = st.UnsteadyInputs(
    cross_sections=[xs1000, xs500, xs0],
    initial_wsel=[2.0, 1.5, 1.0],
    initial_q=[14.0, 14.0, 14.0],
    dt=60.0,
    num_steps=5,
    upstream_q_hydrograph=[14.0] * 5,
    downstream_wsel_hydrograph=[1.0] * 5,
    theta=0.6,
    num_slices=100
)

unsteady_results = st.solve_unsteady(unsteady_inputs)
print("Unsteady final step WSELs:", unsteady_results["wsel"][-1])
```

#### Culvert Tier 1 Python example

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
)
results = st.solve_steady(inputs)
print("Culvert control:", results.get("culvert_control_types"))
```

---

## Documentation index

| Document | Audience |
|----------|----------|
| [`README.md`](README.md) | Equations, build, usage, verification |
| [`tech_spec.md`](tech_spec.md) | Host-app architecture |
| [`docs/wasm_integration.md`](docs/wasm_integration.md) | Web / Worker integrators |
| [`docs/wasm_api.types.ts`](docs/wasm_api.types.ts) | TypeScript types |
| [`docs/web_gui_culvert_integration.md`](docs/web_gui_culvert_integration.md) | Culvert GUI handoff — Tier 1 + Tier 2a (companion web app) |
| [`docs/web_gui_tributary_junction.md`](docs/web_gui_tributary_junction.md) | Junction import mapping |
