# Chunk 3.2 — Steady bridge parity

**Roadmap:** [`UNSTEADY_HECRAS_PARITY_ROADMAP.md`](../../../../lillywhite_web/streams1d/docs/UNSTEADY_HECRAS_PARITY_ROADMAP.md) § Chunk 3.2

## Status (2026-06-14)

| Suite | Fixture | Harness | Tolerance | Result |
|-------|---------|---------|-----------|--------|
| Abutments (WSPRO) | `bridge_abutment_hecras.json` | `bridge_abutment_hecras_verification.rs` | ±2 mm HW | **PASS** |
| BU/BD explicit faces | `bridge_bu_bd_hecras.json` | `bridge_bu_bd_hecras_verification.rs` | ±2 mm HW | **PASS** (2 tests) |
| High flow (pressure/weir) | `bridge_high_flow_hecras.json` | `bridge_high_flow_hecras_verification.rs` | ±2 mm HW | **PASS** (6 cases) |
| Roadway embankment (v26) | `bridge_roadway_embankment.json` | `bridge_roadway_embankment_verification.rs` | ±2 mm WSEL | **PASS** (7 tests) |
| Guide-bank contraction | `bridge_guide_bank_contraction.json` | `bridge_guide_bank_contraction_verification.rs` | ±2 mm WSEL | **PASS** |
| Friction weighting (v30) | `bridge_friction_weighting_hecras.json` | `bridge_friction_weighting_hecras_verification.rs` | ±5 mm | **PASS** |
| Reverse-flow rating (v31) | `bridge_reverse_flow_rating.json` | `bridge_reverse_flow_rating_verification.rs` | ±20 mm HL | **PASS** |
| Opening alignment | (inline) | `bridge_opening_alignment_verification.rs` | exact layout | **PASS** (6 tests) |
| Bridge unit suite | — | `cargo test bridge --lib` | — | **PASS** (189 tests) |

**Gate script:**

```bash
python3 verification/oracle/scripts/run_chunk3_bridge_steady_gate.py
```

## Coverage vs roadmap §3.2

| Roadmap item | Evidence |
|--------------|----------|
| WSPRO / Yarnell / energy low-flow | `bridge_bu_bd_hecras_verification`, `bridge_abutment_hecras_verification`, `src/solvers/bridge_tests.rs` |
| Piecewise deck, piers, BU/BD faces | `bridge_bu_bd_hecras_verification`, roadway embankment compose |
| Ineffective areas | BU/BD fixture + `bridge_roadway_embankment_verification` |
| High-flow pressure/weir | `bridge_high_flow_hecras_verification` (6 cases, documented ±2 mm) |

## Exit criteria (§3.2)

- [x] All required steady bridge verification crates green
- [x] High-flow cases documented with stated tolerance in fixture `notes`
