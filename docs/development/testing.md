# Testing and verification

## Quick start (Python)

```bash
maturin develop --features python --release
python3 -m pytest python/
python3 scripts/run_verification_notebook.py
python3 verification/oracle/scripts/run_oracle_ci.py   # HEC-RAS linked oracle (no HEC install)
```

Full catalog: [`verification/README.md`](../../verification/README.md), [`verification/manifest.json`](../../verification/manifest.json).

## What is verified

| Layer | Examples |
|-------|----------|
| Python | `python/test_stream1d.py`, `python/test_hecras_culvert_verification.py`, `python/test_issaquah01_bridge_parity.py` |
| Verification notebook | `python/stream1d_verification.ipynb` — ConSpan, Issaquah01, **§6** simple_channel Q-ramp vs HEC-RAS |
| Linked oracle | `reach_mild` + `simple_channel` open channel; ConSpan mode 4 ramp ≤0.12 ft overall max \|Δ\| vs HEC |
| JSON golden fixtures | ConSpan steady ±0.04 ft; bridge abutment/BU/BD/high-flow ±2 mm |
| Rust unit/integration | Geometry modifiers, culvert/barrel hydraulics, bridge Class A/B/C and high flow |

## Common commands

```bash
python3 -m pytest python/
python3 scripts/run_verification_notebook.py
maturin develop --features python --release
python3 verification/oracle/scripts/run_oracle_ci.py
bash verification/run.sh
cargo test
```

Intentional HEC deltas: [`reference/hecras_parity.md`](../reference/hecras_parity.md).

Interactive notebook: [`python/stream1d_verification.ipynb`](../../python/stream1d_verification.ipynb) — also on [Binder](https://mybinder.org/v2/gh/jlillywh/STREAM-1D/main?filepath=python%2Fstream1d_verification.ipynb).

## Maintainers

Rust/WASM CI checks (`cargo test`, `tests/wasm_json_contract.rs`, `bash build_wasm.sh`) run in GitHub Actions; see [`.github/workflows/ci.yml`](../../.github/workflows/ci.yml).
