# Bridge high-flow — intentional remaining deltas

Expected differences vs HEC-RAS 6.x 1D high flow (pressure, weir, combined). Implementation: `src/solvers/bridge/`, [`equations.md`](../reference/equations.md) §E–§F.

## Extensions (by design — not in HEC-RAS)

| Feature | Guidance |
|---------|----------|
| `bridge_deck_vent_*` | Omit for pure RAS imports; adds parallel vent flow in pressure/combined paths |
| `bridge_high_flow_methods = 1` | Always energy through opening; no explicit weir or vents on this branch |

## Approximations vs RAS

| Topic | STREAM-1D | Notes |
|-------|-----------|-------|
| Opening area under haunched deck | Scalar `profile_opening_area_factor` | Not WSEL-dependent along profile |
| Sluice/orifice switch | Global switch at **max** low chord | Uniform deck aligned; haunched deck simplified |
| Energy fallback / submergence cap | Opening energy only; no vents/weir | When `max_weir_submergence` exceeded or method = energy |
| Unsteady bridge coupling | Post-step `solve_bridge_coupled` (modes 0–4) | Not in network implicit Jacobian |

## Out of scope

Standalone inline weirs, multi-reach unsteady bridge networks, native `.g01` import (host responsibility).

## Verification

[`verification/fixtures/bridge_high_flow_hecras.json`](../../verification/fixtures/bridge_high_flow_hecras.json) — 6 cases, ±2 mm HW. Unit tests: `src/solvers/bridge_tests.rs`.
