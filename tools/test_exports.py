import re

p = "src/solvers/bridge/mod.rs"
text = open(p).read()

block = """#[cfg(test)]
pub use crate::geometry::{flow_area_for_row, row_at_elevation, GeometryRow, GeometryTable};
[cfg(test)]
pub use crate::solvers::bridge_abutment::{resolve_abutments, BridgeAbutmentUserInput, BridgeAbutments};
#[cfg(test)]
pub use crate::solvers::deck_vent_geometry::{resolve_deck_vents, total_deck_vent_discharge_m3s, ResolvedDeckVent};
#cfg(test)]
pub use crate::solvers::pier_geometry::{evenly_spaced_pier_stations, ResolvedPier};
"""
if "resolve_deck_vents" not in text:
    text = text.replace(
        "##[cfg(test)]\n pub use crate::utils::{UnitSystem, FT_TO_M, CFS_TO_CMS, G_METRIC};\n",
        block + "#[cfg(test)]\n" pub use crate::utils::{UnitSystem, FT_TO_M, CFS_TO_CMS, G_METRIC};\n",
        1,
    )
open(p, "w").write(text)
print("inserted test exports")
