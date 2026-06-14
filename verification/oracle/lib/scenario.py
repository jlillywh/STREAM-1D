"""Load linked-verify scenario manifests."""

from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path
from typing import Any


@dataclass(frozen=True)
class LinkedScenario:
    raw: dict[str, Any]
    path: Path
    oracle_root: Path

    @property
    def id(self) -> str:
        return str(self.raw["id"])

    @property
    def title(self) -> str:
        return str(self.raw.get("title", self.id))

    @property
    def mode(self) -> str:
        return str(self.raw.get("mode", "steady"))

    @property
    def tolerance_ft(self) -> float:
        return float(self.raw.get("tolerance_ft", 0.04))

    def resolve(self, rel: str) -> Path:
        """Resolve a path relative to verification/ (not oracle/)."""
        return (self.oracle_root.parent / rel).resolve()

    def linked_project_dir(self) -> Path:
        linked = self.raw["linked_project"]
        return (self.oracle_root / linked["directory"]).resolve()

    def linked_files(self) -> dict[str, Path]:
        linked = self.raw["linked_project"]
        base = self.linked_project_dir()
        files = {
            "geometry": base / linked["geometry"],
        }
        if linked.get("plan"):
            files["plan"] = base / linked["plan"]
        if linked.get("flow"):
            files["flow"] = base / linked["flow"]
        if linked.get("unsteady_flow"):
            files["unsteady_flow"] = base / linked["unsteady_flow"]
        return files


def load_scenario(path: Path) -> LinkedScenario:
    oracle_root = path.parent.parent if path.parent.name == "scenarios" else path.parent
    with path.open("r", encoding="utf-8") as fh:
        raw = json.load(fh)
    return LinkedScenario(raw=raw, path=path.resolve(), oracle_root=oracle_root.resolve())
