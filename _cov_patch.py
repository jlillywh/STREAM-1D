import subprocess, re

files = [
    "src/solvers/bridge_interior.rs",
    "src/solvers/steady.rs",
    "src/geometry/densify.rs",
    "src/solvers/unsteady.rs",
]

unc = {f: set() for f in files}
cur = None
with open("lcov.info") as fh:
    for line in fh:
        line = line.strip()
        if line.startswith("SF:"):
            cur = line[3:]
        elif line == "end_of_record":
            cur = None
        elif cur in unc and line.startswith("DA:"):
            parts = line[3:].split(",")
            for i in range(0, len(parts), 2):
                if int(parts[i+1]) == 0:
                    unc[cur].add(int(parts[i]))

patch_lines = {f: set() for f in files}
diff = subprocess.check_output(
    ["git", "diff", "main...HEAD", "-U0", "--", *files], text=True
)
cur_file = None
for line in diff.splitlines():
    if line.startswith("+++ b/"):
        cur_file = line[6:]
    elif line.startswith("@@") and cur_file in patch_lines:
        m = re.search(r"\+(\d+)(?:,(\d+))?", line)
        if m:
            start = int(m.group(1))
            count = int(m.group(2) or 1)
            patch_lines[cur_file].update(range(start, start + count))

for f in files:
    missing = sorted(unc[f] & patch_lines[f])
    print(f"\n=== {f}: {len(missing)} uncovered in patch ===")
    if not missing:
        continue
    start = prev = missing[0]
    ranges = []
    for ln in missing[1:]:
        if ln == prev + 1:
            prev = ln
        else:
            ranges.append((start, prev))
            start = prev = ln
    ranges.append((start, prev))
    for a,b in ranges:
        print(f"  {a}-{b}" if a!=b else f"  {a}")

for f in files:
    missing = sorted(unc[f] & patch_lines[f])
    if not missing:
        continue
    print(f"\n--- SOURCE {f} ---")
    lines = open(f).read().splitlines()
    for ln in missing[:60]:
        print(f"{ln}: {lines[ln-1]}")
    if len(missing) > 60:
        print(f"... {len(missing)-60} more")
