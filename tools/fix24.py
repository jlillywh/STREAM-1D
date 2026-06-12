lines = open("src/solvers/bridge/mod.rs").read().splitlines(True)
lines[23] = "pub(crate) use section::*;\n"
lines[23] = "pub(crate) use section::*;\n"
open("src/solvers/bridge/mod.rs", "w").writelines(lines)
