# Beaver Creek

Unsteady inline **bridge** example (9 piers, WSPRO, piecewise deck). Development scenario — not part of CI.

Scenario: `scenarios/beaver_unsteady_linked.json`.

```bash
bash verification/oracle/run_oracle.sh \
  --scenario verification/oracle/scenarios/beaver_unsteady_linked.json
python3 verification/oracle/scripts/smoke_beaver_parse.py
```

Sync from sibling web repo (optional): `bash verification/oracle/scripts/sync_linked_projects.sh`
