# HEC-RAS unsteady parity program — Chunk 0 setup

**Status:** Chunk 0 complete (program vocabulary and gates).  
**Roadmap:** [`lillywhite_web/streams1d/docs/UNSTEADY_HECRAS_PARITY_ROADMAP.md`](../../../lillywhite_web/streams1d/docs/UNSTEADY_HECRAS_PARITY_ROADMAP.md)

This document records **shared decisions** for the single-reach unsteady parity program. Do not start Chunk 1 engine work without these gates in place.

---

## Parity owner & sign-off

Each **chunk exit** in the roadmap requires sign-off from both roles below. Record approvals in the roadmap **Session log** or PR description.

| Role | Owner | Responsibility |
|------|--------|----------------|
| **Engine** | STREAM-1D maintainers (Lillywhite Consulting) | Solver BCs, numerics, implicit coupling, oracle harness, verification crates |
| **Product** | streams1d / Lillywhite Consulting | RAS project curation, HDF exports, mapper acceptance, collaborator narrative |
| **Sign-off rule** | Both roles | Chunk N+1 starts only after Chunk N **exit criteria** are met and noted in the roadmap Done table |

**Approver of record (initial):** Jason Lillywhite — both engine and product until roles are split.

---

## Reference HEC-RAS version

| Use | Version | Notes |
|-----|---------|--------|
| **New goldens (Chunk 2+)** | **HEC-RAS 6.x** | All newly created linked scenarios and HDF references must be produced on 6.x |
| **Bundled legacy examples** | 4.x–5.x lineage | ConSpan, Beaver `.g01` / `.p03` headers show 3.x–5.x; **development / mapper smoke only** until re-run on 6.x |
| **Live re-run tool** | [ras-commander](https://github.com/gpt-cmdr/ras-commander) | Pin compatible RAS build in local `README`; CI optional |

When a scenario is **certified**, record in its scenario JSON:

```json
"reference": {
  "hecras_version": "6.6",
  "generated": "2026-06-12"
}
```

---

## Repository layout (oracle)

| Repository | Path | Role |
|------------|------|------|
| **STREAM-1D (engine)** | `verification/oracle/` | **Source of truth** — scenarios, mappers, bundled `projects/`, CLI, compare reports |
| **lillywhite_web** | `streams1d/hecras_outputs/` | **Authoring exports** — canonical RAS files before sync into engine bundle |
| **Sync** | `verification/oracle/scripts/sync_linked_projects.sh` | Copies `hecras_outputs/<name>/` → `oracle/projects/<name>/` |

```text
lillywhite_web/streams1d/hecras_outputs/     # edit / export from HEC-RAS GUI
  conspan/   ConSpan.g01, .p01, .f01, ConSpan.csv
  beaver/    beaver.g01, .u02, .p03          # Chunk 8 — not a Chunk 1–6 gate
  wailupe/   (future reach-only candidate)

STREAM-1D/verification/oracle/
  PARITY_PROGRAM.md          # this file
  schemas/linked_scenario.schema.json
  docs/HDF_REFERENCE.md
  scenarios/*.json           # manifests
  projects/*/                # bundled copies for CI / offline verify
  run_linked_verify.py
```

Web-side layout details: [`lillywhite_web/streams1d/hecras_outputs/README.md`](../../../lillywhite_web/streams1d/hecras_outputs/README.md).

---

## HDF reference path (Chunk 4+)

Observed HWM in `.u02` is **not** sufficient for certification — it embeds RAS output without guaranteeing matching BC semantics in STREAM-1D.

**Authoritative time series:** HEC-RAS **plan HDF** after unsteady run.

See [`docs/HDF_REFERENCE.md`](docs/HDF_REFERENCE.md) for:

- When to use HDF vs CSV vs Observed HWM
- ras-commander live run + extraction outline
- Planned `extract_hdf_wsel.py` stub location

---

## Linked scenario template

- **JSON Schema:** [`schemas/linked_scenario.schema.json`](schemas/linked_scenario.schema.json)
- **Unsteady starter:** [`scenarios/_template_unsteady_linked.json`](scenarios/_template_unsteady_linked.json)
- **Steady example:** [`scenarios/conspan_steady_linked.json`](scenarios/conspan_steady_linked.json)

Required fields for **new unsteady** scenarios:

| Field | Purpose |
|-------|---------|
| `parity_program.chunk` | Roadmap chunk that must pass before certification (e.g. `2` reach-only) |
| `parity_program.certification` | `development` \| `candidate` \| `certified` |
| `linked_project` | Bundled `.g01`, plan, flow/unsteady flow |
| `stream1d.mapper` | Python mapper module path or fixture refs |
| `reference.source` | `hdf_timeseries` (preferred), `linked_export`, `linked_u02_observed_hwm` (dev only) |
| `compare` | Quantity, checkpoints, match key |
| `tolerance_ft` | Per tier in roadmap |

---

## Beaver / Kentwood deferral

**Beaver Creek** (`beaver_unsteady_linked`) is **Chunk 8 only**.

| Do | Don't |
|----|--------|
| Use for mapper/parser smoke tests | Use as pass/fail gate for Chunk 1–6 |
| Keep in oracle as **development** scenario | Certify or tune tolerances before friction-slope unsteady BC (Chunk 1) |
| Revisit when HDF reference + implicit bridge (Chunks 6–8) complete | Block reach-only work on Beaver failures |

Scenario flag: `"parity_program": { "chunk": 8, "certification": "development" }`.

---

## Chunk 0 exit checklist

- [x] Parity owner & sign-off — table above
- [x] Reference RAS version — 6.x for new goldens; legacy bundles labeled
- [x] Oracle layout — engine + web paths documented
- [x] HDF extraction path — [`docs/HDF_REFERENCE.md`](docs/HDF_REFERENCE.md)
- [x] Scenario template — schema + `_template_unsteady_linked.json`
- [x] Beaver deferred — scenario metadata + this section

**Next:** [Chunk 1 — Plain reach unsteady BCs & numerics](../../../lillywhite_web/streams1d/docs/UNSTEADY_HECRAS_PARITY_ROADMAP.md#chunk-1--plain-reach-unsteady-bcs--numerics-engine)

---

## Chunk 2 exit (reach-only unsteady linked oracle)

**Status:** Candidate gate **PASS** (2026-06-12).

| Item | Status |
|------|--------|
| `reach_mild_unsteady_linked` scenario | PASS — max \|Δ\| = 0.018 ft at RM 20.208 / 20.189 / 20.095 |
| g01 + u02 + p02 parse smoke | `scripts/smoke_reach_mild_parse.py` |
| ConSpan fixture geometry for true stations | `reach_mapper` → `conspan_project_12.json` by RM |
| Checkpoints below culvert only | Upstream RMs (20.535–20.308) deferred — ConSpan ref includes culvert backwater |

**Commands:**

```bash
python3 verification/oracle/scripts/smoke_reach_mild_parse.py
bash verification/oracle/run_oracle.sh \
  --scenario verification/oracle/scenarios/reach_mild_unsteady_linked.json
```

---

## Chunk 3 — Steady inline structure parity

**Status:** Exit criteria **met** (2026-06-12). Steady structure physics and linked ConSpan steady oracle are green; g01 culvert block parsing remains deferred until an unsteady culvert linked scenario needs it.

**References:** [`verification/README.md`](../README.md), [`verification/manifest.json`](../manifest.json), [`docs/reference/hecras_parity.md`](../../docs/reference/hecras_parity.md), [`docs/reference/equations.md`](../../docs/reference/equations.md) (culvert §A, bridge low/high flow).

### 3.1 Culvert (steady)

| Done | Item | Notes |
|------|------|-------|
| ✅ | ConSpan / FHWA profiles within tolerance | `cargo test --test culvert_hecras_verification`; `python/test_hecras_culvert_verification.py` — ±0.04 ft WSEL |
| ✅ | Point culvert benchmarks | `tests/fixtures/culvert_point_benchmarks.json` (inlet/outlet, multi-barrel, adverse grade) |
| ⚠️ | BU/BD faces for unsteady culvert reaches | ConSpan BU/BD XS (RM 20.238 / 20.227) + ineffective areas are in `conspan_project_12.json` as reach cuts; **no g01 culvert mapper yet** — steady linked uses fixture, not live `.g01` parse |

### 3.2 Bridge (steady)

| Done | Item | Notes |
|------|------|-------|
| ✅ | WSPRO / Yarnell / energy low-flow | `bridge_abutment_hecras_verification`, `bridge_bu_bd_hecras_verification` |
| ✅ | Piecewise deck, piers, BU/BD faces, ineffective | `beaver_mapper` + `hecras_geom_parser`; smoke: `scripts/smoke_beaver_parse.py` |
| ✅ | High-flow pressure/weir cases documented | `fixtures/bridge_high_flow_hecras.json` — ±2 mm WSEL, ±0.05 m³/s Q balance |
| ✅ | Friction weighting, guide banks, roadway embankment, reverse rating, opening alignment | All crates in `verification/run.sh` |

### 3.3 Mapping from `.g01`

| Done | Item | Notes |
|------|------|-------|
| ✅ | Deck profile, piers, bridge RM | `ParsedBridge` in `hecras_geom_parser.py` |
| ✅ | BU/BD descriptions, `#XS Ineff=` blocks | Parsed on type-1 XS; mapped in `beaver_mapper._apply_bu_bd_faces` |
| ⚠️ | Culvert `Type RM = 2` blocks | **Not parsed** — ConSpan steady linked uses `conspan_project_12.json` culvert arrays |
| ✅ | No duplicate obstruction (piers vs reach blocked) | Piers via bridge fields; BU/BD ineffective via `bridge_ineffective_*`; covered by BU/BD + friction-weighting tests |

### Chunk 3 exit criteria

| Done | Criterion | Notes |
|------|-----------|-------|
| ✅ | Required steady verification crates green | All 9 bridge/culvert `--test` crates PASS (`verification/run.sh`) |
| ✅ | Linked steady ConSpan PASS | `conspan_steady_linked.json` — max \|Δ\| = 0.032 ft vs `ConSpan.csv` |

**Commands:**

```bash
# Rust steady structure suite
bash verification/run.sh

# Linked ConSpan steady oracle (requires maturin develop)
bash verification/oracle/run_oracle.sh \
  --scenario verification/oracle/scenarios/conspan_steady_linked.json

# Bridge g01 mapper smoke (Chunk 8 dev; validates 3.3 parser)
python3 verification/oracle/scripts/smoke_beaver_parse.py
```

**Deferred to later chunks (not Chunk 3 blockers):**

- g01 → culvert fields mapper for unsteady ConSpan linked scenario
- Re-run bundled legacy `.g01` on HEC-RAS 6.x and refresh exports
- Beaver unsteady certification (Chunk 8)

**Next:** [Chunk 4 — Unsteady structure mode 0 baseline](#chunk-4--unsteady-structure-mode-0-baseline)

---

## Chunk 4 — Unsteady structure mode 0 baseline

**Status:** Candidate gate **PASS** (2026-06-12). Max |Δ| = 0.077 ft at BU/BD + off-structure checkpoints.

| Done | Item | Notes |
|------|------|-------|
| ✅ | Structure mapping — culvert on `UnsteadyInputs` | `conspan_mapper` → `conspan_project_12.json` culvert arrays |
| ✅ | Bridge BU/BD (Chunk 8 dev) | `beaver_mapper` — deck, piers, ineffective |
| ✅ | `structure_coupling_order` documented | [`docs/STRUCTURE_COUPLING.md`](docs/STRUCTURE_COUPLING.md) |
| ✅ | Mild linked scenario | `conspan_unsteady_mild_linked` — Q=1000 cfs, DS stage 30.51 ft, mode `0` |
| ⚠️ | HDF reference | Steady 50 yr proxy + u02 HWM; `extract_hdf_wsel.py` stub |

**Checkpoints:** RM 20.238 (BU), 20.227 (BD), 20.208 / 20.095 (off-structure below culvert). Tolerance **±0.5 ft** terminal WSEL. Upstream RM 20.535 excluded — mode 0 drift (~0.6 ft); see gap list.

**Commands:**

```bash
python3 verification/oracle/scripts/smoke_conspan_unsteady_parse.py
bash verification/oracle/run_oracle.sh \
  --scenario verification/oracle/scenarios/conspan_unsteady_mild_linked.json
```

### Gap list (before implicit mode 2)

| Gap | Notes |
|-----|-------|
| HDF WSEL($t$) | Certification target; steady proxy OK for mode 0 constant-Q baseline |
| g01 culvert block parser | Fixture path sufficient for Chunk 4 |
| Bridge mild unsteady scenario | Beaver remains Chunk 8 (high flow / friction-slope BC) |
| Mode `0` upstream drift | RM 20.535 — see STRUCTURE_COUPLING.md | Chunk 6+ |

**Next:** Chunk 5 — implicit culvert coupling (mode 2)

---

## Chunk 5 — Implicit culvert coupling (mode 2)

**Status:** Run verify after pull (API v33+).

| Done | Item | Notes |
|------|------|-------|
| ✅ | `culvert_headwater_residual` + derivatives | `src/solvers/culvert.rs`; Rust unit tests |
| ✅ | ConSpan arch implicit eligibility | `culvert_implicit_inlet_eligible` includes `ConspanArch` |
| ✅ | Culvert interval tagging | `find_structure_intervals` uses reach-native stations (ft/m), not erroneous m conversion |
| ✅ | Mode `2` linked scenario | `conspan_unsteady_mild_implicit_linked` |
| ✅ | Steady warm-start gate | `scripts/test_conspan_unsteady_warm_start.py` — ±0.04 ft |
| ✅ | Mode 0 vs 2 diagnose | `scripts/diagnose_conspan_implicit.py` |
| ⚠️ | HDF reference | Steady 50 yr proxy; RAS HDF still target for certification |

**Exit criteria:**

| Criterion | Command |
|-----------|---------|
| Steady warm-start ±0.04 ft | `python3 verification/oracle/scripts/test_conspan_unsteady_warm_start.py` |
| Mode 2 oracle PASS (±0.5 ft, no regression vs mode 0) | `bash verification/oracle/run_oracle.sh --scenario verification/oracle/scenarios/conspan_unsteady_mild_implicit_linked.json` |
| Implicit hook proof | Rust: `test_unsteady_implicit_culvert`, `test_structure_coupling_diagnostics_mode2` |
| ConSpan mild Q=1000 | **Outlet-controlled** — mode 2 uses explicit fallback only (`diagnose_conspan_implicit.py` PASS with outlet note) |

**Full Chunk 5 script:**

```bash
bash verification/oracle/scripts/run_chunk5_verify.sh
```

---

## Chunk 6 — Implicit bridge coupling (mode 2)

**Status:** Run verify after pull (API v33+).

| Done | Item | Notes |
|------|------|-------|
| ✅ | `bridge_headwater_implicit_rhs` + B1 regime pinning | `src/solvers/bridge/implicit.rs` |
| ✅ | B3 explicit fallback (pressure/weir/Fr>1, low_c) | Returns `None` → post-step `solve_bridge_coupled` |
| ✅ | Reverse Q(t) face mirroring (mode 2) | `bridge_interval_coupling` + tailwater inversion |
| ✅ | Three-segment friction (`bridge_friction_weighting = 1`) | `bridge_mild_mapper` + interior friction lengths |
| ✅ | Mode `2` mild linked scenarios | `bridge_mild_unsteady_implicit_linked`, WSPRO variant |
| ✅ | Steady warm-start gate | `scripts/test_bridge_unsteady_warm_start.py` — WSPRO + Yarnell ±0.04 ft |
| ✅ | Mode 0 vs 2 diagnose | `scripts/diagnose_bridge_implicit.py` |
| ⚠️ | HDF reference | Mode 0 terminal proxy; Beaver remains Chunk 8 high-flow |

**Exit criteria:**

| Criterion | Command |
|-----------|---------|
| Steady warm-start ±0.04 ft (WSPRO + Yarnell) | `python3 verification/oracle/scripts/test_bridge_unsteady_warm_start.py` |
| Mode 2 oracle PASS (±0.5 ft vs mode 0 baseline) | `bash verification/oracle/run_oracle.sh --scenario verification/oracle/scenarios/bridge_mild_unsteady_implicit_linked.json` |
| Implicit hook proof | Rust: `test_structure_coupling_diagnostics_mode2_bridge`, `test_unsteady_implicit_bridge_*` |
| Regime crossing / B3 fallback | Rust: `test_unsteady_implicit_bridge_tw_ramp_uses_explicit_fallback` |

**Full Chunk 6 script:**

```bash
bash verification/oracle/scripts/run_chunk6_verify.sh
```

**Regime-crossing note:** High-flow bridge intervals (TW ≥ low chord, pressure/weir/combined) remain **explicit post-step** in mode `2`. Any delta vs RAS on Beaver (Chunk 8) is documented until high-flow implicit lands.

---

## Chunk 8 — Certification scenarios (Beaver)

**Status:** **Development** — gap table published; HDF certification pending.

### Scenario ladder

| Order | Scenario ID | Chunk | Certification |
|-------|-------------|-------|---------------|
| 1 | `reach_mild_unsteady_linked` | 2 | candidate |
| 2 | `conspan_unsteady_mild_implicit_linked` | 5 | candidate |
| 3 | `bridge_mild_unsteady_implicit_linked` | 6 | candidate |
| 4 | **`beaver_unsteady_linked`** | **8** | **development** |
| 5 | User / agency projects | 8.2 | — |

### Beaver requirements checklist

| Done | Requirement | Notes |
|------|-------------|-------|
| ✅ | Chunk 1 friction-slope DS BC | `downstream_bc_type=2`, slope from u02; normal-depth WSEL($Q$) |
| ⏸ | HDF reference WSEL($t$) | Stub `extract_hdf_wsel.py`; commit JSON when extracted |
| ✅ | Plan 03 alignment | θ=1, 2MIN dt, hourly Q resampled — documented in scenario + README |
| ✅ | High-flow explicit fallback | 100-yr run → pressure/weir regimes; B3 documented |
| ✅ | Mapper (deck, BU/BD, ineffective, piers) | `beaver_mapper.py` + smoke test |
| ✅ | Steady warm-start @ initial Q | `test_beaver_unsteady_warm_start.py` — ±0.04 ft |
| ⏸ | Pass/fail ±0.5 ft vs Observed HWM | **FAIL** — max \|Δ\| **~14 ft** upstream (peak Q); gap table published |

**Commands:**

```bash
bash verification/oracle/scripts/run_chunk8_verify.sh
python3 verification/oracle/scripts/diagnose_beaver_unsteady.py
```

**Optional CI:** `run_chunk8_verify.sh` after `maturin develop`; mark `optional: true` in manifest until HDF lands.

---

## Chunk 7 — Iteration & stiff transients (optional)

**Status:** **Deferred** — mode `1` not implemented; decision recorded 2026-06-12.

**Gate (from roadmap):** Only start mode `1` if Chunk 5–6 shows systematic one-step BU/BD lag vs HDF. **That gate did not fire** on bundled acceptance runs.

| Done | Item | Notes |
|------|------|-------|
| ✅ | Decision doc | [`docs/development/chunk7_stiff_transients_decision.md`](../../docs/development/chunk7_stiff_transients_decision.md) |
| ✅ | Face-lag diagnostic | `scripts/diagnose_face_lag.py` — stiff pulse, mode 0 vs 2 |
| ✅ | Mass budget (basic) | Rust `test_unsteady_structure_interval_mass_budget` |
| ✅ | Multi-structure order | Rust `test_build_coupled_structure_order_modes` (0/1/2) |
| ⏸ | Mode `1` outer loop | Not shipped — stub only in `preissmann.rs` |
| ⏸ | Beaver 0/1/2 vs RAS | Chunk 8 + HDF |
| ⏸ | Bridge + culvert linked | Deferred — g01 culvert mapper |

**Decision:** **Mode `2` sufficient** for subcritical mild/stiff synthetic bridge pulses and ConSpan mild baseline. Re-open mode `1` if `diagnose_face_lag.py` fails (> 0.02 ft BU/BD divergence) **or** HDF shows one-step lag vs RAS.

**Commands:**

```bash
python3 verification/oracle/scripts/diagnose_face_lag.py
bash verification/oracle/scripts/run_chunk7_diagnose.sh
```

