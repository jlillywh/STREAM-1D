# Bridge extensions (pier, deck, ice, reverse flow)

Optional bridge fields beyond core Class A/B/C and high-flow pressure/weir. Field names: [`wasm_api.types.ts`](../wasm_api.types.ts). Core theory: [`equations.md`](../reference/equations.md) §6.

## Pier geometry (v27–v28)

| Need | Fields |
|------|--------|
| Legacy prism | `bridge_pier_widths`, `bridge_pier_stations`, `bridge_num_piers` |
| Tapered shaft | `bridge_pier_top_widths`, `bridge_pier_bottom_widths`; optional `bridge_pier_top_elevations`, `bridge_pier_base_elevations` |
| Width profile | `bridge_pier_width_elevations`, `bridge_pier_width_values` (≥ 2 points; wins over top/bottom pair) |
| Footing flare | `bridge_pier_footing_top_elevations`, `bridge_pier_footing_widths`, `bridge_pier_footing_bottom_elevations` |
| Plan nosing | `bridge_pier_nosing_lengths`, `bridge_pier_nosing_widths` |
| Nose shape (one per bridge) | `bridge_pier_shapes` — see table below |

**Precedence per pier:** width profile → top/bottom pair → legacy `bridge_pier_widths`.

### `bridge_pier_shapes` (v29)

| Code | Name | Yarnell $K$ | Momentum $C_D$ |
|-----:|------|------------:|---------------:|
| 0 | Square | 1.25 | 2.00 |
| 1 | Semicircular | 0.90 | 1.20 |
| 2 | Twin-cylinder w/ diaphragm | 0.95 | 1.33 |
| 3 | Triangular 90° | 1.05 | 1.60 |
| 4 | Twin-cylinder no diaphragm | 1.05 | 1.33 |
| 5 | Ten-pile trestle | 2.50 | 2.00 |
| 6–8 | Elliptical 2:1 / 4:1 / 8:1 | 0.90† | 0.60 / 0.32 / 0.29 |
| 9–11 | Triangular 30° / 60° / 120° | 1.05‡ | 1.00 / 1.39 / 1.72 |

†‡ Elliptical / acute triangular Yarnell $K$ uses documented fallbacks when Yarnell is selected.

Not supported: per-pier shape array, user $K$/$C_D$ overrides, fender polygons, wing walls.

## Deck vents (v29, STREAM-1D extension)

Not in HEC-RAS 1D. Optional `bridge_deck_vent_*` segments for grates/slots above main soffit. Omit for pure HEC imports.

| Field | Role |
|-------|------|
| `bridge_deck_vent_stations`, `_inverts`, `_soffits`, `_widths` | Segment geometry in opening frame |
| `bridge_deck_vent_coeffs`, `_types` | Orifice $C_d$; type `1` = slot weir then orifice |

## Roadway embankment compose (v26)

`bridge_roadway_embankments[b]` merges deck, abutment, ineffective blocks, and embankment blocked tops before hydraulics. Flat `bridge_deck_*`, `bridge_abutment_*`, `bridge_ineffective_*` still work; **explicit flat fields win** when fully specified. Details: [`equations.md`](../reference/equations.md) §G2, [`roadway_embankment_unified.md`](roadway_embankment_unified.md).

## Ice / debris (v32)

| Field | Effect |
|-------|--------|
| `bridge_opening_blockage_factors` | Scalar on net opening area / conveyance |
| `bridge_pier_debris_widths`, `_heights` | Floating debris block at WSEL per pier |
| `bridge_ice_thicknesses`, `bridge_ice_modes` | Constant ice under deck (`ice_mode = 2` jam deferred) |
| `bridge_deck_ice_thicknesses` | Lowers weir crest |

Do not duplicate pier blockage on BU/BD `blocked_obstructions` when pier fields are set.

## Reverse flow (v31)

| Supported | Not supported |
|-----------|---------------|
| Negative `q_values` on `computeBridgeRatingCurve`; `tw_wsel_reverse` | Culvert reversal |
| Steady `flow_rate < 0` with bridge mirror | Network / junction reversal |
| Unsteady post-step bridge when section `Q < 0` | Infer direction from stages alone; `Q = 0` rating samples |

Sign convention: `Q > 0` downstream along the reach. When `Q < 0`, hydraulic upstream is **BD**, tailwater is **BU**; reach labels BU/BD do not swap.
