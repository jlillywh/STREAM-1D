# STREAM-1D Technical Specification

**System architecture and integration blueprint for host applications (web, Python, and batch pipelines).**

This document describes how STREAM-1D fits into a larger application. Mathematical formulations, verification results, and build instructions are in [`README.md`](README.md). GUI integration specs for the hosted [stream1d.com](https://stream1d.com) web application are maintained in that product's repository, not here.

**Core language:** Rust (compiled to WebAssembly and a native Python extension via maturin)

**Target environments:** Modern web browsers (WASM + Web Workers), Python 3.7+, Node.js (`pkg-node`)

---

## 1. System Architecture Overview

The computational core is **stateless**: no project files, hidden globals, or file I/O inside the engine. The **companion web application** (separate from this repository) owns persistence, GIS, cross-section editing, and HEC-RAS geometry import; visualization and Workers wrap the WASM calls.

```
+-------------------------------------------------------------------------+
|                              MAIN THREAD                                |
|  [GUI Layer] - React/Vue/TS UI, Leaflet/Mapbox GIS, Canvas/D3 plots     |
+-------------------------------------------------------------------------+
         |                                                 ^
         | structured inputs / typed arrays                | result arrays
         v                                                 |
+-------------------------------------------------------------------------+
|                        WEB WORKER THREAD (Background)                   |
|  [Worker Wrapper] - message listener, WASM init, payload marshalling      |
|                                                                         |
|    +---------------------------------------------------------------+    |
|    |                      WASM CORE (stream1d)                    |    |
|    |  Geometry processor  -->  Steady / Unsteady solvers           |    |
|    +---------------------------------------------------------------+    |
+-------------------------------------------------------------------------+
```

---

## 2. Implemented Engine Modules

### Module A: Cross-Section Geometry Processor (`src/geometry/processor.rs`)

Transforms arbitrary $(x, y)$ cross-section polylines into vertical lookup tables (default 100 slices):

* Area, wetted perimeter, top width, conveyance
* Horton–Einstein composite Manning's $n$ when $n$ varies by station
* Optional channel / overbank subdivision via `is_overbank` → `channel_area` for structure-adjacent calculations
* Cross-section modifiers — see [`docs/reference/equations.md`](docs/reference/equations.md) §H0

### Module B: Steady-State Solver (`src/solvers/steady.rs`, `junction.rs`, `bridge.rs`, `bridge_abutment.rs`, `culvert.rs`)

* Standard Step backwater / drawdown (subcritical, supercritical, mixed regime)
* Inline culverts (FHWA-style inlet/outlet control) and bridges (HEC-RAS Class A/B/C low flow; Yarnell, momentum, energy, and WSPRO; sluice-gate and submerged-orifice pressure flow; Bradley submerged-weir reduction; piecewise deck profiles; **per-side abutment geometry** via `bridge_abutment.rs`, API v21; supercritical tailwater coupling)
* **One** main-stem + **one** tributary junction (`solve_steady` with junction fields) — **steady, subcritical only**

### Module C: Unsteady Solver (`src/solvers/unsteady.rs`)

* Preissmann implicit Saint-Venant routing (Thomas algorithm)
* Upstream discharge and downstream stage hydrographs
* **Single reach only** — no tributary junction routing in unsteady mode

---

## 3. Scope Boundaries (Important for Integrators)

See the full **[Limitations (read before comparing to HEC-RAS)](README.md#limitations-read-before-comparing-to-hec-ras)** section in `README.md` for what STREAM-1D does and does not model relative to HEC-RAS.

| Feature | Steady | Unsteady |
|---------|--------|----------|
| Single reach | Yes | Yes |
| Culverts / bridges on main stem | Yes | **Yes** — inline culverts and bridges via explicit post-step coupling (not implicit in Preissmann Jacobian). Stronger coupling planned: [`docs/development/unsteady_implicit_bridge_coupling.md`](docs/development/unsteady_implicit_bridge_coupling.md) |
| One tributary junction | Yes (subcritical) | **No** |
| Multiple tributaries / networks | **No** | **No** |
| 2D floodplain, sediment, water quality | **No** | **No** |

Host **web apps** importing HEC-RAS geometry with three reaches at a confluence must merge upper and lower main stems into one `cross_sections` array before calling WASM, then pass the tributary reach via `tributary_cross_sections` with `junction_main_station` at the shared node. (This import workflow is **not** part of the Python bindings in this repository.)

---

## 4. WASM API Surface

Entry points (see `src/lib.rs`):

| Function | Purpose |
|----------|---------|
| `init()` | Load WASM module (generated by wasm-pack) |
| `getEngineVersion()` | Engine semver string |
| `getWasmApiMetadata()` | API contract: `api_version`, culvert inlet/shape enums, Tier 1 field names |
| `validateSteadyInputs(inputs)` | Parse-check payload without solving |
| `solveSteady(inputs)` | Steady GVF + structures → `SteadyResult` |
| `solveUnsteady(inputs)` | Unsteady routing → `UnsteadyResult` |
| `computeCulvertRatingCurve(inputs)` | Culvert headwater vs $Q$ at fixed tailwater |
| `computeBridgeRatingCurve(inputs)` | Bridge upstream headwater vs $Q$ at fixed tailwater |

Inputs and outputs are plain JavaScript objects using **snake_case** field names (same schema as Python JSON). Steady junction runs additionally return `tributary_wsel`, `tributary_velocity`, and `tributary_froude` when tributary fields are set.

Check `getWasmApiMetadata().api_version` on each upgrade — history in [`docs/reference/api_changelog.md`](docs/reference/api_changelog.md). Types and examples: [`docs/wasm_api.types.ts`](docs/wasm_api.types.ts), [`examples/wasm/`](../examples/wasm/). Bridge BU/BD design: [`docs/BRIDGE_INTERIOR_SECTIONS_API.md`](docs/BRIDGE_INTERIOR_SECTIONS_API.md).

**Build outputs:** `./build_wasm.sh` produces `./pkg` (web) and `./pkg-node` (Node), runs JSON contract tests, and executes a Node smoke test.

---

## 5. Data Transfer Recommendations

For interactive web apps, avoid re-parsing large JSON on every keystroke when possible:

* Pass cross-section coordinates and Manning breaks as typed arrays (`Float64Array`) in Worker messages
* Use [Transferable objects](https://developer.mozilla.org/en-US/docs/Web/API/Web_Workers_API/Transferable_objects) for zero-copy handoff where buffers are rebuilt each solve
* Keep the WASM module in a dedicated Worker so profile updates do not block the UI thread

The current crate accepts structured objects; flat-buffer or SharedArrayBuffer optimizations are host-application concerns.

---

## 6. UI Layer (Companion Web Application — not in this repository)

STREAM-1D does not ship a GUI. A separate **web application** built on this engine typically provides:

* **Plan / profile views:** Canvas, SVG, or chart libraries (e.g. uPlot, D3)
* **GIS:** Mapbox GL, Leaflet, or similar for reach centerlines and cut lines
* **Cross-section editing:** Interactive geometry and Manning's *n* editing in the browser
* **HEC-RAS geometry import:** Parse `.g01` (and related) files to populate reaches and structures, then map to `SteadyInputs` / `UnsteadyInputs` before each WASM solve
* **Workers:** Background execution and progress callbacks

Python users supply `cross_sections` arrays directly; there is no HEC-RAS importer in the `stream1d` package.

---

## 7. Verification

Automated checks ship with the repository:

* `cargo test` — Rust unit and integration tests (geometry, culvert, bridge Yarnell/abutments, junction, steady/unsteady)
* `tests/bridge_abutment_hecras_verification.rs` — per-side abutment hand-calc / WSPRO benchmarks (`verification/fixtures/bridge_abutment_hecras.json`)
* `tests/wasm_json_contract.rs` — JSON schema contract (including API v21 abutment fields)
* `python/test_stream1d.py`, `python/test_python_bindings.py` — Python binding and HEC-RAS ConSpan benchmark (run `maturin develop --features python` after engine changes)
* [`python/stream1d_verification.ipynb`](python/stream1d_verification.ipynb) — interactive Binder notebook with HEC-RAS WSEL overlay

ConSpan steady benchmark tolerance: **±0.04 ft** WSEL vs HEC-RAS at key stations (see README verification table).

---

## 8. Related Documentation

| Document | Purpose |
|----------|---------|
| [`README.md`](README.md) | Equations, build, usage examples, verification summary |
| [`docs/wasm_api.types.ts`](docs/wasm_api.types.ts) | TypeScript definitions for WASM integrators |
| [`examples/wasm/`](../examples/wasm/) | Worker reference and Node smoke test |
