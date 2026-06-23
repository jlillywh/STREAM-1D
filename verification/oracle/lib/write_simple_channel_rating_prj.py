"""Write staged simple_channel.prj for Plan 03 (rating-curve DS)."""

from __future__ import annotations

from pathlib import Path

from .hecras_text_io import write_ras_lines

_LINES = [
    "Proj Title=Simple trapezoidal channel",
    "Current Plan=p03",
    "Default Exp/Contr=0.3,0.1",
    "SI Units=0",
    "English Units=1",
    "Geom File=g01",
    "Geom Title=Simple trapezoidal channel (reach-only tutorial)",
    "Unsteady File=u03",
    "Unsteady Title=Simple channel rating-curve DS unsteady",
    "Plan File=p03",
    "Plan Title=Simple trapezoidal channel unsteady (rating DS)",
]


def write_rating_prj(path: Path) -> None:
    write_ras_lines(path, _LINES)
