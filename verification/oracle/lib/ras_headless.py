"""Headless HEC-RAS execution and HDF WSEL extraction for linked oracle verify."""

from __future__ import annotations

import os
import re
import shutil
import subprocess
import sys
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from .stage_paths import hecras_stage_dir


class RasHeadlessError(RuntimeError):
    """HEC-RAS headless workflow failed."""


@dataclass(frozen=True)
class TerminalWselCheckpoint:
    rm: float
    wsel_ft: float
    river: str
    reach: str
    station: str


@dataclass(frozen=True)
class TimeseriesWselCheckpoint:
    rm: float
    hour: float
    wsel_ft: float
    river: str
    reach: str
    station: str


def is_wsl() -> bool:
    """True only when running Linux Python inside WSL (not Windows Python on a WSL UNC cwd)."""
    if sys.platform != "linux":
        return False
    try:
        with open("/proc/version", encoding="utf-8") as fh:
            return "microsoft" in fh.read().lower()
    except OSError:
        return False


def _is_wsl_unc_path(path: Path) -> bool:
    normalized = str(path).replace("/", "\\")
    lower = normalized.lower()
    return lower.startswith("\\\\wsl.localhost\\") or lower.startswith("\\\\wsl$\\")


def _needs_windows_staging(source_dir: Path) -> bool:
    if is_wsl():
        return True
    return sys.platform == "win32" and _is_wsl_unc_path(source_dir)


def _windows_local_app_data() -> Path | None:
    """Resolve Windows LOCALAPPDATA in a form usable on the current OS."""
    if sys.platform == "win32":
        local = os.environ.get("LOCALAPPDATA")
        return Path(local) if local else None
    if not is_wsl():
        return None
    try:
        raw = subprocess.check_output(
            ["cmd.exe", "/c", "echo", "%LOCALAPPDATA%"],
            text=True,
            stderr=subprocess.DEVNULL,
        ).strip().replace("\r", "")
    except (subprocess.CalledProcessError, FileNotFoundError):
        return None
    if not raw or len(raw) < 2 or raw[1] != ":":
        return None
    drive = raw[0].lower()
    rest = raw[2:].replace("\\", "/")
    return Path(f"/mnt/{drive}{rest}")


def _stage_ignore_fn():
    return shutil.ignore_patterns(
        "*.hdf",
        "*.tmp.hdf",
        "*.data_errors.txt",
        "_compute_*.log",
        "_run_*.bat",
        "simple_channel.g02",
        "simple_channel.f01",
        "*.g02",
    )


_RAS_TEXT_SUFFIXES = {".prj", ".g01", ".u02", ".u03", ".u04", ".u05", ".p01", ".p02", ".p03", ".p04", ".p05", ".f01"}


def _write_ras_text_crlf(path: Path, text: str) -> None:
    """Write HEC-RAS legacy text with Windows CRLF and no blank-line corruption."""
    lines = text.splitlines()
    path.write_bytes(("\r\n".join(lines) + "\r\n").encode("utf-8"))


def _copy_ras_project_file(src: Path, dest: Path) -> None:
    if sys.platform == "win32" and src.suffix.lower() in _RAS_TEXT_SUFFIXES:
        _write_ras_text_crlf(dest, src.read_text(encoding="utf-8", errors="replace"))
        return
    shutil.copy2(src, dest)


def _refresh_stage_from_source(source_dir: Path, stage: Path) -> None:
    """Copy project inputs into an existing stage dir, skipping locked files."""
    ignore = _stage_ignore_fn()
    ignored = set(ignore(str(source_dir), [p.name for p in source_dir.iterdir()]))
    stage.mkdir(parents=True, exist_ok=True)
    for stale in stage.glob("*.hdf"):
        try:
            stale.unlink()
        except OSError:
            pass
    for stale in stage.glob("_run_p*.bat"):
        try:
            stale.unlink()
        except OSError:
            pass
    for stale in stage.glob("_compute_p*.log"):
        try:
            stale.unlink()
        except OSError:
            pass
    for src in source_dir.iterdir():
        if src.name in ignored:
            continue
        dest = stage / src.name
        try:
            if src.is_dir():
                if dest.exists():
                    shutil.rmtree(dest, ignore_errors=True)
                shutil.copytree(src, dest, ignore=ignore)
            else:
                _copy_ras_project_file(src, dest)
        except PermissionError:
            print(
                f"Warning: could not refresh locked staged file (close HEC-RAS if needed):\n"
                f"  {dest}",
                flush=True,
            )


def stage_project_for_hecras(source_dir: Path) -> tuple[Path, Path | None]:
    """
    Return (run_dir, sync_back_to).

    When the project lives on a Linux filesystem (WSL) or a \\wsl.localhost UNC
    path (Windows Python), copy to Documents\\hecras_testing before HEC-RAS runs.
    """
    source_dir = source_dir.resolve()
    if not _needs_windows_staging(source_dir):
        return source_dir, None

    stage = hecras_stage_dir(source_dir.name)
    stage.parent.mkdir(parents=True, exist_ok=True)
    ignore = _stage_ignore_fn()
    if stage.exists():
        print(f"Reusing staged project (refreshing inputs):\n  {stage}", flush=True)
        _refresh_stage_from_source(source_dir, stage)
    else:
        stage.mkdir(parents=True, exist_ok=True)
        ignore = _stage_ignore_fn()
        ignored = set(ignore(str(source_dir), [p.name for p in source_dir.iterdir()]))
        for src in source_dir.iterdir():
            if src.name in ignored:
                continue
            dest = stage / src.name
            if src.is_dir():
                shutil.copytree(src, dest, ignore=ignore)
            else:
                _copy_ras_project_file(src, dest)
    print(
        f"Staged project for HEC-RAS on Windows drive:\n"
        f"  {source_dir}\n"
        f"  -> {stage}",
        flush=True,
    )
    return stage, source_dir


def _sync_run_artifacts(run_dir: Path, dest_dir: Path, plan_key: str) -> None:
    stem = dest_dir.name
    patterns = [
        f"{stem}.p{plan_key}.hdf",
        f"{stem}.p{plan_key}.computeMsgs.txt",
        f"{stem}.g01.hdf",
        f"{stem}.p{plan_key}.data_errors.txt",
    ]
    for pattern in patterns:
        for src in run_dir.glob(pattern):
            shutil.copy2(src, dest_dir / src.name)
            print(f"Synced result: {dest_dir / src.name}", flush=True)


def wsl_to_windows_path(path: Path) -> str:
    """Convert a WSL /mnt/c/... path to C:\\... for Windows executables."""
    path = path.resolve()
    if is_wsl():
        win = subprocess.check_output(
            ["wslpath", "-w", str(path)],
            text=True,
        ).strip()
        return win.replace("\\", "/")
    return str(path).replace("\\", "/")


def _resolve_ras_exe(
    *,
    ras_version: str | None = None,
    ras_exe: str | None = None,
) -> Path:
    candidate = ras_exe or os.environ.get("HECRAS_RAS_EXE")
    if candidate and Path(candidate).is_file():
        return Path(candidate)
    if ras_version:
        from ras_commander import get_ras_exe  # type: ignore[import-not-found]

        exe = get_ras_exe(ras_version)
        if exe and Path(str(exe)).is_file():
            return Path(str(exe))
    try:
        from ras_commander import get_ras_exe  # type: ignore[import-not-found]

        exe = get_ras_exe(None)
        if exe and Path(str(exe)).is_file():
            return Path(str(exe))
    except Exception:
        pass
    raise RasHeadlessError(
        "HEC-RAS Ras.exe not found. Set HECRAS_RAS_EXE to the full path "
        "(WSL example: /mnt/c/Program Files (x86)/HEC/HEC-RAS/7.0.1/Ras.exe)."
    )


def _prefer_ras_commander_on_windows() -> bool:
    return os.environ.get("HECRAS_USE_RAS_COMMANDER", "").strip().lower() in {
        "1",
        "true",
        "yes",
    }


def _run_plan_via_windows_cmd(
    run_dir: Path,
    plan_key: str,
    ras_exe: Path,
    *,
    num_cores: int = 1,
) -> None:
    """
    Run HEC-RAS on Windows from WSL via a helper .bat file.

    HEC-RAS parses GetCommandLine() and requires quoted paths. Passing argv
    from WSL or cmd.exe /c strings gets quoting wrong; a .bat in the staged
    project folder matches ras-commander's shell=True pattern reliably.
    """
    del num_cores
    stem = run_dir.name
    prj = run_dir / f"{stem}.prj"
    plan = run_dir / f"{stem}.p{plan_key}"
    if not prj.is_file():
        raise RasHeadlessError(f"Missing project file: {prj}")
    if not plan.is_file():
        raise RasHeadlessError(f"Missing plan file: {plan}")

    win_exe = wsl_to_windows_path(ras_exe)
    win_prj = wsl_to_windows_path(prj)
    win_plan = wsl_to_windows_path(plan)
    bat_path = run_dir / f"_run_p{plan_key}.bat"
    log_path = run_dir / f"_compute_p{plan_key}.log"
    # Use absolute Windows paths — relative names fail if Ras.exe cwd is not the project folder.
    bat_path.write_text(
        "@echo off\r\n"
        f'set "RAS_EXE={win_exe}"\r\n'
        f'if not exist "{win_prj}" (echo MISSING PRJ: {win_prj}& exit /b 1)\r\n'
        f'if not exist "{win_plan}" (echo MISSING PLAN: {win_plan}& exit /b 1)\r\n'
        f'"%RAS_EXE%" -c "{win_prj}" "{win_plan}"\r\n'
        "exit /b %ERRORLEVEL%\r\n",
        encoding="utf-8",
    )
    print(f"Batch Ras.exe line: \"{win_exe}\" -c \"{win_prj}\" \"{win_plan}\"", flush=True)
    print(
        f"Staged files: prj={prj.is_file()} plan={plan.is_file()} g01={(run_dir / f'{stem}.g01').is_file()} u02={(run_dir / f'{stem}.u02').is_file()}",
        flush=True,
    )
    win_bat = wsl_to_windows_path(bat_path)
    print(f"Running HEC-RAS via batch file:\n  {bat_path}\n  log -> {log_path}", flush=True)
    wait_start = time.monotonic()
    with log_path.open("w", encoding="utf-8", errors="replace") as logfh:
        proc = subprocess.Popen(
            ["cmd.exe", "/c", win_bat],
            cwd=str(run_dir),
            stdout=logfh,
            stderr=subprocess.STDOUT,
        )
        last_status = wait_start
        while proc.poll() is None:
            elapsed = time.monotonic() - wait_start
            if elapsed > 900:
                proc.kill()
                raise RasHeadlessError(
                    "HEC-RAS batch run timed out after 15 min. "
                    "Check the Windows desktop for a blocking RAS dialog."
                )
            if time.monotonic() - last_status >= 30:
                print(f"  ... HEC-RAS still running ({elapsed:.0f}s)", flush=True)
                last_status = time.monotonic()
            time.sleep(2)
        result = subprocess.CompletedProcess(proc.args, proc.returncode)
    if log_path.is_file():
        tail = log_path.read_text(encoding="utf-8", errors="replace").strip().splitlines()[-15:]
        if tail:
            print("HEC-RAS log (last lines):", flush=True)
            for line in tail:
                print(f"  {line}", flush=True)
    if result.returncode != 0:
        raise RasHeadlessError(
            f"Ras.exe exited with code {result.returncode}. See {log_path.name} in the staged project."
        )


def _compute_succeeded(result: Any) -> bool:
    if result is False or result is None:
        return False
    success = getattr(result, "success", None)
    if success is not None:
        if isinstance(success, str):
            return success.upper() in {"SUCCESS", "OK", "TRUE", "1"}
        return bool(success)
    return bool(result)


def _plan_output_hint(run_dir: Path, plan_key: str, *, elapsed: float) -> str:
    hints: list[str] = []
    if elapsed < 10:
        hints.append(
            "HEC-RAS finished in under 10 s — ras-commander may have auto-dismissed error "
            "dialogs (common messages: 'Error in Loading Unsteady Flow Data', "
            "'Error determining XS cut lines')."
        )
    u02 = next(run_dir.glob("*.u*"), None)
    if u02 and u02.is_file():
        hints.append(
            f"Check {u02.name}: boundary hydrograph ordinate count must equal simulation "
            "duration / Interval + 1 (48 h @ 1HOUR → 49 values), and downstream RM must "
            "match geometry (use 20.0 not 20.000)."
        )
    return "\n".join(hints)


def _require_ras_commander():
    try:
        from ras_commander import RasCmdr, init_ras_project, ras  # type: ignore[import-not-found]
        from ras_commander import HdfResultsXsec  # type: ignore[import-not-found]
    except ImportError as exc:
        raise RasHeadlessError(
            "ras-commander is required for headless HEC-RAS runs.\n"
            "Install with: pip install ras-commander\n"
            "Also requires a local HEC-RAS 6.x installation (Windows)."
        ) from exc
    return RasCmdr, init_ras_project, ras, HdfResultsXsec


def _parse_station_rm(station: str) -> float:
    """Parse HDF/g01 river station tokens (e.g. ``20.208*``)."""
    token = station.strip()
    if not token:
        raise ValueError("empty station")
    cleaned = re.sub(r"[^0-9.\-+]", "", token)
    if not cleaned:
        raise ValueError(f"no numeric river station in {station!r}")
    return float(cleaned)


def _find_prj(project_dir: Path) -> Path:
    prj_files = sorted(project_dir.glob("*.prj"))
    if not prj_files:
        raise RasHeadlessError(f"No .prj file in {project_dir}")
    preferred = project_dir / f"{project_dir.name}.prj"
    if preferred.is_file():
        return preferred
    if len(prj_files) == 1:
        return prj_files[0]
    raise RasHeadlessError(f"Multiple .prj files in {project_dir}; expected {preferred.name}")


def _collect_run_diagnostics(project_dir: Path, plan_key: str) -> str:
    lines: list[str] = []
    stem = project_dir.name
    for name in (
        f"_compute_p{plan_key}.log",
        f"_run_p{plan_key}.bat",
        f"{stem}.p{plan_key}.computeMsgs.txt",
        f"{stem}.p{plan_key}.data_errors.txt",
        f"{stem}.b{plan_key}",
    ):
        path = project_dir / name
        if path.is_file():
            text = path.read_text(encoding="utf-8", errors="replace").strip()
            if text:
                lines.append(f"--- {name} ---\n{text[-4000:]}")
    hdf_files = sorted(project_dir.glob("*.hdf"))
    if hdf_files:
        lines.append("--- HDF files present ---\n" + "\n".join(p.name for p in hdf_files))
    else:
        lines.append("--- HDF files present ---\n(none)")
    return "\n\n".join(lines)


def _resolve_ras_version_string(
    *,
    ras_version: str | None = None,
    ras_exe: str | Path | None = None,
) -> str:
    if ras_version:
        return str(ras_version)
    env = os.environ.get("HECRAS_VERSION")
    if env:
        return str(env)
    if ras_exe:
        match = re.search(r"HEC-RAS[/\\]([\d.]+)", str(ras_exe), flags=re.IGNORECASE)
        if match:
            return match.group(1)
    return "7.0"


def run_plan_headless(
    project_dir: Path,
    plan_number: str = "01",
    *,
    ras_version: str | None = None,
    ras_exe: str | None = None,
    num_cores: int = 1,
    clear_geompre: bool = False,
) -> Path:
    """
    Run a HEC-RAS plan and return the plan HDF path.

    Under WSL, the project is staged to Windows LOCALAPPDATA and Ras.exe is
    invoked via cmd.exe with wslpath-converted C:\\ paths (ras-commander alone
    passes /mnt/c/... which HEC-RAS rejects).
    """
    source_dir = project_dir.resolve()
    run_dir, sync_back = stage_project_for_hecras(source_dir)
    plan_key = plan_number.zfill(2)
    exe = _resolve_ras_exe(ras_version=ras_version, ras_exe=ras_exe)

    print(
        f"\nRunning HEC-RAS plan {plan_key} (expect 1–15 min, little or no console output)...",
        flush=True,
    )
    t0 = time.monotonic()

    if is_wsl():
        print(
            "WSL: running HEC-RAS via Windows cmd.exe + Ras.exe (ras-commander cannot "
            "invoke Ras.exe from Linux).\n"
            "Unsteady runs often take 1–15 min. If nothing happens after ~60 s, check the "
            "Windows desktop for a HEC-RAS dialog and dismiss it.",
            flush=True,
        )
        _run_plan_via_windows_cmd(run_dir, plan_key, exe, num_cores=num_cores)
        ras = None
    elif sys.platform == "win32" and not _prefer_ras_commander_on_windows():
        print(
            "Windows: running HEC-RAS via Ras.exe batch (no ras-commander dialog watchdog).\n"
            "If a RAS error dialog appears on the desktop, read it before dismissing.\n"
            "Set HECRAS_USE_RAS_COMMANDER=1 to use ras-commander instead.",
            flush=True,
        )
        _run_plan_via_windows_cmd(run_dir, plan_key, exe, num_cores=num_cores)
        ras = None
    else:
        RasCmdr, init_ras_project, ras, _ = _require_ras_commander()
        prj = _find_prj(run_dir)
        version = _resolve_ras_version_string(ras_version=ras_version, ras_exe=exe)
        print(f"Running via ras-commander (RAS {version}) on:\n  {run_dir}", flush=True)
        prev_cwd = Path.cwd()
        try:
            os.chdir(run_dir)
            init_ras_project(str(prj.resolve()), version, hide_intro=True)
            result = RasCmdr.compute_plan(
                plan_key,
                num_cores=num_cores,
                clear_geompre=clear_geompre,
            )
        finally:
            os.chdir(prev_cwd)
        if not _compute_succeeded(result):
            diag = _collect_run_diagnostics(run_dir, plan_key)
            raise RasHeadlessError(
                f"HEC-RAS plan {plan_key} failed (RasCmdr.compute_plan): {result!r}\n{diag}"
            )

    elapsed = time.monotonic() - t0
    print(f"HEC-RAS compute finished in {elapsed:.1f}s", flush=True)

    expected_hdf = _plan_hdf_path(run_dir, ras if not is_wsl() else None, plan_key)
    if not expected_hdf.is_file():
        diag = _collect_run_diagnostics(run_dir, plan_key)
        hint = _plan_output_hint(run_dir, plan_key, elapsed=elapsed)
        extra = f"\n{hint}" if hint else ""
        raise RasHeadlessError(
            f"HEC-RAS plan HDF not found after run: {expected_hdf}{extra}\n{diag}"
        )

    if sync_back is not None:
        _sync_run_artifacts(run_dir, sync_back, plan_key)
        # Also sync helper logs from the staged Windows run.
        for helper in (f"_compute_p{plan_key}.log", f"_run_p{plan_key}.bat"):
            src = run_dir / helper
            if src.is_file():
                shutil.copy2(src, sync_back / helper)
        project_dir = sync_back

    hdf_path = _plan_hdf_path(project_dir, ras, plan_key)
    if not hdf_path.is_file():
        hdf_path = _plan_hdf_path(run_dir, ras, plan_key)
        if hdf_path.is_file() and sync_back is not None:
            dest = sync_back / hdf_path.name
            shutil.copy2(hdf_path, dest)
            hdf_path = dest

    if not hdf_path.is_file():
        diag = _collect_run_diagnostics(project_dir, plan_key)
        if not diag and run_dir != project_dir:
            diag = _collect_run_diagnostics(run_dir, plan_key)
        raise RasHeadlessError(
            f"HEC-RAS plan HDF not found after run: {hdf_path}\n{diag}"
        )
    return hdf_path


def _plan_hdf_path(project_dir: Path, ras: Any, plan_key: str) -> Path:
    plan_num = plan_key.lstrip("0") or "0"
    try:
        row = ras.plan_df.loc[ras.plan_df["plan_number"].astype(str).str.lstrip("0") == plan_num]
        if not row.empty and row.iloc[0].get("hdf_path"):
            return Path(str(row.iloc[0]["hdf_path"]))
    except Exception:
        pass
    stem = project_dir.name
    for pattern in (f"{stem}.p{plan_key}.hdf", f"p{plan_key}.hdf", f"*.p{plan_key}.hdf"):
        matches = sorted(project_dir.glob(pattern))
        if matches:
            return matches[0]
    return project_dir / f"{stem}.p{plan_key}.hdf"


def _is_unc_path(path: Path) -> bool:
    text = str(path.resolve())
    return text.startswith("\\\\") or text.lower().startswith("//")


def _prepare_hdf_for_read(hdf_path: Path) -> tuple[Path, Any | None]:
    """
    HDF5 on Windows cannot reliably open files on \\\\wsl.localhost\\ (file lock errors).

    Copy to a native temp path when needed; caller should hold the returned cleanup object.
    """
    if sys.platform != "win32" or not _is_unc_path(hdf_path):
        return hdf_path, None
    import tempfile

    tmp = tempfile.TemporaryDirectory(prefix="stream1d_hdf_")
    local = Path(tmp.name) / hdf_path.name
    shutil.copy2(hdf_path, local)
    return local, tmp


def extract_terminal_wsel_at_rms(
    hdf_path: Path,
    checkpoints_rm: list[float],
    *,
    river: str | None = None,
    reach: str | None = None,
    rm_tol: float = 0.001,
) -> list[TerminalWselCheckpoint]:
    """Read terminal (last timestep) WSEL from plan HDF at requested river miles."""
    _, _, _, HdfResultsXsec = _require_ras_commander()
    if not hdf_path.is_file():
        raise RasHeadlessError(f"HDF not found: {hdf_path}")

    read_path, _cleanup = _prepare_hdf_for_read(hdf_path)
    if read_path is not hdf_path:
        print(f"  (HDF copied to native Windows path for read: {read_path})")

    ds = HdfResultsXsec.get_xsec_timeseries(read_path)
    if "Water_Surface" not in ds:
        raise RasHeadlessError(f"No Water_Surface in {hdf_path}")

    last_wsel = ds["Water_Surface"].isel(time=-1)
    rivers = [str(v).strip() for v in ds.coords["River"].values]
    reaches = [str(v).strip() for v in ds.coords["Reach"].values]
    stations = [str(v).strip() for v in ds.coords["Station"].values]

    rows: list[tuple[float, str, str, str, float]] = []
    for riv, rch, sta, wsel in zip(rivers, reaches, stations, last_wsel.values):
        if river and riv != river:
            continue
        if reach and rch != reach:
            continue
        try:
            rm = _parse_station_rm(sta)
        except ValueError:
            continue
        rows.append((rm, riv, rch, sta, float(wsel)))

    if not rows:
        raise RasHeadlessError(
            f"No cross-section rows matched in {hdf_path} "
            f"(river={river!r}, reach={reach!r})"
        )

    out: list[TerminalWselCheckpoint] = []
    for target_rm in checkpoints_rm:
        best = min(rows, key=lambda r: abs(r[0] - target_rm))
        if abs(best[0] - target_rm) > rm_tol:
            raise RasHeadlessError(
                f"No HDF cross section within {rm_tol} RM of {target_rm} "
                f"(closest RS={best[3]!r} @ RM {best[0]})"
            )
        out.append(
            TerminalWselCheckpoint(
                rm=target_rm,
                wsel_ft=best[4],
                river=best[1],
                reach=best[2],
                station=best[3],
            )
        )
    return out


def _xsec_rows_by_rm(
    hdf_path: Path,
    *,
    river: str | None,
    reach: str | None,
    time_index: int,
) -> list[tuple[float, str, str, str, float]]:
    _, _, _, HdfResultsXsec = _require_ras_commander()
    if not hdf_path.is_file():
        raise RasHeadlessError(f"HDF not found: {hdf_path}")

    read_path, _cleanup = _prepare_hdf_for_read(hdf_path)
    if read_path is not hdf_path:
        print(f"  (HDF copied to native Windows path for read: {read_path})")

    ds = HdfResultsXsec.get_xsec_timeseries(read_path)
    if "Water_Surface" not in ds:
        raise RasHeadlessError(f"No Water_Surface in {hdf_path}")

    n_times = int(ds.sizes.get("time", 0))
    if time_index < 0:
        time_index = n_times + time_index
    if time_index < 0 or time_index >= n_times:
        raise RasHeadlessError(
            f"time index {time_index} out of range for {hdf_path.name} (n_times={n_times})"
        )

    wsel = ds["Water_Surface"].isel(time=time_index)
    rivers = [str(v).strip() for v in ds.coords["River"].values]
    reaches = [str(v).strip() for v in ds.coords["Reach"].values]
    stations = [str(v).strip() for v in ds.coords["Station"].values]

    rows: list[tuple[float, str, str, str, float]] = []
    for riv, rch, sta, value in zip(rivers, reaches, stations, wsel.values):
        if river and riv != river:
            continue
        if reach and rch != reach:
            continue
        try:
            rm = _parse_station_rm(sta)
        except ValueError:
            continue
        rows.append((rm, riv, rch, sta, float(value)))
    if not rows:
        raise RasHeadlessError(
            f"No cross-section rows matched in {hdf_path} "
            f"(river={river!r}, reach={reach!r}, time_index={time_index})"
        )
    return rows


def extract_wsel_timeseries_at_rms(
    hdf_path: Path,
    checkpoints_rm: list[float],
    time_checkpoints_hr: list[float],
    *,
    river: str | None = None,
    reach: str | None = None,
    rm_tol: float = 0.001,
) -> list[TimeseriesWselCheckpoint]:
    """Read WSEL from plan HDF at requested simulation hours (1:1 with hourly indices)."""
    if not time_checkpoints_hr:
        raise RasHeadlessError("time_checkpoints_hr is empty")

    out: list[TimeseriesWselCheckpoint] = []
    for hour in time_checkpoints_hr:
        time_index = int(round(float(hour)))
        rows = _xsec_rows_by_rm(
            hdf_path,
            river=river,
            reach=reach,
            time_index=time_index,
        )
        for target_rm in checkpoints_rm:
            best = min(rows, key=lambda r: abs(r[0] - target_rm))
            if abs(best[0] - target_rm) > rm_tol:
                raise RasHeadlessError(
                    f"No HDF cross section within {rm_tol} RM of {target_rm} "
                    f"at hour {hour} (closest RS={best[3]!r} @ RM {best[0]})"
                )
            out.append(
                TimeseriesWselCheckpoint(
                    rm=target_rm,
                    hour=float(hour),
                    wsel_ft=best[4],
                    river=best[1],
                    reach=best[2],
                    station=best[3],
                )
            )
    return out


def checkpoints_to_reference_doc(
    checkpoints: list[TerminalWselCheckpoint],
    *,
    source: str,
    hdf_path: Path | None = None,
    coupling_mode: int = 0,
) -> dict[str, Any]:
    doc: dict[str, Any] = {
        "source": source,
        "coupling_mode": coupling_mode,
        "checkpoints": [{"rm": c.rm, "max_wsel_ft": c.wsel_ft} for c in checkpoints],
    }
    if hdf_path is not None:
        doc["hdf_path"] = str(hdf_path)
    return doc


def timeseries_checkpoints_to_reference_doc(
    checkpoints: list[TimeseriesWselCheckpoint],
    *,
    source: str,
    time_checkpoints_hr: list[float],
    hdf_path: Path | None = None,
    coupling_mode: int = 0,
) -> dict[str, Any]:
    by_rm: dict[float, dict[str, float]] = {}
    terminal: dict[float, float] = {}
    for c in checkpoints:
        hour_key = str(int(c.hour) if float(c.hour).is_integer() else c.hour)
        by_rm.setdefault(c.rm, {})[hour_key] = c.wsel_ft
        terminal[c.rm] = c.wsel_ft

    doc: dict[str, Any] = {
        "source": source,
        "coupling_mode": coupling_mode,
        "time_checkpoints_hr": [float(h) for h in time_checkpoints_hr],
        "checkpoints": [
            {
                "rm": rm,
                "max_wsel_ft": terminal.get(rm, max(hours.values())),
                "wsel_ft_by_hour": dict(sorted(hours.items(), key=lambda kv: float(kv[0]))),
            }
            for rm, hours in sorted(by_rm.items(), key=lambda row: -row[0])
        ],
    }
    if hdf_path is not None:
        doc["hdf_path"] = str(hdf_path)
    return doc


def write_reference_json(path: Path, doc: dict[str, Any]) -> None:
    import json

    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(doc, indent=2) + "\n", encoding="utf-8")


def update_u02_observed_hwm(
    u02_path: Path,
    checkpoints: list[TerminalWselCheckpoint],
    *,
    river: str,
    reach: str,
) -> None:
    rm_to_wsel = {c.rm: c.wsel_ft for c in checkpoints}
    lines: list[str] = []
    for line in u02_path.read_text(encoding="utf-8").splitlines():
        if line.startswith("Observed HWM="):
            parts = [p.strip() for p in line.split("=", 1)[1].split(",")]
            rm = float(re.sub(r"[^0-9.\-]", "", parts[2]))
            wsel = rm_to_wsel.get(rm)
            if wsel is not None:
                line = f"Observed HWM={river}    ,{reach} ,{rm:.1f}    ,,{wsel:.4f}"
        lines.append(line)

    existing_rms = {
        float(re.sub(r"[^0-9.\-]", "", p.split(",", 3)[2]))
        for p in (ln.split("=", 1)[1] for ln in lines if ln.startswith("Observed HWM="))
    }
    for c in checkpoints:
        if c.rm not in existing_rms:
            lines.append(
                f"Observed HWM={river}    ,{reach} ,{c.rm:.1f}    ,,{c.wsel_ft:.4f}"
            )
    u02_path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def hecras_available() -> tuple[bool, str]:
    try:
        _require_ras_commander()
    except RasHeadlessError as exc:
        return False, str(exc)
    ras_exe = os.environ.get("HECRAS_RAS_EXE")
    if ras_exe and Path(ras_exe).is_file():
        return True, f"ras-commander ok; HECRAS_RAS_EXE={ras_exe}"
    try:
        from ras_commander import get_ras_exe  # type: ignore[import-not-found]

        exe = get_ras_exe(None)
        if exe and Path(str(exe)).is_file():
            return True, f"ras-commander ok; Ras.exe={exe}"
    except Exception as exc:
        return False, f"ras-commander import ok but Ras.exe not found: {exc}"
    return False, "ras-commander import ok but HEC-RAS Ras.exe was not located (set HECRAS_RAS_EXE)"
