# HEC-RAS parity checklist (reach_mild → automated verify)

Work **top to bottom**. Do not open HEC-RAS or refresh binaries until **Phase 2**.
Each step has a **pass criterion** — only continue when it passes.

Goal: STREAM-1D unsteady results match HEC-RAS at checkpoints RM 20.208, 20.189, 20.095 (±0.15 ft WSEL).

---

## Phase 0 — No HEC-RAS (WSL / Linux only)

These steps never call `Ras.exe`. Safe to run every day.

| # | Step | Command | Pass criterion |
|---|------|---------|----------------|
| 0.1 | Parse project bundle | `python3 verification/oracle/scripts/smoke_reach_mild_parse.py` | Prints `OK`; 8 cross sections; Q/stage hydrograph lengths match |
| 0.2 | STREAM-1D vs committed reference | `python3 verification/oracle/scripts/run_ras_reference.py --scenario verification/oracle/scenarios/reach_mild_unsteady_linked.json --skip-ras-run --verify` | Exit 0; report shows PASS or documented FAIL at checkpoints |
| 0.3 | Full linked verify (optional) | `bash verification/oracle/run_oracle.sh --scenario verification/oracle/scenarios/reach_mild_unsteady_linked.json` | Same as 0.2 via oracle runner |

**Phase 0 complete when:** 0.1 and 0.2 pass. You have a repeatable “no RAS” gate for CI.

**If 0.2 fails:** fix STREAM-1D / mapper / fixture — not HEC-RAS.

---

## Phase 1 — Reference truth (one-time, minimal HEC-RAS)

Use **GUI once** to get a trusted reference. Avoid headless until Phase 3.

| # | Step | What to do | Pass criterion |
|---|------|------------|----------------|
| 1.1 | Open project in HEC-RAS 7 | Windows: open `verification/oracle/projects/reach_mild/reach_mild.prj` (or staged copy under `%LOCALAPPDATA%\stream1d_oracle\reach_mild\` after one script run) | Project opens; no geometry errors |
| 1.2 | Fix unsteady flow if prompted | Unsteady Flow Editor → save `u02` from GUI | No “Error Loading Unsteady Flow Data” when opening plan 02 |
| 1.3 | Run plan 02 manually | Compute → plan 02 | `reach_mild.p02.hdf` appears next to project |
| 1.4 | Capture checkpoint WSEL | Note terminal WSEL at RM 20.208, 20.189, 20.095 (from profile/HDF) | Three numbers written down |
| 1.5 | Commit reference | Copy GUI-saved `reach_mild.u02` + optional `p02.hdf` into repo; update `reference_wsel_reach_mild_unsteady.json` via script **or** hand | 0.2 still passes after updating reference if STREAM-1D was wrong; or 0.2 fails until STREAM-1D is fixed toward new truth |

Refresh reference from HDF (after 1.3):

```powershell
py -3 verification\oracle\scripts\run_ras_reference.py `
  --scenario verification\oracle\scenarios\reach_mild_unsteady_linked.json `
  --skip-ras-run `
  --hdf path\to\reach_mild.p02.hdf
```

**Phase 1 complete when:** committed reference JSON reflects **your** GUI run and you trust the three checkpoint WSEL values.

---

## Phase 2 — Freeze inputs (stop churn)

Lock files so we stop editing HEC-RAS text by hand.

| # | Step | Action | Pass criterion |
|---|------|--------|----------------|
| 2.1 | Regenerate bundle from ConSpan | `python3 verification/oracle/scripts/bootstrap_reach_mild_project.py` | Only when geometry RMs change |
| 2.2 | Overlay GUI u02 | Replace repo `reach_mild.u02` with GUI-exported file from 1.2 | Headless not required yet |
| 2.3 | Document RAS version | Add `hecras_version` to reference JSON (e.g. `7.0.1`) | Recorded in reference metadata |

**Phase 2 complete when:** repo `reach_mild.u02` is the GUI-exported file, not hand-edited.

---

## Phase 3 — Headless refresh (optional automation)

Only after Phase 1–2. Expect occasional Windows + dialog issues.

| # | Step | Command | Pass criterion |
|---|------|---------|----------------|
| 3.1 | Debug run (logged) | `.\verification\oracle\scripts\iterate_ras_debug.ps1 -ReachMildOnly` | Read `verification/oracle/logs/ras_iterate_latest.log` |
| 3.2 | Headless refresh | `py -3 verification\oracle\scripts\run_ras_reference.py --scenario verification\oracle\scenarios\reach_mild_unsteady_linked.json --no-verify` | `reach_mild.p02.hdf` created; no error dialog |
| 3.3 | Full refresh + verify | Same without `--no-verify` | Exit 0; reference JSON updated; 0.2 passes |

**Tips**

- Kill stray `Ras.exe` / `PipeServer.exe` before 3.x.
- Default: batch `Ras.exe` (no auto-dismiss). Use `-UseRasCommander` on iterate script for 2 s fail + dialog text in log.
- Wipe stage if stuck: `Remove-Item -Recurse -Force $env:LOCALAPPDATA\stream1d_oracle\reach_mild`

**Phase 3 complete when:** 3.2 produces HDF without opening GUI.

---

## Phase 4 — Automated checking (target steady state)

| # | Step | Where | Pass criterion |
|---|------|-------|----------------|
| 4.1 | CI: verify only | WSL job on every PR | `run_ras_reference.py --skip-ras-run --verify` exit 0 |
| 4.2 | CI / manual: refresh | Windows runner or monthly manual | Phase 3.2 when geometry or BCs change |
| 4.3 | Expand scenarios | After reach_mild stable | ConSpan mild unsteady, then beaver (Chunk 8) |

---

## What to ignore for now

- `simple_channel` HEC-RAS project (deprioritized)
- Linux-native HEC-RAS binaries
- Headless from WSL (use Windows Python + staging)
- ConSpan full culvert model (unless reach_mild blocked — use only for u02 format reference)

---

## Current recommended focus

**You are here → Phase 0**

1. Run 0.1 and 0.2 in WSL until both pass.
2. If 0.2 fails, tune STREAM-1D against bootstrap reference (ConSpan CSV peaks) — no RAS.
3. When ready for truth, do **Phase 1 once** in HEC-RAS GUI.
4. Come back for Phase 3 when you want headless refresh.

---

## Quick commands (copy-paste)

**WSL — daily gate (no RAS):**

```bash
cd ~/Lillywhite_Consulting/lillywhite_engine/STREAM-1D
source .venv/bin/activate
python3 verification/oracle/scripts/smoke_reach_mild_parse.py
python3 verification/oracle/scripts/run_ras_reference.py \
  --scenario verification/oracle/scenarios/reach_mild_unsteady_linked.json \
  --skip-ras-run --verify
```

**Windows — iterate (when you reach Phase 3):**

```powershell
cd \\wsl.localhost\Ubuntu\home\jason\Lillywhite_Consulting\lillywhite_engine\STREAM-1D
.\verification\oracle\scripts\iterate_ras_debug.ps1 -ReachMildOnly
Get-Content verification\oracle\logs\ras_iterate_latest.log -Tail 40
```
