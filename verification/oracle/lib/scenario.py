"""Load linked-verify scenario manifests."""

from __future__ import annotations

import json
import re
from dataclasses import dataclass
from pathlib import Path
from typing import Any

_PLAN_SUFFIX_RE = re.compile(r"\.p(\d+)$", re.IGNORECASE)


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

    @property
    def enforce_tolerance(self) -> bool:
        """When false, reports show Δ only (no PASS/FAIL gate); verify exits 0."""
        compare = self.raw.get("compare", {})
        if "enforce_tolerance" in compare:
            return bool(compare["enforce_tolerance"])
        parity = self.raw.get("parity_program", {})
        if str(parity.get("certification", "")).lower() == "diagnostic":
            return False
        return True

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

    def plan_number(self) -> str:
        """HEC-RAS plan key (e.g. ``02``) from linked_project.plan_number or plan filename."""
        return plan_number_from_linked(self.raw["linked_project"])


def plan_number_from_linked(linked: dict[str, Any]) -> str:
    if linked.get("plan_number") is not None:
        return str(linked["plan_number"]).zfill(2)
    plan_file = str(linked.get("plan", ""))
    match = _PLAN_SUFFIX_RE.search(plan_file)
    if match:
        return match.group(1).zfill(2)
    return "01"


def _oracle_root_from_scenario(path: Path) -> Path:
    """Return ``verification/oracle/`` for a scenario JSON under ``scenarios/``."""
    parent = path.parent.resolve()
    if parent.name == "generated" and parent.parent.name == "scenarios":
        return parent.parent.parent
    if parent.name == "scenarios":
        return parent.parent
    return parent


def load_scenario(path: Path) -> LinkedScenario:
    oracle_root = _oracle_root_from_scenario(path)
    with path.open("r", encoding="utf-8") as fh:
        raw = json.load(fh)
    return LinkedScenario(raw=raw, path=path.resolve(), oracle_root=oracle_root.resolve())
