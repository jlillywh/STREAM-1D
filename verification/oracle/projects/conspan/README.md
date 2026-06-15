# ConSpan linked oracle example

HEC-RAS **ConSpan** arch culvert reach used for steady certification and unsteady
structure-coupling diagnostics. STREAM-1D inputs are mapped from `ConSpan.g01` with
reach geometry from [`../../../fixtures/conspan_project_12.json`](../../../fixtures/conspan_project_12.json).

## Bundled files

| File | Role |
|------|------|
| `ConSpan.g01` | Geometry (arch culvert, embankment XS modifiers) |
| `ConSpan.u07` | Unsteady flow — Q ramp 500→1000 cfs, downstream stage hydrograph |
| `ConSpan.p08` | Unsteady plan 08 (15 min interval, θ=1, friction slope method 2) |
| `ConSpan.p01` / `ConSpan.f01` | Steady plan / flows |
| `reference_wsel_timeseries_ramp_full.json` | Committed WSEL(t) from plan 08 HDF (ramp matrix) |

## Scenarios

| Scenario | Mode | Gate |
|----------|------|------|
| [`conspan_steady_linked.json`](../../scenarios/conspan_steady_linked.json) | Steady | **Certification** (±0.04 ft vs CSV export) |
| [`conspan_unsteady_ramp_matrix.json`](../../scenarios/conspan_unsteady_ramp_matrix.json) | Unsteady Q ramp | **Diagnostic** — compare only, no pass/fail |
| [`conspan_unsteady_ramp_matrix_mode4.json`](../../scenarios/conspan_unsteady_ramp_matrix_mode4.json) | Unsteady Q ramp (mode 4) | **CI gate** — overall max \|Δ\| ≤ 0.12 ft vs HEC |

### Run mode 4 CI gate locally

```bash
maturin develop --features python --release
PYTHONPATH=python python3 verification/oracle/run_linked_verify.py \
  --scenario verification/oracle/scenarios/conspan_unsteady_ramp_matrix_mode4.json \
  --format matrix
```

Exit **1** when overall max \|Δ\| > 0.12 ft. Requires bundled `ConSpan.u07`, `ConSpan.p08`, and `reference_wsel_timeseries_ramp_full.json`.

### Run unsteady ramp matrix (diagnostic, mode 2)

```bash
maturin develop --features python --release
PYTHONPATH=python python3 verification/oracle/run_linked_verify.py \
  --scenario verification/oracle/scenarios/conspan_unsteady_ramp_matrix.json \
  --format matrix
```

Reports **Δ = STREAM − HEC** (ft) at 10 river miles × 13 times (every 4 hr, 48 hr ramp).
Exit code is always **0** for this scenario; it does not fail CI on tolerance.

### Run steady linked verify

```bash
bash verification/oracle/run_oracle.sh \
  --scenario verification/oracle/scenarios/conspan_steady_linked.json
```

## Unsteady parity summary (mode 2 implicit culvert coupling)

As of the current branch, vs HEC-RAS plan 08 reference (Q ramp 500→1000 cfs):

| Metric | Value (ft) | Notes |
|--------|------------|-------|
| **Overall matrix max \|Δ\|** | **~0.47** | Peak at approach pool RM 20.308 (ramp) |
| RM 20.251 (immediate approach) | ~0.05 | |
| **Culvert face max \|Δ\|** | **~0.02–0.12** | RM 20.238 (BU), 20.227 (BD) |
| Downstream open channel (RM ≤ 20.095) | &lt; 0.08 | |

Outlet control throughout the ramp. **Face parity (~0.07 ft) and approach-pool backwater (~0.8–1.0 ft) are separate issues.** Isolated `solve_culvert(Q, HEC TW)` matches HEC BU within ~0.01 ft; the limiting error is reach backwater propagation upstream of the culvert during the Q ramp.

### Culvert solver check (isolated)

```bash
PYTHONPATH=python python3 verification/oracle/scripts/conspan_culvert_face_diagnostic.py
```

At 36–48 h, `solve_culvert(Q, HEC tailwater)` matches HEC BU within **~0.01 ft**.
Isolated FHWA/ConSpan rating is not the limiting error; chained approach backwater during the Q ramp is the open item.

### Implementation notes (STREAM-1D)

- **Swell head** (HEC-RAS Eqs 2-94–2-97): culvert cell + one upstream reach interval (`UPSTREAM_SPREAD = 1`); gated off culvert cell when \|HW residual\| ≤ tolerance (constant-Q stability).
- **Mode 2 (hybrid):** immediate interval — capped post-step each structure pass; intervals 2–20 upstream — implicit relaxed `solve_step` Jacobian during Q transients only (`|dQ/dt|` gate); post-step sweep beyond the Jacobian pool.
- **Mode 3 (monolithic Newton, experimental):** outer Newton each time step on Preissmann + culvert HW + approach `solve_step` rows (cells 2–32, ω ≈ 0.75); no swell/post-step patches. Scenario: [`conspan_unsteady_ramp_matrix_mode3.json`](../../scenarios/conspan_unsteady_ramp_matrix_mode3.json). Set `unsteady_structure_coupling_mode: 3` or oracle `coupling_mode: 3`.

**Mode 3 Newton diagnostics** (on `UnsteadyResult` when mode 3):

| Field | Meaning |
|-------|---------|
| `monolithic_newton_converged` | Per step: max \|RHS\| ≤ 0.003 m before stopping |
| `monolithic_newton_iterations` | Outer Newton iterations used |
| `monolithic_newton_initial_residual` | max \|RHS\| before first update |
| `monolithic_newton_max_residual` | max \|RHS\| after last update |
| `monolithic_newton_momentum_residual` | max \|momentum-row RHS\| (last iter) |
| `monolithic_newton_continuity_residual` | max \|continuity-row RHS\| (last iter) |

```bash
PYTHONPATH=python python3 verification/oracle/scripts/conspan_monolithic_newton_diagnostic.py
```
- **Departure tailwater**: small friction forcing on reach below BD (ω ≈ 0.30); stability/parity knob, not a documented HEC term.
- **Upstream swell spread** (`e > 1` on far cells): tested; worsens parity with current hybrid coupling.

## Refresh HEC reference

See [`../../README.md`](../../README.md) — `run_ras_reference.py` with plan 08 on Windows when HEC-RAS is installed.
