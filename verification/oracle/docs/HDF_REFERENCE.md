# HEC-RAS HDF reference extraction (linked verify)

**Required for:** Parity roadmap **Chunk 4+** (unsteady linked verify with WSEL($t$)).  
**Program setup:** [`../PARITY_PROGRAM.md`](../PARITY_PROGRAM.md)

---

## Reference hierarchy

| Source | Use | Certification |
|--------|-----|----------------|
| **Plan HDF** | WSEL($t$) at river miles / cross sections | **Preferred** for unsteady |
| **Profile CSV** | Steady WSEL vs station | ConSpan steady (Chunk 3–4) |
| **Observed HWM** (`.u02`) | Peak WSEL at listed RMs | **Development only** until BC + numerics aligned |
| **Hand-entered JSON** | Regression goldens | Frozen fixtures, not linked certify |

---

## Workflow (local, HEC-RAS 6.x)

### 1. Run plan

With bundled project under `verification/oracle/projects/<name>/`:

```bash
# Requires HEC-RAS 6.x + ras-commander
pip install ras-commander h5py pandas

python - <<'PY'
from pathlib import Path
from ras_commander import RasPrj, RasCmdr

project_dir = Path("verification/oracle/projects/beaver")  # example
RasPrj(project_dir)
RasCmdr.compute_plan("03")
print("Done — HDF next to project or in plan folder")
PY
```

A full `.prj` may be required depending on install; create once in HEC-RAS GUI if `RasPrj` fails.

### 2. Locate HDF

After unsteady post-processing, RAS writes HDF5 (name/plan dependent), commonly:

- `<project>.pXX.hdf` in the project directory, or
- Path returned by ras-commander plan metadata

Enable **Write HDF5** in plan if missing (Beaver plan 03 may need migration on 6.x).

### 3. Extract WSEL time series

**Planned harness script:** `verification/oracle/scripts/extract_hdf_wsel.py` (Chunk 2–4).

Until shipped, extract manually:

```python
import h5py
import numpy as np

hdf_path = "path/to/plan.hdf"
# Structure varies by RAS version — inspect:
with h5py.File(hdf_path, "r") as f:
    def visit(name, obj):
        if isinstance(obj, h5py.Dataset) and "WSEL" in name.upper():
            print(name, obj.shape)
    f.visititems(visit)
```

For **HEC-RAS 6.x** unsteady results, datasets often live under paths like:

- `/Results/Unsteady/Output/Output Blocks/Base Output/Unsteady Time Series/Cross Sections/...`

Use ras-commander helpers when available (`RasHdf` / plan result accessors — check installed version docs).

### 4. Normalize to oracle compare format

Target JSON for linked verify (future):

```json
{
  "schema_version": 1,
  "source": "hecras_hdf",
  "hecras_version": "6.6",
  "plan": "03",
  "checkpoints": [
    { "river_mile": 5.99, "times_s": [0, 120, 240], "wsel_ft": [220.0, 220.1, 220.2] }
  ]
}
```

Compare modes:

- `max_wsel` — max over time per RM (current Beaver oracle)
- `wsel_timeseries` — RMSE or max abs Δ per RM (Chunk 4)

---

## Integration with `run_linked_verify.py`

| Flag / reference | Behavior |
|------------------|----------|
| `reference.source: linked_export` | Steady CSV (implemented) |
| `reference.source: linked_u02_observed_hwm` | Parse Observed HWM lines (implemented) |
| `reference.source: hdf_timeseries` | Load normalized JSON from `reference.hdf_extract` (**Chunk 4**) |
| `--live-ras` | Run plan via ras-commander; extract when script exists |

---

## Environment notes

- **WSL / Windows:** Run HEC-RAS and ras-commander on the host where RAS is licensed; mount project path consistently.
- **CI:** Linked HDF scenarios should **commit extracted JSON** (small) rather than HDF binaries (large); optional nightly live-ras job.
- **Legacy projects:** Beaver 4.x/5.x geometry may need HEC-RAS 6.x migration before HDF path is stable — record `hecras_version` in extract metadata.

---

## Related

- Live run stub: [`../lib/ras_reference.py`](../lib/ras_reference.py) (`try_live_ras_run`)
- Scenario schema: [`../schemas/linked_scenario.schema.json`](../schemas/linked_scenario.schema.json)
