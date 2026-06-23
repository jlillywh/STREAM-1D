# API changelog

JSON/Python input schema version (`api_version` in WASM metadata). Current version: **37**.

| Version | Change |
|---------|--------|
| 37 | Culvert HEC-RAS inline structure reach: `culvert_approach_reach_stations`, `culvert_departure_reach_stations` — inserts US/DS bounding nodes on the densified grid and couples culverts on that interval (mirrors bridge approach/departure reach) |
| 36 | Unsteady structure coupling modes **`3`** (`MonolithicNewton`, experimental) and **`4`** (`QuasiSteadyParticular`, quasi-steady re-anchor + mode-2 physics — recommended for culvert approach backwater on Q ramps); WASM metadata entries for both modes |
| 35 | Unsteady downstream BC types mirror steady: `downstream_bc_type` (`0` WSEL hydrograph default, `1` critical depth, `2` dynamic friction slope, `3` rating curve), `downstream_bc_slope`, `downstream_bc_rating_*`; reserved upstream stage fields; `theta` default **0.6** (clamp $[0.55,1.0]$) — see [`equations.md` §4](equations.md) |
| 34 | Hybrid structure coupling diagnostics on `solve_unsteady` when inline structures are present: `structure_coupling_converged`, `structure_implicit_interval_count`, `structure_explicit_fallback_count` (per time step); WASM metadata `unsteady_structure_coupling_outputs`. Mode `2` enum renamed `HybridImplicit` (description: implicit where eligible + explicit fallback). |
| 33 | Unsteady Preissmann structure coupling: `unsteady_structure_coupling_mode` — see [`unsteady_structure_coupling.md`](../development/unsteady_structure_coupling.md) |
| 32 | Bridge ice / debris (v32): optional opening blockage, pier debris, ice thickness — see [`bridge_extensions.md`](../development/bridge_extensions.md) |
| 31 | Bi-directional bridge rating and reverse-flow coupling — see [`bridge_extensions.md`](../development/bridge_extensions.md) |
| 30 | Bridge friction weighting (HEC-RAS §4.2): `bridge_friction_weighting` (`0` = opening only, `1` = approach + opening + departure), `bridge_approach_friction_lengths`, `bridge_departure_friction_lengths`; rating curve keys `friction_weighting`, `approach_friction_length`, `departure_friction_length` — energy / WSPRO low-flow friction uses three segments when weighting is `1` |
| 29 | Extended `bridge_pier_shapes` 4–11 — see [`bridge_extensions.md`](../development/bridge_extensions.md) |
| 28 | Pier footings and nosing — see [`bridge_extensions.md`](../development/bridge_extensions.md) |
| 27 | Tapered pier widths — see [`bridge_extensions.md`](../development/bridge_extensions.md) |
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
