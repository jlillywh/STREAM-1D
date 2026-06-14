# Linked HEC-RAS oracle

**Live cross-check** between STREAM-1D and HEC-RAS for scenarios where both tools model the **same geometry source**.

**Parity program (unsteady):** [`PARITY_PROGRAM.md`](PARITY_PROGRAM.md) — Chunk 0 setup, scenario schema, HDF reference, Beaver deferral. Roadmap: [`lillywhite_web/streams1d/docs/UNSTEADY_HECRAS_PARITY_ROADMAP.md`](../../../lillywhite_web/streams1d/docs/UNSTEADY_HECRAS_PARITY_ROADMAP.md).

**RAS authoring exports:** [`lillywhite_web/streams1d/hecras_outputs/`](../../../lillywhite_web/streams1d/hecras_outputs/README.md) → sync via [`scripts/sync_linked_projects.sh`](scripts/sync_linked_projects.sh).

This is distinct from frozen golden fixtures in [`../fixtures/`](../fixtures/):

| Layer | What it proves | HEC-RAS required? |
|-------|----------------|-------------------|
| **Frozen goldens** (`fixtures/*.json`) | Regression against documented reference values | No |
| **Linked oracle** (this folder) | STREAM-1D vs output from a **bundled HEC-RAS project** (`.g01` + plan + flow) | Optional for re-run |

## Linked verify workflow

```text
  HEC-RAS project bundle          STREAM-1D mapped inputs
  (.g01 + .pXX + .fXX)     ←→     (fixtures/*_project_*.json)
           │                              │
           ▼                              ▼
    Run HEC-RAS (optional)          solve_steady / solve_unsteady
           │                              │
           └──────── compare WSEL/Q ──────┘
```

1. **Geometry source of truth** — HEC-RAS `.g01` (bundled under `projects/` or supplied by the user).
2. **STREAM-1D inputs** — JSON mapped from that geometry (documented in the scenario manifest).
3. **HEC-RAS reference** — profile/time-series export from running the linked project (`fixtures/ConSpan.csv` for the ConSpan example), or a fresh run via `--live-ras`.
4. **Report** — station-by-station diff with pass/fail against tolerance.

Same geometry source makes the comparison defensible to agency reviewers: you are not comparing against hand-waved numbers; you are comparing against HEC-RAS output from the project files sitting next to the scenario.

## Scope (v1)

Supported in linked scenarios:

- **Single reach**
- **Reach-only unsteady** (Chunk 2: `reach_mild_unsteady_linked`) — no structures; dynamic DS BC
- **Inline culverts** and/or **inline bridges**
- **Steady** profiles

Out of scope for linked verify (decline in scenario notes):

- Multi-reach unsteady networks, junctions, tributaries in unsteady mode
- Features with documented intentional STREAM-1D deltas vs HEC-RAS (see [`../../docs/reference/hecras_parity.md`](../../docs/reference/hecras_parity.md))

## Quick start

```bash
# Sync linked project files from lillywhite_web (if available)
bash verification/oracle/scripts/sync_linked_projects.sh

# From repo root — ConSpan culvert steady (default scenario)
bash verification/oracle/run_oracle.sh

# Beaver Creek unsteady inline bridge (Chunk 8 development)
bash verification/oracle/run_oracle.sh \
  --scenario verification/oracle/scenarios/beaver_unsteady_linked.json

# Reach-only unsteady (Chunk 2 certification candidate)
bash verification/oracle/run_oracle.sh \
  --scenario verification/oracle/scenarios/reach_mild_unsteady_linked.json
```

**Phase 0 gate (local or CI, no HEC-RAS):**

```bash
python3 verification/oracle/scripts/run_phase0.py
```

Runs parse smoke (0.1), committed-reference verify (0.2), and `run_oracle.sh` (0.3). On every PR, GitHub Actions job `reach-mild-phase0` in [`.github/workflows/ci.yml`](../../.github/workflows/ci.yml) runs the same script on `ubuntu-latest`.

**Requires:** `maturin develop --features python` (same as [`python/test_hecras_culvert_verification.py`](../../python/test_hecras_culvert_verification.py)).

**Optional live re-run:** [ras-commander](https://github.com/gpt-cmdr/ras-commander) (`pip install ras-commander`) and a compatible HEC-RAS installation (6.x+ recommended; legacy bundled examples may need migration).

## Headless HEC-RAS reference capture (Option A)

**Use the right terminal:** PowerShell commands use `\` paths and `.ps1` scripts — they do **not** work in WSL bash.

| Step | Terminal | Command |
|------|----------|---------|
| **1. Refresh reference** | **Windows PowerShell** (recommended) | `.\verification\oracle\scripts\run_ras_reference.ps1` |
| **1 alt.** | WSL bash (stages to Windows) | `bash verification/oracle/scripts/run_ras_reference.sh` |
| **2. Verify STREAM-1D** | WSL or Windows | `bash verification/oracle/run_oracle.sh --scenario verification/oracle/scenarios/reach_mild_unsteady_linked.json` |

### Step 1 — Windows PowerShell (recommended)

Open **PowerShell** (not WSL). From the repo root:

```powershell
cd \\wsl.localhost\Ubuntu\home\jason\Lillywhite_Consulting\lillywhite_engine\STREAM-1D
.\.venv\Scripts\Activate.ps1
pip install -r verification\requirements-oracle-hecras.txt
.\verification\oracle\scripts\run_ras_reference.ps1
```

Or set `HECRAS_RAS_EXE` explicitly:

```powershell
$env:HECRAS_RAS_EXE = 'C:\Program Files (x86)\HEC\HEC-RAS\7.0.1\Ras.exe'
python verification\oracle\scripts\run_ras_reference.py `
  --scenario verification\oracle\scenarios\reach_mild_unsteady_linked.json
```

### Step 1 alt — WSL bash (if you stay in Ubuntu)

```bash
cd ~/Lillywhite_Consulting/lillywhite_engine/STREAM-1D
source .venv/bin/activate
pip install -r verification/requirements-oracle-hecras.txt
export HECRAS_RAS_EXE='/mnt/c/Program Files (x86)/HEC/HEC-RAS/7.0.1/Ras.exe'
bash verification/oracle/scripts/run_ras_reference.sh
```

### Step 2 — Verify only (WSL, no HEC-RAS)

```bash
cd ~/Lillywhite_Consulting/lillywhite_engine/STREAM-1D
source .venv/bin/activate
python3 verification/oracle/scripts/run_ras_reference.py \
  --scenario verification/oracle/scenarios/reach_mild_unsteady_linked.json \
  --skip-ras-run --verify
```

**Gate scenario:** `reach_mild_unsteady_linked` — eight open-channel ConSpan cross sections (no culvert), Q=1000 cfs, DS stage 30.51 ft. Bootstrap reference is ConSpan 50 yr steady until you refresh from plan HDF.

```bash
python3 verification/oracle/scripts/smoke_reach_mild_parse.py
pip install -r verification/requirements-oracle-hecras.txt   # ras-commander, h5py
```

To add a new case later: bundle HEC-RAS files under `projects/<name>/`, add a scenario JSON pointing at `reference.file`, run `run_ras_reference.py --scenario ...`.

## Sync from web repo

Authoring exports live under **`lillywhite_web/streams1d/hecras_outputs/`** (Beaver, ConSpan, Wailupe, etc.). The engine bundles self-contained copies under `projects/` so linked verify runs without the web app.

```bash
# Copy Beaver (and optional future exports) from sibling web repo
bash verification/oracle/scripts/sync_linked_projects.sh
# Override web root: LILLYWHITE_WEB_ROOT=/path/to/lillywhite_web bash ...
```

| Bundle | Source | Notes |
|--------|--------|-------|
| `projects/conspan/` | Shipped in engine | Steady culvert example |
| `projects/reach_mild/` | Shipped in engine | Chunk 2 reach-only unsteady (trimmed ConSpan upstream) |
| `projects/beaver/` | `hecras_outputs/beaver/` via sync | Chunk 8 only |

`reach_mild` is **not** synced from web — it is maintained in-engine for the Chunk 2 gate.

## Adding a linked scenario

1. Place HEC-RAS project files under `projects/<name>/` (`.g01`, `.pXX`, `.fXX` or `.uXX`).
2. Add STREAM-1D mapped inputs under `../fixtures/` (geometry + structures + BCs derived from the `.g01`).
3. Add HEC-RAS reference export (profile CSV or HDF extract) under `../fixtures/`.
4. Create `scenarios/<name>_linked.json` — see [`scenarios/_template_unsteady_linked.json`](scenarios/_template_unsteady_linked.json) or [`scenarios/conspan_steady_linked.json`](scenarios/conspan_steady_linked.json). Validate against [`schemas/linked_scenario.schema.json`](schemas/linked_scenario.schema.json).
5. Run `run_linked_verify.py --scenario ...` and confirm pass.

## Layout

```text
verification/oracle/
  PARITY_PROGRAM.md         # Chunk 0 program setup (owners, RAS version, gates)
  README.md                 # this file
  docs/HDF_REFERENCE.md     # WSEL(t) extract from RAS HDF (Chunk 4+)
  schemas/linked_scenario.schema.json
  run_linked_verify.py      # CLI entry point
  run_oracle.sh             # wrapper (optional skip if no Python ext)
  lib/
    scenario.py             # manifest loader
    stream1d_runner.py      # build inputs + run STREAM-1D
    ras_reference.py        # CSV export + optional live RAS
    compare.py              # diff report
  projects/
    conspan/                # bundled HEC-RAS example (.g01, .p01, .f01)
    reach_mild/             # Chunk 2 reach-only unsteady (no structures)
    beaver/                 # Chunk 8 development (not certification gate)
  scenarios/
    _template_unsteady_linked.json
    conspan_steady_linked.json
    reach_mild_unsteady_linked.json
    beaver_unsteady_linked.json
```

## Trust narrative

For open-source reviewers and agency partners:

> Import or use our bundled HEC-RAS project. STREAM-1D inputs are mapped from the same geometry. Run `verification/oracle/run_oracle.sh` to see STREAM-1D vs HEC-RAS WSEL at every profile station — on your machine, against the linked project files.

The web application is not required. This harness lives entirely in the engine repo.

## Related

- Frozen verification catalog: [`../README.md`](../README.md)
- HEC-RAS parity scope: [`../../docs/reference/hecras_parity.md`](../../docs/reference/hecras_parity.md)
- Prep item in implicit coupling checklist: HEC-RAS steady golden / oracle alignment
