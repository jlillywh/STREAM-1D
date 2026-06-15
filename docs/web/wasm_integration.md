# WebAssembly integration

Build: `bash build_wasm.sh` → `./pkg` (browser), `./pkg-node` (Node). Python: [`python/getting_started.md`](../python/getting_started.md).

## Entry points

| Function | Purpose |
|----------|---------|
| `init()` | Load module |
| `getWasmApiMetadata()` | `api_version`, field lists, enums |
| `validateSteadyInputs` | Parse-check without solve |
| `solveSteady` / `solveUnsteady` | Profile routing |
| `computeCulvertRatingCurve` / `computeBridgeRatingCurve` | Structure rating |

Payloads use **snake_case** (same as Python JSON). Types: [`wasm_api.types.ts`](../wasm_api.types.ts). Changelog: [`reference/api_changelog.md`](../reference/api_changelog.md).

## Minimal steady example

```javascript
import init, { solveSteady } from './pkg/stream1d.js';

await init();
const results = solveSteady({
  cross_sections: [
    { station: 1000, x: [0,0,10,10], y: [6,1,1,6], n_stations: [0], n_values: [0.025], unit_system: 'Metric' },
    { station: 0,   x: [0,0,10,10], y: [5,0,0,5], n_stations: [0], n_values: [0.025], unit_system: 'Metric' },
  ],
  flow_rate: 15,
  regime: 0,
  downstream_wsel: 1.5,
});
console.log(results.wsel);
```

## More examples

- Worker reference: [`examples/wasm/`](../../examples/wasm/)
- Culvert Tier 1 JSON: [`tests/fixtures/wasm_steady_culvert_tier1.json`](../../tests/fixtures/wasm_steady_culvert_tier1.json)
- Bridge BU/BD v22: [`tests/fixtures/wasm_steady_bridge_bu_bd_v22.json`](../../tests/fixtures/wasm_steady_bridge_bu_bd_v22.json)

Host architecture: [`development/tech_spec.md`](../development/tech_spec.md).
