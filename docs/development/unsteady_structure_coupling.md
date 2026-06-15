# Unsteady structure coupling

How inline culverts and bridges attach to the Preissmann step. User-facing summary: [README § Unsteady flow](../../README.md#unsteady-flow-and-water-surface-elevation).

## Coupling modes (`unsteady_structure_coupling_mode`)

| Mode | Name | Behavior |
|------|------|----------|
| **0** | Post-step only | Standard Preissmann reach step; structures update face WSEL in up to 5 post-step passes (default). |
| **1** | Reach–structure–reach | Reserved — not implemented. |
| **2** | Hybrid implicit | Replace reach momentum rows at eligible intervals with implicit culvert / subcritical-bridge headwater residuals; overtopping, outlet control, high-flow bridge, and supercritical use explicit post-step fallback. |
| **3** | Monolithic Newton | Outer Newton on Preissmann + culvert HW each step (experimental). |
| **4** | Quasi-steady particular | $y = y_{qs}(Q,TW) + \eta$: re-anchor to steady profile each step; Preissmann + post-step use **mode 2** physics. Recommended for culvert approach backwater on long $Q$ ramps. |

WASM enum names: `PostStepOnly`, `HybridImplicit`, `MonolithicNewton`, `QuasiSteadyParticular`.

## Per time step

```text
[Mode 4] steady profile y_qs → re-anchor y
Preissmann θ-scheme (upstream Q, downstream WSEL)
[Modes 2–4] implicit structure rows where eligible; swell-head friction on culvert approach
Post-step structure coupling (≤ 5 passes): culvert HW, bridge HW, optional approach backwater
[Mode 4] η reconcile / constant-Q snap; optional culvert face refresh
Enforce BCs; record WSEL at user cross sections
```

## Implicit eligibility (mode 2 / 4)

| Structure | In Jacobian when | Explicit fallback when |
|-----------|------------------|------------------------|
| Culvert | Forward $Q$; no roadway overtopping (`culvert_crest_elevs` unset) | Overtopping, zero $Q$, reverse $Q$ (unsupported) |
| Bridge | Subcritical low-flow Class A/B | High-flow pressure/weir, Class C supercritical, failed implicit residual |

## Diagnostics (API v34)

When structures are present, `solve_unsteady` returns per-step: `structure_coupling_converged`, `structure_implicit_interval_count`, `structure_explicit_fallback_count`; mode 3 adds Newton residual fields.

## Code

- Time loop: `src/solvers/unsteady.rs`
- Preissmann: `src/solvers/unsteady/preissmann.rs`
- Post-step: `src/solvers/unsteady/structure_coupling.rs`
- Mode 4: `src/solvers/unsteady/quasi_steady.rs`

## Deferred

- Multi-reach unsteady networks
- Culvert reverse flow
- Full high-flow bridge in Preissmann Jacobian
- Mode 1 outer reach–structure–reach loop
