# STREAM-1D Technical Specification

**System architecture and integration blueprint for host applications (web, Python, and batch pipelines).**

This document describes how STREAM-1D fits into a larger application. Mathematical formulations, verification results, and build instructions are in [`README.md`](README.md). Web GUI tributary import guidance is in [`docs/web_gui_tributary_junction.md`](docs/web_gui_tributary_junction.md).

**Core language:** Rust (compiled to WebAssembly and a native Python extension via maturin)

**Target environments:** Modern web browsers (WASM + Web Workers), Python 3.7+, Node.js (`pkg-node`)

---

## 1. System Architecture Overview

The computational core is **stateless**: no project files, hidden globals, or file I/O inside the engine. Host applications own persistence, GIS, and visualization.

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
|    |                      WASM CORE (streams1d)                    |    |
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

### Module B: Steady-State Solver (`src/solvers/steady.rs`, `junction.rs`, `bridge.rs`, `culvert.rs`)

* Standard Step backwater / drawdown (subcritical, supercritical, mixed regime)
* Inline culverts (FHWA-style inlet/outlet control) and bridges (Yarnell pier loss, pressure orifice, weir overtopping)
* **One** main-stem + **one** tributary junction (`solve_steady` with junction fields) — **steady, subcritical only**

### Module C: Unsteady Solver (`src/solvers/unsteady.rs`)

* Preissmann implicit Saint-Venant routing (Thomas algorithm)
* Upstream discharge and downstream stage hydrographs
* **Single reach only** — no tributary junction routing in unsteady mode

---

## 3. Scope Boundaries (Important for Integrators)

| Feature | Steady | Unsteady |
|---------|--------|----------|
| Single reach | Yes | Yes |
| Culverts / bridges on main stem | Yes | Limited (via steady coupling in places) |
| One tributary junction | Yes | **No** |
| Multiple tributaries / networks | **No** | **No** |

Host apps importing HEC-RAS geometry with three reaches at a confluence must merge upper and lower main stems before calling WASM. See [`docs/web_gui_tributary_junction.md`](docs/web_gui_tributary_junction.md).

---

## 4. WASM API Surface

Entry points (see `src/lib.rs`):

* `solveSteady(inputs: SteadyInputs) -> SteadyResult`
* `solveUnsteady(inputs: UnsteadyInputs) -> UnsteadyResult`

Inputs and outputs are JSON-serializable objects (Python uses the same schema via `_streams1d`). Steady junction runs additionally return `tributary_wsel`, `tributary_velocity`, and `tributary_froude` when tributary fields are set.

**Build outputs:** `./build_wasm.sh` produces `./pkg` (web) and `./pkg-node` (Node).

---

## 5. Data Transfer Recommendations

For interactive web apps, avoid re-parsing large JSON on every keystroke when possible:

* Pass cross-section coordinates and Manning breaks as typed arrays (`Float64Array`) in Worker messages
* Use [Transferable objects](https://developer.mozilla.org/en-US/docs/Web/API/Web_Workers_API/Transferable_objects) for zero-copy handoff where buffers are rebuilt each solve
* Keep the WASM module in a dedicated Worker so profile updates do not block the UI thread

The current crate accepts structured objects; flat-buffer or SharedArrayBuffer optimizations are host-application concerns.

---

## 6. UI Layer (Host Application)

STREAM-1D does not ship a GUI. Expected host responsibilities:

* **Plan / profile views:** Canvas, SVG, or chart libraries (e.g. uPlot, D3)
* **GIS:** Mapbox GL, Leaflet, or similar for reach centerlines and cut lines
* **Import:** Geometry parsing (HEC-RAS, GeoJSON, custom) and reach-merge workflows before solver calls
* **Workers:** Background execution and progress callbacks

---

## 7. Verification

Automated checks ship with the repository:

* `cargo test` — Rust unit and integration tests (geometry, culvert, bridge Yarnell, junction, steady/unsteady)
* `python/test_streams1d.py`, `python/test_python_bindings.py` — Python binding and HEC-RAS ConSpan benchmark
* [`python/streams1d_verification.ipynb`](python/streams1d_verification.ipynb) — interactive Binder notebook with HEC-RAS WSEL overlay

ConSpan steady benchmark tolerance: **±0.04 ft** WSEL vs HEC-RAS at key stations (see README verification table).

---

## 8. Related Documentation

| Document | Purpose |
|----------|---------|
| [`README.md`](README.md) | Equations, build, usage examples, verification summary |
| [`docs/web_gui_tributary_junction.md`](docs/web_gui_tributary_junction.md) | Tributary junction API and import merge modal spec |
