# Web GUI: Culvert Integration Spec (Tier 1 + Tier 2a)

**Audience:** Companion web app team  
**Engine:** STREAM-1D WASM (`api_version` **3**)  
**Scope:** Wire the existing **Culverts** tab to full culvert hydraulics — inputs, solve results, diagnostics, and optional rating curves.

This document supersedes [`web_gui_culvert_tier1.md`](web_gui_culvert_tier1.md) as the single handoff spec. Tier 1 and Tier 2a are described together; you may ship GUI work in two passes (see [Rollout](#rollout-suggested-phasing)).

---

## Engine references (STREAM-1D repo)

| Resource | Purpose |
|----------|---------|
| [`wasm_api.types.ts`](wasm_api.types.ts) | TypeScript interfaces — copy into web app |
| [`wasm_integration.md`](wasm_integration.md) | Worker pattern, general WASM setup |
| [`tests/fixtures/wasm_steady_culvert_tier1.json`](../tests/fixtures/wasm_steady_culvert_tier1.json) | Example `solveSteady` payload with Tier 1 fields |
| [`examples/wasm/worker_solve_steady.mjs`](../examples/wasm/worker_solve_steady.mjs) | Reference Web Worker |

**After upgrading the WASM package:**

```javascript
import init, {
  getEngineVersion,
  getWasmApiMetadata,
  validateSteadyInputs,
  solveSteady,
  computeCulvertRatingCurve,
} from './pkg/streams1d.js';

await init();
const meta = getWasmApiMetadata();
console.log(getEngineVersion(), 'API', meta.api_version); // expect api_version === 3
```

Warn in the app if `meta.api_version < 3` — culvert diagnostics and rating curves will be missing.

---

## What the engine provides

### Tier 1 — structure inputs & control type

| Capability | WASM inputs | WASM outputs |
|------------|-------------|--------------|
| Explicit FHWA inlet types | `culvert_inlet_types` | — |
| Optional invert elevations | `culvert_z_ups`, `culvert_z_downs` | — |
| Roadway / embankment overtopping | `culvert_crest_elevs`, `culvert_weir_coeffs`, `culvert_weir_lengths` | — |
| Controlling mechanism | — | `culvert_control_types` (`inlet` \| `outlet` \| `overtopping`) |

### Tier 2a — extended diagnostics & rating curve

| Capability | WASM API | Notes |
|------------|----------|-------|
| Inlet vs outlet headwater split | `solveSteady` → `culvert_wsel_inlet`, `culvert_wsel_outlet` | Per culvert, user units |
| Barrel vs weir flow split | `culvert_q_barrels`, `culvert_q_weirs` | When overtopping active |
| Barrel hydraulics | `culvert_barrel_depths`, `culvert_barrel_velocities`, `culvert_barrel_froude` | At downstream end of barrel |
| Headwater vs Q curve | `computeCulvertRatingCurve(inputs)` | Fixed tailwater; independent of reach solve |

**Not in scope (engine):** unsteady culvert routing, culvert skew, per-barrel geometry, supercritical culvert solve.

---

## Existing UI (baseline — do not replace)

The Culverts tab already includes:

| Section | Fields | WASM mapping (verify wired) |
|---------|--------|----------------------------|
| **General** | Station, Shape | `culvert_stations`, `culvert_shape_types` |
| **Geometry** | Span/Dia, Rise, Length | `culvert_spans`, `culvert_rises`, `culvert_lengths` |
| **Roughness & Blockage** | n (Top/Sides), n (Bottom), Bottom n Depth, Depth Blocked | `culvert_roughness_ns`, `culvert_roughness_n_bottoms`, `culvert_depth_bottom_ns`, `culvert_depth_blockeds` |
| **Loss Coefficients** | Ke, Kx, barrels | `culvert_entrance_loss_coeffs`, `culvert_exit_loss_coeffs`, `culvert_barrels` |

Culvert list columns today: **No.**, **Station**, **Type / Size**, **Del**.

Cross-section profile (bottom pane) draws the culvert barrel and WSEL/EGL lines.

**This task extends** the panel and results display — it does not redesign the tab.

---

## Culvert app state model

Extend each culvert object in project state / persistence:

```typescript
import type { CulvertControlType } from './streams1d'; // from wasm_api.types.ts

interface CulvertModel {
  // --- existing (already in app) ---
  station: number;
  shapeType: number;           // 0 Circular, 1 Box, 2 Arch, 3 Conspan
  span: number;
  rise: number;
  length: number;
  roughnessN: number;
  roughnessNBottom: number;
  depthBottomN: number;
  depthBlocked: number;
  entranceLossCoeff: number;
  exitLossCoeff: number;
  numBarrels: number;

  // --- Tier 1 inputs ---
  inletType: number;             // default 0 = legacy Ke inference
  useChannelBedInverts: boolean; // default true
  zUp?: number;                  // when useChannelBedInverts === false
  zDown?: number;
  overtoppingEnabled: boolean;   // default false
  crestElev?: number;            // required when overtoppingEnabled
  weirCoeff?: number;            // 0 or empty → engine default (2.6 US / 1.44 metric)
  weirLength?: number;           // 0 or empty → span × numBarrels

  // --- Tier 1 + 2a results (read-only, from last solve) ---
  controlType?: CulvertControlType;
  wselInlet?: number;
  wselOutlet?: number;
  qBarrel?: number;
  qWeir?: number;
  barrelDepth?: number;
  barrelVelocity?: number;
  barrelFroude?: number;
}
```

**Backward-compatible defaults** for old projects: `inletType: 0`, `useChannelBedInverts: true`, `overtoppingEnabled: false`. Result fields stay undefined until first solve with API v3.

---

## GUI layout — new & updated sections

Add collapsible sections **below Loss Coefficients**. Match existing styling (rounded inputs, `GENERAL` / `GEOMETRY` section headers).

### A. INLET (Tier 1)

| Control | Type | Behavior |
|---------|------|----------|
| Inlet type | Dropdown | Options from `getWasmApiMetadata().culvert_inlet_types`, **filtered by shape** |

**Shape → inlet codes**

| Shape (`shapeType`) | Show codes |
|---------------------|------------|
| Circular (0) | 0 Legacy, 1 Square headwall, 2 Groove end, 3 Beveled 45°, 4 Projecting |
| Box (1) | 0 Legacy, 10 Square edge, 11 Flared wingwalls, 12 Beveled top |
| Arch (2) or Conspan (3) | 0 Legacy, 20 Projecting, 21 Smooth entry headwall |

- Dropdown label = metadata `description`; stored value = `code`.
- Default **Legacy (0)** for imports and new culverts.
- Helper: *"Legacy uses entrance loss Ke to pick the inlet nomograph. Choose an explicit type for HEC-RAS parity."*

### B. INVERT ELEVATIONS (Tier 1)

| Control | Type | Behavior |
|---------|------|----------|
| Use channel bed inverts | Checkbox (default **on**) | When on, omit `culvert_z_ups` / `culvert_z_downs` from payload |
| Upstream invert elev. | Number | Enabled when checkbox off |
| Downstream invert elev. | Number | Enabled when checkbox off |

- Units from reach `unit_system`.
- When checkbox on, read-only hint: *"Uses adjacent cross-section bed elevation at solve time."*
- Nice-to-have: pick inverts from profile click (not required for v1).

### C. ROADWAY OVERTOPPING (Tier 1)

| Control | Type | Behavior |
|---------|------|----------|
| Model overtopping | Checkbox (default **off**) | When off, omit `culvert_crest_elevs` |
| Crest elevation | Number | Required when enabled |
| Weir coefficient Cw | Number (optional) | Placeholder 2.6 US / 1.44 metric |
| Weir length | Number (optional) | Placeholder `span × barrels` |

- Helper: *"Adds weir flow when headwater exceeds crest."*
- Draw **dashed horizontal line** at crest on cross-section profile when enabled.

### D. HYDRAULICS (Tier 2a — read-only results)

New section shown **after a successful steady solve** with culverts modeled. All values from `solveSteady` result arrays (index = culvert index).

| Display label | Result field | Format |
|---------------|--------------|--------|
| Controlling mechanism | `culvert_control_types[i]` | Badge (see below) |
| Headwater (controlling) | upstream WSEL at culvert station | From reach `wsel` (existing) |
| Inlet-control HW | `culvert_wsel_inlet[i]` | Elev., 2 decimals |
| Outlet-control HW | `culvert_wsel_outlet[i]` | Elev., 2 decimals |
| Barrel discharge | `culvert_q_barrels[i]` | Q, 1 decimal |
| Weir discharge | `culvert_q_weirs[i]` | Q; show `—` when ≈ 0 |
| Barrel depth | `culvert_barrel_depths[i]` | Depth, 2 decimals |
| Barrel velocity | `culvert_barrel_velocities[i]` | V, 2 decimals |
| Barrel Froude | `culvert_barrel_froude[i]` | Fr, 2 decimals |

**Control badge colors (suggested):**

| Value | Badge | Tooltip |
|-------|-------|---------|
| `inlet` | Blue **Inlet** | Inlet nomograph governs headwater |
| `outlet` | Amber **Outlet** | Tailwater / barrel friction governs |
| `overtopping` | Orange **Overtopping** | Roadway weir carries part or all of flow |

Show badge on culvert list row, properties header (`CULVERT #1 · Inlet control`), and HYDRAULICS section.

When `culvert_wsel_inlet[i] ≈ culvert_wsel_outlet[i]` within tolerance, controlling HW equals both (rare).

### E. RATING CURVE (Tier 2a — optional panel)

Add a **"Headwater curve"** button on culvert properties (or toolbar) that opens a modal/side panel:

1. User sets **Q min**, **Q max**, **number of points** (or explicit `q_values` list).
2. **Tailwater** defaults to current downstream boundary WSEL at solve time (editable).
3. Call `computeCulvertRatingCurve` in the Worker with culvert geometry from the selected culvert model.

**Payload builder** (flattened JSON — same field names as `CulvertRatingCurveInputs` in types):

```javascript
function buildRatingCurveInputs(culvert, qValues, unitSystem, tailwaterWsel) {
  return {
    q_values: qValues,
    tw_wsel: tailwaterWsel,
    units: unitSystem,
    shape_type: culvert.shapeType,
    inlet_type: culvert.inletType ?? 0,
    span: culvert.span,
    rise: culvert.rise,
    roughness_n: culvert.roughnessN,
    length: culvert.length,
    entrance_loss_coeff: culvert.entranceLossCoeff,
    exit_loss_coeff: culvert.exitLossCoeff,
    z_down: culvert.useChannelBedInverts ? /* bed at ds */ : culvert.zDown,
    z_up: culvert.useChannelBedInverts ? /* bed at us */ : culvert.zUp,
    manning_n_bottom: culvert.roughnessNBottom || culvert.roughnessN,
    depth_bottom_n: culvert.depthBottomN ?? 0,
    depth_blocked: culvert.depthBlocked ?? 0,
    num_barrels: culvert.numBarrels ?? 1,
    ...(culvert.overtoppingEnabled && culvert.crestElev != null && {
      crest_elev: culvert.crestElev,
      weir_coeff: culvert.weirCoeff ?? 0,
      weir_length: culvert.weirLength ?? 0,
    }),
  };
}
```

**Chart:** Plot `result.wsel` vs `result.q`. Optional second axis or tooltip: `control_types`, `q_barrel` / `q_weir` at each point. Color-code regions by control type if feasible.

**Worker handler:**

```javascript
case 'computeCulvertRatingCurve':
  validateSteadyInputs is NOT used for this call;
  const curve = computeCulvertRatingCurve(event.data.inputs);
  postMessage({ type: 'ratingCurveResult', curve });
  break;
```

---

## WASM: `solveSteady` payload builder

Extend existing `buildSteadyInputs(culverts)`. Parallel arrays stay **index-aligned** with `culvert_stations`.

```javascript
function culvertArraysFromModels(culverts) {
  const stations = [];
  const inletTypes = [];
  const zUps = [];
  const zDowns = [];
  const crestElevs = [];
  const weirCoeffs = [];
  const weirLengths = [];
  let sendZ = false;
  let sendCrest = false;

  for (const c of culverts) {
    stations.push(c.station);
    // ... push existing arrays (shape, span, rise, n, Ke, Kx, barrels, etc.) ...

    inletTypes.push(c.inletType ?? 0);

    if (!c.useChannelBedInverts) {
      sendZ = true;
      zUps.push(c.zUp);
      zDowns.push(c.zDown);
    }

    if (c.overtoppingEnabled && c.crestElev != null) {
      sendCrest = true;
      crestElevs.push(c.crestElev);
      weirCoeffs.push(c.weirCoeff ?? 0);
      weirLengths.push(c.weirLength ?? 0);
    }
  }

  return {
    culvert_stations: stations,
    culvert_inlet_types: inletTypes,
    // ... existing culvert_* arrays ...
    ...(sendZ && { culvert_z_ups: zUps, culvert_z_downs: zDowns }),
    ...(sendCrest && {
      culvert_crest_elevs: crestElevs,
      culvert_weir_coeffs: weirCoeffs,
      culvert_weir_lengths: weirLengths,
    }),
  };
}
```

**Array alignment rules**

- `culvert_inlet_types`: always send when culverts exist (same length as `culvert_stations`).
- When **any** culvert uses custom inverts, send full-length `culvert_z_ups` / `culvert_z_downs` for **all** culverts (use bed-derived values for others).
- Same pattern for overtopping arrays when any culvert has overtopping enabled.

Call `validateSteadyInputs(inputs)` before `solveSteady` during development.

---

## WASM: mapping solve results back to culvert models

After `solveSteady`:

```javascript
function applyCulvertResults(culverts, result) {
  if (!result.culvert_control_types) return;

  culverts.forEach((c, i) => {
    c.controlType = result.culvert_control_types[i];
    c.wselInlet = result.culvert_wsel_inlet?.[i];
    c.wselOutlet = result.culvert_wsel_outlet?.[i];
    c.qBarrel = result.culvert_q_barrels?.[i];
    c.qWeir = result.culvert_q_weirs?.[i];
    c.barrelDepth = result.culvert_barrel_depths?.[i];
    c.barrelVelocity = result.culvert_barrel_velocities?.[i];
    c.barrelFroude = result.culvert_barrel_froude?.[i];
  });
}
```

Tier 2a fields are omitted when `api_version < 3` or no culverts in the model — hide HYDRAULICS section in that case.

---

## Full field reference

### Culvert inputs (`SteadyInputs` — parallel arrays)

| Field | Tier | Default if omitted |
|-------|------|-------------------|
| `culvert_stations` | base | — |
| `culvert_shape_types` | base | 0 (circular) |
| `culvert_spans` | base | — |
| `culvert_rises` | base | — |
| `culvert_roughness_ns` | base | — |
| `culvert_lengths` | base | — |
| `culvert_entrance_loss_coeffs` | base | — |
| `culvert_exit_loss_coeffs` | base | — |
| `culvert_barrels` | base | 1 |
| `culvert_roughness_n_bottoms` | base | top n |
| `culvert_depth_bottom_ns` | base | 0 |
| `culvert_depth_blockeds` | base | 0 |
| `culvert_inlet_types` | 1 | 0 (legacy) |
| `culvert_z_ups` | 1 | adjacent bed |
| `culvert_z_downs` | 1 | adjacent bed |
| `culvert_crest_elevs` | 1 | no overtopping |
| `culvert_weir_coeffs` | 1 | 2.6 US / 1.44 metric |
| `culvert_weir_lengths` | 1 | span × barrels |

### Culvert outputs (`SteadyResult` — parallel arrays)

| Field | Tier | Meaning |
|-------|------|---------|
| `culvert_control_types` | 1 | `inlet` \| `outlet` \| `overtopping` |
| `culvert_wsel_inlet` | 2a | Inlet-control headwater elev. |
| `culvert_wsel_outlet` | 2a | Outlet-control headwater elev. |
| `culvert_q_barrels` | 2a | Total barrel discharge |
| `culvert_q_weirs` | 2a | Weir discharge (overtopping) |
| `culvert_barrel_depths` | 2a | Flow depth in barrel |
| `culvert_barrel_velocities` | 2a | Mean barrel velocity |
| `culvert_barrel_froude` | 2a | Barrel Froude number |

### Rating curve (`computeCulvertRatingCurve`)

Returns `CulvertRatingCurveResult`: arrays `q`, `wsel`, `control_types`, `wsel_inlet`, `wsel_outlet`, `q_barrel`, `q_weir`, `barrel_depth`, `barrel_velocity`, `barrel_froude` — one entry per `q_values` point.

---

## HEC-RAS import mapping (when applicable)

| RAS field (typical) | Map to |
|---------------------|--------|
| Inlet description | `inletType` via metadata lookup |
| Upstream / downstream invert | `zUp`, `zDown`, `useChannelBedInverts: false` |
| Roadway crest / high chord | `crestElev`, `overtoppingEnabled: true` |
| Number of barrels | `numBarrels` (existing) |
| Entrance / exit loss | `entranceLossCoeff`, `exitLossCoeff` (existing) |

If RAS inlet cannot be mapped confidently, use `inletType: 0` (legacy) and preserve Ke.

---

## Rollout (suggested phasing)

| Phase | GUI work | WASM `api_version` |
|-------|----------|---------------------|
| **A** (ship first) | Tier 1 sections A–C; control badge; payload builder | ≥ 2 (≥ 3 recommended) |
| **B** | HYDRAULICS read-only section; map Tier 2a result fields | 3 |
| **C** | Rating curve modal + chart | 3 |

Phases A and B can ship in one release if launch timing allows. Phase C is independent of steady solve UI.

---

## Acceptance criteria

### Tier 1

- [ ] Properties panel: **Inlet**, **Invert elevations**, **Roadway overtopping** sections
- [ ] Inlet dropdown filtered by shape; defaults to Legacy (`0`)
- [ ] "Use channel bed inverts" omits invert arrays when on
- [ ] Overtopping off omits crest/weir arrays
- [ ] `solveSteady` payload validates via `validateSteadyInputs`
- [ ] Control badge on list + properties after solve
- [ ] Old projects load with backward-compatible defaults
- [ ] New fields persist in project save/load
- [ ] Unit tests for `culvertArraysFromModels()` (legacy, explicit inlet, inverts, overtopping)

### Tier 2a

- [ ] HYDRAULICS section shows inlet/outlet HW, Q split, barrel depth/velocity/Fr
- [ ] Results mapped from `culvert_wsel_inlet`, `culvert_q_barrels`, etc.
- [ ] Section hidden when Tier 2a fields absent (old WASM)
- [ ] `computeCulvertRatingCurve` wired in Worker; chart renders Q vs headwater
- [ ] Rating curve uses culvert model + tailwater from UI
- [ ] `getWasmApiMetadata().api_version` checked on Worker init; warn if `< 3`

---

## Non-goals

- Cross-section or bridge tab changes
- Third reach / junction geometry (see [`web_gui_tributary_junction.md`](web_gui_tributary_junction.md))
- Culvert sizing / inverse solvers
- Unsteady culvert routing (engine does not support yet)
- Engine changes in the web repo

---

## QA checklist

1. **Baseline:** ConSpan or circular test reach, steady solve — `culvert_control_types` populated.
2. **Explicit inlet:** Change inlet type vs Legacy — headwater may shift; badge updates.
3. **Inverts:** Disable bed inverts, raise upstream invert — headwater increases.
4. **Overtopping:** Low crest + high Q — `overtopping` badge, `culvert_q_weirs > 0`, WSEL above crest.
5. **Tier 2a diagnostics:** Inlet control case — `culvert_wsel_inlet ≈` controlling HW; outlet HW lower. Outlet case — reverse.
6. **Rating curve:** 10 Q points, fixed TW — monotonic increasing WSEL vs Q for typical barrel; chart matches table.
7. **Regression:** Old project file without new fields solves identically to pre-upgrade behavior.
8. **HEC-RAS:** Where applicable, compare key WSELs within app tolerance.

---

## Copy-paste ticket / agent prompt

<details>
<summary><strong>Short prompt for Cursor or Jira</strong></summary>

Wire the web app **Culverts** tab to STREAM-1D WASM **API v3** (Tier 1 + Tier 2a).

**Baseline UI:** Culvert list + properties (General, Geometry, Roughness & Blockage, Loss Coefficients). Extend — do not replace.

**Tier 1 — inputs**
1. **Inlet** dropdown from `getWasmApiMetadata().culvert_inlet_types`, filtered by shape; default `0`.
2. **Invert elevations** — "Use channel bed inverts" (default on); optional `zUp`/`zDown` → `culvert_z_ups`/`culvert_z_downs`.
3. **Roadway overtopping** — checkbox + crest + optional weir Cw/length → `culvert_crest_elevs`, `culvert_weir_coeffs`, `culvert_weir_lengths`.
4. **Control badge** from `culvert_control_types` after `solveSteady`.

**Tier 2a — results & tools**
5. **HYDRAULICS** read-only section: `culvert_wsel_inlet`, `culvert_wsel_outlet`, `culvert_q_barrels`, `culvert_q_weirs`, barrel depth/velocity/Froude.
6. **Rating curve** — `computeCulvertRatingCurve` in Worker; chart headwater vs Q at fixed tailwater.

**WASM:** `validateSteadyInputs` + `solveSteady`; types in `docs/wasm_api.types.ts`; fixture `tests/fixtures/wasm_steady_culvert_tier1.json`.

**Constraints:** Backward-compatible project load; snake_case JSON; no engine code in web repo.

Full spec: `docs/web_gui_culvert_integration.md` in STREAM-1D repo.

</details>
