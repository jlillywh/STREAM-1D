# Migration: v19/v20 bridge modifiers → v26 unified embankment

For hosts upgrading from flat ineffective/blocked fields to `bridge_roadway_embankments` (API **v26**). Flat fields remain supported indefinitely.

**Semantics:** [`equations.md`](../reference/equations.md) §H0 · **Unified compose:** §G2 · **Design:** [`roadway_embankment_unified.md`](roadway_embankment_unified.md)

---

## v19 — multiple ineffective blocks

`bridge_ineffective_*` arrays accept **nested** shapes per bridge:

| Before (v18) | After (v19+) |
|--------------|--------------|
| `"bridge_ineffective_left_stations": [5.0]` | `"bridge_ineffective_left_stations": [[5.0, 12.0]]` |
| one block per bridge | `[[s0, s1], [s2]]` = multiple blocks on bridge 0 |

Per-face overrides (v18): `*_upstream` / `*_downstream` still override shared fields on that face only. Stations are **opening frame** (`s = 0` at left deck edge); remap via `bridge_opening_reach_station_origins`.

---

## v20 — `blocked_obstructions` on cross sections

Permanent fill on any `CrossSection` (reach lateral `x`). Raises effective bed; removes storage and conveyance below the polyline crest. Not interchangeable with ineffective — see §H0.

Use on **reach** cuts for culvert embankment / general fill. At bridges, prefer unified embankment (v26) or explicit BU/BD polylines — see below.

---

## v26 — unified `bridge_roadway_embankments`

One object per bridge composes flat deck, abutment, ineffective, and embankment **blocked tops** before hydraulics.

```json
"bridge_roadway_embankments": [{
  "deck": {
    "stations": [0, 10],
    "low_elevations": [5, 5],
    "high_elevations": [6.5, 6.5]
  },
  "left": {
    "embankment_profile": { "stations": [-6, 0], "elevations": [1.5, 6.5] },
    "abutment": { "width": 1.0, "top_elevation": 0.0 }
  }
}]
```

**Rating curve:** same inner shape under `roadway_embankment` (singular).

### Flat field mapping

| Old flat fields (v13–v21) | Unified source |
|---------------------------|----------------|
| `bridge_deck_*`, `bridge_low_chords` / `bridge_high_chords` | `deck` |
| `bridge_abutment_left_*` / `bridge_abutment_right_*` | `left.abutment` / `right.abutment` |
| `bridge_ineffective_*` (opening frame) | `embankment_profile` points, `ineffective_blocks`, or `ineffective_faces` |
| BU/BD fill under embankment grade | `embankment_profile` → runtime blocked merge at **bridge solve** (not on reach layout nodes) |

**Precedence:** if flat arrays are already fully specified, **flat wins** for that group; unified fills omissions only.

### Explicit BU/BD cuts (v22)

| Modifier | Where to put it |
|----------|-----------------|
| Ineffective at opening | `bridge_ineffective_*` (or unified compose) — **not** on BU `ineffective_flow_areas` unless you mean reach-`x` on that cut |
| Roadway fill (v26) | Let compose merge blocked at solve; **do not** pre-bake the same polylines on explicit BU/BD `blocked_obstructions` (changes reach layout WSEL) |
| Reach-frame ineffective on the cut | `ineffective_flow_areas` on BU/BD — wins over `bridge_ineffective_*` when present |

### Minimal migration steps

1. Keep existing flat JSON working (no change required).
2. To consolidate HEC-RAS deck editor data: add `bridge_roadway_embankments[b]`; omit redundant flat groups you want compose to fill.
3. Set `bridge_opening_reach_station_origins[b]` so opening `s` aligns with BU lateral `x`.
4. Remove duplicate `blocked_obstructions` on explicit BU/BD if you switch to unified embankment profiles.
5. Optional: `derive_ineffective: false` on a side when you only want deck+abutment merge.

**Verification:** `tests/bridge_roadway_embankment_verification.rs` · fixture `python/verification/bridge_roadway_embankment.json`.
