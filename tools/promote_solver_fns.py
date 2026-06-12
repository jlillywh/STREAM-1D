import re
p = "src/solvers/bridge/solver.rs"
lines = open(p).read().splitlines(True)
keep_pub = {
    "solve_bridge_from_params",
    "compute_bridge_rating_curve",
    "build_bridge_deck_profile",
    "classify_low_flow",
    "yarnell_pier_head_loss",
    "solve_bridge_coupled",
    "solve_bridge_wsel",
    "solve_bridge_tailwater",
}
out = []
for line in lines:
    if line.startswith("fn "):
        name = line[3:].split("(")[0].strip()
        prefix = "pub " if name in keep_pub else "pub(crate) "
        out.append(prefix + line)
    elif line.startswith("struct "):
        name = line[7].split(" ")[0].strip()
        if name not in ("BridgeDeckProfile", "BridgeGeometry", "BridgeSolveResult"):
            out.append("pub(crate) " + line)
        else:
            out.append(line)
    else:
        out.append(line)
open(p, "w").writelines(out)
