l=open("src/solvers/bridge/mod.rs").read().splitlines(True)
for i,s in enumerate(l):
    if s.startswith("#"):
        l[i]=s.lstrip("#")
print(l.end)
open("src/solvers/bridge/mod.rs","w").writelines(l)
