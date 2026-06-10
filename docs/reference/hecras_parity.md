# HEC-RAS scope and parity

Solver limits relative to HEC-RAS. This repository is the Rust/Python/WASM solve core only — no project database, RAS Map, or HEC-RAS file importer.

## Limitations (read before comparing to HEC-RAS)

STREAM-1D is the Rust/Python/WASM solve core in this repository, not a full HEC-RAS installation. Stateless API: `cross_sections` and boundary inputs in, profile arrays out. No user interface, project database, RAS Map, 2D meshing, or native HEC-RAS Plan/Unsteady file workflow in this repo.

The hosted product at [stream1d.com](https://stream1d.com) provides cross-section editing, HEC-RAS geometry import (e.g. `.g01`), project persistence, and visualization on top of this engine. That web application is a separate product (see [License](#license)). This repository is the solver core only: it accepts geometry arrays via WASM or Python and does not include a HEC-RAS file importer or cross-section editor.

### What the STREAM-1D engine supports

| Area | Supported |
|------|-----------|
| **Steady flow** | Standard Step backwater/drawdown; subcritical, supercritical, and mixed regime (`regime` 0/1/2) |
| **Boundary conditions (steady)** | Known WSEL, critical depth, normal depth, rating curve (upstream and downstream) |
| **Cross-sections** | Arbitrary $(x,y)$ polylines; composite Manning's *n*; optional channel/overbank subdivision (`is_overbank`); **blocked obstructions**; **ineffective flow** (`ineffective_flow_areas` on reach cuts, steady and unsteady) |
| **Main stem + tributary (steady)** | One tributary joining one main channel at a shared WSEL node — main stem above/below the junction plus tributary inflow (`tributary_cross_sections`, `tributary_flow_rate`, `junction_main_station`); subcritical only |
| **Culverts (steady, main stem)** | Circular, box, arch, ConSpan, **pipe-arch**, **elliptical**, and **horseshoe**; FHWA-style inlet/outlet control with signed barrel slope (adverse grade supported). Explicit inlet types, invert elevations, roadway overtopping, composite bottom roughness, sediment blockage, control reporting (`inlet` / `outlet` / `overtopping`), extended diagnostics (inlet/outlet HW, barrel vs weir $Q$, barrel depth/velocity/Froude), `computeCulvertRatingCurve`, barrel **skew** (`culvert_skew_angles`), **active barrel count** (`culvert_active_barrels`), **per-barrel geometry** (`culvert_barrel_spans` / `culvert_barrel_rises`) with capacity-based flow split, and **supercritical culvert routing** (`regime` 1/2) via headwater inversion |
| **Culverts (unsteady, single reach)** | Same culvert input fields as steady on `UnsteadyInputs`; **iterated post-step headwater coupling** (tolerance-based convergence, up to 5 passes per step); returns culvert diagnostics each time step |
| **Bridges (steady, main stem)** | HEC-RAS **Class A/B/C** low-flow (`bridge_low_flow_methods`: Yarnell, momentum, energy, WSPRO); high-flow **pressure** (sluice-gate / submerged orifice) and **Bradley weir** overtopping with submergence fallback to energy; **piecewise deck profiles** (`bridge_deck_*`); **per-side abutments** (`bridge_abutment_left_*` / `bridge_abutment_right_*`, API v21); **BU/BD interior cuts** (`bridge_upstream_cross_sections`, `bridge_downstream_cross_sections`, `bridge_internal_cross_sections`, `bridge_opening_reach_station_origins`, API v22); **guide banks** (v24); **skew** (`bridge_skew_angles`); **pier spacing** (`bridge_pier_stations`); HEC-RAS **ineffective flow** (`bridge_ineffective_*` and `ineffective_flow_areas` on BU/BD cuts); **supercritical tailwater coupling** (`regime` 1/2); `computeBridgeRatingCurve` |
| **Bridges (unsteady, single reach)** | Same bridge input fields as steady on `UnsteadyInputs` (including BU/BD interior cuts, API v22); BU/BD reach layout densification; **iterated post-step headwater coupling** (up to 5 passes per step); returns per-step bridge flow regime, upstream/downstream WSEL, and head-loss diagnostics |
| **Unsteady flow** | Preissmann Saint-Venant on a **single reach**; upstream $Q(t)$ and downstream WSEL($t$) hydrographs; optional **inline culverts** and **inline bridges** (see rows above) |
| **Outputs** | WSEL, critical WSEL, velocity, area, top width, Froude number, energy grade slope (+ `tributary_wsel`, `tributary_velocity`, `tributary_froude` when a junction is modeled; + `culvert_control_types` and culvert diagnostic arrays when culverts are modeled; + bridge flow regime and head-loss arrays on **`solve_steady`** and **`solve_unsteady`** when bridges are modeled) |

### Companion web application features ([stream1d.com](https://stream1d.com))

These are implemented in the **STREAM-1D web application**, not in the Rust/WASM/Python solver crate in this repository:

| Feature | Description |
|---------|-------------|
| **Cross-section editing** | Interactive editing of reach geometry and Manning's *n* in the browser |
| **HEC-RAS geometry import** | Import HEC-RAS geometry files (e.g. `.g01`) to build reaches, cross-sections, and structures automatically, then map to solver inputs (including merging upper + lower main stem at a junction when needed) |

### HEC-RAS gap analysis

Compared to a full HEC-RAS installation, the engine does not model everything in the table below. Rows marked **partial parity** list what STREAM-1D implements today alongside remaining scope limits — they are not “unsupported” feature lists.

| Category | HEC-RAS capability | STREAM-1D today |
|----------|-------------------|-----------------|
| **Dimensionality** | 1D, 2D, and coupled 1D/2D | **1D only** |
| **River networks** | Dendritic systems, multiple junctions, looped reaches | **One** main stem + **one** tributary (**steady only**); no general network graph |
| **Unsteady scope** | Networks, structures, storage areas, lateral inflows | **Single reach** with optional **inline culverts** and **inline bridges** (iterated explicit post-step headwater coupling + per-step diagnostics); **no** multi-reach networks in unsteady |
| **Storage & diversions** | Ponds, reservoirs, split flow, lateral structures, pumps, gates | Not modeled |
| **Inline weirs & dams** | Standalone weirs, inline structures, dam breach | Not modeled (bridge roadway overtopping only) |
| **Bridge hydraulics** *(partial parity)* | Bridges on tributary reaches and arbitrary multi-reach unsteady networks; standalone inline weirs separate from bridge decks; implicit structure coupling inside the unsteady solver Jacobian; multi-segment friction through interior bridge cuts | **Main-stem steady** and **single-reach unsteady**: Class A/B/C low-flow; Yarnell, momentum, energy, WSPRO; sluice-gate/submerged-orifice pressure; Bradley weir submergence; **piecewise deck profiles** (`bridge_deck_*`); **per-side abutments** (API v21); **explicit BU/BD face cuts** with reach layout and **min(BU, BD)** opening weighting (API v22); **skew**; **pier spacing**; ineffective flow; blocked obstructions; supercritical tailwater coupling; `computeBridgeRatingCurve` — all via **explicit post-step coupling** (interior cuts affect reach layout/friction length; multi-segment hydraulics through interiors not yet routed) |
| **Culvert hydraulics** *(partial parity)* | Full HEC-RAS culvert catalog (all standard shapes), culverts in multi-reach unsteady networks | FHWA nomograph (circular, box, arch, ConSpan, pipe-arch, elliptical, horseshoe) with explicit inlet types; multi-barrel capacity-based $Q$ split with optional per-barrel span/rise; skew angles and blocked-barrel count; invert offsets, roadway overtopping, extended culvert diagnostics and rating-curve API; **supercritical culvert routing** in mixed-regime steady profiles; **inline culverts** in single-reach unsteady (iterated explicit coupling, not implicit in Preissmann Jacobian) |
| **Ineffective flow** *(partial parity)* | Roadway embankment blocking; full RAS storage-area coupling | Reach `ineffective_flow_areas` (steady, unsteady), `blocked_obstructions`, `bridge_ineffective_*`, approach/departure cuts — [`equations.md` §H0](equations.md) |
| **Terrain & mapping** | RAS Terrain, TIN/bathymetry authoring, RAS Map | **Not in the engine** — the companion **web app** may edit cross-sections and import HEC-RAS geometry; the solver only receives $(x,y)$ sections and stations |
| **Sediment & morphology** | Mobile bed, sediment transport, scour | Not modeled (optional fixed culvert blockage depth only) |
| **Water quality & ice** | Temperature, water quality, ice jams | Not modeled |
| **Project workflow** | Full HEC-RAS `.prj` with Plan, Geometry, and Unsteady files | **Not in the engine** — no built-in project format; the **web app** may import geometry and manage projects, then call WASM per solve |
| **Regulatory reporting** | FEMA, flood insurance, HEC-RAS report templates | Not included |

### Practical guidance

* Supply reach geometry as `cross_sections` arrays (Python or JSON). No built-in HEC-RAS `.g01` importer in this repository.
* Steady junction: merge upper and lower main stem into one `cross_sections` array; pass tributary as `tributary_cross_sections` with `junction_main_station` at the confluence.
* Not supported: multi-reach unsteady networks, 2D routing, FEMA report templates, general pump/gate/storage workflows.
* Unsteady stabilization for steep transients remains in development; see open issues.

For host-application architecture (Web Workers, data transfer, GIS integration), see [`tech_spec.md`](tech_spec.md).
