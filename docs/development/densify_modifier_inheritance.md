# 2.2 Densified reach sections — ineffective / blocked inheritance

Design for how **reach** `ineffective_flow_areas`, `blocked_obstructions`, and `guide_banks` apply on **interior** nodes created by `max_spacing` densification (not explicit user cuts, BU/BD, or bridge layout inserts).

Modifier semantics: [`reference/equations.md`](../reference/equations.md) §H0.

---

## Current behavior (audit)

Reach densification inserts nodes between user `cross_sections` when spacing exceeds `max_spacing`. Geometry hydraulics come from `interpolate_geometry_table` (linear blend of parent lookup rows vs depth).

| Path | Interior `CrossSection` | Blocked | Ineffective / guide banks |
|------|-------------------------|---------|---------------------------|
| **Steady** (`solve_steady_single_reach`) | `None` | Parent tables include blocked fill from `generate_lookup_table`; blend carries **hydraulic** effect only | **Not applied** (`xs` absent → static table only) |
| **Unsteady** (`solve_unsteady`) | Clone **upstream** user XS (`station` updated) | Same table blend **plus** upstream polylines on wrong geometry | Upstream modifiers evaluated on **upstream polyline**, not interpolated shape |
| **Bridge layout insert** (`bridge_interior`) | Upstream `Option` clone or explicit cut | Explicit cut uses its own XS; interpolated insert copies upstream `Option` | Per [`BRIDGE_INTERIOR_SECTIONS_API.md`](../BRIDGE_INTERIOR_SECTIONS_API.md) — out of scope here |

**Consequences**

1. After phase 2.1 reach ineffective, **steady** standard-step nodes between user sections ignore `ineffective_flow_areas` even when both parents carry the same blocks.
2. **Steady vs unsteady** disagree on interior modifier handling.
3. [`equations.md`](../reference/equations.md) H2 (“interior points do not inherit blockage”) is true for **polylines** but misleading: blocked **hydraulics** already leak via table interpolation.
4. There is no `interpolate_cross_section` — only table interpolation — so dynamic modifiers cannot be applied on a true mid-reach polyline today.

---

## Policy options

### 1. No inheritance (`none`) — steady default today

Interior nodes: interpolated table only; `densified_xs = None`; no reach modifiers.

| Pros | Cons |
|------|------|
| Matches H2 wording; no change to existing steady profiles | Reach ineffective ineffective between sparse user XS in steady; discontinuity at user stations |
| Simple | Unsteady still inconsistent unless aligned to `None` |

### 2. Inherit from upstream parent (`upstream`)

Copy `ineffective_flow_areas`, `blocked_obstructions`, `guide_banks` from the **upstream** user section onto a synthetic interior XS.

| Pros | Cons |
|------|------|
| Matches common HEC-RAS mental model (modifier defined at a surveyed section until the next definition) | Step if upstream/downstream definitions differ |
| Aligns unsteady with a documented rule | Requires synthetic interpolated polyline for correct dynamic geometry |

### 3. Inherit from downstream parent (`downstream`)

Same as (2) but from downstream user section.

Use when modeling encroachment that is tied to the downstream survey; rare as default.

### 4. Nearest parent (`nearest`)

Choose upstream or downstream user section by along-reach distance to interior station.

| Pros | Cons |
|------|------|
| Symmetric; avoids arbitrary upstream bias at midpoints | Discontinuity when closer parent switches; more complex |

### 5. Linear blend (`blend`)

Interpolate activation elevations, blocked crests, and lateral stations between parents.

| Pros | Cons |
|------|------|
| Smooth transitions on paper | **Ill-defined** for OR ineffective blocks, mismatched `x` grids, and blocked fill; easy to double-count with table blend |
| | No HEC-RAS benchmark in repo yet |

**Recommendation:** defer **blend** until a verified HEC-RAS case requires it. Not part of 2.2 implementation.

---

## Recommended policy (2.2)

### Reach densification

| Modifier | Policy |
|----------|--------|
| **`ineffective_flow_areas`** | **`upstream` inherit** on interior nodes (synthetic XS + dynamic geometry). |
| **`guide_banks`** | Same as ineffective — **`upstream` inherit** (guide banks are reach modifiers on the cut). |
| **`blocked_obstructions`** | **Do not copy polylines** onto interior XS. Keep **hydraulic** effect via existing table interpolation from parent tables (blocked baked at user stations). Document explicitly in §H2. |

Rationale:

- Ineffective is reach-lateral and stage-dependent; it must run through `geometry_row_at_elevation` on an interior cut, not table blend alone.
- Blocked already affects interior conveyance through blended tables; copying polylines onto a synthetic XS risks **double application** once dynamic geometry is enabled.
- Upstream inherit is the smallest change that fixes steady/unsteady parity and matches “modifier stays until redefined at next river station.”

### API (proposed `API_VERSION` bump)

Add optional field on steady and unsteady inputs:

```text
densify_reach_modifier_policy: u8
  0 = none          — interior nodes: no reach modifiers (current steady behavior)
  1 = upstream      — recommended default for new models with reach ineffective
  2 = downstream
  3 = nearest
```

**Backward compatibility:** default **`0` (`none`)** so existing steady profiles unchanged. README / limitations: integrators using reach ineffective with `max_spacing` should set **`1`**.

### Bridge / explicit cuts

Unchanged:

- Explicit BU/BD/approach/departure `CrossSection` values win on their station.
- Bridge layout interpolated inserts without explicit XS keep today’s upstream-`Option` clone (bridge module), not the reach policy enum.

---

## Implementation outline (follow-on tasks)

- [ ] **`interpolate_cross_section(up, down, t)`** — interpolate `x`/`y`, `n_stations`/`n_values`, `is_overbank`; set `station`; leave modifiers empty.
- [ ] **`apply_reach_modifier_policy(synthetic, up, down, t, policy)`** — copy modifier fields per policy table above.
- [ ] **`densify_reach_between(...)`** — shared helper; returns `(GeometryTable, z_min, Option<CrossSection>)`; replace duplicated loops in `steady.rs` and `unsteady.rs`.
- [ ] **Steady:** push `Some(synthetic_xs)` when policy ≠ `none`.
- [ ] **Tests:** two user XS, same upstream ineffective, `max_spacing` interior node — steady conveyance at interior matches user XS when policy = `upstream`; policy = `none` unchanged.
- [ ] **Docs:** update §H2 footnote (blocked hydraulics vs polylines); `hecras_parity.md` one line; `api_changelog.md` when field ships.

---

## Open questions

1. **HEC-RAS verification** — Does RAS interpolate ineffective between river stations, or hold the upstream definition? Capture one sparse-XS project export before changing default from `none` to `upstream`.
2. **Blocked table vs polyline** — Long term, should blocked move fully dynamic (no bake in `generate_lookup_table`) to one code path? Out of scope for 2.2; note for geometry refactor.
3. **Default flip** — After one release with opt-in `upstream`, consider default `1` in a major version with changelog migration note.

---

## Checklist (phase 2.2)

- [x] **Design** — policy above (`upstream` inherit for ineffective/guide; blocked via table only; API enum default `none`)
- [x] **Implement** — shared densify helper + steady/unsteady wiring
- [ ] **Test** — steady/unsteady parity on interior ineffective
- [ ] **Document** — equations §H2, api_changelog, README limitation line
