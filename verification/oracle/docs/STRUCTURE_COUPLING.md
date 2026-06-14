# Structure coupling order (unsteady)

When inline culverts or bridges are present, STREAM-1D applies structure hydraulics at face intervals between cross sections.

## `structure_coupling_order`

| Value | Name | Behavior |
|-------|------|----------|
| `0` | Downstream-first (default) | Process structures from downstream to upstream within each post-step pass |
| `1` | Upstream-first | Reserved — not implemented |

ConSpan mild scenarios use `0` (default).

## `unsteady_structure_coupling_mode`

| Value | Name | Behavior |
|-------|------|----------|
| `0` | Post-step only | Explicit `converge_culvert_headwater` / `solve_bridge_coupled` after each Preissmann step |
| `1` | Reach–structure–reach outer loop | Reserved — not implemented |
| `2` | Hybrid implicit | Replace reach momentum row at eligible structure intervals with implicit headwater residual; explicit fallback when ineligible |

### Culvert implicit eligibility (mode `2`)

Inlet-controlled **circular**, **box**, and **ConSpan arch** (`shape_type` 3), single barrel, no overtopping. Outlet control, multi-barrel, and unsupported shapes defer to explicit post-step coupling that step.

### Known gap — mode `0` upstream drift

At constant Q=1000 cfs with known DS stage 30.51 ft, mode `0` shows ~0.6 ft terminal WSEL drop at RM 20.535 vs steady 50 yr reference. BU/BD and off-structure checkpoints below the culvert remain within the Chunk 4 gate (±0.5 ft). Mode `2` targets reduced upstream drift via `culvert_headwater_residual` in the Preissmann Jacobian.

## Diagnostics (API v34)

When structures are present, `solve_unsteady` returns per-step:

- `structure_implicit_interval_count` — intervals where implicit hook ran
- `structure_explicit_fallback_count` — explicit post-step passes
- `structure_coupling_converged` — structure coupling converged flag

Chunk 5 exit: `structure_implicit_interval_count > 0` on subcritical steps with mode `2`.
