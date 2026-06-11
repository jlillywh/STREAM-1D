# API changelog

JSON/Python input schema version (`api_version` in WASM metadata). Current version: **31**.

| Version | Change |
|---------|--------|
| 31 | Bi-directional bridge rating and reverse-flow reach coupling (Phase 4.3): negative `q_values` in `computeBridgeRatingCurve`; `tw_wsel_reverse` on rating params; `solve_bridge_coupled` mirror for `Q < 0`; steady subcritical/supercritical sweeps and mixed regime for negative `flow_rate`; unsteady post-step bridge coupling uses direction-aware TW/HW faces. **Limitations:** culvert reversal unsupported; `Q=0` rating samples skipped; direction not inferred from stages — [`bridge_reverse_flow_rating.md`](../development/bridge_reverse_flow_rating.md) |
| 30 | Bridge friction weighting (HEC-RAS §4.2): `bridge_friction_weighting` (`0` = opening only, `1` = approach + opening + departure), `bridge_approach_friction_lengths`, `bridge_departure_friction_lengths`; rating curve keys `friction_weighting`, `approach_friction_length`, `departure_friction_length` — energy / WSPRO low-flow friction uses three segments when weighting is `1` |
| 29 | Extended `bridge_pier_shapes` / `pier_shape_type` values `4`–`11`: twin-cylinder without diaphragm, ten-pile trestle bent, elliptical 2:1/4:1/8:1, triangular 30°/60°/120° — HEC-RAS Yarnell $K$ and momentum $C_D$ — see [`extended_pier_shape_catalog.md`](../development/extended_pier_shape_catalog.md) |
| 28 | Pier footings and nosing: `bridge_pier_footing_top_elevations` / `_widths` / `_bottom_elevations`, `bridge_pier_nosing_lengths` / `_widths`; rating curve keys `pier_footing_*`, `pier_nosing_*` — footing composes into shaft width profile; nosing adds submerged plan area and opening-plane width — see [`pier_footings_nosing.md`](../development/pier_footings_nosing.md) |
| 27 | Tapered pier widths: `bridge_pier_top_widths` / `bridge_pier_bottom_widths`, optional `bridge_pier_width_elevations` / `bridge_pier_width_values`, and cap/base elevations; rating curve keys `pier_top_widths`, etc. — integrated submerged pier area in Yarnell / momentum / pressure — see [`pier_tapered_width.md`](../development/pier_tapered_width.md) |
| 26 | `bridge_roadway_embankments` per bridge (steady/unsteady) and `roadway_embankment` on bridge rating curve — unified deck + abutment + ineffective + embankment blocked tops from grade profiles — see [`equations.md` §G2](equations.md) |
| 25 | `densify_reach_modifier_policy` on steady/unsteady inputs (`0` none, `1` upstream, `2` downstream, `3` nearest); reach ineffective/blocked/guide banks on `max_spacing` interior nodes; interpolated bridge BU/BD inherit `bridge_ineffective_*` — see [`equations.md` §H1](equations.md) |
| 24 | Guide banks on approach/departure cuts (`CrossSection.guide_banks`, `bridge_approach_*` / `bridge_departure_*` fields); resolved on `BridgeSectionContext`; guided active area in WSPRO/energy when guide banks configured |
| 23 | Bridge opening anchor modes (`bridge_opening_anchor_modes`, `bridge_opening_anchor_reach_stations`); reach river station ↔ opening origin resolution; `validateSteadyInputs` returns `{ warnings }` (bridge opening vs parent XS width) |
| 3 | Culvert extended diagnostics; culvert rating curve |
| 4 | Culvert skew angles; active barrel count |
| 5 | Per-barrel span and rise arrays |
| 6 | Culvert shapes: pipe-arch, elliptical, horseshoe |
| 7 | Culvert fields on unsteady inputs; supercritical culvert routing in mixed-regime steady |
| 8 | Unsteady culvert diagnostics per time step; stronger culvert coupling |
| 9 | Bridge fields on unsteady inputs; unsteady bridge coupling and diagnostics |
| 10 | `structure_coupling_order` for culvert/bridge post-step order |
| 11 | Bridge WSPRO and energy low-flow methods; `bridge_lengths`, `bridge_wspro_coeffs` |
| 12 | Bridge high-flow pressure (sluice gate, submerged orifice) and Bradley weir; submergence fallback |
| 13 | Piecewise bridge deck profiles (`bridge_deck_*`) |
| 14 | Bridge ineffective flow blocks (`bridge_ineffective_*`) |
| 15 | Bridge skew; explicit pier stations |
| 16 | Bridge rating curve function |
| 17 | Explicit bridge high-flow energy method |
| 18 | Separate upstream/downstream ineffective elevations per bridge face |
| 19 | Multiple ineffective blocks per bridge side (nested arrays) |
| 20 | `blocked_obstructions` on cross sections |
| 21 | Per-side bridge abutment geometry (`bridge_abutment_left_*` / `bridge_abutment_right_*`) |
| 22 | BU/BD bridge interior cross sections; `ineffective_flow_areas` on cuts; BU→BD reach layout and friction length |

WASM hosts can read `getWasmApiMetadata().api_version` after each upgrade. Python callers do not set a version; field compatibility follows the installed extension build.
