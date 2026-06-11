# 3.2 Pier footings, nosing, fender/wing walls — API design

Plan-view and vertical pier attachments beyond the shaft prism ([`pier_tapered_width.md`](pier_tapered_width.md) / API v27). **§A footing and §B nosing implemented** (API v28); §C plan polygons and §D wing walls remain design-only.

**Today (v27):** pier shaft obstruction is integrated from perpendicular width vs elevation (`PierWidthSpec` in [`pier_geometry.rs`](../../src/solvers/pier_geometry.rs)). Pier **nose shape** for Yarnell $K$ and momentum $C_D$ is a scalar per bridge via `bridge_pier_shapes` (square, semicircular, …). There is no plan-view extension (nosing), no explicit footing block below the shaft toe, and no pier-attached polygon fill.

**Gap:** Real piers often have **wider footings / pile caps** below the column, **upstream nosing** or cutwaters, **fender/wing walls** in plan, and occasional **soffit / pile** obstructions under the deck. Hosts today encode footings as extra points in `bridge_pier_width_elevations` / `bridge_pier_width_values` (valid but easy to mis-author) or duplicate geometry on BU/BD `blocked_obstructions` (breaks pier-loss accounting).

---

## HEC-RAS mental model

| HEC-RAS concept | STREAM-1D today | 3.2 addition |
|-----------------|-----------------|--------------|
| Pier elevation–width table (footing flare as width step) | `bridge_pier_width_elevations` / `_values` (v27) | Optional **footing shorthand** (§A) or keep profile-only |
| Pier nose shape (Yarnell / momentum $K$, $C_D$) | `bridge_pier_shapes` | unchanged — **not** the same as geometric nosing length |
| Pier area below ground clipped at invert | `z_bed` clip in `PierWidthSpec` | explicit **footing toe** elevation optional |
| Wing walls at bridge opening (WSPRO) | — | **Bridge-level** wing walls (§D), not per-pier |
| Fill / blockage under embankment polyline | `blocked_obstructions` on BU/BD cuts; roadway embankment compose (v26) | **Pier-local** plan polygons (§C) composed into pier plan area |

HEC-RAS 1D does **not** expose separate “footing elevation” or “nosing length” fields — footing and haunch are modeled as **width changes at elevations** in the pier table. STREAM-1D 3.2 adds **convenience fields** and **plan-view attachments** that compile into the same integrated $A_{pier}(WSEL)$ used by Yarnell, momentum, and pressure. Full editor field mapping and gaps: [`hecras_parity.md`](../reference/hecras_parity.md) § Bridge pier editor.

---

## Scope split

| Feature | Attached to | Primary effect |
|---------|-------------|----------------|
| **Footing** (§A) | Pier shaft, vertical | Wider submerged **plan area** below shaft base |
| **Nosing** (§B) | Pier shaft, plan (flow-normal upstream) | Extra **opening-plane width** at and above bed |
| **Plan polygon** (§C) | Pier (fenders, wings, pile cap bulge in plan) | Arbitrary **plan obstruction** vs WSEL |
| **Soffit / below-deck polygon** (§C) | Pier, vertical band under low chord | Blockage between bed and deck soffit |
| **Bridge wing walls** (§D) | Bridge opening (abutment side) | WSPRO / contraction losses — **not** pier Yarnell $\alpha$ |

§A–§C are **per pier** (`[bridge][pier]` aligned with `bridge_pier_stations`). §D is **per bridge**.

---

## Proposed fields (steady / unsteady)

Per bridge `b`, per pier `i` (same indexing as [`pier_tapered_width.md`](pier_tapered_width.md)).

### §A — Footing shorthand (optional)

Use when the host knows footing top elevation and width but does not want to author a full width profile.

| Field | Shape | Description |
|-------|-------|-------------|
| `bridge_pier_footing_top_elevations` | `[bridge][pier]` | Absolute elevation of **top of footing** (bottom of pier shaft). Default: resolved pier base (`bridge_pier_base_elevations`, bed, or profile lowest point). |
| `bridge_pier_footing_widths` | `[bridge][pier]` | Footing width **perpendicular to flow** at `footing_top` (typically ≥ shaft bottom width). |
| `bridge_pier_footing_bottom_elevations` | `[bridge][pier]` | Optional toe elevation below footing top. Default: `min(bed, footing_top − ε)`. |

**Compose rule (implementation):** expand into two or three profile points appended to (or merged with) the pier width spec:

```text
… → (z_footing_bottom, W_footing) → (z_footing_top, W_footing) → (z_shaft_base, W_shaft_bottom) → … shaft …
```

When `bridge_pier_width_elevations` / `_values` already define points below the shaft, **profile wins**; footing shorthand applies only if no profile point exists below `footing_top`.

### §B — Nosing (plan extension, optional)

Upstream (flow-normal) extension of the pier nose in **plan view**, added to opening-plane width at WSEL when the pier is wet.

| Field | Shape | Description |
|-------|-------|-------------|
| `bridge_pier_nosing_lengths` | `[bridge][pier]` | Length **perpendicular to flow** from pier centerline upstream (−flow direction). 0 = omit. |
| `bridge_pier_nosing_widths` | `[bridge][pier]` | Optional width of the nosing block perpendicular to flow. Default: shaft `width_perp_at(WSEL)` at evaluation depth. |

**Compose rule:** at each WSEL, effective plan half-width upstream adds `L_nosing / cos θ` in opening coordinates (same skew convention as §I in [`equations.md`](../reference/equations.md)). Nosing contributes to $A_{pier}$ as a rectangular or trapezoidal plan appendage attached to the upstream face of the integrated shaft area — not a change to Yarnell $K$ (shape coefficient remains from `bridge_pier_shapes`).

### §C — Pier-attached polygon obstructions (optional)

For fender walls, wing walls tied to a pier, hammerhead extensions, or soffit/pile clutter that are not well modeled by §A/§B alone.

| Field | Shape | Description |
|-------|-------|-------------|
| `bridge_pier_plan_obstruction_stations` | `[bridge][pier][poly][point]` | Opening-frame **lateral** coordinates. Either **absolute** opening `s` (default, same frame as `bridge_pier_stations`) **or** offset from pier center when `bridge_pier_plan_obstruction_relative` is true (see below). |
| `bridge_pier_plan_obstruction_elevations` | `[bridge][pier][poly][point]` | Absolute elevation (user units) at each plan vertex. |
| `bridge_pier_plan_obstruction_relative` | `[bridge][pier]` | Optional bool per pier. `true`: stations are **offsets** from pier centerline station (negative = left / upstream-left in opening frame). Default `false` (absolute opening `s`). |

Each `[poly]` is a closed or open polyline in the **opening vertical plane** (lateral × elevation):

- **Plan fender / wing:** constant elevation band → horizontal segment in `(s, z)`.
- **Soffit / pile below deck:** polygon spanning `z` from bed (or footing toe) to `bridge_low_chords` at pier station.
- **Below-soffit only:** vertices with `z` ≤ low chord at that pier; pier shaft profile still defines Yarnell shaft; polygon adds incremental $\Delta A_{pier}(WSEL)$ via horizontal slice integration (same trapezoid rules as `blocked_obstructions` crest clipping).

Multiple `[poly]` per pier are summed. Overlap with shaft §A/§B: **union** of submerged plan area (avoid double-count — implementation merges polygons before integration).

**Alternative (hosts that already author BU/BD fill):** continue using `blocked_obstructions` on explicit BU/BD cuts for reach-wide fill; §C is for **pier-indexed** geometry that must move with `bridge_pier_stations` and `opening_origin` remap.

### §D — Bridge wing walls (opening-level, optional)

HEC-RAS **WSPRO / opening** wing walls — not pier Yarnell blockage. Separate from §B fender geometry.

| Field | Shape | Description |
|-------|-------|-------------|
| `bridge_wing_wall_types` | `[bridge]` | `0` = none, `1` = angular, `2` = rounded (HEC-RAS enum). |
| `bridge_wing_wall_lengths` | `[bridge]` | Flow-normal wing length (user units). |
| `bridge_wing_wall_angles` | `[bridge]` | Degrees for angular type (HEC-RAS “angle of wing wall”). |
| `bridge_wing_wall_entrance_radii` | `[bridge]` | Entrance rounding radius for rounded type. |

**Solver use (later):** WSPRO / contracted-opening energy and optional contraction losses — see HEC-RAS appendix D. Does **not** add to pier $A_{pier}$ in Yarnell.

---

## Precedence (per pier)

1. **Width profile** (`bridge_pier_width_elevations` / `_values`) — authoritative shaft + optional footing steps if authored explicitly (v27).
2. **Footing shorthand** (§A) — inserts footing points only where profile does not already cover `[z_footing_bottom, z_footing_top]`.
3. **Top/bottom pair** / legacy width (v27) — shaft only; §A may append footing below.
4. **Plan polygon** (§C) — adds $\Delta A$ at each WSEL; merged with shaft + nosing (§B).
5. **Nosing** (§B) — plan appendage; applied after shaft width at WSEL is known.

Bridge wing walls (§D) are independent of pier list.

---

## Coordinate frames

| Axis | Frame |
|------|--------|
| Horizontal (§A, §C absolute) | Opening `s` — same as `bridge_pier_stations` / deck (§I) |
| Horizontal (§C relative) | Offset from pier centerline in opening `s` |
| Horizontal (§B) | Flow-normal upstream; converted to opening plane via skew |
| Vertical | Absolute elevation (metric or US), same as deck and v27 pier widths |
| Width / length | **Perpendicular to flow** unless noted; skew: $L' = L/\cos\theta$, $W' = W/\cos\theta$ |

Preprocessor: §C absolute stations remap with `opening_origin` like other opening-frame pier fields ([`BRIDGE_INTERIOR_SECTIONS_API.md`](../BRIDGE_INTERIOR_SECTIONS_API.md) §1.3.2).

---

## Rating curve (`computeBridgeRatingCurve`)

Flattened keys on `BridgeSolveParams` (no `bridge_` prefix):

| Steady / unsteady | Rating curve |
|-------------------|--------------|
| `bridge_pier_footing_top_elevations` | `pier_footing_top_elevations` |
| `bridge_pier_footing_widths` | `pier_footing_widths` |
| `bridge_pier_footing_bottom_elevations` | `pier_footing_bottom_elevations` |
| `bridge_pier_nosing_lengths` | `pier_nosing_lengths` |
| `bridge_pier_nosing_widths` | `pier_nosing_widths` |
| `bridge_pier_plan_obstruction_stations` | `pier_plan_obstruction_stations` |
| `bridge_pier_plan_obstruction_elevations` | `pier_plan_obstruction_elevations` |
| `bridge_pier_plan_obstruction_relative` | `pier_plan_obstruction_relative` |
| `bridge_wing_wall_*` | `wing_wall_*` (same suffix) |

Existing pier keys from v27 unchanged.

---

## Planned solver use (implementation — not in 3.2 API doc)

Extend `ResolvedPier` / `PierWidthSpec` (or sibling `PierAttachments`) so `total_submerged_pier_area_m2` returns:

$$A_{pier}(WSEL) = A_{shaft}(WSEL) + A_{nosing}(WSEL) + A_{polygons}(WSEL)$$

with $A_{shaft}$ from v27 integration (including composed footing shorthand). Use in:

- `obstructed_hydraulics`, `net_opening_area_at_low_chord`, Yarnell $\alpha$, momentum drag plan area
- Unchanged: $K$ and $C_D$ from `PierShape` / `bridge_pier_shapes`

§D wing walls feed WSPRO / energy contraction terms only.

---

## JSON examples

**Footing shorthand (pier 0, one bridge):** shaft 1.0 m → 2.0 m taper (v27) plus 3.0 m footing 0.5 m tall:

```json
"bridge_pier_stations": [[5.0]],
"bridge_pier_top_widths": [[1.0]],
"bridge_pier_bottom_widths": [[2.0]],
"bridge_pier_footing_top_elevations": [[98.0]],
"bridge_pier_footing_widths": [[3.0]],
"bridge_pier_footing_bottom_elevations": [[97.5]]
```

**Nosing + square shaft:**

```json
"bridge_pier_stations": [[5.0]],
"bridge_pier_widths": [1.5],
"bridge_pier_nosing_lengths": [[0.75]],
"bridge_pier_shapes": [0]
```

**Pier fender polygon (absolute opening stations, trapezoid wing):**

```json
"bridge_pier_stations": [[10.0]],
"bridge_pier_plan_obstruction_stations": [[[8.0, 8.0, 12.0, 12.0]]],
"bridge_pier_plan_obstruction_elevations": [[[100.0, 103.0, 103.0, 100.0]]]
```

**Relative stations (±2 m from pier center at `s=10`):**

```json
"bridge_pier_stations": [[10.0]],
"bridge_pier_plan_obstruction_relative": [true],
"bridge_pier_plan_obstruction_stations": [[[-2.0, -2.0, 2.0, 2.0]]],
"bridge_pier_plan_obstruction_elevations": [[[99.0, 102.0, 102.0, 99.0]]]
```

**Bridge angular wing walls (WSPRO):**

```json
"bridge_wing_wall_types": [1],
"bridge_wing_wall_lengths": [12.0],
"bridge_wing_wall_angles": [45.0]
```

---

## Validation (implementation)

| Check | Severity |
|-------|----------|
| Pier index aligns with `bridge_pier_stations` / `num_piers` | Error |
| `footing_width` > 0; `footing_top` > `footing_bottom` | Error |
| Footing top above shaft base (when both known) | Warning |
| `nosing_length` ≥ 0 | Error |
| Plan polygon ≥ 3 points; elevations monotonic in closed rings | Error |
| Polygon entirely below bed or above high chord | Warning (no effect) |
| §A + profile both define same elevation band | Warning: profile wins |
| §C polygon + BU `blocked_obstructions` overlap | Warning (possible double blockage) |

---

## Non-goals (3.2)

- Separate US/DS pier footing or nosing (single definition per pier, as v27 widths).
- 3D pier mesh or CFD nose pressure distribution.
- Floating debris (HEC-RAS pier editor option) — future phase.
- Replacing `bridge_pier_shapes` with geometric nose CAD.
- Nested pier object in JSON (flat arrays match existing pier fields).

---

## Checklist

- [x] **API** — footing elevation/width shorthand, nosing length, pier plan polygon, bridge wing walls (this document)
- [x] **Types** — serde on `SteadyInputs` / `UnsteadyBridgeInputs` / `BridgeSolveParams`; `wasm_api.types.ts` (API v28)
- [x] **Resolve** — compose footing into profile; `PierAttachmentsUserInput` / `ResolvedPierNosing` on `ResolvedPier` (plan polygons §C pending)
- [x] **Geometry** — footing area below shaft base; nosing adds plan area and flow width (drag / contraction)
- [ ] **Hydraulics** — polygon slice integration; wing walls → WSPRO (nosing $A$ wired via `submerged_area_m2`)
- [x] **Solver** — composed area/width through `obstructed_hydraulics` → Yarnell / momentum / pressure
- [x] **Tests** (§A–§B implemented scope)
  - [x] footing+nosing unit area (`footing_adds_area_below_shaft_base`, `nosing_adds_submerged_area_and_flow_width`, explicit nosing width, skew flow width)
  - [x] attachment extractors + serde (`pier_attachments_user_for_bridge_index`, `pier_attachments_from_rating_params`)
  - [x] **Submerged footing reduces opening area** — `test_submerged_footing_reduces_opening_area` (`obstructed_hydraulics` `a_eff` vs hand calc)
  - [x] nosing contraction (`test_nosing_reduces_obstructed_top_width`) and Yarnell (`test_yarnell_integrated_loss_increases_with_footing`)
  - [x] steady HW (`test_footing_nosing_exceed_shaft_only_headwater_in_solve`, `wasm_footing_nosing_rating_hw_exceeds_shaft_only`)
  - [x] WASM deserialize + solve (`wasm_pier_footing_nosing_deserialize_and_solve`; metadata in `wasm_api_metadata_version`)
  - [ ] polygon vs hand calc (§C — not implemented)
  - [ ] wing wall WSPRO smoke (§D — not implemented)
- [x] **Docs** (aggregate)
  - [x] `api_changelog` v28, `equations.md` §J3, README index
  - [x] **Scope vs HEC-RAS pier editor** — [`hecras_parity.md`](../reference/hecras_parity.md) § Bridge pier editor
