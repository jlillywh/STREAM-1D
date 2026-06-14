# Verification against external sources of truth

This folder is the **canonical catalog** of STREAM-1D regression benchmarks. Each case compares solver output to an independent reference: **HEC-RAS model exports**, **hand-derived hydraulics**, or **published equation forms** (FHWA, Bradley, WSPRO).

Fixtures live in [`fixtures/`](fixtures/). Machine-readable index: [`manifest.json`](manifest.json).

## Quick start

```bash
# Run all HEC-RAS / golden regression tests (Rust)
bash verification/run.sh

# Linked HEC-RAS oracle — STREAM-1D vs bundled RAS project (optional; needs Python ext)
bash verification/oracle/run_oracle.sh

# Culvert profiles only (Python)
PYTHONPATH=python python python/test_hecras_culvert_verification.py

# Interactive plots (Binder or local Jupyter)
jupyter notebook python/stream1d_verification.ipynb
```

## Benchmark summary

| Suite | External source | Fixture | Test harness | Tolerance |
|-------|-----------------|---------|--------------|-----------|
| **ConSpan culvert reach** | HEC-RAS steady profiles (5 / 25 / 50 yr) | [`fixtures/hecras_conspan_profiles.json`](fixtures/hecras_conspan_profiles.json), [`fixtures/conspan_project_12.json`](fixtures/conspan_project_12.json), [`fixtures/ConSpan.csv`](fixtures/ConSpan.csv) | `tests/culvert_hecras_verification.rs`, `python/test_hecras_culvert_verification.py` | ±0.04 ft WSEL |
| **Bridge abutments (WSPRO)** | Hand-calc geometry + WSPRO energy reference HW | [`fixtures/bridge_abutment_hecras.json`](fixtures/bridge_abutment_hecras.json) | `tests/bridge_abutment_hecras_verification.rs` | ±2 mm HW |
| **Bridge BU/BD faces** | HEC-RAS Yarnell / explicit face cuts | [`fixtures/bridge_bu_bd_hecras.json`](fixtures/bridge_bu_bd_hecras.json) | `tests/bridge_bu_bd_hecras_verification.rs` | ±2 mm HW |
| **Bridge high flow** | HEC-RAS 6.x high-flow methodology (sluice, orifice, Bradley weir, energy) | [`fixtures/bridge_high_flow_hecras.json`](fixtures/bridge_high_flow_hecras.json) | `tests/bridge_high_flow_hecras_verification.rs` | ±2 mm HW |
| **Guide-bank contraction** | Steady profile vs reach-only contraction coefficient | [`fixtures/bridge_guide_bank_contraction.json`](fixtures/bridge_guide_bank_contraction.json) | `tests/bridge_guide_bank_contraction_verification.rs` | ±2 mm WSEL |
| **Friction weighting (v30)** | Steady profile: omit/`0` vs `1` at same $Q$ (HEC-RAS §4.2) | [`fixtures/bridge_friction_weighting_hecras.json`](fixtures/bridge_friction_weighting_hecras.json) | `tests/bridge_friction_weighting_hecras_verification.rs` | ±5 mm (omit≡0); segments > opening |
| **Roadway embankment (v26)** | Hand-authored equivalent to decomposed flat fields | [`fixtures/bridge_roadway_embankment.json`](fixtures/bridge_roadway_embankment.json) | `tests/bridge_roadway_embankment_verification.rs` | ±2 mm WSEL |
| **Opening alignment** | Preprocessor + skew/offset geometry invariants | (inline in test) | `tests/bridge_opening_alignment_verification.rs` | exact layout |

## Source-of-truth types

| Type | Description | Examples here |
|------|-------------|---------------|
| **HEC-RAS export** | WSEL or geometry taken from a HEC-RAS 6.x model run | ConSpan culvert profiles, ConSpan.csv |
| **Linked HEC-RAS project** | Bundled `.g01` + plan/flow; STREAM-1D mapped from same geometry; compare via [`oracle/`](oracle/) | ConSpan steady linked scenario |
| **Hand-calibrated golden** | Reference HW derived from the same equation set HEC-RAS uses, documented per case in JSON `notes` | Bridge abutment, high-flow cases |
| **Independent hand check** | Sub-step verified (e.g. submerged area) before comparing full solve | Abutment `expected_a_eff_tw_m2` |

Golden values are **frozen** in JSON. Changing the solver requires updating fixtures deliberately and re-documenting `notes`.

## Adding a benchmark

1. Add JSON (or CSV + JSON) under `verification/fixtures/` with `notes`, tolerances, and expected values.
2. Add a row to [`manifest.json`](manifest.json).
3. Add or extend a `tests/*_verification.rs` harness (pattern: `include_str!("../verification/fixtures/…")`).
4. Register in [`docs/development/testing.md`](../docs/development/testing.md) and the root [`README.md`](../README.md) verification table.

## Linked HEC-RAS oracle

For trust-building comparisons against **live or bundled HEC-RAS project files** (not only frozen JSON goldens), see **[`oracle/README.md`](oracle/README.md)**.

```bash
bash verification/oracle/run_oracle.sh
```

Requires the Python extension; optional live HEC-RAS re-run via `--live-ras` when ras-commander and HEC-RAS are installed locally.

**Scenarios:**

| Scenario | Mode | Linked project |
|----------|------|----------------|
| `scenarios/conspan_steady_linked.json` | Steady culvert | `projects/conspan/` |
| `scenarios/beaver_unsteady_linked.json` | Unsteady bridge | `projects/beaver/` |

```bash
# Beaver Creek unsteady bridge (Observed HWM from beaver.u02)
bash verification/oracle/run_oracle.sh \
  --scenario verification/oracle/scenarios/beaver_unsteady_linked.json
```

## What is *not* here

- **Unit tests** (`src/**`, `#[test]` in modules) — internal consistency, not external truth.
- **WASM contract tests** — schema/metadata only.
- **Full HEC-RAS project builder** — linked verify uses bundled or user-supplied `.g01` + plan/flow; STREAM-1D inputs are mapped fixtures, not auto-generated from `.g01` at runtime (yet).

Known intentional deltas vs HEC-RAS (deck vents, energy-path limits, etc.): [`docs/development/pressure_weir_combined_flow_audit.md`](../docs/development/pressure_weir_combined_flow_audit.md#intentional-remaining-deltas).

## Legacy path

Fixtures previously lived under `python/verification/`. That directory now redirects here; use **`verification/fixtures/`** for all new work.
