Here is a comprehensive, production-ready technical specification for your engineering team. It outlines a modular, decoupled architecture designed to deliver a high-performance, web-native 1D hydraulics engine.

---

# Technical Specification: Project Shrine-1D

**System Architecture & Implementation Blueprint**

**Target Environment:** Modern Web Browsers (WASM + Web Workers + WebGL/Canvas)

**Core Language:** Rust (or Modern C++17/20) compiled to WebAssembly

**GUI Framework:** Decoupled JavaScript (React, Vue, or Vanilla JS/TS)

---

## 1. System Architecture Overview

To achieve an interactive, 60 FPS user experience without thread-locking the browser UI, the system must strictly isolate execution layers. No physical file I/O or global state is permitted inside the computational core.

```
+-------------------------------------------------------------------------+
|                              MAIN THREAD                                |
|  [GUI Layer] - React/Vue/TS UI, Leaflet/Mapbox GIS, Canvas/D3 Plots     |
+-------------------------------------------------------------------------+
         |                                                 ^
         | (Transferable Arrays)                           | (Transferable Arrays)
         | Event: "RUN_SIMULATION"                         | Event: "SIM_COMPLETE"
         v                                                 |
+-------------------------------------------------------------------------+
|                        WEB WORKER THREAD (Background)                   |
|  [Worker Wrapper] - Message Event Listener, WASM Memory Orchestrator    |
|                                                                         |
|    +---------------------------------------------------------------+    |
|    |                      WASM CORE ENGINE                         |    |
|    |  [Rust/C++ Layer] Stateless Geometry & Numerical Solvers       |    |
|    |                                                               |    |
|    |  +--------------------+             +----------------------+  |    |
|    |  | Geometry Processor | ----------->| Hydraulic Solvers    |  |    |
|    |  | (Raw XS -> Curves) |             | (Steady / Unsteady)  |  |    |
|    |  +--------------------+             +----------------------+  |    |
|    +---------------------------------------------------------------+    |
+-------------------------------------------------------------------------+

```

---

## 2. Core Computational Modules (WASM Engine)

The engine must be written as a pure, stateless library. It exposes deterministic functions that ingest flat arrays and return flat arrays.

### Module A: Cross-Section Geometry Processor

* **Objective:** Transform arbitrary $X,Y$ coordinate cross-sections into discrete hydraulic lookup tables before running numerical simulations.
* **Inputs:**
* Flat array of $X$ stations (`Float64Array`).
* Flat array of $Y$ elevations (`Float64Array`).
* Manning's $n$ break points (`Float64Array` of stations, `Float64Array` of values).


* **Processing Pipeline:**
1. For a given cross-section, identify global $Y_{min}$ and $Y_{max}$.
2. Slice the vertical domain into $N$ calculation slices (default $N=100$).
3. At each slice elevation ($y_i$), calculate polygon intersections to resolve:
* **Area ($A$):** Flow area.
* **Wetted Perimeter ($P$):** Perimeter touching channel boundary.
* **Top Width ($T$):** Surface width of water.
* **Conveyance ($K$):** Evaluated using composite Manning's equation if $n$ varies across the section:

$$K = \frac{1.486}{n} A R^{2/3}$$






* **Outputs:** A unified, flat geometric lookup array per cross-section stored in WASM linear memory.

### Module B: Steady-State Solver (Gradually Varied Flow)

* **Objective:** Compute backwater and drawdown curves using the 1D Energy Equation and the Standard Step Method.
* **Mathematical Formulation:**

$$y_2 + z_2 + \alpha_2 \frac{V_2^2}{2g} = y_1 + z_1 + \alpha_1 \frac{V_1^2}{2g} + h_e$$



Where friction loss ($h_f$) between cross-sections is approximated via average conveyance:

$$h_f = L \bar{S}_f = L \left( \frac{Q}{\bar{K}} \right)^2$$


* **Numerical Implementation:**
* **Root Finder:** Newton-Raphson or Bisection method targeting the unknown Water Surface Elevation ($WSEL$) at the next section.
* **Directionality:**
* *Subcritical Flow Regime:* Step sequentially from downstream to upstream.
* *Supercritical Flow Regime:* Step sequentially from upstream to downstream.
* *Mixed Regime:* Evaluate critical depth ($y_c$) at each section to detect hydraulic jumps.





### Module C: Unsteady-State Solver (Dynamic Routing)

* **Objective:** Solve the 1D Saint-Venant equations for transient wave routing.
* **Mathematical Formulation:**

$$\text{Continuity: } \frac{\partial A}{\partial t} + \frac{\partial Q}{\partial x} = 0$$


$$\text{Momentum: } \frac{\partial Q}{\partial t} + \frac{\partial}{\partial x} \left( \frac{Q^2}{A} \right) + gA \left( \frac{\partial y}{\partial x} - S_0 + S_f \right) = 0$$


* **Numerical Implementation:**
* **Discretization:** Preissmann four-point implicit finite-difference scheme.
* **Matrix Structure:** The network topology yields a sparse, tridiagonal linear system at each time step.
* **Linear Solver:** Utilize the **Thomas Algorithm** ($O(N)$ time complexity) to solve the sparse matrix array directly without dense inversion matrix operations.



---

## 3. Data Architecture & WASM Interoperability

To eliminate performance degradation caused by JSON serialization across the Web Worker boundary, all data must be transferred as raw typed arrays.

### Input/Output Memory Management

* **No Objects Over the Bridge:** Do not compile complex structs across the boundary. Expose flat C-style pointer APIs via `wasm-bindgen` (Rust) or `EMSCRIPTEN_KEEPALIVE` (C++).
* **Transferables:** Wrap array buffers in JavaScript `Transferable Objects` inside `postMessage()`. This clears the memory buffer from the main thread and maps it directly to the Worker thread with zero copy penalty.

```javascript
// Example Main Thread Orchestration
const geometryBuffer = new Float64Array(xsData points);
worker.postMessage({
    type: 'RUN_STEADY',
    payload: { geometry: geometryBuffer }
}, [geometryBuffer.buffer]); // Zero-copy optimization

```

---

## 4. UI Layer Architecture (Main Thread)

The frontend GUI must remain lean, serving only as a visual shell for data presentation and creation.

* **GIS View:** Mapbox GL or Leaflet tracking reach alignments and cross-section cutlines via GeoJSON structures.
* **Profile/XS Canvas Plotter:** Use custom HTML5 Canvas rendering loops or high-performance visualization packages (e.g., `uPlot` or `D3.js`) to plot cross-sections and HGL profiles instantly during computation.

---

## 5. Development Milestones & Phased Execution

```
                       [PROJECT PHASING SCHEDULE]
  
  MILESTONE 1: Core Geometry Engine
  ├─ Implement XS Polygon Slicing Loop
  └─ Output A, P, T, K Lookup Tables 
  
  MILESTONE 2: Steady State Math Verification
  ├─ Implement Standard Step Root Finder
  └─ Verify against standard M1, M2, S1 backwater curves
  
  MILESTONE 3: WASM Compilation & Worker Integration
  ├─ Configure wasm-pack / Emscripten build pipelines
  └─ Establish zero-copy Transferable Array data pipeline
  
  MILESTONE 4: GUI Framework Integration
  ├─ Canvas profile rendering engines
  └─ Interactive cross-section editor

```

---

## 6. Verification and Validation Controls

* **Unit Test Suite:** The engine must include a suite of automated unit tests compiled natively in the host language (C++/Rust).
* **Mathematical Baseline:** Validate engine results directly against standardized HEC-RAS verification datasets (e.g., uniform trapezoidal channels, standard step drawdown profiles). Computed water surface profiles must match established analytical baselines to within less than $0.01\text{ ft}$ ($0.003\text{ m}$).