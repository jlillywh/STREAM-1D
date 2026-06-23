# Beaver Creek

Unsteady inline **bridge** example (9 piers, WSPRO, piecewise deck). **Development only** — not part of CI.

## Recommended workflow (no live HEC-RAS)

Headless ras-commander from WSL is **unreliable** (GUI dialogs, hung `Ras.exe`, polluted stage folders). For parity work, **do not depend on it**.

| Goal | Command |
|------|---------|
| Isolate bridge physics (mapping → steady → unsteady) | `python3 verification/oracle/scripts/diagnose_beaver_restart.py` |
| Compare STREAM-1D vs committed RAS reference (~5 s) | `bash verification/oracle/scripts/run_beaver_verify.sh` |
| Mapping smoke only | `python3 verification/oracle/scripts/smoke_beaver_parse.py` |

Committed reference: `reference_wsel_hdf_plan03.json` (terminal WSEL from plan 03 HDF). Scenario sets `live_ras_optional: true` — refreshing it is a **manual, rare** step, not part of the dev loop.

### Refresh reference (manual, when g01/u02/p03 change)

1. Open `beaver.prj` in HEC-RAS 7.0.1 on Windows.
2. Run **plan 03** from the GUI (~3–8 min). Confirm `beaver.p03.hdf` exists.
3. Extract checkpoints:

```bash
python3 verification/oracle/scripts/extract_hdf_wsel.py \
  --hdf /path/to/beaver.p03.hdf \
  --out verification/oracle/projects/beaver/reference_wsel_hdf_plan03.json
```

4. Commit the updated JSON. Re-run `run_beaver_verify.sh`.

Regenerate `beaver.u02` before a GUI run if hydrograph format was touched:

```bash
python3 verification/oracle/scripts/write_beaver_u02.py
```

### Steady profiles (plan 01 / f01)

Three profiles aligned with unsteady diagnostic flows — upstream Q at RM 5.99, downstream friction slope 0.002 at RM 5.0:

| Profile | Q (cfs) |
|---------|---------|
| Initial | 500 |
| Peak | 14,000 |
| Recession | 6,181 |

```bash
python3 verification/oracle/scripts/write_beaver_steady.py   # regenerate f01 + p01
```

**HEC-RAS GUI:** open `beaver.prj` → select **plan 01** → Run. Steady Flow Editor shows `beaver.f01`. Output: `beaver.p01.hdf` (HDF5 enabled).

Compare STREAM-1D steady: `diagnose_beaver_restart.py --skip-unsteady` (Layer 2).

## Layered diagnostic (restart 2026)

The old Chunk 8 pass/fail gate compared full 100-yr unsteady max WSEL to Observed HWM in the u02. After major bridge API changes that comparison is no longer meaningful as a gate.

Use the **layered restart** instead:

| Layer | Script | What it isolates |
|-------|--------|------------------|
| 1 Mapping | `smoke_beaver_parse.py` | g01 → bridge fields, BU/BD faces |
| 2 Steady | `diagnose_beaver_restart.py` | `solve_steady` at initial Q and peak Q vs HWM |
| 3 Unsteady | `diagnose_beaver_restart.py` | Full hydrograph max WSEL (diagnostic) |

```bash
# Quick mapping check
python3 verification/oracle/scripts/smoke_beaver_parse.py

# Full layered diagnostic (writes restart_report_beaver.json)
python3 verification/oracle/scripts/diagnose_beaver_restart.py

# Skip slow unsteady layer
python3 verification/oracle/scripts/diagnose_beaver_restart.py --skip-unsteady

# All layers + warm-start internal check
bash verification/oracle/scripts/run_chunk8_verify.sh
```

**Certification target:** HDF timeseries at checkpoint RMs — compare via `run_beaver_verify.sh`, not Observed HWM alone.

### Headless HEC-RAS (optional — not recommended on WSL)

Automated ras-commander runs sometimes complete in ~15 s and sometimes hang indefinitely (error dialogs, `RasPlotDriver.exe`, stale stage dir). Treat as **experimental** only.

```bash
# Only if you accept debugging hung Ras.exe / stage pollution:
HECRAS_RUN_TIMEOUT_SEC=3600 bash verification/oracle/scripts/run_beaver_ras.sh --no-verify
# Kill stuck processes:
#   taskkill.exe /F /IM Ras.exe /IM RasPlotDriver.exe /IM PipeServer.exe
# Reset polluted stage:
#   rm -rf "$USERPROFILE/Documents/hecras_testing/beaver"   # Windows path
```

Prefer the **GUI + extract** path above for reference refresh.
Upstream hydrograph in `beaver.u02` must have **49** ordinates (48 h @ 1HOUR) with **8-character fixed-width** flow fields (see `scripts/write_beaver_u02.py`); preflight catches count mismatches before headless RAS.
Project is staged to `%USERPROFILE%\Documents\hecras_testing\beaver` on WSL runs.

Scenario: `scenarios/beaver_unsteady_linked.json`.

Sync from sibling web repo (optional): `bash verification/oracle/scripts/sync_linked_projects.sh`
