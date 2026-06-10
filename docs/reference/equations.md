# Equations and structure hydraulics

Theory and hydraulics. Field names: [`wasm_api.types.ts`](../wasm_api.types.ts). Versions: [`api_changelog.md`](api_changelog.md). Doc index: [`../README.md`](../README.md).

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
* **Control reporting:** `solve_steady` and `solve_unsteady` return `culvert_control_types` aligned with `culvert_stations` (per culvert on steady; `[time_step][culvert]` on unsteady).
* **Extended diagnostics:** Both solvers return `culvert_wsel_inlet`, `culvert_wsel_outlet`, `culvert_q_barrels`, `culvert_q_weirs`, `culvert_barrel_depths`, `culvert_barrel_velocities`, and `culvert_barrel_froude`. Barrel slope $S$ in the inlet nomograph includes adverse grade (upstream invert above downstream).
* **Rating curve:** `computeCulvertRatingCurve` samples headwater vs discharge at fixed tailwater for a single culvert (same geometry/loss fields as the steady solver).
* **Barrel skew:** Optional `culvert_skew_angles` (degrees from normal to flow) adjust projected inlet span ($B' = B\cos\theta$) and friction length ($L' = L/\cos\theta$), clamped to 59°.
* **Active barrels:** Optional `culvert_active_barrels` (open barrels ≤ `culvert_barrels`) splits total discharge among open barrels only and reduces default overtopping weir length.
* **Per-barrel geometry:** Optional `culvert_barrel_spans` and `culvert_barrel_rises` (nested arrays per culvert) assign span/rise to each open barrel; flow splits by barrel capacity at a shared headwater. Omit entries to use culvert-level `culvert_spans` / `culvert_rises`.
* **Multi-barrel hydraulics:** Parallel barrels share one upstream pool elevation. With uniform geometry, discharge divides equally among `culvert_active_barrels`. With per-barrel span/rise, the solver bisects on headwater and assigns each barrel the flow its geometry carries at that elevation (capacity-based split). Reported barrel depth, velocity, and Froude are flow-weighted across active barrels.
* **Supercritical / mixed-regime routing (steady):** In the upstream-to-downstream supercritical sweep (`regime` 1 or 2), culvert intervals invert the rating curve: given upstream headwater and discharge, the solver finds the minimum downstream tailwater that reproduces that headwater (`solve_culvert_from_headwater`). Bridge intervals use `solve_bridge_tailwater` (Class A/B/C low flow or pressure/weir high flow), not a critical-depth stub.
* **Unsteady inline culverts:** After each Preissmann time step, culvert intervals apply the FHWA culvert solver with tolerance-based headwater iteration (up to 12 inner iterations per culvert) and up to **5 outer coupling passes** per time step (downstream culverts first). Initial conditions warm-start from a subcritical steady profile that includes culvert fields. Coupling is explicit (not embedded in the Preissmann Jacobian) but returns the same extended culvert diagnostics as steady solves each step.
* **Unsteady inline bridges:** After each Preissmann time step (and on initial-condition warm-start), bridge intervals apply the steady bridge solver (`solve_bridge_coupled`) with up to **5 outer coupling passes** per time step. Returns per-step `bridge_flow_regimes`, `bridge_wsel_upstream`, `bridge_wsel_downstream`, and `bridge_head_losses` (`[time_step][bridge_index]`).
* **Combined structure coupling:** When both culverts and bridges are present, `structure_coupling_order` controls post-step processing: `0` (default) merges structures and couples **downstream-first** by reach interval; `1` = all culverts then all bridges (legacy); `2` = all bridges then all culverts.

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

#### A. Low Flow Classification (HEC-RAS Classes A, B, and C)
Before computing losses, the solver classifies low flow by comparing downstream specific force to critical specific force in the bridge constriction (the more constricted of the upstream/downstream bridge sections):

* **Class A** — completely subcritical through the bridge ($M_{down} \geq M_{crit}$).
* **Class B** — passes through critical depth in the constriction ($M_{down} < M_{crit}$); solved with a momentum balance through the critical section and pier drag.
* **Class C** — completely supercritical through the bridge (downstream Froude $\geq 1$ below the low chord); solved with supercritical momentum and pier drag.

Set `bridge_low_flow_methods` per bridge: `0` = auto (classify A/B/C; Class A uses Yarnell when piers are present, WSPRO when abutments dominate, else energy), `1` = Yarnell, `2` = momentum, `3` = energy (standard step through the obstructed opening), `4` = WSPRO (FHWA contracted-opening energy with discharge coefficient `C` from `bridge_wspro_coeffs`, default 0.8). Friction reach length $L$ uses the **BU → BD** path: explicit face `CrossSection.station` values (summing interior cuts when provided), else the densified BU–BD interval spacing, else `bridge_lengths` when faces coincide (legacy). `bridge_lengths` no longer overrides a shorter explicit BU/BD spacing. Conveyance weighting uses average of BU and BD face conveyance at the respective WSELs; skew applies $L' = L/\cos\theta$. Class B falls back to energy/WSPRO when momentum fails or when methods 3/4 are selected.

#### B. Low Flow Pier Loss (Yarnell Equation, Class A)
For Class A low flow with piers and auto/Yarnell method selected, the water surface rise from the downstream section to the upstream section is computed with the HEC-RAS Yarnell equation:
$$H_{3-2} = 2K(K + 10\omega - 0.6)(\alpha + 15\alpha^4)\frac{V^2}{2g}$$
where:
* $K$ is the Yarnell pier shape coefficient ($0.90$ semicircular, $0.95$ twin-cylinder with diaphragm, $1.05$ triangular, $1.25$ square).
* $\omega = (V^2/2g) / y$ is the velocity-head-to-depth ratio at the downstream section.
* $\alpha = A_{piers} / (A_{flow} - A_{piers})$ is the pier obstruction ratio over unobstructed flow area.
* $V$ is the mean velocity at the downstream section ($Q / A_{flow}$).

*Limitations:* Yarnell is intended for uniform channel sections without overbank storage, where piers dominate losses. For abutment-dominated openings use WSPRO (`4`) or auto; for general openings use energy (`3`), momentum (`2`), or auto.

#### C. Energy and WSPRO Low Flow (Class A and Class B fallback)
**Energy** (`3`) balances upstream and downstream energy through the bridge reach: friction loss from conveyance, plus contraction/expansion losses using the reach `coeff_contraction` / `coeff_expansion` inputs on velocity-head differences. **WSPRO** (`4`) uses the FHWA contracted-opening formulation with user coefficient `C` (`bridge_wspro_coeffs`) on the ratio of upstream to contracted opening areas. Both methods account for pier and abutment obstruction in effective area and conveyance.

**Guide banks (v24):** When configured on the approach cut, $A_1$ uses guided active area; otherwise BU obstructed area. $K_c$ / $C$ unchanged. See [`BRIDGE_INTERIOR_SECTIONS_API.md`](../BRIDGE_INTERIOR_SECTIONS_API.md).

#### D. Abutment Blocking (API v21)
Pass `bridge_abutment_block_widths` (legacy total horizontal width encroached by left + right abutments, perpendicular to flow), or per-side fields:

| Field | Description |
|-------|-------------|
| `bridge_abutment_left_widths` / `bridge_abutment_right_widths` | Width per side, perpendicular to flow |
| `bridge_abutment_left_stations` / `bridge_abutment_right_stations` | Outer-face station in opening coordinates (default: left/right deck edge) |
| `bridge_abutment_left_top_elevations` / `bridge_abutment_right_top_elevations` | Constant top elevation (omit for full-height blockage to the low chord) |
| `bridge_abutment_left_top_profile_stations` / `_elevations` (and right pair) | Piecewise top profile `[bridge][point]`, ≥ 2 points |

**Coordinate frame:** Same horizontal frame as `bridge_deck_stations` and `bridge_pier_stations` — station 0 at the left edge of the opening, increasing rightward. Left abutment grows from its outer face rightward; right abutment grows leftward from its outer face. Skew (`bridge_skew_angles`) converts perpendicular input widths to opening-aligned widths ($W' = W/\cos\theta$).

**One-sided abutment:** Set only the side you need — e.g. `bridge_abutment_left_widths: [3.0]` with no right width (or `bridge_abutment_right_widths: [0]`). Omitting a per-side width when the other side is set leaves that face open.

**Legacy split:** When only `bridge_abutment_block_widths` is provided, each side receives half the total width with full-height tops.

Submerged abutment plan area is integrated per side (trapezoidal rule along the face, including piecewise tops) and subtracted from effective opening area at each WSEL for Yarnell, momentum, energy, WSPRO, and pressure/weir hydraulics.

**Steady / unsteady JSON** (same keys on `SteadyInputs` and `UnsteadyInputs`):

```json
"bridge_stations": [500.0],
"bridge_low_chords": [5.0],
"bridge_high_chords": [7.0],
"bridge_low_flow_methods": [4],
"bridge_abutment_left_widths": [1.0],
"bridge_abutment_right_widths": [4.0],
"bridge_abutment_left_top_elevations": [0.0],
"bridge_abutment_right_top_elevations": [2.5]
```

**Rating curve** — flattened keys (no `bridge_` prefix) on `computeBridgeRatingCurve` / `BridgeRatingCurveInputs`: `abutment_block_width` (legacy), `abutment_left_width`, `abutment_right_width`, `abutment_left_station`, `abutment_right_station`, `abutment_left_top_elevation`, `abutment_right_top_elevation`, and optional `abutment_*_top_profile_stations` / `_elevations`. Discover the full list via `getWasmApiMetadata().bridge_fields.rating_curve_inputs`.

#### E. High Flow: Pressure (Sluice Gate and Submerged Orifice)
When the upstream energy grade exceeds the low chord, pressure flow is evaluated and compared to the low-flow answer (the higher headwater is used). HEC-RAS selects the equation automatically:

* **Sluice gate** (downstream tailwater below the low chord): FHWA sluice-gate form with $C_d$ from Y3/Z (0.27–0.5) unless `bridge_pressure_flow_coeffs_inlet` is set.
* **Submerged orifice** (both sides under the deck): $Q = C A_{net}\sqrt{2g(E_{up} - TW_{down})}$ using `bridge_orifice_coeffs` as the submerged coefficient (typical 0.8).

#### F. High Flow: Weir Overtopping (Combined Flow)
When upstream energy exceeds the high chord, flow is split between pressure flow under the deck and weir overtopping:
$$Q_{total} = Q_{pressure} + Q_{weir}$$
$$Q_{weir} = C_w f_s L_{road} (E_{up} - H_{road})^{1.5}$$
where $f_s$ is the Bradley (1978) submergence factor from downstream tailwater. If submergence exceeds `bridge_max_weir_submergence` (default 0.98), the solver switches to the energy method through the opening instead of pressure/weir equations.

#### F2. High-Flow Method Selection
Set `bridge_high_flow_methods` per bridge when downstream tailwater is at or above the low chord:

* `0` — **Pressure and weir** (default): sluice-gate / submerged-orifice pressure flow plus Bradley weir overtopping; energy is used only when weir submergence exceeds `bridge_max_weir_submergence`.
* `1` — **Energy**: always balance upstream and downstream energy through the obstructed opening (same formulation as the submergence fallback). Uses WSPRO contraction loss when `bridge_low_flow_methods` is `4` or auto with abutments; otherwise standard contraction/expansion velocity-head losses. Reported as flow regime `energy`.

#### G. Deck Geometry Profiles
Optional piecewise-linear deck/roadway profiles per bridge (HEC-RAS deck editor analogue):

* `bridge_deck_stations` — horizontal stations across the opening (user units, monotonic)
* `bridge_deck_low_elevations` — low chord (soffit) at each station
* `bridge_deck_high_elevations` — high chord (roadway crest) at each station

When provided (≥ 2 points each), the solver uses profile extrema: **minimum** low chord for free-flow limits, **maximum** low chord for pressure-flow EGL trigger, **minimum** high chord for weir onset, and segment-wise **effective weir length** and **trapezoidal opening area** for pressure flow. Scalar `bridge_low_chords` / `bridge_high_chords` remain required fallbacks when profiles are omitted.

#### H0. Geometry modifiers — blocked vs ineffective vs bridge ineffective

Three HEC-RAS cross-section modifiers change how properties are computed. They are **not interchangeable**.

| Modifier | Where defined | Coordinate frame | Below threshold | Storage `area` | Conveyance `active_area` / `conveyance` |
|----------|---------------|------------------|-----------------|----------------|----------------------------------------|
| **Blocked obstruction** | `blocked_obstructions` on `CrossSection` | Reach lateral `x` | WSEL below obstruction **crest** | **Removed** (raises effective bed) | **Removed** |
| **Normal ineffective** | `ineffective_flow_areas` (alias `ineffective_areas`) on `CrossSection` | Reach lateral `x` | WSEL `<` block **activation elevation** | **Retained** (ponds storage) | **Removed** in ineffective zones |
| **Bridge ineffective** | `bridge_ineffective_*` on steady/unsteady inputs; or `ineffective_flow_areas` on explicit BU/BD cuts | Opening station `s` (legacy fields, shifted by `bridge_opening_reach_station_origins`); reach `x` on BU/BD `CrossSection` | Same as normal ineffective | Same as normal ineffective | Same as normal ineffective |

**Choosing a modifier**

* Permanent fill, culvert embankment, or raised bed under a polyline → `blocked_obstructions`.
* Overbank or floodplain that can pond but does not convey until a higher stage → `ineffective_flow_areas` on the reach cut (or BU/BD cut at a bridge face).
* Ineffective tied to the bridge opening in HEC-RAS opening coordinates → `bridge_ineffective_*` (or explicit BU/BD `ineffective_flow_areas` in reach `x`).

**OR logic (ineffective only):** multiple left/right blocks per side merge with OR semantics — a wetted segment is ineffective if **any** matching block triggers (`x < station` and WSEL `< elevation` on the left; `x > station` on the right).

**`GeometryRow` fields:** `area` is total submerged storage; `active_area` and `conveyance` exclude ineffective zones and guide-bank clipping but include ponded ineffective volume in `area`. Blocked obstructions reduce both.

#### H. Ineffective Flow Areas
Optional HEC-RAS ineffective-flow blocks per bridge at the upstream and downstream bridge faces. Each side may have **multiple blocks** (OR logic: a segment is ineffective if any block on that side triggers).

* **Legacy shared fields** (apply to both faces when per-face fields are omitted): `bridge_ineffective_left_stations`, `bridge_ineffective_left_elevations`, `bridge_ineffective_right_stations`, `bridge_ineffective_right_elevations`
* **Upstream face:** `bridge_ineffective_left_stations_upstream`, `bridge_ineffective_left_elevations_upstream`, `bridge_ineffective_right_stations_upstream`, `bridge_ineffective_right_elevations_upstream`
* **Downstream face:** `bridge_ineffective_left_stations_downstream`, `bridge_ineffective_left_elevations_downstream`, `bridge_ineffective_right_stations_downstream`, `bridge_ineffective_right_elevations_downstream`

**Array shape:** flat `[s0, s1]` = one block per bridge (backward compatible); nested `[[s0, s1], [s2]]` = multiple blocks on bridge 0, one on bridge 1. The same pattern applies to elevations and per-face overrides.

Per-face values override legacy shared fields on that face only. Semantics: **§H0**. BU/BD resolution order: [`BRIDGE_INTERIOR_SECTIONS_API.md`](../BRIDGE_INTERIOR_SECTIONS_API.md).

#### H2. Blocked Obstructions (Cross Sections)
Permanent fill on any `CrossSection`. Semantics: **§H0**.

* **Field:** `blocked_obstructions` — `{ stations, elevations }[]` (≥ 2 points, monotonic stations).
* Overlapping polylines use the maximum crest elevation at each lateral station.

Example — 2 m tall blockage across 12–18 m on a trapezoidal section:

```json
"blocked_obstructions": [
  { "stations": [12.0, 18.0], "elevations": [2.0, 2.0] }
]
```

Blocked obstructions on **user** cross sections are baked into lookup tables at section build time. On **`max_spacing` interior nodes** with `densify_reach_modifier_policy: 0` (default), blocked hydraulics still appear via **table interpolation** between parents; obstruction polylines are not copied. With policy `1`/`2`/`3`, blocked polylines are **copied** from the chosen parent onto the synthetic interpolated cut and the interior table is rebuilt from that cut.

#### I. BU / BD interior cross sections (API v22)

HEC-RAS uses dedicated **BU** (bridge upstream face) and **BD** (bridge downstream face) cuts. Optional explicit sections override reach interval geometry for bridge hydraulics:

* `bridge_upstream_cross_sections` — BU cut per bridge (`[bridge]` → `CrossSection`)
* `bridge_downstream_cross_sections` — BD cut per bridge
* `bridge_internal_cross_sections` — optional interior cuts `[bridge][section]`, US → DS (stored; multi-segment routing in a future release)
* `bridge_opening_reach_station_origins` — explicit reach XS lateral `x` at opening station 0 (overrides anchor mode when set).
* `bridge_opening_anchor_modes` — `0` = BU left `min(x)`, `1` = reach river station, `2` = explicit lateral `x`.
* `bridge_opening_anchor_reach_stations` — longitudinal reach station for mode `1` (densified grid lookup).

**Opening ↔ reach alignment:** Hosts pass deck, pier, abutment, and bridge ineffective stations in opening coordinates (station 0 = left deck edge). When `opening_origin` is resolved, the preprocessor maps them to reach lateral `x` via `reach_x = origin + opening_s` before bridge hydraulics. Plan-view and longitudinal diagrams: [`docs/BRIDGE_INTERIOR_SECTIONS_API.md` § Coordinate convention diagram](../BRIDGE_INTERIOR_SECTIONS_API.md#coordinate-convention-diagram-13).

**Reach layout:** after `max_spacing` densification, the solver inserts densified nodes at resolved BU/BD (and internal) river stations. Bridge hydraulics run on the interval `BU → BD`, not the wider reach interval around `bridge_stations`. Legacy models with only `bridge_stations` (no explicit faces, zero `bridge_lengths`) keep the prior center-station interval match.

**HEC-RAS weighting & ineffective resolution:** [`BRIDGE_INTERIOR_SECTIONS_API.md`](../BRIDGE_INTERIOR_SECTIONS_API.md).

Rating curve: `xs_up` / `xs_down` are BU/BD; optional `opening_reach_station_origin` and `xs_internal`.

**Example (steady JSON, one bridge with BU + internal + BD):**

```json
{
  "bridge_stations": [50.0],
  "bridge_low_chords": [5.0],
  "bridge_high_chords": [7.0],
  "bridge_low_flow_methods": [1],
  "bridge_opening_reach_station_origins": [0.0],
  "bridge_upstream_cross_sections": [{
    "station": 52.0,
    "x": [0.0, 0.0, 10.0, 10.0],
    "y": [10.05, 0.05, 0.05, 10.05],
    "n_stations": [0.0],
    "n_values": [0.03],
    "unit_system": "Metric"
  }],
  "bridge_downstream_cross_sections": [{
    "station": 48.0,
    "x": [0.0, 0.0, 10.0, 10.0],
    "y": [10.0, 0.0, 0.0, 10.0],
    "n_stations": [0.0],
    "n_values": [0.03],
    "unit_system": "Metric",
    "ineffective_flow_areas": {
      "left_blocks": [{ "station": 2.0, "elevation": 3.0 }],
      "right_blocks": []
    }
  }],
  "bridge_internal_cross_sections": [[{
    "station": 50.0,
    "x": [0.0, 0.0, 10.0, 10.0],
    "y": [10.025, 0.025, 0.025, 10.025],
    "n_stations": [0.0],
    "n_values": [0.03],
    "unit_system": "Metric"
  }]]
}
```

Full working fixture: [`examples/wasm/steady_bridge_bu_bd_v22.json`](examples/wasm/steady_bridge_bu_bd_v22.json). TypeScript types: [`docs/wasm_api.types.ts`](docs/wasm_api.types.ts). Design notes: [`docs/BRIDGE_INTERIOR_SECTIONS_API.md`](docs/BRIDGE_INTERIOR_SECTIONS_API.md).

#### J. Bridge Skew and Pier Spacing
* `bridge_skew_angles` — skew from normal to flow, degrees per bridge (0–59°, same convention as `culvert_skew_angles`). Adjusts projected opening width ($W' = W\cos\theta$), weir length, deck profile segments, friction reach length ($L' = L/\cos\theta$), and flow-normal pier blockage ($W_{pier}' = W_{pier}/\cos\theta$).
* `bridge_pier_stations` — pier centerline stations across the opening per bridge `[bridge][pier]` in the same horizontal frame as `bridge_deck_stations`. When omitted, piers are evenly spaced across the deck opening span. Pier count is taken from the station array length when provided.

#### K. Bridge Rating Curve
* **Rating curve:** `computeBridgeRatingCurve({ q_values, ...bridge fields })` samples upstream headwater vs discharge at fixed tailwater for a single bridge opening. Uses the same hydraulics as the steady bridge solver (`solve_bridge_coupled`) without a full reach profile.
* **Input fields** (flattened, not `bridge_*` prefixed): `low_chord`, `high_chord`, `z_up`, `z_down`, `tw_wsel`, `units`, plus optional pier/deck/ineffective/skew/coupling fields (`pier_width`, `num_piers`, `deck_stations`, `skew_deg`, `pier_stations`, `ineffective_left_station` or `ineffective_left_stations` / `ineffective_left_elevations` vectors, etc.). **Abutments** use the same per-side keys as steady bridge fields but without the `bridge_` prefix: `abutment_block_width` (legacy total), `abutment_left_width`, `abutment_right_width`, `abutment_left_station`, `abutment_right_station`, `abutment_left_top_elevation`, `abutment_right_top_elevation`, and optional `abutment_left_top_profile_stations` / `_elevations` (and right pair). Defaults to rectangular approach/departure channels via `channel_width` (10 user units) when `xs_up` / `xs_down` are omitted.
* **Outputs:** `q`, `wsel` (upstream headwater), `wsel_down`, `flow_regimes` (`low_a` / `low_b` / `low_c` / `pressure` / `weir`), `head_losses`. Discover field names via `getWasmApiMetadata().bridge_fields.rating_curve_inputs` and `rating_curve_outputs`.

---

### 7. Core Solver Assumptions & Corrections

#### A. Channel vs. Overbank Flow at Structures
When cross-sections are subdivided into channel and overbank zones (`is_overbank`), stagnant overbank storage can inflate total area near structures.
* **Implementation:** Geometry tables include a **`channel_area`** lookup (main channel only). At cross-sections adjacent to bridges and culverts, Standard Step and Yarnell pier losses use `channel_area` instead of total area where subdivision is present.

#### B. Culvert Outlet Velocity Head ($\alpha$)
In culvert **outlet control**, contracted approach/departure velocities use a velocity-head multiplier of $\alpha \approx 1.3$ on the downstream and upstream channel velocities when evaluating exit-loss and energy balance terms. The general Standard Step sweep between cross-sections uses $\alpha = 1.0$.

---
