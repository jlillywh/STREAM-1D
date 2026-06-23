# Simple trapezoidal channel — Chunk 1 friction-slope DS

Four trapezoidal cross sections (RM 3.0–0.0), bed slope **0.001 ft/ft**, Manning **n = 0.03**.

| File | Role |
|------|------|
| `simple_channel.g01` | Geometry |
| `simple_channel.u02` | Unsteady BCs — Q=150 cfs upstream, **Friction Slope=0.001** at RM 0.0 |
| `simple_channel.p01` | Unsteady plan 01 (θ=1, 48 h) |
| `simple_channel.prj` | Project index |
| `reference_wsel_simple_channel_unsteady.json` | Oracle reference (from Plan 01 HDF) |

## Forcing

- Upstream RM **3.0**: constant **Q = 150 cfs** (49 × 1 hour)
- Downstream RM **0.0**: **friction slope S₀ = 0.001** (matches channel bed)
- Plan 01: unsteady, θ = 1

At terminal time, WSEL should match a steady normal-depth profile (Chunk 1 type-2 BC certification).

## Windows staging

GUI-friendly copies are staged under **`%USERPROFILE%\Documents\hecras_testing\<project>`** by default, or set **`STREAM1D_HECRAS_STAGE`** to override the root folder (see `lib/stage_paths.py`).

For this project: `%USERPROFILE%\Documents\hecras_testing\simple_channel\`

## Chunk 1 workflow

### 1. Prep (PowerShell, from repository root)

```powershell
$env:HECRAS_RAS_EXE = "C:\Program Files (x86)\HEC\HEC-RAS\7.0.1\Ras.exe"
py -3 verification\oracle\scripts\chunk1_simple_channel_prep.py --open-ras
```

### 2. HEC-RAS GUI

1. Open the staged `simple_channel.prj` under your `hecras_testing` folder
2. **Unsteady Flow Data** → confirm RM **3.0** flow 150 cfs, RM **0.0** **Friction Slope 0.001**
3. **Run Plan 01** → expect `simple_channel.p01.hdf`

### 3. Capture (PowerShell)

```powershell
py -3 verification\oracle\scripts\chunk1_simple_channel_capture.py
# Or pass an explicit HDF path:
# py -3 verification\oracle\scripts\chunk1_simple_channel_capture.py --hdf path\to\simple_channel.p01.hdf
```

### 4. Verify (Linux / macOS)

```bash
python3 verification/oracle/scripts/run_chunk1_simple_channel.py
```

**Pass:** max |Δ| ≤ **0.05 ft** at RM 3.0, 2.0, 1.0, 0.0.

## Regenerate u02

```bash
python3 verification/oracle/scripts/write_simple_channel_u02.py
```

**u02 layout:** compact `reach_mild` pattern; downstream must be `Friction Slope=0.001,0` (HEC-RAS 7 1D Normal Depth flag). Prep runs `ras-commander` `set_normal_depth_boundary` when available to strip any stray Stage Hydrograph.

**If GUI still shows empty Stage Hydrograph:** the staged copy is stale. Run prep (it wipes that folder), reopen the project, or set RM 0.0 → **Normal Depth** / friction slope **0.001** manually, save, then:

```powershell
py -3 verification\oracle\scripts\chunk1_import_gui_u02.py
```

Linked scenario: `scenarios/simple_channel_unsteady_linked.json`

## Ramp transient (Plans 04 / 05)

Slow upstream **Q ramp** 100 → 200 → 100 cfs over **48 hr** (8-hour checkpoints). Same geometry; separate unsteady/plan files:

| File | Role |
|------|------|
| `simple_channel.u04` / `p04` | Friction-slope DS (type 2) — **certified** vs Plan 04 HDF |
| `simple_channel.u05` / `p05` | Rating-curve DS (type 3) — development until Plan 05 HDF |
| `reference_wsel_simple_channel_ramp_unsteady.json` | Oracle reference (Plan 04 HDF) |
| `reference_wsel_simple_channel_ramp_rating_unsteady.json` | Bootstrap placeholder (replace after Plan 05) |

### Regenerate u04/u05

```bash
python3 verification/oracle/scripts/write_simple_channel_ramp.py
```

### Prep (Windows, Plan 04)

```powershell
py -3 verification\oracle\scripts\chunk1_simple_channel_ramp_prep.py --plan 04 --open-ras
```

Warm-start: flat Q=100 run → Prior WS from that HDF → restore ramp hydrograph → compute Plan 04.

### Capture (Windows)

```powershell
py -3 verification\oracle\scripts\chunk1_simple_channel_ramp_capture.py --plan 04
```

### Verify friction ramp (Linux / macOS)

```bash
bash verification/oracle/scripts/run_simple_channel_ramp_verify.sh
```

**Pass:** max |Δ| ≤ **0.05 ft** at 4 RMs × hours 0/6/12/18/24/30/36/48.

Linked scenarios: `scenarios/simple_channel_ramp_unsteady_linked.json`, `scenarios/simple_channel_ramp_rating_unsteady_linked.json`
