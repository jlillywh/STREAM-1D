# HEC-RAS scope and parity

What this solver implements vs a full HEC-RAS installation. Compare results via [`verification/`](../verification/) and [`verification/oracle/`](../verification/oracle/).

## Engine vs hosted app

This repository is the **stateless solve core** (geometry arrays in, profile arrays out). [stream1d.com](https://stream1d.com) adds editing, `.g01` import, and visualization separately.

## Supported today

| Area | STREAM-1D |
|------|-----------|
| Steady GVF | Subcritical, supercritical, mixed; standard step |
| Steady junction | One tributary on main stem (subcritical) |
| Cross-sections | Polylines, composite *n*, overbank subdivision, blocked obstructions, ineffective flow, guide banks |
| Culverts | FHWA inlet/outlet, 7 shape families, multi-barrel, skew, overtopping, blockage |
| Bridges | Class A/B/C, Yarnell/momentum/energy/WSPRO, pressure/weir, deck profiles, abutments, BU/BD cuts, piers (v27–v29), ice/debris (v32), reverse rating (v31) |

## Major gaps

| HEC-RAS | STREAM-1D |
|---------|-----------|
| 2D / coupled 1D–2D | **1D only** |
| Multi-reach networks | Single reach; one tributary junction |
| Storage, pumps, gates, lateral structures | Not modeled |
| Standalone inline weirs | Bridge/culvert roadway weir only |
| Culvert reverse flow | Not modeled |
| Dynamic ice jam / reach ice transport | Bridge-local constant ice only (v32) |
| Full culvert shape catalog | 7 implemented families |
| Native `.prj` workflow | Host/oracle mappers only |

## Bridge pier mapping (summary)

HEC pier editor → flat JSON. Full field list: [`bridge_extensions.md`](development/bridge_extensions.md).

| HEC concept | STREAM-1D |
|-------------|-----------|
| Pier station, count | `bridge_pier_stations`, `bridge_num_piers` |
| Width vs elevation | `bridge_pier_width_elevations` / `_values` or top/bottom widths |
| Footing / nosing | `bridge_pier_footing_*`, `bridge_pier_nosing_*` |
| Nose shape | `bridge_pier_shapes` (one enum per bridge) |
| Floating debris | `bridge_pier_debris_widths` / `_heights` (partial) |

Not modeled: per-pier shape, fenders, wing walls.


## High-flow bridge deltas

Intentional remaining differences vs HEC: [`development/pressure_weir_combined_flow_audit.md`](development/pressure_weir_combined_flow_audit.md).

## Linked oracle status

| Scenario | Gate |
|----------|------|
| `conspan_steady_linked` | ±0.04 ft |

See [`verification/oracle/README.md`](../verification/oracle/README.md).

## Practical guidance

- Supply reach geometry as `cross_sections` arrays; map HEC projects in the host app or oracle mappers.
- Set `densify_reach_modifier_policy: 1` when reach ineffective flow must apply on `max_spacing` interior nodes ([`equations.md`](reference/equations.md) §H1).

Host architecture: [`development/tech_spec.md`](development/tech_spec.md).
