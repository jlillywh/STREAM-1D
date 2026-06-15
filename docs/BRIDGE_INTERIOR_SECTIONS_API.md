# Bridge interior cross sections (API v22+)

Explicit **BU** (bridge upstream), **BD** (bridge downstream), and optional **internal** cuts. When omitted, bridge hydraulics use reach interval geometry at `bridge_stations`.

## Coordinate frames

| Frame | Used for |
|-------|----------|
| Reach lateral `x` | `CrossSection.x` on reach / BU / BD cuts; `ineffective_flow_areas` on those cuts |
| Opening `s` | `bridge_deck_*`, `bridge_pier_stations`, `bridge_abutment_*`, `bridge_ineffective_*` — `s = 0` at left deck edge |

Mapping: `reach_x = opening_origin + s`. Default `opening_origin = min(BU.x)`.

### Opening anchor

| `bridge_opening_anchor_modes[b]` | Origin |
|------------------------------------|--------|
| `0` (default) | `min(BU.x)` |
| `1` | `min(x)` on reach XS at `bridge_opening_anchor_reach_stations[b]` |
| `2` | `bridge_opening_reach_station_origins[b]` (explicit; always wins when set) |

Skew (`bridge_skew_angles`) scales perpendicular widths and friction length; opening stations stay in opening frame.

## Key fields

| Field | Role |
|-------|------|
| `bridge_upstream_cross_sections` | BU cut polyline |
| `bridge_downstream_cross_sections` | BD cut |
| `bridge_internal_cross_sections` | Optional interior cuts (layout + friction length; hydraulics BU→BD) |
| `bridge_opening_reach_station_origins` | Reach `x` at opening `s = 0` |
| `bridge_opening_anchor_modes`, `bridge_opening_anchor_reach_stations` | Anchor without manual offset |

On BU/BD: use reach-frame `ineffective_flow_areas` **or** opening-frame `bridge_ineffective_*` (shifted by origin) — not both for the same effect. Semantics: [`equations.md`](reference/equations.md) §H0, §H1.

## Reach layout

After `max_spacing` densification, the engine inserts nodes at BU, BD, and internal river stations. Bridge coupling runs on the **BU→BD** interval (not the wider interval around bridge center).

## Rating curve

Flattened keys: `xs_up` (BU), `xs_down` (BD), `opening_reach_station_origin`, `xs_internal`. Reverse $Q$: see [`bridge_extensions.md`](development/bridge_extensions.md).

## Validation

`validateSteadyInputs` may warn when opening lateral extent exceeds parent BU/approach `x` range.

Fixture: [`examples/wasm/steady_bridge_bu_bd_v22.json`](../examples/wasm/steady_bridge_bu_bd_v22.json). Types: [`wasm_api.types.ts`](wasm_api.types.ts).
