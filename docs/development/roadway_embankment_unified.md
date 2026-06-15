# Unified roadway embankment (API v26)

`bridge_roadway_embankments[b]` composes deck, abutment, ineffective activation blocks, and embankment blocked tops before bridge hydraulics. Rating curve: singular `roadway_embankment`.

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

| Unified source | Composed flat fields |
|----------------|----------------------|
| `deck` | `bridge_deck_*`, scalar low/high chords |
| `left` / `right`.abutment | `bridge_abutment_left_*` / `_right_*` |
| `embankment_profile`, `ineffective_faces` | `bridge_ineffective_*`, runtime blocked merge at solve |

**Precedence:** fully specified flat arrays override compose for that group.

**Migration from v19–v21 flat fields:** optional — keep flat JSON working; add unified object when consolidating HEC deck editor data. Set `bridge_opening_reach_station_origins` for opening-frame alignment. Do not duplicate embankment fill on explicit BU/BD `blocked_obstructions` when using compose.

Semantics: [`equations.md`](../reference/equations.md) §H0, §G2. Tests: `tests/bridge_roadway_embankment_verification.rs`.
