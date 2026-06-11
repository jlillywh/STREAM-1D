# 4.3 Reverse flow / bi-directional bridge rating — design

**Phase 4.3 · Implemented (API v31)** — bi-directional rating, steady reverse flow, and unsteady post-step bridge coupling for negative section `Q`.

**API v31:** `computeBridgeRatingCurve` accepts negative `q_values`; `tw_wsel_reverse` sets BU tailwater when `Q < 0` (defaults to `tw_wsel`). Steady `flow_rate < 0` uses reversed subcritical/supercritical sweeps and mirrored bridge coupling. Outputs `wsel` / `wsel_down` are **hydraulic** headwater / tailwater (BD HW when `Q < 0`).

**References:** [`bridge.rs`](../../src/solvers/bridge.rs) (`solve_bridge_coupled`, `compute_bridge_rating_curve`, `solve_bridge_tailwater`), [`equations.md`](../reference/equations.md) §6, [`BRIDGE_INTERIOR_SECTIONS_API.md`](../BRIDGE_INTERIOR_SECTIONS_API.md), [`hecras_parity.md`](../reference/hecras_parity.md).

---

## Goals

| Goal | Priority |
|------|----------|
| **Bi-directional rating curve** — hosts pass positive and negative `q_values` (or explicit direction) and receive consistent HW/TW pairs per flow arrow | **P0** |
| **Documented sign convention** — same as steady `flow_rate` and unsteady `initial_q` | **P0** |
| **Tailwater-driven reversal** — rating API expresses “fixed stage on the hydraulic tailwater face” when flow reverses | **P0** |
| **Steady reach** `flow_rate < 0` with inline bridge coupling | **P1** (follow-on) |
| **Unsteady** negative section `Q` with post-step bridge coupling | **P2** (follow-on) |

**Non-goals (4.3):** multi-reach network reversal, 2D bridge hydraulics, automatic detection of reversal from stages alone without a signed `Q`, deck-vent / weir physics when the water surface inverts under the deck (document limitations).

---

## Reach & sign convention

STREAM-1D uses the standard 1D reach convention (same as HEC-RAS export to this engine):

```text
flow +Q ──────────────────────────────► downstream
river station:   100 (US)    52 BU    50 center    48 BD    0 (DS)
```

| Symbol | Meaning |
|--------|---------|
| `Q > 0` | Discharge in **downstream** direction (US → DS along decreasing station) |
| `Q < 0` | Discharge **upstream** (DS → US); **reverse** flow |
| **BU** | Bridge upstream **face** (higher river station) |
| **BD** | Bridge downstream **face** (lower river station) |

**Hydraulic roles vs reach labels:** When `Q < 0`, the **hydraulic upstream** (high-energy / headwater side for the bridge solver) is **BD**, and the **hydraulic downstream** (tailwater side) is **BU**. Reach names BU/BD do **not** swap — only the flow arrow reverses.

---

## Current behavior (as-is)

| Path | Positive Q | `Q ≤ 0` today |
|------|------------|----------------|
| `solve_bridge_wsel` / `solve_bridge_coupled` | BU headwater from BD tailwater | Undefined; `q_metric ≤ 1e-5` short-circuits Yarnell; friction uses `Q²` |
| `solve_bridge_tailwater` | BD tailwater from BU headwater (supercritical helper) | Not used for reverse subcritical |
| `compute_bridge_rating_curve` | Loops `q_values`, passes signed `Q` | No validation; likely inconsistent HW |
| `classify_low_flow` | Uses signed `Q` in `v = Q/A`, Froude | Negative `v` / `Fr` breaks Class A/B/C intent |
| `head_loss` in `solve_bridge_coupled` | `(wsel_up − tw_wsel).max(0.0)` | Clamps reverse gradients to zero |
| Steady `flow_rate` | Standard step + bridge post-step | Negative reach flow not a supported product path |
| Unsteady section `Q` | Coupling uses section `Q` magnitude at structure | No reversal-specific bridge path |

**Implication:** Phase 4.3 must **not** rely on passing negative `Q` through the existing headwater solve unchanged. Use a **direction-aware mirror** (below) or reject until implemented.

---

## HEC-RAS mental model

HEC-RAS 1D steady profiles assume flow in the downstream direction for the standard step method. **Unsteady** simulations allow section velocities (and discharges) to change sign when tailwater or tributary forcing reverses the reach.

For **bridges**:

- Low-flow Class A/B/C, Yarnell, WSPRO, and energy methods are formulated for flow **through** the opening from approach → departure in the **direction of discharge**.
- High-flow pressure/weir uses upstream EGL vs tailwater; reversal implies the “driving” energy is on the opposite face.
- Bridge **rating** in RAS is typically tabulated for positive flow; reverse ratings in practice are either a second table with reversed boundary assignment or taken from unsteady extrema.

**STREAM-1D alignment target:** For rating, match the engineering pattern “fix tailwater on the **downstream side of the flow arrow**, solve headwater on the **upstream side of the flow arrow**” for **each** sign of `Q`, using the same obstruction geometry (piers, deck, abutments) with faces mirrored in the solver.

---

## Proposed behavior

### 1. Direction-aware bridge solve (core primitive)

Introduce an internal **flow direction** flag derived from `sign(Q)`:

```text
direction = sign(Q)  (+1 downstream, −1 upstream)
q_mag = |Q|
```

For `direction = +1`, keep today’s path (BU = hydraulic US, BD = hydraulic DS).

For `direction = −1`, run the **same** equation set with `q_mag` on a **mirrored bridge context**:

| Quantity | `direction = +1` | `direction = −1` (mirror) |
|----------|------------------|---------------------------|
| Hydraulic US table / bed | `table_up`, `z_up` (BU) | `table_down`, `z_down` (BD) |
| Hydraulic DS table / bed | `table_down`, `z_down` (BD) | `table_up`, `z_up` (BU) |
| Tailwater WSEL input | `tw_wsel` at BD | `tw_wsel` at BU |
| Headwater result | WSEL at BU | WSEL at BD |
| Approach / departure (friction `1`) | approach → BU, BD → departure | **swap** approach ↔ departure cuts and segment lengths |
| Ineffective blocks | `ineffective_up` on BU, `ineffective_down` on BD | swap per face |
| Guide banks | approach / departure | swap |
| Pier / deck / abutment geometry | unchanged in opening frame | unchanged (symmetric obstruction model) |
| `head_loss` | `HW_us − TW_ds` (signed, ≥ 0 when subcritical forward) | same formula on **hydraulic** US − DS |

**Magnitude-only terms** (friction `Q²`, velocity head `Q²/(2gA²)`, Yarnell, pressure `√(2gΔH)`): always use `q_mag`.

**Signed terms** (expansion vs contraction in energy/WSPRO when direction matters): apply `direction` so expansion is **along** the flow arrow (implementation detail in 4.3.1).

Return both **reach-frame** and **hydraulic** labels in results (see §4).

### 2. Bi-directional rating curve (`computeBridgeRatingCurve`)

**P0 deliverable:** extend rating curve so one call can sample `q_values` with mixed signs.

#### Boundary semantics (recommended)

Physical reach faces stay labeled BU/BD. Tailwater inputs depend on flow direction:

| Field | Role |
|-------|------|
| `tw_wsel` (existing) | Tailwater at **BD** when `Q > 0` (unchanged default) |
| `tw_wsel_reverse` (new, optional) | Tailwater at **BU** when `Q < 0`. If omitted, use `tw_wsel` (symmetric boundary — document as convenience, not HEC-RAS import default). |

**Per sample:**

```text
if Q > 0:
  TW := tw_wsel on BD
  solve HW on BU
else if Q < 0:
  TW := tw_wsel_reverse ?? tw_wsel on BU
  solve HW on BD
else:
  Q = 0 → return TW on both faces (or omit sample / NaN — see validation)
```

#### Output semantics (reach-frame, direction-aware)

Keep array keys backward compatible; clarify meaning in docs/types:

| Output | Meaning |
|--------|---------|
| `q` | Signed sample discharge (downstream-positive convention) |
| `wsel` | **Headwater** WSEL on the hydraulic upstream face (BU when `Q>0`, BD when `Q<0`) |
| `wsel_down` | **Tailwater** WSEL on the hydraulic downstream face (BD when `Q>0`, BU when `Q<0`) |
| `head_losses` | `wsel − wsel_down` (signed drop in flow direction; ≥ 0 for subcritical solved points) |
| `flow_regimes` | Same strings (`low_a`, `pressure`, …) — regime math uses mirrored context when `Q<0` |

**Monotonicity:** For `Q > 0`, `wsel` should increase with `|Q|` at fixed BD tailwater. For `Q < 0`, `wsel` (at BD) should increase with `|Q|` at fixed BU tailwater. Mixed-sign arrays are **not** required to be monotonic in array index order.

#### Optional explicit direction (alternative API)

If mixed tailwaters are insufficient, allow per-sample direction without negative `Q`:

```json
"q_values": [10, 20, 10, 20],
"rating_flow_directions": [1, 1, -1, -1]
```

`rating_flow_directions[b]` or per-sample array: `1` = downstream, `−1` = upstream. **Precedence:** explicit direction × `|q|` wins over `sign(q)` when both provided. Defer unless hosts need unsigned `q_values` only.

### 3. Tailwater-driven reversal (rating vs reach)

Two related scenarios:

| Scenario | Context | 4.3 handling |
|----------|---------|--------------|
| **A. Rating with negative `Q`** | Host requests reverse rating (fixed BU stage, vary `|Q|`) | §2 — mirror solve |
| **B. Reach stages imply reversal** | Steady/unsteady: US WSEL < DS WSEL but reach still carries `Q > 0` | Outside pure rating — profile solver may need wetting/dry checks; bridge coupling still uses signed section `Q` from reach |
| **C. Unsteady `Q(t) < 0`** | Hydrograph reverses | P2 — post-step coupling calls direction-aware solve with section `Q` |

**Rating API does not infer `Q` from stages alone.** Host supplies signed `q_values` (or direction array). For scenario B, steady/unsteady must propagate signed `Q` from the reach solution (P1/P2).

### 4. Steady / unsteady reach integration (P1 / P2)

**P1 — `solve_steady` with `flow_rate < 0`:**

- Reverse standard-step march direction (already partially supported by regime; verify spacing and loss term signs).
- At bridge intervals, call direction-aware coupling with `Q = flow_rate`.
- BU/BD layout unchanged; only hydraulic mirror inside bridge kernel.

**P2 — `solve_unsteady`:**

- When structure interval `Q < 0`, mirror in post-step bridge coupling (same primitive as rating).
- Iteration cap (5 passes) unchanged; document that convergence with reversal may need smaller `dt` (existing stabilization caveat).

**Supercritical:** `solve_bridge_tailwater` remains the forward-direction inverse for `Q > 0`. For `Q < 0` supercritical, mirror tailwater solve (BD HW → BU TW) — same transform as §1.

---

## Validation & errors

| Condition | Behavior |
|-----------|----------|
| `Q = 0` in rating | Skip sample (no output row) |
| `tw_wsel` below bed at tailwater face | Clamp to `z_bed + ε` (existing) |
| Mixed-sign `q_values` without `tw_wsel_reverse` | Allowed — reverse branch uses `tw_wsel` on BU (document symmetric-boundary convenience) |
| Steady `flow_rate < 0` | Supported (reversed sweeps + bridge mirror); culvert intervals not direction-aware |
| Unsupported pier/deck asymmetry under mirror | Mirrored solve is an **approximation** — see limitations table above |

---

## API sketch (API v31 candidate)

| Field | Where | Description |
|-------|-------|-------------|
| `tw_wsel_reverse` | `BridgeSolveParams`, steady/unsteady optional per bridge | BU tailwater for `Q < 0` rating / coupling |
| `rating_flow_direction` | Rating only, optional scalar or per-sample | Overrides `sign(q)` when set |
| Metadata | `getWasmApiMetadata()` | Document `wsel` / `wsel_down` as **hydraulic** HW/TW, not fixed BU/BD labels |

No change to `bridge_*` reach field names; reversal is behavioral, not a new BU/BD swap in JSON geometry.

---

## Implementation phases (after design sign-off)

| Step | Work | Tests |
|------|------|-------|
| **4.3.1** | `bridge_flow_direction(q)`, mirror context builder, direction-aware `solve_bridge_coupled` | Unit: `\|Q\|` symmetry on rectangular channel; `Q>0` vs `Q<0` HW ordering |
| **4.3.2** | `compute_bridge_rating_curve` + `tw_wsel_reverse`; WASM/Python/types v31 | Verification fixture: ±Q at same `\|Q\|`, symmetric channel → same head loss |
| **4.3.3** | Steady negative `flow_rate` + bridge post-step | Steady profile case on mild slope |
| **4.3.4** | Unsteady negative `Q` coupling | Single-reach hydrograph with flow reversal |

---

## Test plan (verification)

1. **Symmetric channel** (BU = BD width, no piers): `HW(|Q|)` for `+Q` with BD TW equals `HW(|Q|)` for `−Q` with BU TW at same stage.
2. **Asymmetric bridge** (piers or abutments): reverse rating is **not** symmetric — golden JSON vs hand check one negative-`Q` WSPRO point.
3. **High-flow reverse** (if in scope for 4.3.1): pressure branch with `Q < 0`, TW above low chord on BU face.
4. **Regression:** all existing positive-`Q` verification suites unchanged.

Fixture: [`verification/fixtures/bridge_reverse_flow_rating.json`](../../verification/fixtures/bridge_reverse_flow_rating.json).

---

## Known limitations (shipped vs incomplete)

Phase 4.3 is **complete for bridges** on single-reach steady/unsteady and bi-directional rating. The rows below are intentional scope limits or approximations — not missing implementation inside the bridge mirror path.

| Area | Status | Limitation |
|------|--------|------------|
| **Bridge rating** | Shipped (v31) | Negative `q_values`; `tw_wsel_reverse` optional; `Q = 0` samples **skipped** (not emitted) |
| **Bridge steady** | Shipped (v31) | Negative `flow_rate`; reversed subcritical/supercritical/mixed sweeps |
| **Bridge unsteady** | Shipped (v31) | Post-step coupling uses `sign(Q)` at bridge interval; direction-aware TW/HW faces |
| **Culverts** | **Not supported** | Steady/unsteady culvert coupling unchanged — **no** mirror for negative `Q` |
| **Junction / networks** | **Not supported** | Tributary steady junction and multi-reach unsteady networks not validated for reversal |
| **Infer Q from stages** | **Not supported** | Rating and coupling use **signed** `q_values` / section `Q`; inverted WSEL gradient alone does not flip direction |
| **Asymmetric BU vs BD** | Approximation | Mirror swaps hyd US/DS tables and approach/departure; different deck slopes or cuts US vs DS may differ from HEC-RAS |
| **Deck vents / weir** | Audit pending | High-flow weir/overtopping under reverse flow not separately benchmarked vs HEC-RAS |
| **HEC-RAS import** | Host responsibility | `.g01` exports one rating direction; supply `tw_wsel_reverse` (or symmetric `tw_wsel`) for reverse table |
| **`rating_flow_direction`** | Deferred | Unsigned `q_values` + per-sample direction array not implemented; use signed `q_values` |
| **Unsteady convergence** | Caveat | Rapid `Q(t)` sign changes may need smaller `dt`; explicit post-step coupling (≤5 passes), not implicit Jacobian |
| **Mixed-sign rating order** | By design | `wsel` monotonic in array index **not** required when `q_values` mix signs; monotonic within each sign at fixed tailwater |

**Resolved (4.3.1):** Yarnell/momentum use `q_mag` and mirrored face tables; `wsel`/`wsel_down` are **hydraulic** HW/TW; default `tw_wsel_reverse = tw_wsel`; expansion/contraction loss ends swap with mirror.

---

## Checklist linkage

- [x] **4.3 Design** — this document
- [x] **4.3.1** — Direction-aware mirror primitive in `bridge.rs`
- [x] **4.3.2** — Bi-directional `computeBridgeRatingCurve` + API v31
- [x] **4.3.3** — Steady negative `flow_rate` (regime 0/1/2 sweeps)
- [x] **4.3.4** — Unsteady reversal coupling (`apply_structure_internal_boundaries` direction-aware faces)
- [x] **Docs** — `equations.md`, `wasm_api.types.ts`, `api_changelog.md`, `hecras_parity.md`, `BRIDGE_INTERIOR_SECTIONS_API.md`
- [x] **Tests** — Rating curve $Q \in [-Q_{max}, +Q_{max}]$ (`test_bridge_rating_curve_bidirectional_qmax_sweep`, verification suite)
- [x] **Docs** — `testing.md` verification row for reverse-flow fixture
- [x] **Verification** — `bridge_reverse_flow_rating.json` + `tests/bridge_reverse_flow_rating_verification.rs`
- [x] **Docs** — Limitations if incomplete (parity, README, API, equations)

---

## Resolved design decisions (4.3.1)

| Question | Decision |
|----------|----------|
| `Q = 0` in rating | Skip sample (no row in output arrays) |
| Default `tw_wsel_reverse` | `tw_wsel` when omitted |
| Output labels | `wsel` / `wsel_down` = hydraulic HW / TW (not fixed BU/BD labels) |
| Energy expansion/contraction under reverse | Swap loss ends with mirror |
