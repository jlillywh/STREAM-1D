# Bridge interior cross sections (API v22) — design

HEC-RAS models bridge hydraulics with dedicated cross-section cuts at the bridge faces and optional interior cuts between deck limits. STREAM-1D phase **1.2** adds optional explicit **BU** (bridge upstream), **BD** (bridge downstream), and **internal** sections, plus a station-alignment field linking bridge opening coordinates to reach cross-section lateral stations.

## HEC-RAS concepts

| Cut | HEC-RAS name | Role |
|-----|--------------|------|
| Reach XS immediately upstream of structure | Approach | Standard reach geometry before the bridge |
| **BU** | Bridge upstream face | Opening geometry at the US face (piers, abutments, deck soffit encoded in the cut or via bridge fields) |
| **Internal** | Bridge interior | Optional cuts between BU and BD (multi-span, variable deck) |
| **BD** | Bridge downstream face | Opening geometry at the DS face |
| Reach XS immediately downstream | Departure | Standard reach geometry after the bridge |

Today (pre-v22), the steady/unsteady solvers use **reach interval geometry** — the densified cross sections bracketing `bridge_stations[i]` — for bridge area/conveyance. That matches simple models but diverges from HEC-RAS when BU/BD cuts differ from the adjacent reach polylines.

## Coordinate frames

Two horizontal frames are in play:

1. **Reach XS stations** — `CrossSection.x` lateral coordinates on any reach or bridge cut (feet/meters, user units).
2. **Bridge opening stations** — origin at the **left edge of the deck opening**, increasing rightward. Used by `bridge_deck_*`, `bridge_pier_stations`, `bridge_abutment_*`, and ineffective blocks.

**Alignment** maps opening station `s` to reach XS lateral coordinate:

```text
reach_x = bridge_opening_reach_station_origins[bridge] + s
```

When `bridge_opening_reach_station_origins` is omitted, the engine infers the origin as `min(CrossSection.x)` on the resolved BU section (explicit BU or reach US fallback). If no XS is available, opening coordinates are treated as identical to reach `x` (legacy behavior).

## WASM / JSON fields (API v22)

Parallel per-bridge arrays on **`SteadyInputs`**, nested under **`UnsteadyInputs.bridge`**, and discoverable via `getWasmApiMetadata().bridge_fields.inputs`.

| Field | Shape | Description |
|-------|-------|-------------|
| `bridge_upstream_cross_sections` | `[bridge]` → `CrossSection` | **BU** cut. Overrides reach US interval geometry for bridge hydraulics when present. |
| `bridge_downstream_cross_sections` | `[bridge]` → `CrossSection` | **BD** cut. Overrides reach DS interval geometry. |
| `bridge_internal_cross_sections` | `[bridge][section]` → `CrossSection` | Optional interior cuts, ordered **US → DS**. Stored for metadata and future multi-segment hydraulics; **not yet used in the solver Jacobian** (phase 1.3+). |
| `bridge_opening_reach_station_origins` | `[bridge]` | Reach `x` at opening station 0. Omit to infer from BU `min(x)`. |

`CrossSection.station` on BU/BD/internal cuts is **informational** (bridge face reach station); hydraulics use the polyline (`x`, `y`, `n_*`, `blocked_obstructions`, `is_overbank`, `ineffective_flow_areas`).

### Ineffective flow on BU/BD cuts

Each `CrossSection` may carry **`ineffective_flow_areas`** (`left_blocks` / `right_blocks` with reach lateral `station` and activation `elevation`). When an explicit BU or BD cut is supplied:

| Source | Used for bridge hydraulics |
|--------|----------------------------|
| `ineffective_flow_areas` on the explicit BU/BD cut | ✓ (reach-`x` frame on that cut) |
| Ineffective on the adjacent reach face | ✗ (not inherited) |
| `bridge_ineffective_*` opening-frame fields | ✓ only when the explicit cut omits `ineffective_flow_areas` (stations shifted by `bridge_opening_reach_station_origins`) |

When BU/BD are omitted (reach interval fallback), ineffective resolution uses reach `ineffective_flow_areas` first, then opening-frame `bridge_ineffective_*`.

### Rating curve (`computeBridgeRatingCurve`)

Existing flattened keys already accept explicit face geometry:

| Key | Maps to |
|-----|---------|
| `xs_up` | BU |
| `xs_down` | BD |

**v22 adds:**

| Key | Description |
|-----|-------------|
| `opening_reach_station_origin` | Same as `bridge_opening_reach_station_origins` for a single bridge |
| `xs_internal` | Optional interior cuts (stored; solver uses BU/BD only today) |

When `xs_up` / `xs_down` are omitted, the rating-curve API still defaults to rectangular channels (`channel_width`, `z_up`, `z_down`).

## Reach layout (densification)

After base reach densification (`max_spacing`), the engine inserts densified nodes at resolved **BU**, **BD**, and **internal** river stations before profile routing:

1. **Face stations** — `resolve_bridge_face_stations_metric`:
   - Both BU and BD `CrossSection.station` when explicit sections are provided
   - Else `bridge_stations[b] ± bridge_lengths[b]/2` when `bridge_lengths[b] > 0`
   - Else `bridge_stations[b]` for both faces (legacy center-station interval)
2. **Insertion** — `insert_reach_layout_cuts` adds nodes at those stations (or updates geometry when a node already exists). Explicit BU/BD polylines replace interpolated tables at the face nodes.
3. **Interval indexing** — bridge hydraulics run on interval `i` where `densified_stations[i] == BU` and `densified_stations[i+1] == BD` (not the wider reach interval containing the bridge center).

## Resolution rules

For bridge index `b` at reach interval `(i, i+1)` where `stations[i]` / `stations[i+1]` are BU / BD:

```
BU  ← bridge_upstream_cross_sections[b]  ?? reach_xs[i]  ?? table-only fallback
BD  ← bridge_downstream_cross_sections[b] ?? reach_xs[i+1] ?? table-only fallback
origin ← bridge_opening_reach_station_origins[b] ?? min(BU.x) ?? None
internal ← bridge_internal_cross_sections[b] ?? []
```

Geometry tables for the bridge solve are regenerated from BU/BD polylines when explicit sections are supplied (`num_slices` from inputs). Bed elevations `z_up` / `z_down` use the minimum ground elevation of the resolved BU/BD polylines.

**Backward compatibility:** omitting all v22 fields preserves pre-v22 behavior (reach interval tables and beds).

## Solver usage (v22)

| Solver | BU/BD used for | Internal sections |
|--------|----------------|-------------------|
| `solve_steady` | `solve_bridge_wsel` / `solve_bridge_tailwater` area, conveyance, ineffective integration | Carried in `BridgeSectionContext`; not in head-loss integration yet |
| `solve_unsteady` | Post-step `solve_bridge_coupled` | Same |
| `computeBridgeRatingCurve` | `xs_up` / `xs_down` params | `xs_internal` stored only |

### HEC-RAS hydraulics weighting (BU / BD)

| Calculation | BU | BD | Rule |
|-------------|----|----|------|
| Low-flow **Class A/B/C** | ✓ | ✓ | Critical specific force and Class B control use the **more constricted** face (max $M_{crit}$); tailwater force on BD |
| **Yarnell** | — | ✓ | Pier loss on downstream (BD) opening area |
| **Momentum** (Class A/B/C) | ✓ | ✓ | Upstream force on BU; downstream on BD; pier drag on BU |
| **Energy / WSPRO** | ✓ | ✓ | Approach on BU; departure on BD; **contracted opening** = min obstructed area at opening elevation |
| **Pressure / orifice** | ✓ | ✓ | Net opening at low chord = **min(BU, BD)**; sluice opening height = min vertical gap below deck |
| **Weir overtopping** | ✓ | — | Upstream energy grade on BU; effective weir length from deck profile |
| **Friction** ($h_f = L (Q/\bar K)^2$) | ✓ | ✓ | $L$ = sum of reach segments BU → internal cuts → BD; $\bar K = (K_{BU} + K_{BD})/2$ at respective WSELs; skew applies $L' = L/\cos\theta$ |

## Host application mapping (e.g. stream1d.com / HEC-RAS import)

1. Import or author BU/BD polylines from HEC-RAS bridge editor cuts.
2. Set `bridge_opening_reach_station_origins[b]` so deck/pier/abutment opening stations align with BU `x` (typically left deck edge on the cut).
3. Pass BU/BD as full `CrossSection` objects (including `blocked_obstructions` and `ineffective_flow_areas` on the cut when they differ from the approach/departure reach).
4. Optional: attach interior cuts under `bridge_internal_cross_sections[b]` for future multi-segment routing.

## Phase roadmap

| Phase | Scope |
|-------|--------|
| **1.2** (this) | API fields, resolution, BU/BD wired into bridge geometry tables, alignment metadata, docs/tests |
| **1.3** | Multi-segment standard step through internal cuts; per-segment friction conveyance weighting |

## Verification

| Test | File |
|------|------|
| HEC-RAS Yarnell (legacy center) + explicit BU/BD + WSPRO narrow opening | `tests/bridge_bu_bd_hecras_verification.rs` + `python/verification/bridge_bu_bd_hecras.json` |
| 3-section reach (BU + internal + BD) vs 2-face baseline | `three_section_bridge_reach_matches_two_face_baseline` in same file |
| **1.4** | Remap opening-frame ineffective/pier/deck onto BU/BD when origins differ from legacy assumption |

## Example (steady JSON excerpt)

```json
{
  "bridge_stations": [500.0],
  "bridge_low_chords": [12.0],
  "bridge_high_chords": [15.0],
  "bridge_opening_reach_station_origins": [100.0],
  "bridge_upstream_cross_sections": [{
    "station": 500.0,
    "x": [100.0, 100.0, 130.0, 130.0],
    "y": [20.0, 8.0, 8.0, 20.0],
    "n_stations": [0.0],
    "n_values": [0.035],
    "unit_system": "Metric"
  }],
  "bridge_downstream_cross_sections": [{
    "station": 498.0,
    "x": [100.0, 100.0, 130.0, 130.0],
    "y": [19.5, 7.5, 7.5, 19.5],
    "n_stations": [0.0],
    "n_values": [0.035],
    "unit_system": "Metric"
  }]
}
```

Pier at opening station `15` → reach `x = 115` when `bridge_opening_reach_station_origins[0] = 100`.
