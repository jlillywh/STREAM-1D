# Verification

Regression benchmarks against external references: **HEC-RAS exports**, **bundled RAS projects**, or **hand-derived goldens**.

```bash
bash verification/run.sh                              # Rust fixture suites
bash verification/oracle/run_oracle.sh                # linked HEC-RAS oracle
python3 verification/oracle/scripts/run_oracle_ci.py  # CI gate (no HEC install)
PYTHONPATH=python python python/test_hecras_culvert_verification.py
```

Fixtures: [`fixtures/`](fixtures/). Index: [`manifest.json`](manifest.json).

## Suites

| Suite | Reference | Harness |
|-------|-----------|---------|
| ConSpan culvert (steady) | HEC-RAS profiles | `tests/culvert_hecras_verification.rs` |
| Bridge (abutment, BU/BD, high flow, …) | JSON goldens | `tests/bridge_*_verification.rs` |
| Linked oracle | Bundled `.g01` + committed WSEL | [`oracle/README.md`](oracle/README.md) |

## Linked oracle (high level)

STREAM-1D inputs are mapped from the same geometry as a bundled HEC-RAS project. Compare WSEL (steady profile or unsteady timeseries) against a committed reference file.

| Check | Scenario | Notes |
|-------|----------|-------|
| Open channel (known stage DS) | `reach_mild_unsteady_linked.json` | No structures; CI via `run_oracle_ci.py` |
| Open channel (constant Q) | `simple_channel_unsteady_linked.json` | 4-XS trapezoid; friction-slope DS; CI |
| Open channel (Q ramp) | `simple_channel_ramp_unsteady_linked.json` | Same geometry; Q ramp transient; CI |
| Culvert steady | `conspan_steady_linked.json` | ±0.04 ft certification |
| Culvert unsteady (mode 4) | `conspan_unsteady_ramp_matrix_mode4.json` | **CI gate** — overall max \|Δ\| ≤ 0.12 ft vs HEC |
| Culvert unsteady (mode 2) | `conspan_unsteady_ramp_matrix.json` | Diagnostic matrix only |

Details: [`oracle/README.md`](oracle/README.md).

## Adding a fixture benchmark

1. Add JSON under `fixtures/` with `notes`, tolerances, expected values.
2. Add a row to `manifest.json`.
3. Add `tests/*_verification.rs` harness.
4. Mention in [`docs/development/testing.md`](../docs/development/testing.md) if user-facing.

Intentional STREAM vs HEC-RAS deltas: [`docs/reference/hecras_parity.md`](../docs/reference/hecras_parity.md).
