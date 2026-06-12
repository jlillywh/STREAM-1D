# 4.4 Bridge ice / debris (optional) — API design

Optional modifiers for **floating pier debris** and **ice / jam blockage** at bridge openings. **API v32** — hydraulics implemented in `obstructed_hydraulics`, `net_opening_area_at_low_chord`, and weir helpers.

**Today (v31):** pier obstruction is shaft + footing + nosing ([`pier_footings_nosing.md`](pier_footings_nosing.md)); deck low/high chords define pressure and weir limits ([`equations.md`](../reference/equations.md) §E–§G). HEC-RAS **floating debris** and **river ice at bridges** are listed as gaps in [`hecras_parity.md`](../reference/hecras_parity.md).

**Gap:** High flows with trash racks, ice jams, or frazil slush reduce net opening area and pier-adjacent conveyance. Hosts today duplicate blockage on BU/BD `blocked_obstructions` (breaks pier-loss accounting) or omit the effect.

---

## HEC-RAS mental model

| HEC-RAS concept | Editor / data | Effect in 1D |
|-----------------|---------------|--------------|
| **Floating pier debris** | Pier editor: debris **width** + **height** per pier | Rectangular block **centered on upstream pier**, **top at WSEL**, height extends downward; subtracts opening area and wetted perimeter; overlaps with ground, abutments, or adjacent pier debris are clipped |
| **River ice at bridge** | Bridge options: (1) no ice, (2) **constant thickness** from US XS through bridge, (3) **dynamic jam** at bridge | Ice thickness reduces flow area under the cover; interaction with low chord / deck is case-dependent — RAS warns users to validate |
| **Reach ice thickness** | Cross-section ice tab | Thickness at section; bridge can inherit US thickness (option 2) |

STREAM-1D does **not** model full river-ice hydraulics (frazil, transport, jam formation). Phase 4.4 adds **optional, host-supplied** blockage at the bridge opening — either a simple factor or HEC-RAS-shaped pier debris + constant ice thickness.

References: [HEC-RAS Modeling Floating Pier Debris](https://www.hec.usace.army.mil/confluence/rasdocs/ras1dtechref/6.1/modeling-bridges/unique-bridge-problems-and-suggested-approaches/modeling-floating-pier-debris), [River Ice at bridges](https://www.hec.usace.army.mil/confluence/rasdocs/rasum/6.5/entering-and-editing-geometric-data/river-ice).

---

## Goals

| Goal | Priority |
|------|----------|
| **Host-friendly optional modifier** — single scalar or per-bridge factor for quick studies | **P0** |
| **HEC-RAS pier debris parity** — per-pier width + height floating at WSEL | **P0** |
| **Constant ice thickness through opening** — RAS bridge option “ice remains constant” | **P1** |
| **Rating curve** — same fields on `computeBridgeRatingCurve` | **P0** |
| **Dynamic ice jam at bridge** | **Deferred** (P2) |

**Non-goals (4.4):** reach-wide ice transport, unsteady ice growth, ice-on-culvert catalog, debris projection to upstream XS (RAS projects debris to bounding section — defer), 2D ice accumulation under deck.

---

## Scope split

| Feature | Attached to | Primary effect |
|---------|-------------|----------------|
| **Opening blockage factor** (§A) | Bridge | Scales net opening area / conveyance at structure |
| **Floating pier debris** (§B) | Pier | Extra rectangular blockage at WSEL, upstream of pier |
| **Ice cover thickness** (§C) | Bridge | Raises effective bed / lowers opening under deck |
| **Deck ice** (§D) | Bridge | Optional ice thickness on roadway (weir crest) |

§A is mutually composable with §B–§D; implementation applies §B pier debris first, then §C ice on opening height, then §A factor on remaining area (see Precedence).

---

## Proposed fields (steady / unsteady)

Per bridge `b`. Per pier `i` where noted (same indexing as `bridge_pier_stations`).

### §A — Opening blockage factor (optional, simple)

For rapid sensitivity studies when detailed debris geometry is unknown.

| Field | Shape | Description |
|-------|-------|-------------|
| `bridge_opening_blockage_factors` | `[bridge]` | Multiplier on **net opening area** and **active conveyance** at the bridge solve (0–1). `1.0` or omit = no extra blockage. `0.85` ≈ 15% area reduction. |

**Rating curve key:** `opening_blockage_factor` (scalar per solve params).

**Solver use (planned):** multiply `a_eff` from `obstructed_hydraulics` and pressure-flow `A_net` by factor; do **not** scale pier Yarnell $A_{pier}$ separately (debris on piers is §B).

### §B — Floating pier debris (HEC-RAS-shaped, optional)

| Field | Shape | Description |
|-------|-------|-------------|
| `bridge_pier_debris_widths` | `[bridge][pier]` | **Total** lateral extent of debris rectangle in opening coordinates (perpendicular to flow), centered on pier centerline. Includes pier width + overhang (RAS: pier width + left + right extensions). |
| `bridge_pier_debris_heights` | `[bridge][pier]` | Vertical height of debris block **below WSEL** (user units). Top of block pinned to **local WSEL** at evaluation (RAS: debris floats at water surface). |

**Rating curve keys:** `pier_debris_widths`, `pier_debris_heights` (vectors aligned with `pier_stations` / `num_piers`).

**Compose rule (planned):** at each WSEL, for each wet pier, add debris area

```text
A_debris = min(W_debris, W_opening_at_pier) × min(H_debris, WSEL − z_bed)
```

in opening plane (skew: use `W' = W / cos θ`). Clip against abutments and adjacent pier debris (union, no double count). Add to pier obstruction in `pier_submerged_area_at_wsel` **or** separate term in `obstructed_hydraulics` — implementation must avoid double-counting shaft area under the debris rectangle.

**Enable:** omit arrays or zero width/height → no debris for that pier.

### §C — Ice cover thickness (constant through bridge, optional)

HEC-RAS option: ice thickness at bridge = thickness at cross section immediately upstream.

| Field | Shape | Description |
|-------|-------|-------------|
| `bridge_ice_thicknesses` | `[bridge]` | Constant ice thickness (user units) applied through the bridge opening. |
| `bridge_ice_modes` | `[bridge]` | `0` = **none** (default). `1` = **constant thickness** (§C). `2` = **reserved** dynamic jam (not implemented in 4.4). |

**Rating curve keys:** `ice_thickness`, `ice_mode`.

**Solver use (planned):**

- Raise effective bed for opening area: `z_eff = z_bed + t_ice` (clamped so `z_eff < low_chord`).
- Reduce submerged opening height under deck by `t_ice` in pressure / low-flow area integrals.
- Do **not** change deck high chord unless §D is set.

Hosts running reach-wide ice elsewhere supply the thickness explicitly on the bridge (STREAM-1D has no reach `ice_thickness` on cross sections in v31).

### §D — Deck / roadway ice (optional)

| Field | Shape | Description |
|-------|-------|-------------|
| `bridge_deck_ice_thicknesses` | `[bridge]` | Ice on roadway surface: lowers effective **high chord** for weir overtopping by `t_deck` (i.e. `z_high_eff = z_high − t_deck`). Independent of §C under-deck ice. |

**Rating curve key:** `deck_ice_thickness`.

Omit when only sub-surface ice jam (§C) is modeled.

---

## Precedence

1. **Pier shaft / footing / nosing** (v27–v28) — base $A_{pier}$.
2. **Floating pier debris** (§B) — additive blockage at WSEL; clipped vs abutments and neighbors.
3. **Ice thickness** (§C) — effective bed raise / opening height reduction.
4. **Deck ice** (§D) — weir crest lowering only.
5. **Opening blockage factor** (§A) — final scalar on net opening / conveyance after §B–§D.

**Blocked obstructions on BU/BD:** hosts may still use reach `blocked_obstructions` for permanent fill. Do **not** duplicate the same ice/debris there if §B–§D are set (validation warning).

---

## Coordinate frames

| Quantity | Frame |
|----------|--------|
| `bridge_pier_debris_widths` | Opening `s` lateral extent (total width); pier center from `bridge_pier_stations` |
| Debris height | Absolute depth below WSEL (not elevation) |
| Ice thicknesses | Vertical thickness in user units (metric or US) |
| Skew | Debris width in opening plane; projected flow-normal width uses `bridge_skew_angles` |

---

## Rating curve (`computeBridgeRatingCurve`)

Flattened keys on `BridgeSolveParams` (no `bridge_` prefix):

| Steady / unsteady | Rating curve |
|-------------------|--------------|
| `bridge_opening_blockage_factors` | `opening_blockage_factor` |
| `bridge_pier_debris_widths` | `pier_debris_widths` |
| `bridge_pier_debris_heights` | `pier_debris_heights` |
| `bridge_ice_thicknesses` | `ice_thickness` |
| `bridge_ice_modes` | `ice_mode` |
| `bridge_deck_ice_thicknesses` | `deck_ice_thickness` |

---

## Planned solver hooks (implementation)

| Function / struct | Change |
|-------------------|--------|
| `BridgeGeometry` | `opening_blockage_factor`, `ice_thickness_m`, `ice_mode`, `deck_ice_thickness_m`, per-pier `debris_width_m` / `debris_height_m` |
| `obstructed_hydraulics` | Include §B debris area; apply §C to `z_bed`; apply §A to `a_eff` |
| `net_opening_area_at_low_chord` | §C ice reduces vertical gap; §A scales result |
| `pier_submerged_area_at_wsel` / Yarnell | §B debris as additional pier-side blockage |
| `solve_high_flow` / weir | §D lowers `high_chord_max_m` |
| `build_bridge_geometry` | Resolve fields from steady/unsteady/rating inputs |

**Low flow:** debris and ice affect opening area and pier drag when WSEL is above effective bed + debris height.

**Reverse flow (v31):** mirror applies to hyd US/DS tables; debris remains pinned to **local WSEL** on the pier face in opening coordinates (direction does not flip debris to downstream face in initial 4.4 — document as limitation).

---

## Validation & warnings

| Condition | Behavior |
|-----------|----------|
| `opening_blockage_factor` ∉ (0, 1] | Clamp to `(ε, 1]` or reject at validate |
| `ice_mode = 1` and `ice_thickness = 0` | Treat as no ice |
| `ice_thickness` ≥ (low_chord − bed) | Warning; cap effective ice to keep positive opening |
| Debris width &lt; pier width at pier | Use pier width as minimum (RAS debris includes pier) |
| Both §B and high `opening_blockage_factor` | Warning — likely double-counting |
| `blocked_obstructions` on BU/BD overlapping pier debris | Warning in `validateSteadyInputs` |

---

## JSON examples

**15% uniform opening reduction (flood study):**

```json
"bridge_opening_blockage_factors": [0.85]
```

**HEC-RAS-style debris on two piers** (6 ft pier + 2 ft each side → 10 ft total width, 4 ft height below WSEL):

```json
"bridge_pier_stations": [[40.0, 60.0]],
"bridge_pier_debris_widths": [[10.0, 10.0]],
"bridge_pier_debris_heights": [[4.0, 4.0]]
```

**Constant ice 0.5 m through bridge + deck ice for weir:**

```json
"bridge_ice_modes": [1],
"bridge_ice_thicknesses": [0.5],
"bridge_deck_ice_thicknesses": [0.2]
```

**Rating curve** (flattened):

```json
{
  "q_values": [20.0, 40.0],
  "tw_wsel": 2.5,
  "opening_blockage_factor": 0.9,
  "pier_debris_widths": [8.0],
  "pier_debris_heights": [3.0],
  "ice_thickness": 0.3,
  "ice_mode": 1
}
```

---

## Implementation phases (after design sign-off)

| Step | Work | Tests |
|------|------|-------|
| **4.4.1** | Serde + types v32; `BridgeGeometry` resolve; WASM metadata | Contract: fields deserialize; metadata lists keys |
| **4.4.2** | §A opening factor + §B pier debris in `obstructed_hydraulics` | Unit: debris increases HW at fixed TW; factor scales area |
| **4.4.3** | §C ice thickness + §D deck ice | Hand calc: ice reduces pressure Q; weir onset delayed |
| **4.4.4** | Rating curve + steady/unsteady wiring | Verification fixture vs HEC-RAS debris example |
| **4.4.5** | Docs + parity table update | `equations.md` §, `testing.md` row |

---

## Known limitations (4.4)

| Item | Note |
|------|------|
| Dynamic ice jam (`ice_mode = 2`) | Not in initial scope |
| Reach XS ice inheritance | No `CrossSection.ice_thickness`; hosts set `bridge_ice_thicknesses` |
| Debris projection to approach XS | RAS projects upstream; STREAM-1D opening-local only |
| Reverse flow | Debris stays on upstream pier face in opening frame; not mirrored to BD |
| Culverts | No ice/debris fields on culvert inputs |
| Unsteady time-varying ice | Constant per bridge per run; no `ice_thickness(t)` |

---

## Checklist linkage

- [x] **4.4 Design** — this document
- [x] **4.4.1** — API types + metadata (v32)
- [x] **4.4.2** — Opening factor + pier debris solver
- [x] **4.4.3** — Ice thickness + deck ice
- [ ] **4.4.4** — Rating / reach integration + verification
- [x] **Hydraulics** — reduce opening area / weir length
- [ ] **Docs** — `equations.md`, `testing.md` (wasm types + changelog updated)

---

## Open questions (resolve before 4.4.1)

1. **§A factor scope** — scale only `a_eff` / `A_net`, or also pier $A_{pier}$? **Recommendation:** opening only; pier debris is §B.
2. **Debris width semantics** — total width vs overhang beyond pier? **Recommendation:** total width (RAS convention); document in metadata.
3. **Ice vs low chord** — raise bed or lower soffit? **Recommendation:** raise `z_eff` for area; subtract from vertical gap to low chord.
4. **Compose with deck vents** (3.3) — ice blocks vent invert? **Recommendation:** §C reduces vent submerged height when implemented.
