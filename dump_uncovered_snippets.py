from pathlib import Path

FILES = {
    "src/solvers/bridge_interior.rs": [92, 93, 158, 183, 184, 185, 186, 187, 188, 189, 190, 191, 192, 193, 194, 352, 353, 710, 725, 732, 764, 765, 766, 771, 773, 774, 775, 776, 777, 778, 779, 780, 781, 782, 783, 784, 785, 786, 787, 788, 789, 790, 793, 794, 795, 796, 797, 798, 800, 801, 802, 803, 804, 805, 806, 807],
    "src/solvers/steady.rs": [249, 586, 587, 588, 589, 590, 591, 592, 593, 595, 628, 631, 646, 652, 1020, 1148, 1153, 1154, 1155, 1156, 1158, 1159, 1165, 1170, 1171, 1172, 1173, 1175, 1176, 2078, 2079, 2084, 2085, 2108, 2109, 2114, 2115, 2126, 2127, 2132, 2133, 2996],
    "src/geometry/densify.rs": [24, 25, 45, 48, 59, 65, 185, 186, 187, 208],
    "src/solvers/unsteady.rs": [262, 263, 264, 265, 266, 268, 2108, 2109, 2114, 2115, 2592, 2593, 2598],
}

def ranges(lines):
    if not lines:
        return []
    lines = sorted(lines)
    out = []
    s = e = lines[0]
    for ln in lines[1:]:
        if ln == e + 1:
            e = ln
        else:
            out.append((s, e))
            s = e = ln
    out.append((s, e))
    return out

for path, uncovered in FILES.items():
    text = Path(path).read_text().splitlines()
    print(f"\n######## {path} ({len(uncovered)} patch-uncovered lines) ########")
    for s, e in ranges(uncovered):
        print(f"\n--- L{s}-{e} ---")
        for i in range(s - 1, e):
            print(f"{i+1:5d}| {text[i]}")
