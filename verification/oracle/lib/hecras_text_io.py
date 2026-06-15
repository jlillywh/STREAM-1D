"""Safe read/write for HEC-RAS legacy CRLF text project files."""

from __future__ import annotations

from pathlib import Path

_RAS_TEXT_SUFFIXES = {".prj", ".g01", ".u02", ".p02", ".f01", ".u01", ".u03"}


def read_ras_lines(path: Path) -> list[str]:
    """Return non-empty logical lines from a RAS text file."""
    return [ln for ln in path.read_text(encoding="utf-8", errors="replace").splitlines() if ln.strip()]


def write_ras_lines(path: Path, lines: list[str]) -> None:
    """Write RAS text with Windows CRLF and no blank-line corruption."""
    path.write_bytes(("\r\n".join(lines) + "\r\n").encode("utf-8"))


def copy_ras_text_file(src: Path, dest: Path) -> None:
    write_ras_lines(dest, read_ras_lines(src))


def assert_compact_ras_text(path: Path) -> None:
    """Raise if file contains empty records (HEC-RAS load failure)."""
    data = path.read_bytes()
    if b"\r\n\r\n" in data or b"\n\n" in data:
        raise ValueError(f"{path.name}: blank-line corruption detected")
    lines = read_ras_lines(path)
    if not lines:
        raise ValueError(f"{path.name}: empty RAS text file")


def is_ras_text_file(path: Path) -> bool:
    return path.suffix.lower() in _RAS_TEXT_SUFFIXES
