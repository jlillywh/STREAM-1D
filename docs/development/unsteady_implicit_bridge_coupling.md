# 5.1 Unsteady implicit bridge coupling — design

**Phase 5.1 · Design only** — decide how inline bridge head loss couples to the Preissmann Saint-Venant step: **embed in the Jacobian** vs **multi-pass explicit iteration** (and staged hybrids).

**Today (API v32):** bridges use the full steady bridge solver (`solve_bridge_coupled`) as a **black box after** each Preissmann step. Culverts share the same pattern. See [`equations.md`](../reference/equations.md) (unsteady structure bullets) and [`hecras_parity.md`](../reference/hecras_parity.md) (explicit post-step gap).

**Code anchors:** [`unsteady.rs`](../../src/solvers/unsteady.rs) (`solve_unsteady_step`, `apply_structure_internal_boundaries`, `couple_bridge_interval`), [`bridge.rs`](../../src/solvers/bridge.rs) (`solve_bridge_coupled`, `solve_bridge_headwater_metric`).

**Related concern:** bridge-related source files have grown large enough to impede Phase 5 work — see [Module refactor plan](#module-refactor-plan) below. Refactor is **in scope for 5.1 design** and should land **before or in parallel with** 5.2 coupling implementation.

---

## Goals

| Goal | Priority |
|------|----------|
| **Timestep stability** — bridge-induced stage waves do not lag or oscillate vs a fully coupled reference on steep transients | **P0** |
| **Mass / momentum consistency** — structure interval respects the same $Q$ and stage relation the bridge solver enforces | **P0** |
| **Preserve bridge fidelity** — Class A/B/C, pressure, weir, pier Yarnell, BU/BD weighting unchanged unless explicitly approximated | **P0** |
| **Backward-compatible default** — hosts keep today’s explicit post-step unless they opt into stronger coupling | **P0** |
| **Maintainable module boundaries** — split monolithic `bridge.rs` / duplicated steady–unsteady glue so implicit coupling and tests have clear homes | **P0** |
| **Culvert parity** — same coupling framework applies to inline culverts (shared infrastructure) | **P1** |
| **Multi-bridge / bridge+c culvert reaches** — deterministic coupling order and convergence | **P1** |

**Non-goals (5.1):** multi-reach networks, 2D bridge hydraulics, new bridge physics, interior-cut friction inside the Jacobian (stored `bridge_internal_cross_sections` remain metadata-first until 5.x hydraulics land). **Non-goal:** behavior change during refactor-only PRs.

---

## Current architecture (as-is)

### Timestep flow

```text
for each time step:
  1. solve_unsteady_step()          # Preissmann θ-scheme, block-tridiagonal Thomas
  2. apply_structure_internal_boundaries()   # ≤ 5 passes, downstream-first order
       per structure on interval [i, i+1]:
         read TW from y_metric[tw_face], Q from q_metric[i]
         solve_bridge_coupled(...) or culvert HW iterate
         overwrite y_metric[hw_face] only
  3. record diagnostics; advance y_metric, q_metric
```

| Constant | Value | Role |
|----------|-------|------|
| `CULVERT_STEP_MAX_PASSES` | 5 | Outer structure passes per timestep |
| `BRIDGE_HW_MAX_ITER` | 8 | Inner HW fixed-point per bridge call |
| `structure_coupling_order` | `0` default | Combined downstream-first; `1` culverts-first; `2` bridges-first |

### What Preissmann sees

`solve_unsteady_step` builds standard **continuity + momentum** coefficients on every interval using reach geometry tables (ineffective / blocked / guide-bank aware). Bridge intervals are **not** special-cased — the reach step treats BU→BD like any other pair of nodes.

### What the bridge pass does

`couple_bridge_interval` (forward `Q > 0`):

- **TW** = WSEL at downstream face `i + 1` (BD)
- **HW** = solved upstream headwater at face `i` (BU) via `solve_bridge_coupled`
- Updates **only** `y_metric[i]`; does **not** modify `q_metric` or re-enter Saint-Venant

Reverse flow (`Q < 0`) mirrors hydraulic US/DS faces per [`bridge_reverse_flow_rating.md`](bridge_reverse_flow_rating.md).

### Head loss definition

From `solve_bridge_coupled`:

```text
head_loss = max(HW_hyd − TW_hyd, 0)   # user units, hydraulic faces
```

Low/high flow regime selection, pier losses, and pressure/weir splits happen **inside** `solve_bridge_headwater_metric`; the unsteady driver only sees the final HW and scalar `head_loss`.

---

## Problem statement

| Issue | Symptom | Root cause |
|-------|---------|------------|
| **Split solve** | Bridge faces jump after the reach step; interior profile was computed without structure constraint | Jacobian unaware of $H_\text{loss}(y_{BU}, y_{BD}, Q)$ |
| **Single reach pass** | ≤ 5 face updates without re-solving Saint-Venant | Structure correction not fed back into mass/momentum system |
| **$Q$ not coupled** | Section discharge from Preissmann may be inconsistent with bridge rating at the new HW | Only `y` is overwritten at one face |
| **Regime switches** | Pressure/weir onset during a transient may lag one or more steps | Explicit post-step + frozen regime history |
| **Multiple structures** | Upstream structure sees downstream face that was just moved by another structure | Gauss–Seidel-style passes, no global convergence guarantee |

HEC-RAS 1D unsteady generally **iterates** reach and structure until the timestep converges (structures embedded in the network solve rather than a single explicit correction). STREAM-1D documents this as a parity gap ([`hecras_parity.md`](../reference/hecras_parity.md) — “Implicit network solve” vs “Explicit post-step ≤5 iterations”).

---

## Coupling options

### Option A — Multi-pass explicit iteration (evolve status quo)

Keep `solve_bridge_coupled` as the structure model. Strengthen the **outer** timestep loop.

| Variant | Description |
|---------|-------------|
| **A0 (today)** | One Preissmann solve + ≤ 5 structure face passes |
| **A1 — Reach–structure–reach** | Repeat: Preissmann → structures until $\max |\Delta y_\text{face}| \le \tau$ (cap e.g. 8–12 outers) |
| **A2 — Relaxation** | $y_\text{face}^{k+1} = \omega y_\text{solved} + (1-\omega) y_\text{face}^{k}$, $\omega \in (0,1]$ on stiff transients |
| **A3 — Frozen $Q$ sweep** | Optional inner loop on $q_\text{metric}[i]$ when bridge pressure rating is sensitive (defer unless A1 insufficient) |

**Pros:** Reuses full bridge solver (all regimes, v32 ice/debris, reverse flow). Minimal Jacobian risk. Incremental ship path.

**Cons:** Multiple Preissmann solves per step (CPU). Still not a true implicit structure Jacobian; may need many outers on steep waves.

**Recommendation:** **Default near-term implementation (Phase 5.2)** — A1 with configurable `unsteady_coupling_max_outer_iterations` and per-step diagnostic `structure_coupling_iterations`.

---

### Option B — Embed bridge head loss in Preissmann Jacobian

Treat the bridge as an **internal boundary** on interval $i$ linking nodes $i$ (BU) and $i+1$ (BD).

#### Subcritical submodel (low flow, single regime)

Replace or supplement the momentum row with an energy–head relation:

```text
E_HW(y_i, Q) = E_TW(y_{i+1}, Q) + H_loss(y_i, y_{i+1}, Q)
```

where $E$ uses obstructed area from BU/BD tables and $H_\text{loss}$ comes from the **active** low-flow method (Yarnell, momentum, energy, WSPRO) or a tabulated `computeBridgeRatingCurve` slice at current $Q$.

Linearize for the Newton step (unknowns $\Delta y_i$, $\Delta y_{i+1}$, $\Delta Q_i$, $\Delta Q_{i+1}$):

```text
∂R/∂y_i · Δy_i + ∂R/∂y_{i+1} · Δy_{i+1} + ∂R/∂Q · ΔQ = −R
```

Pack into the existing `Mat2` block at nodes $i$ and $i+1` in `solve_unsteady_step` (see interval assembly ~lines 1462–1597).

#### High flow (pressure / weir / combined)

$H_\text{loss}$ is **piecewise** (low chord, deck, submergence cap, energy fallback). Branches are not smoothly differentiable.

**Jacobian strategy:**

| Approach | Use when |
|----------|----------|
| **B1 — Regime pinning** | Freeze `flow_regime` from previous iterate within the timestep; refresh after convergence |
| **B2 — Smooth envelope** | `HW = max(HW_low, HW_high)` with softened `max` for derivatives (analysis only) |
| **B3 — Explicit fallback** | If regime ∈ {pressure, weir, energy} or $Fr > 1$, delegate interval to Option A for that step |

**Pros:** Single linear solve per outer Newton (if one Newton enough). Better phase alignment on mild transients.

**Cons:** Heavy engineering; risk of Jacobian inconsistency; high-flow branches need B3 fallback anyway.

**Recommendation:** **Phase 5.3+** — B1 + B3 hybrid: implicit only for pinned low-flow subcritical; explicit `solve_bridge_coupled` when pinned regime is high-flow or Froude indicates supercritical.

---

### Option C — Monolithic coupled Newton

Augment the global system with extra equations per structure (unknown HW, optional $Q_\text{structure}$). Simultaneously solve all reach intervals and structure residuals.

**Pros:** Theoretically clean.

**Cons:** Large refactor of `solve_unsteady_step`; poor match to existing block-tridiagonal Thomas solver; highest risk.

**Recommendation:** **Defer** unless Option B stalls on multi-structure networks.

---

### Option D — Rating-curve boundary (lumped)

Precompute `HW = f(Q, TW)` from `computeBridgeRatingCurve` at fixed TW slices; use $\partial f / \partial Q$, $\partial f / \partial TW$ in the Jacobian.

**Pros:** Cheap derivatives; host-friendly.

**Cons:** TW moves every step — table must be interpolated; loses pier/debris/ice state not captured in rating params; poor for simultaneous TW–HW transients.

**Recommendation:** Optional **fast path** for simple bridges (no deck profile, no piers) under Option B; not the general solution.

---

## Recommended staging (decision)

```text
Phase 5.0  Module refactor (no physics change) — bridge/ directory + structure coupling extraction
Phase 5.1  Design (this document) — coupling options + refactor plan
Phase 5.2  Option A1 — reach–structure–reach outer iteration (default off → opt-in → default on if verified)
Phase 5.3  Option B1+B3 — Jacobian internal boundary for low-flow subcritical only
Phase 5.4  Joint verification — steady vs unsteady parity, transient fixtures, HEC-RAS unsteady benchmarks (where available)
Phase 5.5  Culvert reuse — same outer/Jacobian hooks for FHWA culvert HW relation
```

**5.1 decisions:**

1. **Coupling:** Do **not** jump straight to full Jacobian embedding. Ship **A1 first** (low risk, closes most of the HEC-RAS “iterated coupling” gap), then add **B1+B3** for subcritical low-flow intervals where profiling shows outer iterations are costly.
2. **Refactor:** Do **5.0 before 5.3** (Jacobian needs a narrow `bridge_implicit` / `opening` surface). **5.0 can overlap 5.2** if structure coupling is extracted first (PR 1), then outer loop lands in the extracted module (PR 2).

---

## Module refactor plan

### Why refactor now

| File | ~Lines | Problem |
|------|--------|---------|
| [`bridge.rs`](../../src/solvers/bridge.rs) | **4,040** | Types, serde API, geometry build, opening hydraulics, low/high flow, rating, ice/debris, public API — single file |
| [`bridge_tests.rs`](../../src/solvers/bridge_tests.rs) | **3,255** | All bridge unit tests via `#[path]` on `bridge.rs` |
| [`bridge_interior.rs`](../../src/solvers/bridge_interior.rs) | **2,470** | Reach layout, BU/BD face resolution, densification inserts, layout tests |
| [`unsteady.rs`](../../src/solvers/unsteady.rs) | **3,840** | Preissmann core + **~350 lines** structure coupling (bridge + culvert post-step) |
| [`steady.rs`](../../src/solvers/steady.rs) | (large) | Duplicated `bridge_coupling_for`, `bridge_face_geometry_for`, `bridge_deck_profile_for` |

Adding `bridge_headwater_implicit_rhs`, Jacobian interval hooks, and reach–structure outer loops **inside** today’s `bridge.rs` would push a critical file past ~5k lines and make reviews/error isolation harder. Splitting first gives Phase 5 a stable layout.

### Design principles

| Principle | Rule |
|-----------|------|
| **No API break** | `crate::solvers::bridge::{solve_bridge_coupled, BridgeSolveParams, …}` unchanged via `bridge/mod.rs` re-exports |
| **No physics change in 5.0** | Refactor-only PRs; existing tests must pass unchanged |
| **Target size** | ~300–800 lines per module (soft cap ~1k with tests) |
| **Coupling lives once** | Single `bridge/reach_coupling.rs` for steady + unsteady input → `BridgeCouplingParams` + face geometry |
| **Unsteady driver thin** | `solve_unsteady` orchestrates; structure loop in dedicated module |
| **Tests follow physics** | Split `bridge_tests.rs` by submodule or colocate `#[cfg(test)]` per file |

### Target layout

Convert `bridge.rs` into a **directory module** `src/solvers/bridge/`:

```text
src/solvers/bridge/
  mod.rs                 # pub use re-exports (preserve external paths)
  types.rs               # BridgeGeometry, BridgeCouplingParams, enums, BridgeSolveParams/Rating serde
  section.rs             # BridgeSectionContext, mirror, friction lengths, flow direction
  ice_debris.rs          # BridgeIceDebrisParams, ice_debris_params_for_bridge, opening ice helpers
  geometry.rs            # build_bridge_geometry, deck profile, pier resolution entry
  opening.rs             # obstructed_hydraulics, net/gross opening, pier at WSEL, abutment area hooks
  low_flow.rs            # classify_low_flow, Class A/B/C, Yarnell, momentum, energy, WSPRO
  high_flow.rs           # pressure, weir, deck vents, combined high flow, Bradley tables
  headwater.rs           # solve_bridge_headwater_metric, solve_high/low flow dispatch, reconcile
  coupling.rs            # solve_bridge_coupled, solve_bridge_wsel, solve_bridge_tailwater, head_loss
  rating.rs              # compute_bridge_rating_curve, coupling_from_params, solve_bridge_from_params
  reach_coupling.rs      # NEW: bridge_coupling_for_steady/unsteady, face geometry, deck profile (DRY)
  unsteady_coupling.rs   # NEW (5.2): couple_bridge_interval, converge_bridge_headwater, face overwrite
  implicit.rs            # NEW (5.3): bridge_headwater_implicit_rhs + ∂H/∂y, ∂H/∂Q stubs

src/solvers/unsteady/
  mod.rs                 # (optional) split unsteady.rs later; or keep flat:
  structure_coupling.rs  # apply_structure_internal_boundaries, order, culvert+bridge pass loop
  preissmann.rs          # solve_unsteady_step, Thomas solver (future)

# Unchanged siblings (already extracted):
  bridge_abutment.rs
  bridge_interior.rs
  bridge_validation.rs
  bridge_roadway_compose.rs
  pier_geometry.rs
  deck_vent_geometry.rs
```

**`src/solvers/mod.rs`** stays:

```rust
pub mod bridge;  // now bridge/mod.rs
pub use bridge::{compute_bridge_rating_curve, solve_bridge_from_params, ...};
```

### Dependency graph (allowed edges)

```text
types, section ──► geometry ──► opening ──► low_flow ──► headwater ──► coupling
                      │            │           │
                      │            └───────────┴──► high_flow ──► headwater
                      │
ice_debris ───────────┘

reach_coupling ──► coupling, rating (reads SteadyInputs / UnsteadyBridgeInputs)
unsteady_coupling ──► coupling, reach_coupling, bridge_interior (face tables)
implicit ──► opening, low_flow, headwater (5.3 only)

structure_coupling (unsteady) ──► unsteady_coupling, culvert
```

No circular imports: `opening` must not call `headwater`; `implicit` reads pinned regime from `headwater` dispatch.

### Extraction map (today → target)

| Current location | Target module | Notes |
|------------------|---------------|-------|
| `BridgeSolveParams`, rating serde | `types.rs` | Keep `pub` for WASM/Python |
| `BridgeIceDebrisParams`, ice helpers in opening | `ice_debris.rs` | v32 fields |
| `obstructed_hydraulics` … `yarnell_*` | `opening.rs` / `low_flow.rs` | Split at Yarnell boundary |
| `segment_weir_*`, `combined_high_flow_*` | `high_flow.rs` | Pressure/weir audit tests move with it |
| `solve_bridge_coupled` | `coupling.rs` | Primary steady/unsteady/rating entry |
| `steady.rs` `bridge_coupling_for` | `reach_coupling.rs` | Parameterize `BridgeInputs` trait or enum |
| `unsteady.rs` `couple_bridge_interval` | `unsteady_coupling.rs` | Phase 5.2 outer loop lives here |
| `unsteady.rs` `apply_structure_internal_boundaries` | `unsteady/structure_coupling.rs` | Shared culvert + bridge |
| Future Jacobian row | `implicit.rs` | Isolated from 4k-line monolith |

### Duplication to eliminate (5.0)

Today **`bridge_coupling_for`**, **`bridge_face_geometry_for`**, and **`bridge_deck_profile_for`** are copy-pasted between [`steady.rs`](../../src/solvers/steady.rs) and [`unsteady.rs`](../../src/solvers/unsteady.rs) (~150 lines each). Consolidate into `reach_coupling.rs`:

```rust
pub struct BridgeReachCouplingContext<'a> {
    pub b_idx: usize,
    pub raw_units: UnitSystem,
    pub coupling: &'a BridgeCouplingParams,
    pub face_geo: BridgeFaceSolveGeometry,  // from bridge_interior
    pub deck: Option<BridgeDeckProfile>,
}
```

Steady/unsteady each build context from their input shape, then call shared `solve_bridge_coupled` wrappers.

### Test split (optional 5.0b)

| Current | Target |
|---------|--------|
| `bridge_tests.rs` (3,255 lines) | `bridge/tests/opening.rs`, `low_flow.rs`, `high_flow.rs`, `coupling.rs`, `rating.rs` |
| Layout tests in `bridge_interior.rs` | Keep colocated (already separate file) |
| `unsteady` bridge tests | Move to `bridge/tests/unsteady_coupling.rs` or `unsteady/tests/structures.rs` |

Use `bridge/mod.rs`:

```rust
#[cfg(test)]
mod tests {
    mod opening;
    mod low_flow;
    // ...
}
```

### Refactor PR sequence (recommended)

| PR | Scope | Risk |
|----|-------|------|
| **R1** | `bridge/mod.rs` + move `types`, `section`, `ice_debris` only; re-export; no logic change | Low |
| **R2** | `opening`, `geometry`, `low_flow`, `high_flow`, `headwater`, `coupling`, `rating` | Medium — run full `cargo test` + verification |
| **R3** | `reach_coupling.rs`; delete duplicate helpers in steady/unsteady | Low |
| **R4** | `unsteady/structure_coupling.rs` + `bridge/unsteady_coupling.rs` | Medium |
| **R5** | Split `bridge_tests.rs` (optional, can defer) | Low |

Each PR should be **behavior-neutral** (diff should be mostly `use` path and `mod` shuffles). Avoid mixing R2 with 5.2 feature work.

### How refactor enables Phase 5 coupling

| Phase 5 need | Module home |
|--------------|-------------|
| Reach–structure outer loop (A1) | `structure_coupling.rs` — wrap `solve_unsteady_step` + `unsteady_coupling` |
| `bridge_headwater_implicit_rhs` (B) | `implicit.rs` — calls `opening` + pinned `low_flow` only |
| Jacobian interval tag on bridge `i` | `preissmann.rs` or `structure_coupling.rs` passes `BridgeIntervalTag` |
| Per-step diagnostics | `StructureCouplingStepResults` in `structure_coupling.rs` |
| Steady/unsteady parity tests | `bridge/tests/unsteady_coupling.rs` |

Without refactor, implicit coupling code would land as another ~400-line island inside `bridge.rs` and ~200 lines inside `unsteady.rs`, worsening the problem this phase is meant to solve.

---

## Jacobian hook points (technical sketch)

### Interval tagging

At densification / layout time, mark `bridge_intervals: Vec<(i, b_idx)>` (already built for post-step). Pass a `Option<&[BridgeIntervalTag]>` into `solve_unsteady_step` or a wrapper `solve_unsteady_step_with_structures`.

### Subcritical low-flow implicit row

For interval $i$ with pinned regime `low_a` | `low_b` | `low_c`:

1. Evaluate residual $R = y_i - H(y_i, y_{i+1}, Q_{avg})$ where $H$ calls a new **`bridge_headwater_implicit_rhs`** that exposes the same physics as `solve_bridge_headwater_metric` but returns $(H, \partial H/\partial y_{BU}, \partial H/\partial y_{BD}, \partial H/\partial Q)$ via analytic or centered differences on the **metric** faces.

2. **Continuity** row on the interval unchanged.

3. **Momentum** row at node $i+1$: replace $ME$ with the linearized head relation (or add as third constraint and reduce system — prefer **replace** to keep $2n$ unknowns).

4. Boundary faces: upstream $Q$ BC and downstream stage BC remain as today.

### Differentiation sources

| Term | Source |
|------|--------|
| Yarnell $H_\text{pier}$ | `yarnell_pier_head_loss_from_area` — smooth in $A$, $Q$ |
| Energy / WSPRO segment friction | `energy_head_loss` / `wspro_head_loss` paths in `bridge.rs` |
| Obstructed $A(y)$ | `obstructed_hydraulics` + `compute_dk_dy`-style `∂A/∂y` on BU/BD tables |
| Pressure / weir | **No analytic Jacobian in 5.3** — use B3 explicit |

### Thomas solver impact

Structure intervals still yield **block tridiagonal** form if each bridge affects only its two nodes. Multi-bridge reaches remain banded with 2×2 blocks — no solver change if couplings are local.

If a future version couples **interior bridge nodes** (BU, interior cuts, BD), bandwidth grows → consider dedicated bridge-span elimination or Schur complement (Phase 6+).

---

## Proposed API (after implementation)

All fields optional; default preserves today’s behavior.

| Field | Shape | Description |
|-------|-------|-------------|
| `unsteady_structure_coupling_mode` | scalar | `0` = post-step only (default); `1` = reach–structure–reach outer loop; `2` = implicit Jacobian (low-flow subcritical) with explicit fallback |
| `unsteady_structure_coupling_max_outer_iterations` | scalar | Cap for mode `1` (default 8) |
| `unsteady_structure_coupling_tolerance` | scalar | Face WSEL tolerance, user units (default match `CULVERT_STEP_TOL_*`) |
| `unsteady_structure_coupling_relaxation` | scalar | $\omega$ for face updates (default 1.0) |

**Diagnostics (outputs):**

| Field | Shape |
|-------|-------|
| `structure_coupling_outer_iterations` | `[time_step]` |
| `structure_coupling_converged` | `[time_step]` bool |

No `API_VERSION` bump for design-only. Bump when fields ship (proposed **v33**).

---

## Validation plan

| Test | Purpose |
|------|---------|
| **Steady warm-start parity** | Unsteady with constant $Q(t)$, many steps → face WSEL matches `solve_steady` bridge interval |
| **Mild wave transit** | Single bridge, subcritical pulse; compare A0 vs A1 vs (later) B — monitor spurious oscillations at BU |
| **Regime crossing** | TW ramp through low chord; verify explicit fallback engages when implicit pinned regime invalid |
| **Multi-bridge** | Two bridges, opposite coupling order; outer iteration count and face convergence |
| **Bridge + culvert** | `structure_coupling_order` 0/1/2 under A1 |
| **Reverse $Q(t)$** | Hydrograph sign change; direction-aware faces still correct under stronger coupling |
| **Mass budget** | Numerical check $\int (Q_{i+1}-Q_i)/dx$ vs storage on structure interval per step |

Fixtures: extend `verification/` with a simple unsteady bridge transient JSON once A1 lands.

---

## Open questions (resolve before 5.2)

1. **$Q$ at structure** — Hold Preissmann section $Q$, or require bridge rating to adjust $Q$ at the face (true internal discharge boundary)?
2. **Supercritical unsteady** — Does coupling belong on BD (tailwater solve) when $Fr > 1$ at the bridge? Steady uses `solve_bridge_tailwater`; unsteady today always HW solve except reverse BC case.
3. **Default mode** — When A1 is proven stable, flip default from `0` → `1` or keep explicit single-pass for WASM host backward compatibility?
4. **Culvert first?** — Culvert HW relation is smoother (rating curves); some teams may prefer proving outer loop on culverts before bridges.
5. **Theta / $\Delta t$** — Does A1 need smaller effective $\theta$ on structure steps for stability, or is relaxation enough?
6. **Refactor depth** — Full `bridge/` split (R1–R4) before any 5.2 feature, or minimal extract (`unsteady_coupling` + `structure_coupling` only)? **Recommendation:** at least **R1 + R3 + R4** before 5.2; full hydraulics split (R2) before 5.3.
7. **`unsteady.rs` split** — Co-split Preissmann into `unsteady/preissmann.rs` in 5.0 or defer? **Recommendation:** defer unless file exceeds ~4.5k lines after R4.

---

## Implementation checklist (post–sign-off)

| Step | Work | Tests |
|------|------|-------|
| **5.1** | This design doc (coupling + refactor) | — |
| **5.0 R1–R3** | `bridge/` directory module; `reach_coupling` DRY | Full test suite; no golden changes |
| **5.0 R4** | Extract `structure_coupling` + `unsteady_coupling` | Existing unsteady bridge tests |
| **5.2** | Option A1 outer loop + diagnostics + `unsteady_structure_coupling_*` inputs | Unit: convergence on synthetic bridge reach; contract: metadata fields |
| **5.3** | `bridge/implicit.rs` + Jacobian interval replace (B1+B3) | Unit: derivative sanity; parity vs explicit on low-flow case |
| **5.4** | Verification fixture + `equations.md` § update | `verification/` transient bridge case |
| **5.5** | Culvert implicit/outer sharing | Culvert unsteady regression |
| **5.0 R5** | Split `bridge_tests.rs` (optional) | Same tests, new module paths |

---

## Cross-links

- Unsteady structure bullets: [`equations.md`](../reference/equations.md)
- HEC-RAS gap (implicit vs explicit): [`hecras_parity.md`](../reference/hecras_parity.md)
- Bridge BU/BD layout: [`BRIDGE_INTERIOR_SECTIONS_API.md`](../BRIDGE_INTERIOR_SECTIONS_API.md)
- Reverse-flow face rules: [`bridge_reverse_flow_rating.md`](bridge_reverse_flow_rating.md)
- High-flow regime complexity: [`pressure_weir_combined_flow_audit.md`](pressure_weir_combined_flow_audit.md)
