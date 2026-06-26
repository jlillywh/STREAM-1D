# Host application integration

How STREAM-1D fits into web apps and batch pipelines. Solver usage: [README](../README.md). WASM details: [`web/wasm_integration.md`](../web/wasm_integration.md).

## Architecture

Stateless Rust core — no project DB or file I/O. Host owns persistence, GIS, and HEC import; calls WASM or Python per solve.

```text
UI thread  →  JSON / typed arrays  →  Worker  →  WASM (geometry + steady)
```

## Scope (integrators)

| Feature | Supported |
|---------|-----------|
| Single reach | Yes |
| Culverts / bridges | Yes |
| One tributary junction | Yes (subcritical) |
| Networks, 2D, sediment | No |

Merge split HEC main stems into one `cross_sections` array; pass tributary via `tributary_cross_sections` + `junction_main_station`.

## WASM entry points

`solveSteady`, `computeCulvertRatingCurve`, `computeBridgeRatingCurve`, `validateSteadyInputs`, `getWasmApiMetadata`.

Check `api_version` on upgrade ([`api_changelog.md`](../reference/api_changelog.md)). Types: [`wasm_api.types.ts`](../wasm_api.types.ts).

## Performance tips

- Run WASM in a dedicated Worker
- Prefer typed arrays for large coordinate payloads
- Use Transferables when rebuilding buffers each solve

## Verification

[`testing.md`](testing.md), [`verification/`](../verification/).
