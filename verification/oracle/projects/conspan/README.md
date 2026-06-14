# ConSpan — linked HEC-RAS project bundle

Bundled HEC-RAS project files for **linked verify** (`verification/oracle/`).

| File | Role |
|------|------|
| `ConSpan.g01` | Geometry — cross-sections, culvert, reach layout |
| `ConSpan.p01` | Plan — steady subcritical standard step |
| `ConSpan.f01` | Flow — 5 / 10 / 25 / 50 yr profile discharges and downstream BCs |
| `conspan.u02` | Unsteady flow — constant Q=1000 cfs, DS stage 30.51 ft (Chunk 4 mild case) |
| `conspan.p02` | Unsteady plan — 15MIN interval, theta=1.0 |

**Chunk 4 unsteady scenario:** [`../../scenarios/conspan_unsteady_mild_linked.json`](../../scenarios/conspan_unsteady_mild_linked.json) — mode `0` post-step culvert coupling via `conspan_mapper`.

**STREAM-1D mapping:** [`../../fixtures/conspan_project_12.json`](../../fixtures/conspan_project_12.json) — cross sections and culvert fields derived from this geometry.

**Reference output:** [`../../fixtures/ConSpan.csv`](../../fixtures/ConSpan.csv) — WSEL profile export from running this project in HEC-RAS.

**Profiles checked in linked verify:** 5 yr, 25 yr, 50 yr (from [`../../fixtures/hecras_conspan_profiles.json`](../../fixtures/hecras_conspan_profiles.json)).

This is a legacy example model. Live re-run with modern HEC-RAS (6.x+) may require project migration; the CSV export remains the authoritative linked reference for CI.
