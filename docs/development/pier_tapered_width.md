# 3.1 Tapered piers — API design

Elevation-varying pier width for bridge openings (HEC-RAS tapered-pier analogue). **API v27** — integrated submerged pier area in Yarnell, momentum, and pressure solvers.

**Today:** one constant `bridge_pier_widths[b]` (perpendicular to flow) × depth → rectangular submerged pier block in [`bridge.rs`](../../src/solvers/bridge.rs) (`pier_submerged_area_geom`, Yarnell `A_piers`). Horizontal placement: `bridge_pier_stations[b][i]` in opening frame (§I in [`equations.md`](../reference/equations.md)).

**Gap:** HEC-RAS pier editor supports **top and bottom width** (and shape) per pier; STREAM-1D treats all piers as prisms with one width.

---

## HEC-RAS mental model

| RAS pier field | STREAM-1D today | 3.1 addition |
|----------------|-----------------|--------------|
| Pier centerline station | `bridge_pier_stations` | unchanged |
| Pier shape (square, round, …) | `bridge_pier_shapes` | unchanged |
| Width at invert / ground | implicit = `bridge_pier_widths` | `bridge_pier_bottom_widths` |
| Width at deck soffit (low chord) | same as bottom | `bridge_pier_top_widths` |
| Multi-point width curve | — | optional width profile |

Full HEC-RAS pier editor mapping (v27–v28): [`hecras_parity.md`](../reference/hecras_parity.md) § Bridge pier editor.

---

## Proposed fields (steady / unsteady)

Per bridge `b`, per pier `i` (index aligns with `bridge_pier_stations[b]` when explicit; else `i = 0..num_piers-1` in auto-spaced order).

### Option A — top/bottom pair (recommended default for importers)

| Field | Shape | Description |
|-------|-------|-------------|
| `bridge_pier_top_widths` | `[bridge][pier]` | Width **perpendicular to flow** at pier cap (typically low-chord / deck-soffit elevation) |
| `bridge_pier_bottom_widths` | `[bridge][pier]` | Width at pier base (typically BU/BD invert or user-specified toe elevation) |

When only `bridge_pier_widths[b]` is set (legacy), top = bottom = that width for every pier.

### Option B — piecewise width profile (full taper / haunch)

| Field | Shape | Description |
|-------|-------|-------------|
| `bridge_pier_width_elevations` | `[bridge][pier][point]` | Absolute elevation (user units), strictly increasing |
| `bridge_pier_width_values` | `[bridge][pier][point]` | Perpendicular width at each elevation |

≥ 2 points per pier; linear segments between points (same pattern as `bridge_abutment_*_top_profile_*`).

### Optional pier cap / base anchors (only if profile omitted)

| Field | Shape | Description |
|-------|-------|-------------|
| `bridge_pier_top_elevations` | `[bridge][pier]` | Elevation of top width (default: bridge low chord at pier station from deck profile) |
| `bridge_pier_base_elevations` | `[bridge][pier]` | Elevation of bottom width (default: min bed on BU/BD at solve) |

Omit when using explicit width profiles (elevations embedded in profile).

---

## Precedence (per pier)

1. **Width profile** (`width_elevations` + `width_values`) when valid (≥ 2 points, aligned lengths).
2. Else **top + bottom pair** when both widths are present (linear taper between resolved top/base elevations).
3. Else **legacy** scalar `bridge_pier_widths[b]` applied to every pier (constant prism).

Per-pier overrides within a bridge are allowed (mixed legacy + tapered in one opening).

---

## Coordinate frames

| Axis | Frame |
|------|--------|
| Horizontal | Opening `s` — same as `bridge_pier_stations` / `bridge_deck_stations` (§I) |
| Vertical | **Absolute elevation** in user units (metric or US), same as deck and abutment profiles |
| Width | **Perpendicular to flow**; skew converts to opening-plane width: $W' = W/\cos\theta$ ([`equations.md`](../reference/equations.md) §I) |

---

## Rating curve (`computeBridgeRatingCurve`)

Flattened keys on `BridgeSolveParams` (no `bridge_` prefix):

| Steady / unsteady | Rating curve |
|-------------------|--------------|
| `bridge_pier_top_widths` | `pier_top_widths` |
| `bridge_pier_bottom_widths` | `pier_bottom_widths` |
| `bridge_pier_width_elevations` | `pier_width_elevations` |
| `bridge_pier_width_values` | `pier_width_values` |
| `bridge_pier_top_elevations` | `pier_top_elevations` |
| `bridge_pier_base_elevations` | `pier_base_elevations` |

Existing `pier_width`, `pier_stations`, `num_piers`, `pier_shape_type` unchanged.

---

## Planned solver use (implementation — not in 3.1)

Replace constant `pier_width_m × depth` with per-pier **submerged plan area**:

$$A_{pier}(WSEL) = \sum_i \int_{z_{base,i}}^{\min(WSEL,\, z_{top,i})} w_i(z)\, dz$$

with $w_i(z)$ from profile or linear top/bottom pair. Use:

- **Obstructed area / conveyance** — $A_{piers}$ at evaluation WSEL (`obstructed_hydraulics`, pressure-orifice area).
- **Top width** — sum of $w_i(WSEL)$ for piers with wet tops (not constant `effective_pier_width_m`).
- **Yarnell** — same integrated $A_{piers}$ for $\alpha$ (not `num_piers × width × depth`).
- **Momentum drag** — unchanged $C_D$ per shape; blockage via updated $A_{eff}$.

Pier shape (`PierShape`) still selects Yarnell $K$ and drag $C_D$; taper affects **plan area only** (HEC-RAS first-order parity).

---

## JSON examples

**Legacy (unchanged):**

```json
"bridge_pier_widths": [1.5],
"bridge_num_piers": [2],
"bridge_pier_shapes": [0]
```

**Tapered pair (two piers, bridge 0):**

```json
"bridge_pier_stations": [[3.0, 7.0]],
"bridge_pier_top_widths": [[1.0, 1.0]],
"bridge_pier_bottom_widths": [[2.0, 2.0]],
"bridge_pier_shapes": [0]
```

**Profile (pier 0):**

```json
"bridge_pier_width_elevations": [[[98.0, 102.0, 105.0]]],
"bridge_pier_width_values": [[[2.2, 1.6, 1.0]]]
```

---

## Validation (implementation)

| Check | Severity |
|-------|----------|
| Profile elevations strictly increasing; widths > 0 | Error |
| `top_widths` / `bottom_widths` length matches pier count | Error |
| Top elevation ≤ low chord; base ≥ bed (when resolvable) | Warning |
| Profile + top/bottom both set for same pier | Warning: profile wins |

---

## Non-goals (3.1)

- Pier shaft **shape** beyond plan-width taper (round vs square still via `bridge_pier_shapes`).
- Separate US/DS pier widths (single definition per pier).
- Pier groups as nested object (flat arrays match existing `bridge_pier_stations`).

---

## Checklist

- [x] **API** — pier width table vs elevation or top/bottom width pair per pier (this document)
- [x] **Types** — serde on `SteadyInputs` / `UnsteadyBridgeInputs` / `BridgeSolveParams`; `wasm_api.types.ts`
- [x] **Resolve** — build per-pier `PierWidthSpec` in `BridgeGeometry` ([`pier_geometry.rs`](../../src/solvers/pier_geometry.rs), `build_bridge_geometry`)
- [x] **Hydraulics** — integrated submerged area, top width, Yarnell α (`pier_submerged_area_at_wsel`, `yarnell_pier_head_loss_integrated`)
- [x] **Solver** — Yarnell / momentum / pressure net area use tapered obstruction (`obstructed_hydraulics`, `net_opening_area_at_low_chord`, `pier_drag_momentum_with_table`, `BridgeSectionContext.pier_widths`)
- [x] **Tests** — trapezoid area unit tests; tapered vs rectangular WSEL; tapered vs mean constant width; profile precedence/resolve; skew; steady/unsteady/rating WASM contract; tapered HW > legacy constant
- [x] **Docs** — `api_changelog` v27, README, `equations.md` §J2; pier editor scope in [`hecras_parity.md`](../reference/hecras_parity.md) § Bridge pier editor

**Next:** pier footings, nosing, plan polygons — [`pier_footings_nosing.md`](pier_footings_nosing.md) (3.2).
