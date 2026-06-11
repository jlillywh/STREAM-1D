# 2.3 Unified roadway embankment model — design

Single **bridge-side** input that composes **deck high/low chord**, **abutments**, and **bridge ineffective** blocks from one HEC-RAS-shaped embankment description. Reduces host preprocessing: importers send one object per bridge instead of coordinating three flat field groups and duplicate scalar chords.

**Shipped behavior today (unchanged until implementation):** deck §G, abutments §D, ineffective §H in [`reference/equations.md`](../reference/equations.md); opening remap in [`BRIDGE_INTERIOR_SECTIONS_API.md`](../BRIDGE_INTERIOR_SECTIONS_API.md).

**Parity gap:** [`reference/hecras_parity.md`](../reference/hecras_parity.md) — HEC-RAS *roadway embankment blocking* is not a first-class import path; hosts must derive `bridge_ineffective_*` manually from embankment grades.

---

## Problem (current split API)

Bridge opening geometry is three **independent** field groups, all in the **opening frame** (`s = 0` at left deck edge) but authored separately:

| Concern | Flat fields today | Solver use |
|---------|-------------------|------------|
| Deck / roadway crest | `bridge_deck_*`, `bridge_low_chords`, `bridge_high_chords` | Pressure/weir hydraulics, opening lateral extent |
| Abutments | `bridge_abutment_*` | Subtracted opening area & conveyance (plan integration) |
| Ineffective (BU/BD faces) | `bridge_ineffective_*` (+ per-face overrides) | Conveyance clip on interpolated/explicit BU/BD cuts |

**Host burden**

1. **Alignment** — abutment outer stations must match deck left/right edges unless explicitly offset; ineffective toes must use the same `opening_origin` / anchor as deck and abutments.
2. **Duplication** — scalar `bridge_low_chords` / `bridge_high_chords` must be kept consistent with piecewise `bridge_deck_*` extrema.
3. **HEC-RAS import** — RAS deck editor ties embankment grades to ineffective blocks at bridge faces; hosts re-derive `bridge_ineffective_*` from embankment polylines today.
4. **Per-face asymmetry** — US vs DS ineffective elevations are separate nested arrays even when a single embankment definition could drive both faces with optional overrides.

The engine already **remaps** all three groups when `opening_origin` resolves ([`BRIDGE_INTERIOR_SECTIONS_API.md`](../BRIDGE_INTERIOR_SECTIONS_API.md) preprocessor table). The gap is **authoring semantics**, not runtime frame math.

---

## HEC-RAS mental model

| HEC-RAS deck editor concept | STREAM-1D today | Unified model role |
|-----------------------------|-----------------|-------------------|
| Deck low / high chord profile | `bridge_deck_*` | **`deck`** — authoritative piecewise profile |
| Left / right abutment (width, top) | `bridge_abutment_*` | **`left` / `right`.abutment** — optional nested block |
| Roadway embankment grade (toe → crest) | Host-derived `bridge_ineffective_*` | **`left` / `right`.embankment** — drives ineffective blocks |
| Opening station 0 | `bridge_opening_anchor_*` | Unchanged — not embedded in embankment object |

**Non-goals for 2.3**

- Replace reach `ineffective_flow_areas` on approach/departure cuts (remain reach-frame modifiers).
- Model permanent fill as `blocked_obstructions` inside the unified object (hosts keep explicit BU/BD `blocked_obstructions` for fill under the polyline).
- Change bridge hydraulics equations (composition is **input normalization** only).
- Require hosts to migrate off flat fields (flat fields remain indefinitely).

---

## Proposed API (`API_VERSION` bump on **implementation**, not this design doc)

Add optional per-bridge object on steady, unsteady (`bridge` nested), and rating-curve inputs:

```text
bridge_roadway_embankments: [bridge] → BridgeRoadwayEmbankment | null
```

When `bridge_roadway_embankments[b]` is present, the engine runs **`compose_bridge_opening_geometry`** once per bridge **before** existing deck/abutment/ineffective resolution. Output feeds the **same** internal structs (`BridgeDeckProfile`, `BridgeAbutmentUserInput`, ineffective block lists) used today.

### Type sketch (JSON / TypeScript)

```typescript
/** Opening frame: s=0 left deck edge, increasing rightward (may be negative outside deck). */
interface BridgeRoadwayEmbankment {
  /** Piecewise deck soffit and crest (≥ 2 points). Required when unified object is set. */
  deck: {
    stations: number[];
    low_elevations: number[];
    high_elevations: number[];
  };

  /** Left overbank embankment + optional abutment at the opening. */
  left?: RoadwayEmbankmentSide;
  right?: RoadwayEmbankmentSide;

  /**
   * Per-bridge-face ineffective overrides. When omitted, derived from left/right embankment
   * (same blocks on US and DS). When set, only listed faces are overridden.
   */
  ineffective_faces?: {
    upstream?: BridgeIneffectiveOverride;
    downstream?: BridgeIneffectiveOverride;
  };

  /**
   * When true (default), composed ineffective blocks replace omitted flat ineffective fields.
   * When false, embankment only composes deck + abutment; ineffective must be flat or on BU/BD cuts.
   */
  derive_ineffective?: boolean; // default true
}

interface RoadwayEmbankmentSide {
  /**
   * Embankment ineffective toe(s) in opening frame. Each block: { station, elevation }.
   * Left side: ineffective where x < station when WSEL < elevation (OR across blocks).
   * Right side: ineffective where x > station when WSEL < elevation.
   * Default when omitted: single block at deck edge station with elevation = deck high at that edge.
   */
  ineffective_blocks?: Array<{ station: number; elevation: number }>;

  abutment?: {
    /** Outer-face station (default: deck edge on that side). */
    outer_station?: number;
    /** Width perpendicular to flow (skew-adjusted in solver as today). */
    width: number;
    top_elevation?: number;
    top_profile?: { stations: number[]; elevations: number[] };
  };
}

interface BridgeIneffectiveOverride {
  left_blocks?: Array<{ station: number; elevation: number }>;
  right_blocks?: Array<{ station: number; elevation: number }>;
}
```

**Rating curve** — flattened key `roadway_embankment` (singular) with the same inner shape; discover via `getWasmApiMetadata().bridge_fields.rating_curve_inputs`.

---

## Composition rules

Preprocessor: **`compose_bridge_opening_geometry(embankment, existing_flat, bridge_idx) → ComposedBridgeOpening`**.

All stations remain in **opening frame** until the existing `opening_origin` remap runs (unchanged order).

### 1. Deck → `bridge_deck_*` and scalar chords

| Output field | Rule |
|--------------|------|
| `bridge_deck_stations[b]` | `deck.stations` |
| `bridge_deck_low_elevations[b]` | `deck.low_elevations` |
| `bridge_deck_high_elevations[b]` | `deck.high_elevations` |
| `bridge_low_chords[b]` | `min(deck.low_elevations)` if flat scalar omitted |
| `bridge_high_chords[b]` | `max(deck.high_elevations)` if flat scalar omitted |

If host supplies **both** unified deck and flat `bridge_deck_*`, **flat wins** (see precedence below).

### 2. Abutment → `bridge_abutment_*`

For each side with `abutment` set:

| Output | Rule |
|--------|------|
| `*_widths[b]` | `abutment.width` |
| `*_stations[b]` | `abutment.outer_station ?? deck edge station on that side` |
| `*_top_elevation[b]` | constant top when no profile |
| `*_top_profile_*[b]` | piecewise top when ≥ 2 profile points |

Deck edge stations: `s_left = deck.stations[0]`, `s_right = deck.stations[deck.stations.length - 1]`.

Sides without `abutment` leave corresponding flat abutment fields untouched (not cleared).

### 3. Ineffective → `bridge_ineffective_*`

When `derive_ineffective !== false`:

| Face | Source |
|------|--------|
| Upstream BU | `ineffective_faces.upstream` if set; else composed from `left` / `right` `ineffective_blocks` |
| Downstream BD | `ineffective_faces.downstream` if set; else same as upstream |

**Default blocks** when `ineffective_blocks` omitted on a side but `abutment` or deck exists:

| Side | Default station | Default elevation |
|------|-----------------|-------------------|
| Left | `s_left` | `interpolate(deck, s_left, high)` |
| Right | `s_right` | `interpolate(deck, s_right, high)` |

This matches the common HEC-RAS case: ineffective at the deck edge with activation at roadway crest elevation.

**Multiple blocks** — pass through as nested arrays per existing `bridge_ineffective_*` shape (OR logic unchanged, §H0).

### 4. Opening extent

Validation (`validateSteadyInputs`) should use the **composed** deck + abutment + ineffective toe stations when unified input is present, so embankment toes at `s < 0` or `s > s_right` extend the reported opening span consistently with [`opening_span_opening_frame_user`](../src/solvers/bridge_validation.rs).

---

## Merge precedence (flat vs unified)

Recommended policy — minimizes surprise for existing JSON while giving unified object clear authority for new importers:

| Field group | When `bridge_roadway_embankments[b]` set |
|-------------|------------------------------------------|
| `bridge_deck_*` | Unified fills **only omitted** flat deck arrays; any flat deck array present → **flat wins** for that array |
| `bridge_low_chords` / `bridge_high_chords` | Unified sets scalars **only if** flat scalar is default/zero or omitted |
| `bridge_abutment_*` | Unified fills **per-side** only when that side's flat width is omitted |
| `bridge_ineffective_*` | Unified fills when `derive_ineffective` (default true) and **no** flat ineffective arrays for that face/side |

**Explicit BU/BD `CrossSection.ineffective_flow_areas`** always wins over composed bridge-level ineffective (unchanged v22 resolution order).

Document this table in [`equations.md`](../reference/equations.md) §G/H as **§G2** on implementation.

---

## Processing order (implementation)

```text
1. Parse steady/unsteady JSON
2. For each bridge b with bridge_roadway_embankments[b]:
     compose_bridge_opening_geometry → mutate / overlay flat bridge_* fields per precedence
3. Resolve opening_origin (anchor modes — unchanged)
4. Remap opening-frame deck / abutment / ineffective (unchanged)
5. Existing bridge hydraulics path
```

New module (proposed): `src/solvers/bridge_roadway_compose.rs` with unit tests mirroring `bridge_abutment_hecras.json` cases composed from unified input.

**WASM metadata:** expose `bridge_roadway_embankments` under `bridge_fields.inputs`; add `compose_policy` note in validation warnings when unified + flat conflict (optional warning, non-fatal).

---

## Validation additions

| Check | Severity |
|-------|----------|
| `deck.stations` strictly increasing, ≥ 2 points, aligned array lengths | Error at parse/compose |
| `abutment.top_profile` monotonic stations, ≥ 2 points | Error |
| Abutment width > 0 extends past deck edge into opening | Warning (existing lateral extent check) |
| `ineffective_blocks` station on wrong side of deck edge for left/right | Warning |
| Unified + flat deck both fully specified | Warning: flat deck took precedence |

---

## Implementation outline (follow-on tasks)

- [ ] **`BridgeRoadwayEmbankment`** serde types on `SteadyInputs` / `UnsteadyBridgeInputs` / `BridgeSolveParams`
- [ ] **`compose_bridge_opening_geometry`** + precedence table
- [ ] **Wire** compose step in steady, unsteady, `computeBridgeRatingCurve` ingest
- [ ] **Tests:** round-trip — unified input composes to same hydraulics as hand-authored flat fields for abutment HEC-RAS cases; ineffective default blocks at deck edges
- [ ] **Tests:** `wasm_json_contract` schema snapshot for new field
- [ ] **Docs:** `equations.md` §G2, `wasm_api.types.ts`, `api_changelog` version bump, `hecras_parity.md` row update
- [ ] **Verification (optional):** one HEC-RAS bridge export with roadway embankment → unified JSON → match BU/BD WSEL fixture

---

## Open questions

1. **HEC-RAS embankment outside opening** — Should default ineffective use embankment **toe** polyline (multi-point grade) instead of a single deck-edge block? Defer polyline toes to 2.3.1 unless a verification project requires them at launch.
2. **Blocked vs ineffective on embankment** — RAS sometimes treats low embankment as blocked fill on the approach XS rather than bridge ineffective. Unified model stays ineffective-only; document that hosts map permanent fill to BU/BD `blocked_obstructions`.
3. **Unsteady nested shape** — `bridge_roadway_embankments` as `[bridge]` under `UnsteadyInputs.bridge` (parallel to other `bridge_*` arrays) vs single shared object; recommend **per-bridge array** for consistency.
4. **Default `derive_ineffective`** — `true` may surprise hosts that set unified deck+abutment only; allow `derive_ineffective: false` for deck+abutment-only consolidation.

---

## Checklist (phase 2.3)

- [x] **Design** — unified `bridge_roadway_embankments` schema, composition rules, precedence, processing order (this document)
- [x] **Implement** — `bridge_roadway_compose.rs`; steady/unsteady/rating ingest; embankment profile → ineffective + blocked on BU/BD
- [x] **Test** — compose unit tests; `wasm_json_contract` deserialize/compose; `tests/bridge_roadway_embankment_verification.rs` (typical fill without manual BU/BD `blocked_obstructions`)
- [x] **Document** — equations §G2, `wasm_api.types.ts`, `api_changelog` v26
- [x] **Docs** — migration guide from v19/v20 fields ([`migration_v19_v20_roadway_embankment.md`](migration_v19_v20_roadway_embankment.md))
