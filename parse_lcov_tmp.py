targets = (
    "bridge_interior.rs",
    "steady.rs",
    "densify.rs",
    "unsteady.rs",
)
current = None
current_name = None
uncovered = {t: [] for t in targets}
with open("lcov.info") as f:
    for line in f:
        line = line.strip()
        if line.startswith("SF:"):
            path = line[3:]
            current_name = next((t for t in targets if path.endswith("/" + t) or path.endswith(t)), None)
            current = current_name is not None
        elif current and line.startswith("DA:"):
            ln, hits = line[3:].split(",")
            if int(hits) == 0:
                uncovered[current_name].append(int(ln))
        elif line == "end_of_record":
            current = False
            current_name = None
for t in targets:
    lines = sorted(uncovered[t])
    print(f"=== {t} ({len(lines)} uncovered) ===")
    print(" ".join(map(str, lines)))
