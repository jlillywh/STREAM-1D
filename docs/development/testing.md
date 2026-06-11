# Testing and verification

## Testing & Verification

### 1. HEC-RAS Profile Verification (ConSpan Dataset)
STREAM-1D includes a verification dataset under `python/verification/` extracted from a HEC-RAS model of a channel reach featuring a $28\text{ ft} \times 6\text{ ft}$ ConSpan arch culvert with a composite bottom-roughness layer. Reference WSEL values for **5 yr** ($Q=250\text{ cfs}$), **25 yr** ($Q=600\text{ cfs}$), and **50 yr** ($Q=1000\text{ cfs}$) profiles are in [`python/verification/hecras_conspan_profiles.json`](python/verification/hecras_conspan_profiles.json) (sourced from [`ConSpan.csv`](python/verification/ConSpan.csv)).

All profile stations (10 per event) are checked within **±0.04 ft** vs HEC-RAS (Rust: `tests/culvert_hecras_verification.rs`; Python: `python/test_hecras_culvert_verification.py`).

### 2. Bridge verification

| Check | Tests | Reference |
|-------|-------|-----------|
| Yarnell Class A pier loss | `src/solvers/bridge.rs` (`test_yarnell_pier_head_loss_hec_ras`) | Closed-form HEC-RAS formula ($H_{3-2} \approx 0.00247\text{ m}$ on 10 m channel, $Q=15\text{ cms}$, two 0.5 m piers) |
| Per-side abutment geometry | `src/solvers/bridge_abutment.rs`, `bridge.rs` | Hand-calc submerged area (asymmetric, one-sided) |
| WSPRO headwater with abutments | `tests/bridge_abutment_hecras_verification.rs` | [`python/verification/bridge_abutment_hecras.json`](python/verification/bridge_abutment_hecras.json) — ±2 mm on HW |
| Explicit BU/BD faces (v22) | `tests/bridge_bu_bd_hecras_verification.rs` | [`python/verification/bridge_bu_bd_hecras.json`](python/verification/bridge_bu_bd_hecras.json) — legacy Yarnell ±2 mm; explicit BU/BD + WSPRO golden HW |
| 3-section vs 2-face reach layout | `tests/bridge_bu_bd_hecras_verification.rs` (`three_section_bridge_reach_matches_two_face_baseline`) | BU+internal+BD inserts extra node; BU/BD headwater and friction path match 2-face baseline |
| WASM / JSON contract | `tests/wasm_json_contract.rs` | Steady BU/BD v22 fixture, unsteady BU/BD deserialize, `ineffective_flow_areas` on `CrossSection`; `api_version` metadata |
| Reach ineffective flow | `src/geometry/processor.rs`, `src/solvers/bridge_tests.rs`, `src/solvers/steady.rs` | Blocked vs ineffective semantics; approach-cut storage/conveyance split; BU ineffective headwater; `solve_step` modifier search (plain, ineffective, supercritical, blocked obstruction, mixed regime) |
| Unified roadway embankment (v26) | `tests/bridge_roadway_embankment_verification.rs`, `src/solvers/bridge_roadway_compose.rs`, `tests/wasm_json_contract.rs` | Steady/unsteady/rating compose; precedence (`derive_*`, flat wins, face overrides); blocked merge at solve; JSON contract; WSEL parity vs decomposed flat fields |
| Node WASM smoke | `examples/wasm/bridge_smoke_test.mjs`, `node_smoke_test.mjs` | Culvert Tier 1 + bridge BU/BD steady solve after `build_wasm.sh` |

```bash
cargo test --test bridge_abutment_hecras_verification
cargo test --test bridge_bu_bd_hecras_verification
cargo test --test bridge_roadway_embankment_verification
cargo test bridge_abutment --lib
cargo test --test wasm_json_contract
node examples/wasm/bridge_smoke_test.mjs   # requires pkg-node from build_wasm.sh
```

### 3. Culvert verification

Culvert hydraulics are covered by **76** automated tests (unit, integration, and HEC-RAS benchmarks) across `src/solvers/`, `tests/culvert_hecras_verification.rs`, and `tests/wasm_json_contract.rs`, including:

| Configuration | What is tested |
|---------------|----------------|
| Shapes | Circular, box, arch, ConSpan, pipe-arch, elliptical, horseshoe geometry and full solves |
| Inlet types | All FHWA nomograph codes per shape |
| Control regimes | Inlet, outlet, full/partial roadway overtopping |
| Barrel slope | Adverse, flat, and downhill grade |
| Blockage & roughness | Sediment `depth_blocked`, composite bottom *n* |
| Multi-barrel | Active barrel count, uniform and per-barrel geometry, capacity-based $Q$ split |
| Skew | Projected span / friction length, 59° clamp |
| Diagnostics & rating curve | Extended outputs; monotonic HW vs $Q$ for all shapes; `solve_culvert_from_headwater` round-trip |
| Reach integration (steady) | `solve_steady` with skew, blocked barrels, per-barrel spans, sediment |
| Supercritical routing | `regime` 1/2 culvert intervals (US Customary + Metric); bridge `solve_bridge_tailwater` |
| Unsteady inline | `solve_unsteady` with culvert coupling + per-step culvert diagnostics (Metric + US Customary) |
| HEC-RAS ConSpan | 5/25/50 yr profiles — 10 stations each, ±0.04 ft (`hecras_conspan_profiles.json`) |
| Point culvert benchmarks | Circular inlet/outlet, box inlet, multi-barrel, adverse grade (`tests/fixtures/culvert_point_benchmarks.json`) |

WASM JSON contract tests and Python pytest cases provide additional coverage. Example fixtures: [`tests/fixtures/wasm_steady_culvert_tier1.json`](tests/fixtures/wasm_steady_culvert_tier1.json) (culvert Tier 1), [`tests/fixtures/wasm_steady_bridge_bu_bd_v22.json`](tests/fixtures/wasm_steady_bridge_bu_bd_v22.json) (bridge BU/BD + internal cut).

CI uploads coverage to [Codecov](https://codecov.io) on every push/PR (`.github/workflows/ci.yml`).

### 4. Running the Test Suites

* **Coverage + tests (recommended before commit):**
  ```bash
  ./scripts/install_git_hooks.sh   # once per clone — enables pre-commit hook
  ./scripts/run_coverage.sh        # manual: tests + lcov.info (same as CI)
  ```
* **Rust unit and integration tests:**
  ```bash
  cargo test
  cargo test --test wasm_json_contract
  cargo test --test bridge_abutment_hecras_verification
  cargo test --test bridge_bu_bd_hecras_verification
  cargo test --test culvert_hecras_verification
  ```
* **WASM build + smoke tests** (culvert + bridge BU/BD):
  ```bash
  bash build_wasm.sh
  ```
* **Python pytest suite** (rebuild the native extension after pulling engine changes):
  ```bash
  maturin develop --features python
  PYTHONPATH=python pytest -c /dev/null python/test_stream1d.py
  ```
* **Python HEC-RAS verification (ConSpan 5/25/50 yr profiles):**
  ```bash
  PYTHONPATH=python python python/test_hecras_culvert_verification.py
  ```
* **Rust HEC-RAS + point culvert benchmarks:**
  ```bash
  cargo test --test culvert_hecras_verification
  ```
* **Python bindings smoke test:**
  ```bash
  PYTHONPATH=python python python/test_python_bindings.py
  ```

## Interactive notebook

## Interactive Jupyter Notebook & Binder

To run calculations, view water surface profile charts, and inspect tables interactively on the web without any local installation:

[![Binder](https://mybinder.org/badge_logo.svg)](https://mybinder.org/v2/gh/jlillywh/STREAM-1D/main?filepath=python%2Fstream1d_verification.ipynb)

* **Interactive Notebook:** [python/stream1d_verification.ipynb](python/stream1d_verification.ipynb)
* Click the **Binder** badge above to launch a sandbox environment in your browser. The first launch compiles Rust and may take **5–10 minutes**; later launches reuse the cached image.

