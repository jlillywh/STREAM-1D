# Testing and verification

## Quick start

```bash
cargo test
bash verification/run.sh
maturin develop --features python --release
python3 verification/oracle/scripts/run_oracle_ci.py   # linked HEC oracle gate
```

Full catalog: [`verification/README.md`](../verification/README.md), [`verification/manifest.json`](../verification/manifest.json).

## What is verified

| Layer | Examples |
|-------|----------|
| Rust unit/integration | Geometry modifiers, culvert/barrel hydraulics, bridge Class A/B/C and high flow |
| JSON golden fixtures | ConSpan steady ±0.04 ft; bridge abutment/BU/BD/high-flow ±2 mm |
| Linked oracle | `reach_mild` + `simple_channel` open channel (constant Q and Q ramp); ConSpan mode 4 ramp ≤0.12 ft overall max \|Δ\| vs HEC |
| WASM contract | `tests/wasm_json_contract.rs` — schema version, deserialize samples |
| Python | `python/test_stream1d.py`, `python/test_hecras_culvert_verification.py`, `python/test_issaquah01_bridge_parity.py` |
| Verification notebook | CI executes `python/stream1d_verification.ipynb` headlessly — ConSpan culvert and Issaquah01 bridge WSE/EGL vs HEC-RAS (requires `maturin develop`) |

## Common commands

```bash
cargo test --test culvert_hecras_verification
cargo test --test bridge_high_flow_hecras_verification
cargo test --test wasm_json_contract
bash build_wasm.sh && node examples/wasm/bridge_smoke_test.mjs
./scripts/run_coverage.sh
```

Intentional HEC deltas: [`reference/hecras_parity.md`](../reference/hecras_parity.md).

Interactive notebook: [`python/stream1d_verification.ipynb`](../../python/stream1d_verification.ipynb) — run from repo root via `python3 scripts/run_verification_notebook.py` (CI executes it headlessly).
