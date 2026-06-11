# 3.3 Deck vents & slotted openings — API design

Vertical relief openings through the bridge deck superstructure (vents, grates, slotted drains) that add **supplemental pressure-flow area** above the main low chord. **API design only** — solver not implemented until checklist items after this document land.

**Today (v28):** deck blockage and pressure/weir high flow use piecewise `bridge_deck_*` low/high chords ([`equations.md`](../reference/equations.md) §E–§G). Net pressure-flow area is the trapezoidal opening under the **minimum** low chord minus piers and abutments (`net_opening_area_at_low_chord` in [`bridge.rs`](../../src/solvers/bridge.rs)). A single submerged coefficient per bridge (`bridge_orifice_coeffs`, default 0.8) applies to that net area. There is no way to declare **localized deck slots** with their own invert/soffit band and discharge coefficient.

**Gap:** Box-girder bridges, open-grate decks, and curb slots often pass flow through the deck slab **above** the main opening soffit but **below** the roadway crest. Hosts today fake this by lowering `bridge_deck_low_elevations` locally (distorts low-flow limits) or omit the relief path (under-predicts pressure discharge).

---

## HEC-RAS mental model

| Concept | HEC-RAS 1D | STREAM-1D today | 3.3 addition |
|---------|------------|-----------------|--------------|
| Deck low / high chord profile | Deck editor station table | `bridge_deck_*` | unchanged — defines main opening soffit and weir crest |
| Pressure-flow net area under deck | BU/BD opening minus piers/abutments | `net_opening_area_at_low_chord` × `profile_opening_area_factor` | unchanged for **main** opening |
| Submerged orifice $C_d$ | One `Orifice C` per bridge opening | `bridge_orifice_coeffs` | **Per-segment** $C_d$ on supplemental vents |
| Deck relief vents / slotted drains | **Not a separate 1D field** — hosts lower low chord or add culverts | — | Explicit **vent segments** (§A) |
| Slotted curb / grate along deck | — | — | Same segment model; optional **type** (§B) for solver equation selection |

HEC-RAS models high flow as pressure under the **low chord** and weir over the **high chord** only. Deck vents are a **STREAM-1D extension** for structures where the deck slab contains additional vertical openings. Importers from pure RAS geometry can omit these fields; hosts with as-built grate/slot data can supply them without distorting the main deck profile.

---

## Scope split

| Feature | Geometry | Primary effect |
|---------|----------|----------------|
| **Deck vent** (§A) | Lateral band on deck + invert/soffit elevations | Extra submerged **orifice area** and parallel $Q_{vent}$ in pressure flow |
| **Slotted opening** (§B) | Same segment arrays; `type = 1` | Same API; solver may treat as **slot weir** along segment length when implemented |
| **Main opening** (existing) | `bridge_deck_*` low chord | Low-flow limit, pressure-flow trigger, Yarnell/momentum opening width |

Vents do **not** replace the deck profile. They add flow paths **between** each segment’s invert and soffit when the hydraulic grade exceeds the invert. Segment soffit is typically ≤ local deck high chord at that station; segment invert is typically ≥ main low chord at that station (warning if not).

---

## Proposed fields (steady / unsteady)

Per bridge `b`, per vent segment `v` (0-based index; order is not hydraulic — segments are summed).

### §A — Vent / slot segments (optional)

| Field | Shape | Description |
|-------|-------|-------------|
| `bridge_deck_vent_left_stations` | `[bridge][vent]` | Left edge of segment in **opening** coordinates (`s`, same frame as `bridge_deck_stations`). |
| `bridge_deck_vent_right_stations` | `[bridge][vent]` | Right edge; must be `> left`. |
| `bridge_deck_vent_invert_elevations` | `[bridge][vent]` | Bottom elevation of the opening (invert). Flow through the segment is evaluated only when WSEL / upstream EGL exceeds this elevation. |
| `bridge_deck_vent_soffit_elevations` | `[bridge][vent]` | Top elevation of the opening (soffit / underside of slot). Submerged height at evaluation is `min(WSEL, z_soffit) − z_invert` (clamped ≥ 0). |
| `bridge_deck_vent_discharge_coefficients` | `[bridge][vent]` | Orifice discharge coefficient $C_d$ for this segment in pressure flow. Default: `bridge_orifice_coeffs[b]` when omitted or ≤ 0. Typical 0.6–0.9. |

**Shorthand (optional):** when a segment is symmetric about a pier or grate centerline, hosts may supply center + width instead of left/right:

| Field | Shape | Description |
|-------|-------|-------------|
| `bridge_deck_vent_stations` | `[bridge][vent]` | Centerline station in opening `s`. |
| `bridge_deck_vent_widths` | `[bridge][vent]` | Total width perpendicular to flow at the segment (opening-frame extent). |

**Compose rule:** if `bridge_deck_vent_stations` and `bridge_deck_vent_widths` are set for segment `v` and left/right are omitted, resolve `left = center − width/2`, `right = center + width/2`. If both pair and center forms are set, **left/right wins**.

### §B — Segment type (optional)

| Field | Shape | Description |
|-------|-------|-------------|
| `bridge_deck_vent_types` | `[bridge][vent]` | `0` = **vent** (vertical orifice; default) — $Q = C_d A_{sub} \sqrt{2g\,\Delta H}$. `1` = **slotted** — same geometry; implementation may apply slot-weir exponent on segment length × head (design TBD in §E). |

Omit `bridge_deck_vent_types` → all segments are type `0`.

---

## Precedence

1. **Deck profile** (`bridge_deck_*`, scalars `bridge_low_chords` / `bridge_high_chords`) — authoritative main opening and weir crest (v13+).
2. **Roadway embankment compose** (v26) — may supply `deck` profile; does not auto-create vents.
3. **Vent segments** (§A) — additive supplemental area / discharge; never lowers main low chord.
4. **Global orifice coefficient** (`bridge_orifice_coeffs`) — default $C_d$ for main submerged opening **and** for any vent segment without an explicit coefficient.
5. **Inlet sluice coefficient** (`bridge_pressure_flow_coeffs_inlet`) — unchanged; applies to main opening sluice-gate case only, not per-vent $C_d$.

Vent segments are **ignored** in low flow (WSEL below main low chord) and in pure weir overtopping unless implementation later couples slot overflow to weir (non-goal for initial 3.3).

---

## Coordinate frames

| Axis | Frame |
|------|--------|
| Horizontal | Opening `s` — station 0 at left deck edge, increasing rightward (§G, §I in [`equations.md`](../reference/equations.md)) |
| Elevations | Absolute user units (metric or US), same as `bridge_deck_low_elevations` |
| Width | Lateral extent in opening `s`; projected flow-normal width for area uses skew: $W' = (s_{right} - s_{left}) / \cos\theta$ with `bridge_skew_angles` |

Preprocessor: vent stations remap with `opening_origin` like `bridge_deck_stations` ([`BRIDGE_INTERIOR_SECTIONS_API.md`](../BRIDGE_INTERIOR_SECTIONS_API.md) §1.3.2).

---

## Rating curve (`computeBridgeRatingCurve`)

Flattened keys on `BridgeSolveParams` (no `bridge_` prefix):

| Steady / unsteady | Rating curve |
|-------------------|--------------|
| `bridge_deck_vent_left_stations` | `deck_vent_left_stations` |
| `bridge_deck_vent_right_stations` | `deck_vent_right_stations` |
| `bridge_deck_vent_stations` | `deck_vent_stations` |
| `bridge_deck_vent_widths` | `deck_vent_widths` |
| `bridge_deck_vent_invert_elevations` | `deck_vent_invert_elevations` |
| `bridge_deck_vent_soffit_elevations` | `deck_vent_soffit_elevations` |
| `bridge_deck_vent_discharge_coefficients` | `deck_vent_discharge_coefficients` |
| `bridge_deck_vent_types` | `deck_vent_types` |

---

## Optional nested form (`bridge_roadway_embankments`)

Hosts using the unified embankment object (v26) may alternatively embed vents:

```typescript
bridge_roadway_embankments[b].deck_vents?: {
  left_stations: number[];
  right_stations: number[];
  invert_elevations: number[];
  soffit_elevations: number[];
  discharge_coefficients?: number[];
  types?: number[];
}[];
```

**Precedence:** explicit flat `bridge_deck_vent_*` arrays win when present for bridge `b`; nested `deck_vents` is composed only when flat vent arrays are omitted. Design-only until v26 compose is extended.

---

## Planned solver use (implementation — not in 3.3 API doc)

At pressure-flow evaluation (EGL above main low chord), compute per segment at upstream WSEL / energy grade $E_{up}$:

$$A_{vent,v}(WSEL) = W'_v \cdot \bigl(\min(WSEL,\, z_{soffit,v}) - z_{invert,v}\bigr)^+$$

$$Q_{vent} = \sum_v C_{d,v}\, A_{vent,v}(WSEL)\, \sqrt{2g\,(E_{up} - H_{drive,v})}$$

where $H_{drive,v}$ is downstream tailwater or slot exit grade (initial implementation: same drive head as main submerged orifice — $E_{up} - TW_{down}$).

**Total pressure discharge** (pressure-only regime, EGL above low chord but below weir onset):

$$Q_{pressure} = Q_{opening} + Q_{vent}$$

**Combined high flow** (EGL above high chord — implemented in `combined_high_flow_discharge`):

$$Q_{total} = Q_{opening} + Q_{vent} + Q_{weir}$$

with $A_{net}$ unchanged from today (main opening under deck low chord). $Q_{opening}$ uses sluice-gate or submerged-orifice on $A_{net}$; $Q_{weir}$ is Bradley overtopping on effective deck length; all three terms are summed in `solve_high_flow` / `solve_high_flow_tailwater` balance.

Type `1` (**slotted**): optional $Q \propto L' \,(z_{soffit} - z_{invert})\, (E_{up} - z_{invert})^{3/2}$ when slot acts as broad-crested weir in the deck — §E in [`equations.md`](../reference/equations.md) when implemented.

**Low flow / energy method:** vents do not change `obstructed_hydraulics` pier/deck blockage until WSEL exceeds segment invert (no “phantom” opening area).

---

## JSON examples

**Two grate vents on one bridge** (opening stations in ft, elevations NAVD):

```json
"bridge_deck_vent_left_stations": [[12.0, 38.0]],
"bridge_deck_vent_right_stations": [[14.0, 40.0]],
"bridge_deck_vent_invert_elevations": [[102.5, 102.5]],
"bridge_deck_vent_soffit_elevations": [[104.0, 104.0]],
"bridge_deck_vent_discharge_coefficients": [[0.65, 0.65]]
```

**Center + width shorthand** (10 ft slot centered at `s = 25`):

```json
"bridge_deck_vent_stations": [[25.0]],
"bridge_deck_vent_widths": [[10.0]],
"bridge_deck_vent_invert_elevations": [[101.0]],
"bridge_deck_vent_soffit_elevations": [[103.5]],
"bridge_deck_vent_discharge_coefficients": [[0.75]],
"bridge_deck_vent_types": [[1]]
```

**Default $C_d$ from bridge orifice** (omit per-segment coefficients):

```json
"bridge_orifice_coeffs": [0.8],
"bridge_deck_vent_left_stations": [[5.0]],
"bridge_deck_vent_right_stations": [[8.0]],
"bridge_deck_vent_invert_elevations": [[100.0]],
"bridge_deck_vent_soffit_elevations": [[102.0]]
```

---

## Validation (implementation)

| Check | Severity |
|-------|----------|
| All vent arrays for bridge `b` have equal `[vent]` length when any is set | Error |
| `right_station > left_station` (after center/width resolve) | Error |
| `soffit_elevation > invert_elevation` | Error |
| Segment lateral extent within deck opening span (left/right abutment outer faces or deck profile extent) | Warning |
| `invert` below main low chord at segment mid-station | Warning (vent may never activate before main pressure flow) |
| `soffit` above local deck high chord at mid-station | Warning |
| Overlapping vent segments on same bridge | Warning (double-count risk — implementation may union or sum explicitly) |
| `discharge_coefficient` ≤ 0 with no bridge-level `bridge_orifice_coeffs` | Error |

---

## Non-goals (3.3)

- Replacing `bridge_deck_*` with vents (main low chord remains required for low-flow and weir onset).
- Separate US/DS vent geometry (single definition per segment, as deck profile today).
- 2D plunging flow over deck (HEC-RAS 6.7 2D bridge) — 1D post-step coupling only.
- Auto-detecting slots from BU/BD cross-section cut lines.
- Nested pier-level deck drains (use vent segments at pier station).
- Changing Bradley weir overtopping for roadway crest (vents are below crest).

---

## Checklist

- [x] **API** — vent stations, invert/soffit, discharge coefficient per vent segment (this document)
- [ ] **Types** — serde on `SteadyInputs` / `UnsteadyBridgeInputs` / `BridgeSolveParams`; `wasm_api.types.ts` (API v29)
- [ ] **Resolve** — center/width → left/right; remap stations with `opening_origin`; optional `deck_vents` compose from `bridge_roadway_embankments`
- [x] **Hydraulics** — parallel-path $Q_{vent}$ summed with main pressure flow (`pressure_flow_discharge`); type `0` orifice, type `1` slot weir / full orifice
- [x] **High flow** — combined $Q = Q_{opening} + Q_{vents} + Q_{weir}$ (`combined_high_flow_discharge` in `solve_high_flow` / tailwater inverse)
- [ ] **Solver** — rating curve / WASM metadata (serde types v29)
- [ ] **Tests** — WASM deserialize; rating HW with vents
- [x] **Tests** — partially submerged deck with vents (pressure regime, partial vent area, opening + vent balance)
- [x] **Tests** — segment area at WSEL; summed $Q$ vs hand orifice (`deck_vent_geometry` + `bridge_tests`)
- [x] **Docs** — README high-flow section ([`README.md`](../../README.md) § Bridge high flow)
- [ ] **Docs** — `api_changelog` v29, `equations.md` §E2, `hecras_parity.md`
