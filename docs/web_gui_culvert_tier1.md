# Web GUI: Culvert Tier 1 Integration

Implementation prompt for expanding the **Culverts** geometry editor to use STREAM-1D engine **API version 2** (Tier 1 culvert features).

---

## Background

The STREAM-1D WASM engine (`feat/culvert-tier1` / `api_version: 2`) now supports:

- **Explicit FHWA inlet types** (replacing implicit Ke-threshold guessing)
- **Optional culvert invert elevations** (when barrel invert ≠ channel bed)
- **Roadway / embankment overtopping** (weir flow above crest)
- **Per-culvert control reporting** (`inlet` | `outlet` | `overtopping`) in solve results

The existing Culverts tab already captures station, shape, geometry, roughness, blockage, and loss coefficients. This task **extends** that panel — it does not replace the current layout.

**Engine references (STREAM-1D repo)**

| Resource | Purpose |
|----------|---------|
| [`docs/wasm_api.types.ts`](wasm_api.types.ts) | TypeScript interfaces — copy into web app |
| [`docs/wasm_integration.md`](wasm_integration.md) | Worker pattern, payload mapping |
| [`tests/fixtures/wasm_steady_culvert_tier1.json`](../tests/fixtures/wasm_steady_culvert_tier1.json) | Full example `solveSteady` payload |
| [`examples/wasm/worker_solve_steady.mjs`](../examples/wasm/worker_solve_steady.mjs) | Reference Worker |

After upgrading the WASM package, call `getWasmApiMetadata()` and confirm `api_version === 2`.

---

## Existing UI (baseline)

The Culverts tab currently includes:

| Section | Fields | WASM mapping (already wired) |
|---------|--------|------------------------------|
| **General** | Station, Shape | `culvert_stations`, `culvert_shape_types` |
| **Geometry** | Span/Dia, Rise, Length | `culvert_spans`, `culvert_rises`, `culvert_lengths` |
| **Roughness & Blockage** | n (Top/Sides), n (Bottom), Bottom n Depth, Depth Blocked | `culvert_roughness_ns`, `culvert_roughness_n_bottoms`, `culvert_depth_bottom_ns`, `culvert_depth_blockeds` |
| **Loss Coefficients** | (Ke, Kx, barrels — verify present) | `culvert_entrance_loss_coeffs`, `culvert_exit_loss_coeffs`, `culvert_barrels` |

Culvert list columns: **No.**, **Station**, **Type / Size**, **Del**.

Cross-section profile (bottom pane) already draws the culvert barrel (e.g. ConSpan arch) and WSEL/EGL lines.

---

## Task: Add Tier 1 fields to culvert model + GUI

### 1. Extend culvert app state / persistence

Each culvert object in app state should add (nullable / optional where noted):

```typescript
interface CulvertModel {
  // --- existing fields ---
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

  // --- Tier 1 (new) ---
  inletType: number;             // default 0 = legacy; see inlet enum below
  useChannelBedInverts: boolean; // default true
  zUp?: number;                  // only when useChannelBedInverts === false
  zDown?: number;
  overtoppingEnabled: boolean;   // default false
  crestElev?: number;            // required when overtoppingEnabled
  weirCoeff?: number;            // optional; 0 or empty = engine default
  weirLength?: number;           // optional; 0 or empty = span × numBarrels

  // --- Tier 1 result (read-only, from last solve) ---
  controlType?: 'inlet' | 'outlet' | 'overtopping';
}
```

Persist new fields in project JSON / local storage alongside existing culvert data. Use **backward-compatible defaults** so old projects load without migration errors (`inletType: 0`, `useChannelBedInverts: true`, `overtoppingEnabled: false`).

---

### 2. GUI layout — new sections in Culvert Properties panel

Add three collapsible sections **below Loss Coefficients** (or reorganize if space is tight). Match existing styling (rounded inputs, section headers like `GENERAL`, `GEOMETRY`).

#### A. **INLET** (new section)

| Control | Type | Behavior |
|---------|------|----------|
| Inlet type | Dropdown | Options from `getWasmApiMetadata().culvert_inlet_types`, **filtered by selected Shape** |

**Shape → inlet options filter**

| Shape (`culvert_shape_types`) | Show inlet codes |
|-------------------------------|------------------|
| Circular (0) | 0 Legacy, 1 Square headwall, 2 Groove end, 3 Beveled 45°, 4 Projecting |
| Box (1) | 0 Legacy, 10 Square edge, 11 Flared wingwalls, 12 Beveled top |
| Arch (2) or Conspan (3) | 0 Legacy, 20 Projecting, 21 Smooth entry headwall |

- Display `description` from metadata as the dropdown label; store `code` as `inletType`.
- Default: **Legacy (Ke-based)** = `0` for imported projects and new culverts.
- Helper text: *"Legacy uses entrance loss Ke to infer inlet nomograph. Choose an explicit type for HEC-RAS parity."*

#### B. **INVERT ELEVATIONS** (new section)

| Control | Type | Behavior |
|---------|------|----------|
| Use channel bed inverts | Checkbox (default **on**) | When on, omit `culvert_z_ups` / `culvert_z_downs` from WASM payload |
| Upstream invert elev. | Number (ft or m) | Enabled when checkbox off |
| Downstream invert elev. | Number | Enabled when checkbox off |

- Label units from reach `unit_system`.
- Optional: "Pick from profile" — click upstream/downstream bed on cross-section plot to set values (nice-to-have, not required for v1).
- When checkbox is on, show read-only hint: *"Uses adjacent cross-section bed elevation at solve time."*

#### C. **ROADWAY OVERTOPPING** (new section)

| Control | Type | Behavior |
|---------|------|----------|
| Model overtopping | Checkbox (default **off**) | When off, omit `culvert_crest_elevs` from WASM payload |
| Crest elevation | Number | Required when overtopping enabled |
| Weir coefficient Cw | Number (optional) | Placeholder: `2.6` US / `1.44` metric; send `0` or omit for engine default |
| Weir length | Number (optional) | Placeholder: `span × barrels`; send `0` or omit for engine default |

- Helper text: *"Adds weir flow when headwater exceeds crest. Use for roadway overflow above the barrel."*
- When overtopping is enabled, optionally draw a **dashed horizontal line** at crest elevation on the cross-section profile (same pane as ConSpan arch).

---

### 3. Culvert list — show control type after solve

Add a column or badge on each culvert row:

| Control | Display |
|---------|---------|
| `controlType` | Pill badge: **Inlet** (blue), **Outlet** (amber), **Overtopping** (red/orange) |
| Empty before first solve | `—` or hidden |

Also show in **Culvert #N Properties** header, e.g.  
`CULVERT #1 PROPERTIES · Inlet control`

Populate from `solveSteady` result:

```javascript
result.culvert_control_types[i]  // aligns with culvert_stations[i]
```

---

### 4. WASM payload builder

Extend existing `buildSteadyInputs(culverts)` (or equivalent). Parallel arrays must stay **index-aligned** with `culvert_stations`.

```javascript
function culvertArraysFromModels(culverts, unitSystem) {
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
    // ... existing arrays ...

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
    // ... existing fields ...
    culvert_inlet_types: inletTypes,
    ...(sendZ && { culvert_z_ups: zUps, culvert_z_downs: zDowns }),
    ...(sendCrest && {
      culvert_crest_elevs: crestElevs,
      culvert_weir_coeffs: weirCoeffs,
      culvert_weir_lengths: weirLengths,
    }),
  };
}
```

**Important:** `culvert_inlet_types` should always be sent when culverts exist (array same length as `culvert_stations`). Invert and crest arrays are only needed when overrides / overtopping are active — but if one culvert in the list uses overrides, include arrays for **all** culverts at matching indices (use bed values or `null` handling per your serializer; engine expects aligned indices when field is present).

Recommended: when any culvert uses custom inverts, send full-length `culvert_z_ups` / `culvert_z_downs` for all culverts (bed-derived values for others). Same pattern for overtopping arrays.

Call `validateSteadyInputs(inputs)` in the Worker before `solveSteady` during development.

---

### 5. HEC-RAS import mapping (if applicable)

When importing `.g01` or project JSON:

| RAS field (typical) | Map to |
|---------------------|--------|
| Inlet description | `inletType` code via metadata lookup |
| Upstream / downstream invert | `zUp`, `zDown`, `useChannelBedInverts: false` |
| Roadway crest / high chord | `crestElev`, `overtoppingEnabled: true` |
| Number of barrels | `numBarrels` (existing) |

If RAS inlet cannot be mapped confidently, fall back to `inletType: 0` (legacy) and preserve Ke.

---

### 6. Cross-section profile enhancements (optional but valuable)

When the selected culvert is shown at Station 1257 (or nearest section):

1. Draw **crest elevation** line when `overtoppingEnabled`.
2. Tooltip on control badge explaining controlling mechanism (e.g. *"Outlet control — tailwater governs barrel flow"*).

---

## Acceptance criteria

- [ ] Culvert properties panel includes **Inlet**, **Invert elevations**, and **Roadway overtopping** sections.
- [ ] Inlet dropdown filters options by culvert shape; defaults to Legacy (`0`).
- [ ] "Use channel bed inverts" omits invert arrays from WASM when enabled.
- [ ] Overtopping checkbox omits crest/weir arrays when disabled.
- [ ] `solveSteady` payload includes Tier 1 fields; `validateSteadyInputs` passes.
- [ ] After solve, culvert list and properties show `culvert_control_types` badge.
- [ ] Old projects without Tier 1 fields load and solve identically to before (backward compatible).
- [ ] New fields persist in project save/load.
- [ ] Unit tests for `culvertArraysFromModels()` covering: legacy defaults, explicit inlet, invert override, overtopping on/off.
- [ ] WASM `api_version` checked on Worker init; warn if `< 2`.

---

## Non-goals (this task)

- Do not change cross-section or bridge tabs.
- Do not add new WASM geometry slots or third reach arrays (see [`web_gui_tributary_junction.md`](web_gui_tributary_junction.md)).
- Do not implement culvert sizing / inverse solvers (future enhancement).
- Unsteady culvert routing is **not** supported by the engine yet.

---

## Verification (QA)

1. Load ConSpan test project (station ~1251.8, 28×6). Solve steady — expect `culvert_control_types` populated (likely `outlet` at high tailwater).
2. Set explicit inlet type, re-solve — headwater may shift vs Legacy; badge should update.
3. Enable overtopping with low crest + high Q — expect `overtopping` badge and higher upstream WSEL.
4. Toggle "use channel bed inverts" off, set inverts above bed — headwater should increase.
5. Compare WSEL at key stations against HEC-RAS within existing app tolerance where applicable.

---

## Copy-paste prompt for Cursor / ticket

<details>
<summary><strong>Short agent prompt</strong></summary>

Expand the web app **Culverts** geometry tab to support STREAM-1D WASM **API v2 Tier 1 culvert fields**.

**Existing UI:** Culvert list (No., Station, Type/Size, Del) + properties panel with General (station, shape), Geometry (span, rise, length), Roughness & Blockage, Loss Coefficients. Cross-section profile shows barrel and WSEL lines.

**Add to culvert model + GUI:**
1. **Inlet** dropdown — FHWA inlet type codes filtered by shape; populate labels from `getWasmApiMetadata().culvert_inlet_types`; default code `0` (legacy).
2. **Invert elevations** — checkbox "Use channel bed inverts" (default on); optional upstream/downstream invert fields when off → `culvert_z_ups`, `culvert_z_downs`.
3. **Roadway overtopping** — checkbox + crest elevation + optional weir Cw and length → `culvert_crest_elevs`, `culvert_weir_coeffs`, `culvert_weir_lengths`.
4. **Results** — show `culvert_control_types[i]` badge (`inlet`|`outlet`|`overtopping`) on list row and properties header after `solveSteady`.

**WASM:** Extend `buildSteadyInputs` parallel culvert arrays; use `validateSteadyInputs` + `solveSteady`. Types in STREAM-1D `docs/wasm_api.types.ts`. Fixture: `tests/fixtures/wasm_steady_culvert_tier1.json`.

**Constraints:** Backward compatible project load; snake_case JSON; no engine changes in web repo.

See full spec: `docs/web_gui_culvert_tier1.md` in STREAM-1D repo.

</details>
