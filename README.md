# STREAM-1D

**An open-source 1D open-channel hydraulics engine for the web and Python.**

STREAM-1D is a Rust 1D open-channel hydraulics engine. It provides steady gradually varied flow (Standard Step, including culverts, bridges, and main-stem/tributary junctions) and unsteady Saint-Venant routing on single reaches with optional inline culverts. The core solver is decoupled from any specific user interface and compiles to two primary targets: WebAssembly (WASM) for client-side execution in the browser, and a native Python extension for automated scripting and batch processing. The API is stateless: structured inputs in, result arrays out. It was originally developed as the engine behind the [STREAM-1D web application](https://stream1d.com) and is released here as a standalone, open-source library for embedding, research, and automated validation.

## Project Goals

* **Embeddable Execution:** Run hydraulic simulations in web dashboards or Python data pipelines without requiring desktop hydraulic software.
* **Structural Hydraulics:** Model inline structures and composite roughness on single reaches or a main stem with one joining tributary (steady)—culverts, bridge piers, roadway overtopping, and multi-zone Manning's *n*.
* **Unsteady Routing:** Dynamic routing with upstream flow and downstream stage hydrographs on a single reach, including optional **inline culverts**; stabilization for steep transients and mixed regimes is an active development focus.
* **WebAssembly API:** Browser and Worker integration via `solveSteady` / `solveUnsteady`, `computeCulvertRatingCurve`, metadata discovery (`getWasmApiMetadata`), and input validation (`validateSteadyInputs`). Culvert inputs include explicit inlet types, invert overrides, roadway overtopping, skew angles, active barrel count, per-barrel geometry, extended shape catalog (pipe-arch, elliptical, horseshoe), extended steady diagnostics, headwater rating curves, **supercritical culvert routing** in mixed-regime steady profiles, and the same culvert field set on **`UnsteadyInputs`** for inline single-reach coupling with per-step culvert diagnostics (**API version 8**).

**Web app integrators:** TypeScript contracts are in [`docs/wasm_api.types.ts`](docs/wasm_api.types.ts); a Worker reference is in [`examples/wasm/`](examples/wasm/). Steady tributary junctions use a two-branch API (`cross_sections` + `tributary_cross_sections`, `tributary_flow_rate`, `junction_main_station`). HEC-RAS projects with three reaches at a confluence must merge or concatenate the upper and lower main stems into one `cross_sections` array before calling WASM. GUI integration specs for the hosted product live in the [stream1d.com](https://stream1d.com) web application repository, not in this engine repo.

## Architecture

* **Stateless:** No project files, local file administration, or hidden global state inside the engine. Each call is an independent solve.
* **UI-Agnostic:** The library exposes solver functions only; threading, workers, and visualization are the responsibility of the host application.
* **Open-Channel Focus:** Non-linear cross-section lookup tables with composite Manning's *n*, subdivided overbank/channel geometry, and mixed-regime steady profiles.
* **Intermediate Outputs:** Steady results include section-by-section area, top width, velocity, Froude number, and energy grade slope—useful for capacity review without running an unsteady simulation.

## Limitations (read before comparing to HEC-RAS)

STREAM-1D is an **embeddable 1D hydraulics engine** — the Rust/WASM/Python solve core in this repository, not a complete desktop product like HEC-RAS. It exposes a **stateless** API (`cross_sections` and boundary inputs in, profile arrays out). It does **not** ship a user interface, project database, RAS Map, 2D floodplain meshing, or native HEC-RAS Plan/Unsteady solvers.

The hosted product at [stream1d.com](https://stream1d.com) provides cross-section editing, HEC-RAS geometry import (e.g. `.g01`), project persistence, and visualization on top of this engine. That web application is a separate product (see [License](#license)). This repository is the solver core only: it accepts geometry arrays via WASM or Python and does not include a HEC-RAS file importer or cross-section editor.

### What the STREAM-1D engine supports

| Area | Supported |
|------|-----------|
| **Steady flow** | Standard Step backwater/drawdown; subcritical, supercritical, and mixed regime (`regime` 0/1/2) |
| **Boundary conditions (steady)** | Known WSEL, critical depth, normal depth, rating curve (upstream and downstream) |
| **Cross-sections** | Arbitrary $(x,y)$ polylines; composite Manning's *n*; optional channel/overbank subdivision (`is_overbank`) |
| **Main stem + tributary (steady)** | One tributary joining one main channel at a shared WSEL node — main stem above/below the junction plus tributary inflow (`tributary_cross_sections`, `tributary_flow_rate`, `junction_main_station`); subcritical only |
| **Culverts (steady, main stem)** | Circular, box, arch, ConSpan, **pipe-arch**, **elliptical**, and **horseshoe**; FHWA-style inlet/outlet control with signed barrel slope (adverse grade supported). Explicit inlet types, invert elevations, roadway overtopping, composite bottom roughness, sediment blockage, control reporting (`inlet` / `outlet` / `overtopping`), extended diagnostics (inlet/outlet HW, barrel vs weir $Q$, barrel depth/velocity/Froude), `computeCulvertRatingCurve`, barrel **skew** (`culvert_skew_angles`), **active barrel count** (`culvert_active_barrels`), **per-barrel geometry** (`culvert_barrel_spans` / `culvert_barrel_rises`) with capacity-based flow split, and **supercritical culvert routing** (`regime` 1/2) via headwater inversion |
| **Culverts (unsteady, single reach)** | Same culvert input fields as steady on `UnsteadyInputs`; **iterated post-step headwater coupling** (tolerance-based convergence, up to 5 passes per step); returns Tier 2a culvert diagnostics each time step |
| **Bridges (steady, main stem)** | Yarnell Class A pier loss; pressure (orifice) flow; roadway weir overtopping |
| **Unsteady flow** | Preissmann Saint-Venant on a **single reach**; upstream $Q(t)$ and downstream WSEL($t$) hydrographs; optional **inline culverts** (see row above) |
| **Outputs** | WSEL, critical WSEL, velocity, area, top width, Froude number, energy grade slope (+ `tributary_wsel`, `tributary_velocity`, `tributary_froude` when a junction is modeled; + `culvert_control_types` and Tier 2a culvert arrays on **`solve_steady`** and **`solve_unsteady`** when culverts are modeled) |

### Companion web application features ([stream1d.com](https://stream1d.com))

These are implemented in the **STREAM-1D web application**, not in the Rust/WASM/Python solver crate in this repository:

| Feature | Description |
|---------|-------------|
| **Cross-section editing** | Interactive editing of reach geometry and Manning's *n* in the browser |
| **HEC-RAS geometry import** | Import HEC-RAS geometry files (e.g. `.g01`) to build reaches, cross-sections, and structures automatically, then map to solver inputs (including merging upper + lower main stem at a junction when needed) |

### Not supported (common HEC-RAS features)

| Category | HEC-RAS capability | STREAM-1D today |
|----------|-------------------|-----------------|
| **Dimensionality** | 1D, 2D, and coupled 1D/2D | **1D only** |
| **River networks** | Dendritic systems, multiple junctions, looped reaches | **One** main stem + **one** tributary (**steady only**); no general network graph |
| **Unsteady scope** | Networks, structures, storage areas, lateral inflows | **Single reach** with optional **inline culverts** (explicit post-step headwater coupling); **no** bridges or multi-reach networks in unsteady |
| **Storage & diversions** | Ponds, reservoirs, split flow, lateral structures, pumps, gates | Not modeled |
| **Inline weirs & dams** | Standalone weirs, inline structures, dam breach | Not modeled (bridge roadway overtopping only) |
| **Bridge hydraulics** | Full low-flow classes, pressure/weir combinations, bridge methods, abutments, deck geometry | Yarnell **Class A pier loss** only; simplified pressure + weir overtopping; no abutment or Class B/C low flow |
| **Culvert hydraulics** | Full HEC-RAS culvert catalog (all standard shapes), culverts in multi-reach unsteady networks | FHWA nomograph (circular, box, arch, ConSpan, pipe-arch, elliptical, horseshoe) with explicit inlet types; multi-barrel capacity-based $Q$ split with optional per-barrel span/rise; skew angles and blocked-barrel count; invert offsets, roadway overtopping, extended diagnostics and rating-curve API; **supercritical culvert routing** in mixed-regime steady profiles; **inline culverts** in single-reach unsteady (explicit coupling) |
| **Ineffective flow** | Roadway embankment blocking, blocked obstructions, storage from ineffective areas | Partial: `channel_area` at structure-adjacent sections when overbanks are subdivided — not full RAS ineffective-flow workflow |
| **Terrain & mapping** | RAS Terrain, TIN/bathymetry authoring, RAS Map | **Not in the engine** — the companion **web app** may edit cross-sections and import HEC-RAS geometry; the solver only receives $(x,y)$ sections and stations |
| **Sediment & morphology** | Mobile bed, sediment transport, scour | Not modeled (optional fixed culvert blockage depth only) |
| **Water quality & ice** | Temperature, water quality, ice jams | Not modeled |
| **Project workflow** | Full HEC-RAS `.prj` with Plan, Geometry, and Unsteady files | **Not in the engine** — no built-in project format; the **web app** may import geometry and manage projects, then call WASM per solve |
| **Regulatory reporting** | FEMA, flood insurance, HEC-RAS report templates | Not included |

### Practical guidance

* **Good fit:** Embedding steady or unsteady 1D solves in a **web dashboard** (with optional HEC-RAS import and cross-section editing in that app) or a **Python pipeline** where you supply geometry arrays directly.
* **Poor fit:** Replacing HEC-RAS for FEMA studies, complex multi-reach unsteady networks, 2D overbank flood routing, or models that rely on RAS-specific structure and ineffective-flow workflows without host-app preprocessing.
* **Junction models:** Import upper + lower main stem as one `cross_sections` array; pass the tributary reach as `tributary_cross_sections` with `junction_main_station` at the shared node.
* **Active development:** Unsteady stabilization for steep transients and mixed regimes; broader network and structure support may follow — check release notes and open issues.

For host-application architecture (Web Workers, data transfer, GIS integration), see [`tech_spec.md`](tech_spec.md).

## Repository Structure

```
streams1d/
├── Cargo.toml                  # Rust library and WASM dependencies configuration
├── LICENSE                     # MIT License
├── README.md                   # Project documentation and equations
├── tech_spec.md                # Host-app architecture and integration scope
├── build_wasm.sh               # WSL script to build WASM package
├── scripts/
│   ├── run_coverage.sh         # Local tests + llvm-cov (matches CI)
│   └── install_git_hooks.sh    # Enable pre-commit coverage hook
├── .githooks/
│   └── pre-commit              # Runs run_coverage.sh before commit
├── .github/workflows/ci.yml    # Tests + Codecov upload
├── codecov.yml                 # Codecov status/comment config
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
├── docs/                       # WASM TypeScript contracts for host applications
│   └── wasm_api.types.ts       # TypeScript definitions for WASM inputs/outputs
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
* **Roadway overtopping:** Optional `culvert_crest_elevs` with `culvert_weir_coeffs` (default 2.6 US / 1.44 metric) and `culvert_weir_lengths` (default sum of projected active-barrel spans; omit `culvert_crest_elevs` entirely when overtopping is off). When the roadway crest is exceeded, total discharge splits iteratively between barrel flow and weir flow until balanced.
* **Control reporting:** Steady results include `culvert_control_types` aligned with `culvert_stations`.
* **Extended diagnostics:** Steady results also return `culvert_wsel_inlet`, `culvert_wsel_outlet`, `culvert_q_barrels`, `culvert_q_weirs`, `culvert_barrel_depths`, `culvert_barrel_velocities`, and `culvert_barrel_froude` per culvert. Barrel slope $S$ in the inlet nomograph includes adverse grade (upstream invert above downstream).
* **Rating curve:** `computeCulvertRatingCurve` samples headwater vs discharge at fixed tailwater for a single culvert (same geometry/loss fields as the steady solver).
* **Barrel skew:** Optional `culvert_skew_angles` (degrees from normal to flow) adjust projected inlet span ($B' = B\cos\theta$) and friction length ($L' = L/\cos\theta$), clamped to 59°.
* **Active barrels:** Optional `culvert_active_barrels` (open barrels ≤ `culvert_barrels`) splits total discharge among open barrels only and reduces default overtopping weir length.
* **Per-barrel geometry:** Optional `culvert_barrel_spans` and `culvert_barrel_rises` (nested arrays per culvert) assign span/rise to each open barrel; flow splits by barrel capacity at a shared headwater. Omit entries to use culvert-level `culvert_spans` / `culvert_rises`.
* **Multi-barrel hydraulics:** Parallel barrels share one upstream pool elevation. With uniform geometry, discharge divides equally among `culvert_active_barrels`. With per-barrel span/rise, the solver bisects on headwater and assigns each barrel the flow its geometry carries at that elevation (capacity-based split). Reported barrel depth, velocity, and Froude are flow-weighted across active barrels.
* **Supercritical / mixed-regime routing (steady):** In the upstream-to-downstream supercritical sweep (`regime` 1 or 2), culvert intervals invert the rating curve: given upstream headwater and discharge, the solver finds the minimum downstream tailwater that reproduces that headwater (`solve_culvert_from_headwater`). Bridges in supercritical sweeps still use critical-depth stubs.
* **Unsteady inline culverts:** After each Preissmann time step, culvert intervals apply the FHWA culvert solver with tolerance-based headwater iteration (up to 12 inner iterations per culvert) and up to **5 outer coupling passes** per time step (downstream culverts first). Initial conditions warm-start from a subcritical steady profile that includes culvert fields. Coupling is **explicit** (not embedded in the Preissmann Jacobian) but returns the same Tier 2a culvert diagnostics as steady solves each step.

#### Culvert WASM / JSON field reference (`api_version` 8)

Parallel arrays — index `i` matches `culvert_stations[i]`. Use on **`SteadyInputs`** and **`UnsteadyInputs`** (same keys). Discover enums and field lists via `getWasmApiMetadata()`.

| Field | Required | Description |
|-------|----------|-------------|
| `culvert_stations` | Yes (if modeling culverts) | Station of each culvert along the reach |
| `culvert_shape_types` | Recommended | `0` Circular, `1` Box, `2` Arch, `3` ConSpan, `4` Pipe-arch, `5` Elliptical, `6` Horseshoe |
| `culvert_spans` | Recommended | Diameter (circular) or width (box/arch/ConSpan), user units |
| `culvert_rises` | Recommended | Barrel rise / height, user units |
| `culvert_lengths` | Recommended | Barrel length, user units |
| `culvert_roughness_ns` | Recommended | Manning's *n* (top/sides) |
| `culvert_entrance_loss_coeffs` | Optional | $K_e$ (default 0.5) |
| `culvert_exit_loss_coeffs` | Optional | $K_x$ (default 1.0) |
| `culvert_barrels` | Optional | Total barrel count (default 1) |
| `culvert_inlet_types` | Optional | FHWA nomograph code (see inlet list above); `0` = legacy $K_e$ threshold |
| `culvert_z_ups`, `culvert_z_downs` | Optional | Invert elevations; default to adjacent section bed |
| `culvert_roughness_n_bottoms` | Optional | Bottom/sediment *n* (defaults to `culvert_roughness_ns`) |
| `culvert_depth_bottom_ns` | Optional | Depth to which bottom *n* applies |
| `culvert_depth_blockeds` | Optional | Sediment blockage depth from invert |
| `culvert_crest_elevs` | Optional | Roadway crest for overtopping weir — **omit** when overtopping is disabled |
| `culvert_weir_coeffs` | Optional | Weir $C_w$ (default 2.6 US / 1.44 metric) |
| `culvert_weir_lengths` | Optional | Weir length (default projected span × active barrels) |
| `culvert_skew_angles` | Optional | Skew from normal to flow, degrees (0–59° enforced) |
| `culvert_active_barrels` | Optional | Open barrels ≤ `culvert_barrels`; omit = all open |
| `culvert_barrel_spans` | Optional | `culvert_barrel_spans[i][j]` span of barrel `j` at culvert `i` |
| `culvert_barrel_rises` | Optional | `culvert_barrel_rises[i][j]` rise of barrel `j` at culvert `i` |

**Culvert outputs** (when culverts are present): `culvert_control_types`, `culvert_wsel_inlet`, `culvert_wsel_outlet`, `culvert_q_barrels`, `culvert_q_weirs`, `culvert_barrel_depths`, `culvert_barrel_velocities`, `culvert_barrel_froude`. On **`solve_steady`** these are per culvert; on **`solve_unsteady`** they are `[time_step][culvert_index]` histories alongside WSEL/$Q$/velocity.

**Rating curve:** `computeCulvertRatingCurve({ q_values, ...culvert fields })` — same geometry/loss/skew/barrel fields as steady; `q` in culvert params is ignored.

**API version history:** v3 — Tier 2a diagnostics + rating curve; v4 — `culvert_skew_angles`, `culvert_active_barrels`; v5 — `culvert_barrel_spans`, `culvert_barrel_rises`; v6 — culvert shape types 4–6 (pipe-arch, elliptical, horseshoe); v7 — culvert fields on `UnsteadyInputs` + supercritical culvert routing in steady mixed-regime sweeps; v8 — unsteady culvert Tier 2a diagnostics (`[step][culvert]`) + strengthened per-step culvert coupling.

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
| `getWasmApiMetadata()` | `api_version`, culvert inlet/shape enums, culvert field lists (inputs, diagnostics, geometry) |
| `validateSteadyInputs(inputs)` | Parse-check a payload without solving |
| `solveSteady(inputs)` | Steady GVF + structures → `SteadyResult` |
| `solveUnsteady(inputs)` | Unsteady routing → `UnsteadyResult` |
| `computeCulvertRatingCurve(inputs)` | Headwater vs $Q$ at fixed tailwater → `CulvertRatingCurveResult` |

All payloads use **snake_case** field names (same schema as Python JSON). Check `getWasmApiMetadata().api_version` after each engine upgrade; **version 8** adds unsteady culvert Tier 2a output arrays; version 7 adds culvert fields on `UnsteadyInputs`; version 6 adds culvert shape types 4–6; version 5 adds per-barrel geometry; version 4 adds skew and active-barrel fields.

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
STREAM-1D includes a verification dataset under `python/verification/` extracted from a HEC-RAS model of a channel reach featuring a $28\text{ ft} \times 6\text{ ft}$ ConSpan arch culvert with a composite bottom-roughness layer. Reference WSEL values for **5 yr** ($Q=250\text{ cfs}$), **25 yr** ($Q=600\text{ cfs}$), and **50 yr** ($Q=1000\text{ cfs}$) profiles are in [`python/verification/hecras_conspan_profiles.json`](python/verification/hecras_conspan_profiles.json) (sourced from [`ConSpan.csv`](python/verification/ConSpan.csv)).

All profile stations (10 per event) are checked within **±0.04 ft** vs HEC-RAS (Rust: `tests/culvert_hecras_verification.rs`; Python: `python/test_hecras_culvert_verification.py`).

### 2. Bridge Pier Backwater Validation
The bridge solver implements the HEC-RAS Yarnell equation for Class A low flow. On a 10 m rectangular channel ($Q = 15\text{ cms}$, downstream WSEL $= 3.0\text{ m}$, two $0.5\text{ m}$ square piers), the computed pier head loss is $H_{3-2} \approx 0.00247\text{ m}$, verified by unit tests against the closed-form HEC-RAS formula.

### 3. Culvert verification

Culvert hydraulics are covered by **70+** unit and integration tests in `src/solvers/culvert.rs`, `src/solvers/steady.rs`, and `src/solvers/unsteady.rs`, including:

| Configuration | What is tested |
|---------------|----------------|
| Shapes | Circular, box, arch, ConSpan, pipe-arch, elliptical, horseshoe geometry and full solves |
| Inlet types | All FHWA nomograph codes per shape |
| Control regimes | Inlet, outlet, full/partial roadway overtopping |
| Barrel slope | Adverse, flat, and downhill grade |
| Blockage & roughness | Sediment `depth_blocked`, composite bottom *n* |
| Multi-barrel | Active barrel count, uniform and per-barrel geometry, capacity-based $Q$ split |
| Skew | Projected span / friction length, 59° clamp |
| Diagnostics & rating curve | Extended outputs; monotonic HW vs $Q$ for all shapes; `solve_culvert_from_headwater` round-trip |
| Reach integration (steady) | `solve_steady` with skew, blocked barrels, per-barrel spans, sediment |
| Supercritical routing | `regime` 1/2 culvert intervals (US Customary + Metric); bridge critical-depth stub |
| Unsteady inline | `solve_unsteady` with culvert coupling + per-step Tier 2a diagnostics (Metric + US Customary) |
| HEC-RAS ConSpan | 5/25/50 yr profiles — 10 stations each, ±0.04 ft (`hecras_conspan_profiles.json`) |
| Point culvert benchmarks | Circular inlet/outlet, box inlet, multi-barrel, adverse grade (`tests/fixtures/culvert_point_benchmarks.json`) |

WASM JSON contract tests and Python pytest cases provide additional coverage. Example steady fixture: [`tests/fixtures/wasm_steady_culvert_tier1.json`](tests/fixtures/wasm_steady_culvert_tier1.json).

CI uploads coverage to [Codecov](https://codecov.io) on every push/PR (`.github/workflows/ci.yml`).

### 4. Running the Test Suites

* **Coverage + tests (recommended before commit):**
  ```bash
  ./scripts/install_git_hooks.sh   # once per clone — enables pre-commit hook
  ./scripts/run_coverage.sh        # manual: tests + lcov.info (same as CI)
  ```
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
* **Python HEC-RAS verification (ConSpan 5/25/50 yr profiles):**
  ```bash
  PYTHONPATH=python python python/test_hecras_culvert_verification.py
  ```
* **Rust HEC-RAS + point culvert benchmarks:**
  ```bash
  cargo test --test culvert_hecras_verification
  ```
* **Python bindings smoke test:**
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

**Web app integrators:** TypeScript types and field names are in [`docs/wasm_api.types.ts`](docs/wasm_api.types.ts); a Worker reference is in [`examples/wasm/`](examples/wasm/).

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
    culvert_barrels: [2],
    culvert_active_barrels: [2],           // optional — omit to use all barrels
    culvert_skew_angles: [15.0],           // optional — degrees from normal
    culvert_barrel_spans: [[8.0, 6.0]],    // optional — per-barrel diameters/widths
    culvert_barrel_rises: [[6.0, 6.0]],    // optional — per-barrel rises
};

const results = solveSteady(inputs);
console.log(results.culvert_control_types);  // e.g. ["inlet"]
console.log(results.culvert_wsel_inlet, results.culvert_q_barrels);
```

#### Culvert rating curve (WASM)

```javascript
import { computeCulvertRatingCurve } from './pkg/streams1d.js';

const curve = computeCulvertRatingCurve({
    q_values: [50, 100, 150],
    shape_type: 0,
    span: 5.0,
    rise: 5.0,
    tw_wsel: 12.0,
    z_up: 10.0,
    z_down: 9.0,
    units: 'USCustomary',
    roughness_n: 0.012,
    length: 100.0,
    entrance_loss_coeff: 0.5,
    exit_loss_coeff: 1.0,
    inlet_type: 1,
    num_barrels: 2,
    skew_deg: 0,
    barrel_spans: [5.0, 5.0],
    barrel_rises: [5.0, 5.0],
});
console.log(curve.wsel, curve.control_types);
```

#### Unsteady inline culvert (WASM)

```javascript
import { solveUnsteady } from './pkg/streams1d.js';

const unsteadyInputs = {
    cross_sections: crossSections,
    initial_wsel: [2.5, 2.0, 1.5],
    initial_q: [20.0, 20.0, 20.0],
    dt: 60.0,
    num_steps: 3,
    upstream_q_hydrograph: [20.0, 20.0, 20.0],
    downstream_wsel_hydrograph: [1.5, 1.5, 1.5],
    theta: 0.6,
    num_slices: 50,
    // Same culvert_* keys as steady (api_version 7)
    culvert_stations: [250.0],
    culvert_shape_types: [0],
    culvert_spans: [2.0],
    culvert_rises: [2.0],
    culvert_roughness_ns: [0.013],
    culvert_lengths: [30.0],
    culvert_entrance_loss_coeffs: [0.5],
    culvert_exit_loss_coeffs: [1.0],
    culvert_barrels: [1],
    culvert_inlet_types: [1],
};

const result = solveUnsteady(unsteadyInputs);
console.log(result.wsel[result.wsel.length - 1]); // final-step WSEL at each section
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
    culvert_active_barrels=[2],
    culvert_skew_angles=[15.0],
    culvert_barrel_spans=[[8.0, 6.0]],
    culvert_barrel_rises=[[6.0, 6.0]],
)
results = st.solve_steady(inputs)
print("Culvert control:", results.get("culvert_control_types"))
print("Diagnostics:", results.get("culvert_wsel_inlet"), results.get("culvert_q_barrels"))
```

#### Unsteady inline culvert (Python)

```python
unsteady_culvert = st.UnsteadyInputs(
    cross_sections=[xs1000, xs500, xs0],
    initial_wsel=[2.5, 2.0, 1.5],
    initial_q=[20.0, 20.0, 20.0],
    dt=60.0,
    num_steps=3,
    upstream_q_hydrograph=[20.0] * 3,
    downstream_wsel_hydrograph=[1.5] * 3,
    theta=0.6,
    num_slices=50,
    culvert_stations=[250.0],
    culvert_shape_types=[st.CULVERT_SHAPE_CIRCULAR],
    culvert_spans=[2.0],
    culvert_rises=[2.0],
    culvert_roughness_ns=[0.013],
    culvert_lengths=[30.0],
    culvert_entrance_loss_coeffs=[0.5],
    culvert_exit_loss_coeffs=[1.0],
    culvert_barrels=[1],
    culvert_inlet_types=[1],
)
print(st.solve_unsteady(unsteady_culvert)["wsel"][-1])
```

Python `SteadyInputs` and `UnsteadyInputs` expose the same culvert field names as the WASM/JSON schema (including skew, active barrels, per-barrel geometry, and extended shapes). Shape codes are available as module constants: `st.CULVERT_SHAPE_CIRCULAR` (0) through `st.CULVERT_SHAPE_HORSESHOE` (6).

---

## Documentation index

| Document | Audience |
|----------|----------|
| [`README.md`](README.md) | Equations, build, usage, verification |
| [`tech_spec.md`](tech_spec.md) | Host-app architecture |
| [`docs/wasm_api.types.ts`](docs/wasm_api.types.ts) | TypeScript types for WASM integrators |
| [`examples/wasm/`](examples/wasm/) | Worker reference and Node smoke test |

---

## License

**STREAM-1D engine** (this repository) is released under the [MIT License](https://opensource.org/licenses/MIT) and is free to use, modify, and distribute for any purpose, including commercial and academic work. See [`LICENSE`](LICENSE) for the full license text.

The **STREAM-1D web application** at [stream1d.com](https://stream1d.com) is a separate, proprietary product and is not covered by this license. That application, its user interface, and its hosted infrastructure remain the intellectual property of Lillywhite Water Solutions LLC and are not open source.
