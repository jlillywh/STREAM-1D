# Reach mild

Eight ConSpan cross sections (RM 20.535–20.0), **no culvert**. Constant Q=1000 cfs, downstream stage 30.51 ft.

| File | Role |
|------|------|
| `reach_mild.g01` | Geometry (trimmed from ConSpan) |
| `reach_mild.u02` / `reach_mild.p02` | Unsteady BCs and plan |
| `reference_wsel_reach_mild_unsteady.json` | Committed terminal WSEL reference |

Scenario: `scenarios/reach_mild_unsteady_linked.json`. Run via [`../../README.md`](../../README.md).

Regenerate geometry from ConSpan: `python3 verification/oracle/scripts/bootstrap_reach_mild_project.py`
