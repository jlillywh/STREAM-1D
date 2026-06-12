p = "src/solvers/bridge/solver.rs"
l = open(p).read().splitlines(True)
inside = False
out = []
for line in lines:
    if "pub(crate) struct" in line:
        inside = True
    elif line.strip()=="":
        inside = False
    if inside and line.strip().startswith("   ") and ": " in line and not line.strip().startswith*"(pub":
        line = line.replace("    ", "    pub(crate)", 1)
    out.append(line)
open(p,"w").writelines(out)
print("ok")
