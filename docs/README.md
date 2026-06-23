# Documentation index

One canonical source per topic — link here instead of duplicating tables in README.

## Python & API

| Topic | Doc |
|-------|-----|
| Python examples | [`python/getting_started.md`](python/getting_started.md) |
| Input fields (Python) | [`../python/stream1d/__init__.py`](../python/stream1d/__init__.py) |
| Version history | [`reference/api_changelog.md`](reference/api_changelog.md) |
| Physics & modifiers §H0–§H1 | [`reference/equations.md`](reference/equations.md) |
| HEC-RAS scope & gaps | [`reference/hecras_parity.md`](reference/hecras_parity.md) |
| Tests & verification | [`development/testing.md`](development/testing.md), [`../verification/`](../verification/) |
| Unsteady structure coupling (modes 0–4) | [`development/unsteady_structure_coupling.md`](development/unsteady_structure_coupling.md) |
| Bridge pier / deck / ice / reverse flow | [`development/bridge_extensions.md`](development/bridge_extensions.md) |
| BU/BD interior cuts & opening frames | [`BRIDGE_INTERIOR_SECTIONS_API.md`](BRIDGE_INTERIOR_SECTIONS_API.md) |
| Roadway embankment compose (v26) | [`development/roadway_embankment_unified.md`](development/roadway_embankment_unified.md) |
| High-flow intentional deltas | [`development/pressure_weir_combined_flow_audit.md`](development/pressure_weir_combined_flow_audit.md) |

## Maintainers

| Topic | Doc |
|-------|-----|
| PyPI releases | [`development/publishing.md`](development/publishing.md) |
| JSON schema (WASM contract) | [`wasm_api.types.ts`](wasm_api.types.ts) |
| WASM / browser build | [`web/wasm_integration.md`](web/wasm_integration.md) |
| Hosted app architecture | [`development/tech_spec.md`](development/tech_spec.md) |

**When changing behavior:** update the canonical doc, bump `api_changelog.md` if `API_VERSION` changes, extend tests — do not restate the same content in README.
