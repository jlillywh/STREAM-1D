p="open("src/solvers/bridge/solver.rs","r").read().splitlines(True)
out=[]
for line in l:
    if "apply_barrel_skew" in line and line.startswith("use"):
        continue
    line=line.replace(", DeckVentUserInput","")
    if "ice_debris_params_for_bridge" in line and "nested_bridge" in line:
        line="use super::ice_debris::{clamp_opening_blockage_factor, BridgeIceDebrisParams};\n"
    out.append(line)
open("src/solvers/bridge/solver.rs","w").writelines(out)
