# Documentation index

One canonical source per topic — link instead of copying tables elsewhere.

| Topic | Canonical doc |
|-------|----------------|
| API field names & types | [`wasm_api.types.ts`](wasm_api.types.ts), Rust `SteadyInputs` / `CrossSection` |
| Version history | [`reference/api_changelog.md`](reference/api_changelog.md) (`API_VERSION` in `src/wasm_api.rs`) |
| Physics & geometry modifier semantics | [`reference/equations.md`](reference/equations.md) §H0 |
| Densified-node modifier inheritance (shipped rules) | [`reference/equations.md`](reference/equations.md) §H1 |
| Bridge BU/BD, opening frames, resolution order | [`BRIDGE_INTERIOR_SECTIONS_API.md`](BRIDGE_INTERIOR_SECTIONS_API.md) |
| HEC-RAS scope (incl. bridge pier editor) | [`reference/hecras_parity.md`](reference/hecras_parity.md) § Bridge pier editor |
| Python examples | [`python/getting_started.md`](python/getting_started.md) |
| WASM build & JS usage | [`web/wasm_integration.md`](web/wasm_integration.md) |
| Tests | [`development/testing.md`](development/testing.md) |
| Densified reach modifier inheritance (design) | [`development/densify_modifier_inheritance.md`](development/densify_modifier_inheritance.md) |
| Unified roadway embankment — deck + abutment + ineffective (design) | [`development/roadway_embankment_unified.md`](development/roadway_embankment_unified.md) |
| Migrate v19/v20 ineffective & blocked → v26 embankment | [`development/migration_v19_v20_roadway_embankment.md`](development/migration_v19_v20_roadway_embankment.md) |
| Tapered pier width API (v27) | [`development/pier_tapered_width.md`](development/pier_tapered_width.md) |
| Pier footings, nosing, fender/wing walls API (design) | [`development/pier_footings_nosing.md`](development/pier_footings_nosing.md) |

**When changing behavior:** update the canonical doc above, bump `api_changelog.md` if `API_VERSION` changes, extend `tests/wasm_json_contract.rs` / geometry tests — do not restate the same tables in README or `tech_spec.md`.
