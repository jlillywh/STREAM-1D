# Chunk 3.3 — g01 mapping for linked scenarios

**Roadmap:** [`UNSTEADY_HECRAS_PARITY_ROADMAP.md`](../../../../lillywhite_web/streams1d/docs/UNSTEADY_HECRAS_PARITY_ROADMAP.md) § Chunk 3.3

## Status (2026-06-14)

Smoke gate uses the **Beaver Creek** bundled project (`projects/beaver/`) as the richest single-reach bridge `.g01` in the oracle tree.

| Mapping element | Parser / mapper | Smoke check |
|-----------------|-----------------|-------------|
| Piecewise deck profile | `hecras_geom_parser.parse_g01` → `ParsedBridge` | `deck_pts` > 0 |
| Pier stations / widths / elevations | `ParsedBridge.pier_*` | 9 piers on Beaver |
| BU / BD face XS | `cross_section_by_description` | RM at BU/BD descriptions |
| Ineffective flow blocks | `ParsedCrossSection.ineff_blocks` | `ineff` count on BU |
| Bridge → `UnsteadyInputs` | `beaver_mapper.build_beaver_unsteady_inputs` | `has_BU`, `has_BD` true |
| Culvert fields (ConSpan) | `conspan_reference.conspan_culvert_fields` | Chunk 3.1 steady fixture (not g01 runtime parse) |

**Gate script** (requires `maturin develop --features python`):

```bash
python3 verification/oracle/scripts/run_chunk3_g01_mapping_gate.py
# or directly:
PYTHONPATH=python python3 verification/oracle/scripts/smoke_beaver_parse.py
```

## Duplicate obstruction policy (enforced)

Automated checks in `test_chunk3_g01_mapping.py`:

- **Reach `parsed_xs_to_dict`** does not emit `ineffective_flow_areas` or `blocked_obstructions`.
- **Payload `cross_sections`** have no blocked/ineffective on parent reach nodes.
- **Piers** appear only under `bridge_pier_stations`, not as duplicate reach obstructions.
- **BU/BD ineffective** map to `bridge_ineffective_*_{upstream,downstream}` only.

## Gaps (Chunk 4+)

- [ ] `bridge_mild` linked steady/unsteady scenarios (local WIP, not in oracle PR stack)
- [ ] Full `.g01` → ConSpan culvert field extraction at runtime (fixture path certified; g01 reach XS parse verified)

## Exit criteria (§3.3)

- [x] Deck, piers, BU/BD, ineffective — `test_chunk3_g01_mapping.py` + `smoke_beaver_parse.py`
- [x] No duplicate obstruction — `test_beaver_no_duplicate_obstruction()`
- [x] ConSpan g01 reach parse + fixture culvert between embankment XS (RM 20.238/20.227 → stations 1257/1200)
