# Chunk 3.1 — Steady culvert parity (ConSpan / FHWA)

**Roadmap:** [`UNSTEADY_HECRAS_PARITY_ROADMAP.md`](../../../../lillywhite_web/streams1d/docs/UNSTEADY_HECRAS_PARITY_ROADMAP.md) § Chunk 3.1

## Status (2026-06-14)

| Check | Harness | Result |
|-------|---------|--------|
| ConSpan 5/25/50 yr profiles (10 stations each) | `tests/culvert_hecras_verification.rs`, `python/test_hecras_culvert_verification.py` | **PASS** (±0.04 ft) |
| FHWA point benchmarks (circular, box, arch, …) | `tests/culvert_hecras_verification.rs` (`culvert_point_benchmarks.json`) | **PASS** |
| Culvert unit suite | `cargo test culvert --lib` | **PASS** (58 tests) |
| Linked steady ConSpan oracle | `scenarios/conspan_steady_linked.json` | **PASS** (max \|Δ\| ≈ 0.032 ft) |

**Gate script (local or CI):**

```bash
bash verification/oracle/scripts/run_chunk3_culvert_steady_gate.sh
```

## BU/BD at culvert embankments (ConSpan)

HEC-RAS labels monitoring points **culvert_BU** (RM 20.238, station 1257) and **culvert_BD** (RM 20.227, station 1200). The verified steady fixture `conspan_project_12.json` models these as **reach cross sections** with embankment geometry and composite Manning — not separate `bridge_upstream_cross_sections` / `bridge_downstream_cross_sections` fields.

Steady WSEL at stations **1257** and **1200** is within tolerance on all three ConSpan profiles (linked verify checks every profile station). For Chunk 3.1, embankment XS parity is **met at the reach-XS level**.

Explicit BU/BD **face-cut** fields on `SteadyInputs` / `UnsteadyInputs` are a **Chunk 4** mapper item if unsteady mode-0 certification requires face semantics beyond the current fixture layout.

## Artifacts

| Artifact | Role |
|----------|------|
| `verification/fixtures/conspan_project_12.json` | STREAM-1D geometry + culvert arrays |
| `verification/fixtures/hecras_conspan_profiles.json` | Golden WSEL by station |
| `verification/fixtures/ConSpan.csv` | Linked oracle CSV reference |
| `verification/oracle/projects/conspan/` | Bundled HEC-RAS `.g01` + `.p01` + `.f01` |

## Exit criteria (Chunk 3.1)

- [x] ConSpan / FHWA profiles within existing verification tolerance
- [x] Culvert embankment XS at BU/BD stations verified via steady profiles (reach-XS representation)
- [ ] Explicit BU/BD face fields on unsteady mapper — defer to Chunk 4 if needed
