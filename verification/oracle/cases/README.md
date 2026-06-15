# Parity cases (experimental)

JSON definitions that emit HEC-RAS `.g01`/`.u02`/`.p02` for parse/mapper smoke tests. **Certified verify uses bundled projects under `projects/`**, not generated output.

```bash
PYTHONPATH=python python3 verification/oracle/scripts/smoke_parity_case.py
```

Cases: `reach_mild_stage.json`, `conspan_arch_culvert.json`. Do not commit `projects/generated/` without a reviewed reference.
