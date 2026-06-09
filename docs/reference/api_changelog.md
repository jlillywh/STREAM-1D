# API changelog

JSON/Python input schema version (`api_version` in WASM metadata). Current version: **22**.

| Version | Change |
|---------|--------|
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
