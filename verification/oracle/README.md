# Linked HEC-RAS oracle

Compare STREAM-1D to output from **bundled HEC-RAS project files** (`.g01` + plan + flow). Same geometry source as the mapped STREAM inputs — not hand-waved golden numbers.

Requires `maturin develop --features python`. HEC-RAS is optional (committed references are used by default).

## Run

```bash
# Default: ConSpan steady culvert
bash verification/oracle/run_oracle.sh

# ConSpan unsteady Q-ramp matrix (diagnostic — shows Δ vs HEC, no pass/fail)
PYTHONPATH=python python3 verification/oracle/run_linked_verify.py \
  --scenario verification/oracle/scenarios/conspan_unsteady_ramp_matrix.json \
  --format matrix

# Open-channel unsteady (CI, no structures)
bash verification/oracle/run_oracle.sh \
  --scenario verification/oracle/scenarios/reach_mild_unsteady_linked.json

# All linked-verify checks that need no HEC-RAS install
python3 verification/oracle/scripts/run_oracle_ci.py
```

Includes **reach_mild** open-channel gate and **ConSpan mode 4** culvert ramp matrix (overall max |Δ| ≤ 0.12 ft vs HEC).

## Scenarios

| Scenario | Project | What it checks |
|----------|---------|----------------|
| `conspan_unsteady_ramp_matrix.json` | `projects/conspan/` | Arch culvert Q ramp — WSEL Δ matrix vs HDF (diagnostic, ~0.076 ft at faces) |
| `reach_mild_unsteady_linked.json` | `projects/reach_mild/` | Eight XS, no culvert, constant Q |
| `conspan_steady_linked.json` | `projects/conspan/` | Steady profiles vs CSV export |
| `beaver_unsteady_linked.json` | `projects/beaver/` | Inline bridge (development) |

Scenario manifests live in `scenarios/`. Reference WSEL JSON/CSV paths are listed in each manifest under `reference.file`.

## Refresh HEC-RAS reference (Windows)

```powershell
$env:HECRAS_RAS_EXE = 'C:\Program Files (x86)\HEC\HEC-RAS\7.0.1\Ras.exe'
python verification/oracle/scripts/run_ras_reference.py `
  --scenario verification/oracle/scenarios/conspan_unsteady_ramp_matrix.json
```

Then commit the updated reference JSON under `projects/<name>/`. Verify without RAS:

```bash
python3 verification/oracle/scripts/run_ras_reference.py \
  --scenario verification/oracle/scenarios/conspan_unsteady_ramp_matrix.json \
  --skip-ras-run --verify
```

Optional: `pip install -r verification/requirements-oracle-hecras.txt` (ras-commander, h5py).

## Layout

```text
run_linked_verify.py    CLI
run_oracle.sh           wrapper
lib/                    parsers, mappers, compare
projects/               bundled .g01 + plan + flow + reference JSON
scenarios/              linked scenario JSON
cases/                  experimental JSON→HEC emitter (see cases/README.md)
```

## Related

- Frozen JSON goldens: [`../README.md`](../README.md)
- HEC-RAS scope and known deltas: [`../../docs/reference/hecras_parity.md`](../../docs/reference/hecras_parity.md)
