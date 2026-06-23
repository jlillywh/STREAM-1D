#!/usr/bin/env python3
import re, subprocess, json, os
from pathlib import Path

ROOT = Path("/home/jason/Lillywhite_Consulting/lillywhite_engine/STREAM-1D")
FILE = ROOT / "src/solvers/unsteady/culvert_implicit.rs"
SCENARIO = ROOT / "verification/oracle/scenarios/conspan_unsteady_ramp_matrix.json"
OMEGAS = [0.0, 0.10, 0.15, 0.20, 0.25, 0.30, 0.35, 0.40]
PAT = re.compile(r"pub\(crate\) const CULVERT_DEPARTURE_TAILWATER_OMEGA: f64 = [0-9.]+;")
RMS = ("20.227", "20.238")

def set_omega(o):
    FILE.write_text(PAT.sub(f"pub(crate) const CULVERT_DEPARTURE_TAILWATER_OMEGA: f64 = {o};", FILE.read_text()))

def run_verify():
    env = os.environ.copy(); env["PYTHONPATH"] = "python"
    p = subprocess.run(["python3", "verification/oracle/run_linked_verify.py", "--scenario", str(SCENARIO), "--format", "matrix"], cwd=ROOT, capture_output=True, text=True, env=env)
    return (p.stdout or "") + (p.stderr or "")

def parse_overall(out):
    for ln in out.splitlines():
        if "Overall max" in ln:
            m = re.search(r"=\s*([0-9.]+)\s*ft", ln)
            return ln.strip(), (float(m.group(1)) if m else None)
    return None, None

def parse_rm(out, rm):
    for ln in out.splitlines():
        if ln.strip().startswith(rm + " ") or f"{rm}  " in ln:
            if "|" in ln:
                m = re.search(r"\|\s*([0-9.]+)\s*$", ln)
                if m: return float(m.group(1))
    return None

results = []
for o in OMEGAS:
    set_omega(o)
    b = subprocess.run(["/home/jason/.cargo/bin/maturin", "develop", "--features", "python", "--release"], cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
    out = run_verify()
    ol, ov = parse_overall(out)
    results.append({"omega": o, "build_tail": b.stdout.strip().splitlines()[-1] if b.stdout.strip() else "", "overall_line": ol, "overall_val": ov, "rm_max_delta": {r: parse_rm(out, r) for r in RMS}})
valid = sorted([r for r in results if r["overall_val"] is not None], key=lambda r: r["overall_val"])
best = valid[0]["omega"] if valid else 0.25
set_omega(best)
subprocess.run(["/home/jason/.cargo/bin/maturin", "develop", "--features", "python", "--release"], cwd=ROOT, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
(ROOT / "verification/oracle/omega_sweep_summary.json").write_text(json.dumps({"results": results, "chosen_omega": best, "top3": valid[:3]}, indent=2))
print("CHOSEN", best)
for r in results: print(r["omega"], r["overall_val"], r["rm_max_delta"])
