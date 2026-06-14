# Reach mild — open-channel unsteady gate

Eight ConSpan cross sections (RM 20.535–20.0), **no culvert**. Constant Q=1000 cfs, downstream stage 30.51 ft at RM 20.0.

| File | Role |
|------|------|
| `reach_mild.g01` | Geometry (trimmed from `conspan/ConSpan.g01`) |
| `reach_mild.u02` | Unsteady BCs |
| `reach_mild.p02` | Unsteady plan 02 |
| `reach_mild.prj` | Project index |
| `reference_wsel_reach_mild_unsteady.json` | Oracle reference (refresh via `run_ras_reference.py`) |

## Windows staging folder (GUI-friendly)

Projects are copied to:

`C:\Users\jason\Documents\hecras_testing\reach_mild\`

Override with env var `STREAM1D_HECRAS_STAGE` if needed.

## Phase 1 — GUI reference (Windows, one-time)

**Do not use headless scripts until Phase 1 passes.**

### 1. Prep (PowerShell)

```powershell
cd \\wsl.localhost\Ubuntu\home\jason\Lillywhite_Consulting\lillywhite_engine\STREAM-1D
py -3 verification\oracle\scripts\phase1_prep.py --open-ras
```

`--open-ras` launches HEC-RAS only. **Dismiss** any Windows “open .prj with…” dialog if it appears — open the project inside HEC-RAS instead.

### 2. Open in HEC-RAS (if not auto-opened)

**File → Open Project** → browse:

`Documents` → `hecras_testing` → `reach_mild` → `reach_mild.prj`

### 3. In HEC-RAS GUI

1. Confirm project opens with **no geometry errors**.
2. **Unsteady Flow Data** → verify upstream Q=1000 cfs (RM 20.535), DS stage=30.51 ft (RM 20.0) → **Save** `u02` if prompted.
3. Select **Plan 02** → **Run** (unsteady compute). Expect ~1–5 min.
4. Confirm `reach_mild.p02.hdf` in `Documents\hecras_testing\reach_mild\`.
5. Record **terminal** WSEL at RM **20.208**, **20.189**, **20.095**.

Bootstrap reference (for comparison): 31.60 / 31.48 / 31.00 ft.

### 4. Capture into repo

```powershell
py -3 verification\oracle\scripts\phase1_capture_after_gui.py
```

### 5. Confirm in WSL

```bash
python3 verification/oracle/scripts/run_phase0.py
```

---

Regenerate geometry from ConSpan:

```bash
python3 verification/oracle/scripts/bootstrap_reach_mild_project.py
```

Linked scenario: `scenarios/reach_mild_unsteady_linked.json`
