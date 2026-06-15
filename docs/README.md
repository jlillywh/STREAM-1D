# Documentation index

One canonical source per topic — link here instead of duplicating tables in README.

| Topic | Doc |
|-------|-----|
| API fields & types | [`wasm_api.types.ts`](wasm_api.types.ts) |
| Version history | [`reference/api_changelog.md`](reference/api_changelog.md) |
| Physics & modifiers §H0–§H1 | [`reference/equations.md`](reference/equations.md) |
| HEC-RAS scope & gaps | [`reference/hecras_parity.md`](reference/hecras_parity.md) |
| Python examples | [`python/getting_started.md`](python/getting_started.md) |
| WASM build & JS | [`web/wasm_integration.md`](web/wasm_integration.md) |
| Host-app Workers / GIS | [`development/tech_spec.md`](development/tech_spec.md) |
| Tests & oracle CI | [`development/testing.md`](development/testing.md), [`../verification/`](../verification/) |
| Unsteady structure coupling (modes 0–4) | [`development/unsteady_structure_coupling.md`](development/unsteady_structure_coupling.md) |
| Bridge pier / deck / ice / reverse flow | [`development/bridge_extensions.md`](development/bridge_extensions.md) |
| BU/BD interior cuts & opening frames | [`BRIDGE_INTERIOR_SECTIONS_API.md`](BRIDGE_INTERIOR_SECTIONS_API.md) |
| Roadway embankment compose (v26) | [`development/roadway_embankment_unified.md`](development/roadway_embankment_unified.md) |
| High-flow intentional deltas | [`development/pressure_weir_combined_flow_audit.md`](development/pressure_weir_combined_flow_audit.md) |
| PyPI releases | [`development/publishing.md`](development/publishing.md) |

**When changing behavior:** update the canonical doc, bump `api_changelog.md` if `API_VERSION` changes, extend tests — do not restate the same content in README.
