# Beaver Creek — Chunk 8 certification

> **Status:** `certification: development` — not a gate for Chunks 1–6.

## Scenario

| Field | Value |
|-------|--------|
| ID | `beaver_unsteady_linked` |
| Plan | 03 (100-yr unsteady) |
| Bridge | RM 5.4 — 9 piers, WSPRO, piecewise deck |
| Flow | Peak ~100-yr hydrograph; **high-flow** bridge regimes expected |
| Coupling | Mode `2` (implicit low-flow where eligible; **explicit fallback** for high-flow) |

## Prerequisites (Chunk 8.2)

| Requirement | Status |
|-------------|--------|
| Chunk 1 friction-slope downstream BC | ✅ `downstream_bc_type=2`, slope 0.002 from u02 |
| Chunk 6 implicit bridge (low-flow) | ✅ mode 2 in scenario; high-flow uses B3 explicit |
| Chunk 7 mode 1 outer loop | ⏸ Deferred — not required on synthetic gates |
| HDF WSEL($t$) at ≥10 RMs | ⏸ Run RAS 6.x + `extract_hdf_wsel.py` |
| Published gap table | ✅ `gap_table_beaver_unsteady.json` (from diagnose script) |

## Plan 03 alignment

| HEC-RAS | STREAM-1D |
|---------|-----------|
| Computation interval 2MIN | Q resampled from 1HOUR u02 → 2MIN |
| UNET θ = 1.0 | `theta` from plan parser |
| DS friction slope 0.002 | Normal-depth WSEL($Q$) on downstream XS |
| Observed HWM (10 RMs) | Dev reference for max WSEL compare |
| Initial WSEL | **`solve_steady`** at initial Q (500 cfs) + bridge + friction-slope DS BC |

## Verify

```bash
source .venv/bin/activate
maturin develop --features python
bash verification/oracle/scripts/run_chunk8_verify.sh
```

Or stepwise:

```bash
python3 verification/oracle/scripts/smoke_beaver_parse.py
python3 verification/oracle/scripts/test_beaver_unsteady_warm_start.py
python3 verification/oracle/scripts/diagnose_beaver_unsteady.py
bash verification/oracle/run_oracle.sh \
  --scenario verification/oracle/scenarios/beaver_unsteady_linked.json
```

## HDF certification path

1. Run plan 03 in HEC-RAS 6.x with Write HDF5 enabled.
2. `python3 verification/oracle/scripts/extract_hdf_wsel.py --hdf ... --scenario ... --out projects/beaver/reference_wsel_hdf_plan03.json`
3. Update scenario `reference.source` to `hdf_timeseries` when JSON is populated.
4. Re-run diagnose; promote `certification` to `candidate` or `certified` in scenario JSON.

## Gap table

After `diagnose_beaver_unsteady.py`, see [`gap_table_beaver_unsteady.json`](gap_table_beaver_unsteady.json) for per-RM Δ vs Observed HWM (mode 0 and mode 2). **No silent mapper fudge** — FAIL is published with full deltas.
