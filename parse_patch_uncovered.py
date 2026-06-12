import re
from pathlib import Path

BASE = Path(".")
DIFF = (BASE / "patch_diff.txt").read_text()
FILES = {
    "src/solvers/bridge_interior.rs": BASE / "unc_bridge_interior.txt",
    "src/solvers/steady.rs": BASE / "unc_steady.txt",
    "src/geometry/densify.rs": BASE / "unc_densify.txt",
    "src/solvers/unsteady.rs": BASE / "unc_unsteady.txt",
}

def parse_patch_lines(diff_text: str) -> dict[str, set[int]]:
    out: dict[str, set[int]] = {}
    current = None
    new_line = 0
    in_hunk = False
    for line in diff_text.splitlines():
        if line.startswith("diff --git"):
            current = None
            in_hunk = False
            continue
        m = re.match(r"^\+\+\+ b/(.+)$", line)
        if m:
            current = m.group(1)
            out.setdefault(current, set())
            in_hunk = False
            continue
        m = re.match(r"^@@ -\d+(?:,\d+)? \+(\d+)(?:,(\d+))? @@", line)
        if m and current:
            new_line = int(m.group(1))
            in_hunk = True
            continue
        if not in_hunk or current is None:
            continue
        if line.startswith("+++") or line.startswith("---"):
            continue
        if line.startswith("+"):
            out[current].add(new_line)
            new_line += 1
        elif line.startswith(" "):
            new_line += 1
        elif line.startswith("-"):
            pass
        elif line.startswith("\\"):
            pass
    return out

patch = parse_patch_lines(DIFF)
for path, unc_file in FILES.items():
    uncovered = set(int(x) for x in unc_file.read_text().split() if x.strip())
    in_patch = patch.get(path, set())
    hit = sorted(uncovered & in_patch)
    print(f"=== {path} ===")
    print(f"uncovered_in_file={len(uncovered)} patch_lines={len(in_patch)} uncovered_in_patch={len(hit)}")
    print(" ".join(map(str, hit)))
