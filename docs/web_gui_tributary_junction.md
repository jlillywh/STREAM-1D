# Web GUI: Tributary Junction Integration

This document explains how STREAM-1D models a confluence and how the web app should map HEC-RAS geometry into the WASM API.

## Scope (current engine)

STREAM-1D supports **one junction** on **one main stem** with **one tributary**, steady subcritical only:

| Supported | Not supported (today) |
|-----------|------------------------|
| Main stem + one tributary at a shared WSEL node | Multiple tributaries |
| Steady Standard Step | Unsteady junction routing |
| Subcritical profiles at the junction | Supercritical / mixed-regime junctions |
| Culverts and bridges on the main stem | Structures on the tributary reach |

Internally the solver runs **three** backwater sweeps (main downstream, main upstream, tributary), but the **public API exposes two geometry arrays** — not three independent reach objects.

---

## Why HEC-RAS “3 reaches” ≠ STREAM-1D “3 slots”

In HEC-RAS, a simple confluence is often stored as **three reaches**:

| HEC-RAS reach | Role |
|---------------|------|
| **Wailupe – upper** | Main stem upstream of the junction |
| **Kului Gorge – Headwaters** | Tributary upstream of the junction |
| **Wailupe – lower** | Main stem downstream of the junction |

STREAM-1D’s WASM / Python `SteadyInputs` has **two geometry slots**:

| API field | Meaning |
|-----------|---------|
| `cross_sections` | **Entire main stem** — upstream main **and** downstream main in one array |
| `tributary_cross_sections` | Tributary reach only |

There is no third slot for “lower main” as a separate entity. The UI cannot load all three HEC-RAS reaches as three independent branches without preprocessing.

---

## Recommended mapping (Wailupe example)

```
HEC-RAS                          STREAM-1D SteadyInputs
─────────────────────────────────────────────────────────
Wailupe – upper  ──┐
                   ├──►  cross_sections  (one continuous main stem)
Wailupe – lower  ──┘

Kului Gorge      ────►  tributary_cross_sections
```

### Flow and junction fields

| Field | Meaning |
|-------|---------|
| `flow_rate` | Main-channel discharge **above** the junction (Wailupe upper inflow) |
| `tributary_flow_rate` | Tributary inflow at the junction |
| `junction_main_station` | Main-channel station of the junction node (must match a `cross_sections[].station`) |
| `downstream_wsel` | Tailwater on the **lower** main (normal downstream BC) |

Below the junction, the engine uses `flow_rate + tributary_flow_rate` on the lower main automatically.

---

## Workaround: show all three physical reaches in Plan View

You do **not** need three WASM reach slots. You need **two input arrays** that still represent all three physical channels in the UI.

### Option A — Merge in HEC-RAS (geometry export)

1. In the HEC-RAS Geometry Editor, merge **Wailupe – upper** and **Wailupe – lower** into one continuous reach (e.g. **Wailupe – continuous**).
2. Export the `.g01` (or your usual geometry export).
3. Import to the web app:
   - **Main Stem** → Wailupe – continuous
   - **Tributary** → Kului Gorge – Headwaters
4. Set `junction_main_station` to the junction cross-section on the merged main stem.

Plan View can still **style** upper main, lower main, and tributary as three layers by splitting the merged main polyline at `junction_main_station`.

### Option B — Merge on import (no HEC-RAS edit)

If the user imports three separate reaches from a project file:

1. Concatenate **upper main + lower main** cross-sections into `cross_sections` (preserve station values; ensure the junction station appears once).
2. Load the tributary reach into `tributary_cross_sections`.
3. Store original reach names/IDs in app metadata so Plan View can label “Wailupe – upper”, “Wailupe – lower”, and “Kului Gorge” even though the solver sees two arrays.

Option B is usually easier for users who cannot or should not edit HEC-RAS geometry.

---

## Rendering results

### Main stem

Use the usual result arrays aligned with `cross_sections`:

- `wsel`, `velocity`, `froude`, `area`, `critical_wsel`, …

Split the profile at `junction_main_station` for display:

- Stations **above** the junction → upstream main styling / `flow_rate`
- Stations **at/below** the junction → downstream main styling / combined flow

### Tributary

When a junction is modeled, the engine also returns:

- `tributary_wsel`
- `tributary_velocity`
- `tributary_froude`

These align with `tributary_cross_sections` (same index order). Draw the tributary centerline separately in Plan View; the **mouth** (tributary downstream end) should match the junction WSEL on the main stem.

---

## WASM example

```javascript
const inputs = {
  cross_sections: mainStemSections,      // upper + lower Wailupe combined
  flow_rate: 120.0,                      // cms above junction
  tributary_cross_sections: tribSections, // Kului Gorge
  tributary_flow_rate: 45.0,
  junction_main_station: 8500.0,         // must exist in cross_sections
  downstream_wsel: 2.1,
  downstream_bc_type: 0,
  regime: 0,
  num_slices: 100,
};

const results = solveSteady(inputs);

// Main: results.wsel[i] ↔ cross_sections[i]
// Trib: results.tributary_wsel[j] ↔ tributary_cross_sections[j]
```

All three junction fields must be set together; omitting any one falls back to single-reach mode.

---

## Import / validation checklist

- [ ] Main stem `cross_sections` includes junction station and both upstream and downstream main sections
- [ ] `junction_main_station` matches a main cross-section station (± 1e-4 tolerance)
- [ ] At least **two** main sections at/below the junction and **one** at/above (solver requirement)
- [ ] Tributary has at least one section; mouth is the tributary downstream end
- [ ] `flow_rate` = main inflow above junction; `tributary_flow_rate` = tributary inflow
- [ ] `regime: 0` (subcritical) for junction runs today
- [ ] Plan View draws two result polylines when `tributary_wsel` is present

---

## FAQ

**Can we add a third reach slot later?**  
A general multi-reach network (dendritic or looped) would need a graph-based model, not a third array. The current design intentionally caps scope at one confluence.

**Why not put lower main in `tributary_cross_sections`?**  
That slot is hydraulically a **tributary inflow**, not a continuation of the main channel. Mis-assigning lower main to the tributary slot would apply tributary boundary logic and wrong discharge accounting.

**Does merging upper + lower main break stationing?**  
No — stations should remain as in HEC-RAS. Only the **container** (one API array vs two HEC-RAS reaches) changes.

---

## Web import: reach-merge modal (implementation prompt)

Copy the prompt below into a ticket or agent session for the **website import flow**. Do not hard-code project-specific reach names (e.g. Wailupe / Kului Gorge).

<details>
<summary><strong>Prompt for web dev / Cursor</strong></summary>

### Task

Replace any hard-coded “3-reach junction” import logic with a **generic reach-mapping modal** that runs when the user imports a geometry file containing **two or more reaches**.

The modal must let the user choose which imported reaches are merged into the STREAM-1D **main stem** and which reach is the **tributary**, then perform the merge automatically on confirm.

### Background

STREAM-1D WASM `solveSteady()` accepts:

- `cross_sections` — one continuous **main stem** (may combine multiple HEC-RAS reaches)
- `tributary_cross_sections` — one tributary reach (optional)
- `junction_main_station`, `flow_rate`, `tributary_flow_rate` — required when tributary is set

The engine does **not** accept three separate main reaches. If HEC-RAS has upper main + lower main + tributary, the web app must **merge the two main reaches on import** before calling WASM.

Reference: [`docs/web_gui_tributary_junction.md`](web_gui_tributary_junction.md) in the STREAM-1D repo.

### When to show the modal

Show the modal **after** the file is parsed and reaches are listed, **before** geometry is written to app state / sent to WASM.

| Condition | Behavior |
|-----------|----------|
| Import has **1 reach** | Skip modal; single-reach import (existing flow) |
| Import has **2+ reaches** | Open “Configure reach layout” modal |
| User cancels modal | Abort import; do not partially load reaches |

Do **not** assume reach count = 3 or infer roles from reach names.

### Modal UX

**Title:** Configure reach layout

**Body copy (example):**  
*STREAM-1D models one main channel and one optional tributary. Select which reaches form the main stem (merged in order) and which reach is the tributary, if any.*

**Controls:**

1. **Main stem reaches** — multi-select checklist (or ordered list with drag-to-reorder) of all imported reaches. User selects **two or more** reaches that should be concatenated into one main stem (typical case: upstream main + downstream main). Allow **one** reach for a simple main with no merge.

2. **Tributary reach** — single-select dropdown including **“None (single main channel)”**. Exactly **one** reach, or none.

3. **Junction station** — dropdown populated after main reaches are chosen:
   - Default: auto-detect shared station between the two main reaches that meet at the junction (station within `1e-4` on both reaches’ downstream end of upper / upstream end of lower).
   - If ambiguous, list candidate stations and require user selection.
   - Must end up as a station present in the merged `cross_sections`.

4. **Preview (recommended):** short summary before confirm, e.g.  
   *Main stem: Reach A + Reach C (12 cross-sections) · Tributary: Reach B · Junction at station 8500*

**Actions:** Cancel | Import

**Validation (disable Import until valid):**

- Every imported reach assigned to **exactly one** role (main or tributary), or excluded explicitly if you support “skip reach”.
- At most **one** tributary.
- Main stem must have ≥ 1 reach; if tributary is set, main stem merge must yield ≥ 2 cross-sections at/below junction and ≥ 1 above (see engine rules).
- Junction station required when tributary ≠ None.
- No duplicate reach in both main and tributary.

Show inline errors (e.g. “Select which two main reaches meet at the junction”).

### Merge logic on confirm

Implement a pure function, e.g. `mergeReachImport({ reaches, mainReachIds, tributaryReachId, junctionStation })`:

1. **Main stem:** For each main reach in user order (upstream → downstream), append cross-sections.
2. **Deduplicate junction:** If adjacent reaches share a station within `1e-4`, keep **one** cross-section at that station (prefer the reach downstream in the merge order, or average — document choice; do not duplicate stations).
3. **Sort** merged main `cross_sections` by station **descending** (upstream high station → downstream low), matching STREAM-1D convention.
4. **Tributary:** Copy cross-sections from tributary reach unchanged; mouth = tributary downstream end (lowest station).
5. **Structures:** Merge culvert/bridge arrays from main reaches only; re-map stations if needed. Do not attach main structures to tributary slot.
6. **Metadata for Plan View:** Persist original reach IDs/names and segment boundaries, e.g.  
   `{ reachId, name, role: 'main_upper' | 'main_lower' | 'tributary', stationMin, stationMax }`  
   so the UI can still draw three styled polylines even though WASM sees two arrays.

7. **WASM payload:** Set `cross_sections`, `tributary_cross_sections`, `junction_main_station`, plus flows from plan/boundary UI.

### Edge cases

- **2 reaches, no tributary:** Both listed; user selects both as main OR one main + none tributary — support simple merge of two collinear reaches without a junction.
- **2 reaches, one tributary:** One main + one tributary; junction = trib mouth meeting main (user picks station on main).
- **4+ reaches:** Same modal; user picks which merge into main and which one is tributary; unassigned reaches either rejected or “ignored” with warning.
- **Auto-detect failure:** Force manual junction station pick; never silently guess wrong.

### Non-goals

- Do not require users to merge reaches in HEC-RAS first.
- Do not hard-code reach names or project IDs.
- Do not add a third WASM geometry slot.

### Acceptance criteria

- [ ] Importing a multi-reach file always opens the modal (unless 1 reach).
- [ ] User can merge any two main reaches by selection, not by filename.
- [ ] Confirm produces valid `SteadyInputs` and Plan View still shows separate reach labels via metadata.
- [ ] Cancel leaves prior project unchanged.
- [ ] Unit tests for `mergeReachImport()` covering: shared junction dedup, sort order, validation errors.

</details>

---

## Related source

- Junction solver: `src/solvers/junction.rs`
- Input / output types: `src/solvers/steady.rs` (`SteadyInputs`, `SteadyResult`)
- WASM entry: `solve_steady()` in `src/lib.rs`
