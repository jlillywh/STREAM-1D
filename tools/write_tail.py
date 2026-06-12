open("src/solvers/bridge/mod.rs","a").write("##[cfg(test)]\n##[path = \"../bridge_tests.rs\"]\nmod tests;\n")
