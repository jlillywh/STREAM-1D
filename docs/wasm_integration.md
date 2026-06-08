# WASM Integration Guide (Web App)

This guide is for the **companion web application** that loads STREAM-1D in the browser. The engine exposes a stateless JSON API via WebAssembly — no UI, persistence, or GIS in this repository.

**Related files**

| File | Purpose |
|------|---------|
| [`wasm_api.types.ts`](wasm_api.types.ts) | TypeScript interfaces — copy into the web app |
| [`wasm_integration.md`](wasm_integration.md) | This guide |
| [`web_gui_tributary_junction.md`](web_gui_tributary_junction.md) | Junction / multi-reach import |
| [`../tests/fixtures/wasm_steady_culvert_tier1.json`](../tests/fixtures/wasm_steady_culvert_tier1.json) | Example Tier 1 culvert payload |
| [`../examples/wasm/worker_solve_steady.mjs`](../examples/wasm/worker_solve_steady.mjs) | Web Worker reference |

---

## Build and install

From WSL/Linux in this repository:

```bash
chmod +x ./build_wasm.sh
./build_wasm.sh
```

Outputs:

| Path | Use |
|------|-----|
| `pkg/` | Browser (`import init from './pkg/streams1d.js'`) |
| `pkg-node/` | Node / bundler SSR (`--target nodejs`) |

Copy `pkg/` into the web app (npm file dependency, git submodule, or CI artifact). Rebuild WASM whenever the engine version changes.

---

## API contract

- **Field naming:** `snake_case` (e.g. `cross_sections`, `culvert_inlet_types`) — same as Python JSON.
- **Units:** Per cross-section `unit_system`: `"USCustomary"` or `"Metric"`.
- **Versioning:** Call `getWasmApiMetadata()` after `init()`. Check `api_version` (currently **2** for Tier 1 culvert fields). Bump web app mapping when `api_version` increases.

### Entry points

```javascript
import init, {
  getEngineVersion,
  getWasmApiMetadata,
  validateSteadyInputs,
  solveSteady,
  solveUnsteady,
} from './pkg/streams1d.js';

await init();
console.log(getEngineVersion());        // "0.1.0"
const meta = getWasmApiMetadata();      // inlet codes, shape types, Tier 1 field list
validateSteadyInputs(inputs);           // throws on invalid payload
const result = solveSteady(inputs);
```

---

## Web Worker pattern (recommended)

Run WASM in a dedicated Worker so solves do not block the UI.

```javascript
// main thread
const worker = new Worker(new URL('./streams1d.worker.js', import.meta.url), { type: 'module' });

worker.postMessage({ type: 'solveSteady', inputs: steadyPayload });

worker.onmessage = (event) => {
  if (event.data.type === 'ready') {
    const meta = event.data.metadata;
    // cache meta.culvert_inlet_types for culvert editor dropdowns
  }
  if (event.data.type === 'steadyResult') {
    const { wsel, culvert_control_types } = event.data.result;
    // update profile plot
  }
  if (event.data.type === 'error') {
    console.error(event.data.message);
  }
};
```

See [`examples/wasm/worker_solve_steady.mjs`](../examples/wasm/worker_solve_steady.mjs) for a complete Worker implementation.

**Tips**

- Initialize WASM once per Worker; reuse the module for every solve.
- Call `validateSteadyInputs` in the Worker before `solveSteady` during development.
- Pass plain JSON-serializable objects in `postMessage` (structured clone).

---

## Culvert Tier 1 (web app mapping)

### New input fields (parallel arrays, index = culvert index)

| Field | UI source | Default if omitted |
|-------|-----------|-------------------|
| `culvert_inlet_types` | Inlet type dropdown | `0` (legacy Ke) |
| `culvert_z_ups` | Culvert upstream invert | Adjacent section bed |
| `culvert_z_downs` | Culvert downstream invert | Adjacent section bed |
| `culvert_crest_elevs` | Roadway / embankment crest | No overtopping |
| `culvert_weir_coeffs` | Weir coefficient | 2.6 US / 1.44 metric |
| `culvert_weir_lengths` | Weir length | `span × num_barrels` |

### New output field

| Field | Use in UI |
|-------|-----------|
| `culvert_control_types` | Badge per culvert: `"inlet"` / `"outlet"` / `"overtopping"` |

Populate inlet type dropdown from `getWasmApiMetadata().culvert_inlet_types`.

### Example payload

```javascript
const inputs = {
  cross_sections: [/* ... */],
  flow_rate: 100.0,
  regime: 0,
  downstream_wsel: 3.0,
  culvert_stations: [50.0],
  culvert_shape_types: [0],
  culvert_spans: [5.0],
  culvert_rises: [5.0],
  culvert_roughness_ns: [0.012],
  culvert_lengths: [100.0],
  culvert_entrance_loss_coeffs: [0.5],
  culvert_exit_loss_coeffs: [1.0],
  culvert_barrels: [1],
  culvert_inlet_types: [1],       // square headwall
  culvert_z_ups: [10.0],          // optional
  culvert_z_downs: [9.0],         // optional
  culvert_crest_elevs: [14.0],    // enable overtopping
  culvert_weir_coeffs: [2.6],
  culvert_weir_lengths: [20.0],
};

const result = solveSteady(inputs);
console.log(result.culvert_control_types); // e.g. ["inlet"]
```

Full fixture: [`tests/fixtures/wasm_steady_culvert_tier1.json`](../tests/fixtures/wasm_steady_culvert_tier1.json).

### HEC-RAS import notes

- Map RAS inlet descriptions to `culvert_inlet_types` codes (see metadata enum).
- Map RAS roadway crest to `culvert_crest_elevs` when overtopping is modeled.
- Use `culvert_z_ups` / `culvert_z_downs` when barrel inverts differ from channel thalweg.
- Display `culvert_control_types` in structure results panel for debugging import mismatches.

---

## SteadyInputs / SteadyResult reference

Use [`wasm_api.types.ts`](wasm_api.types.ts) in the web app. All culvert arrays are **optional**; when present, array lengths should match `culvert_stations.length` for defined indices.

---

## Error handling

`solveSteady`, `validateSteadyInputs`, and `getWasmApiMetadata` throw JavaScript `Error` objects on failure (invalid types, missing required fields, solver errors). Wrap Worker calls in try/catch and post `{ type: 'error', message }` to the main thread.

---

## Verification

```bash
# Rust JSON contract (same schema as WASM)
cargo test wasm_json_contract

# Rebuild + Node smoke test (requires wasm-pack)
./build_wasm.sh
node examples/wasm/node_smoke_test.mjs
```
