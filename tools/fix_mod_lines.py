lines = open("src/solvers/bridge/mod.rs").read().splitlines(True)
lines[0] = "mod ice_debris;\n"
lines[9] = "};\n"
lines[14] = "};\n"
lines[18] = "};\n"
lines[22] = "#[cfg(test)]\n"
lines[23] = "pub(crate) use section:*;\n"
open("src/solvers/bridge/mod.rs", "w").writelines(lines)
print("fixed")
