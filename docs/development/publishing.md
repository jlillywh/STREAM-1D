# PyPI publishing

PyPI distribution name and Python import: **stream1d** (`pip install stream1d`, `import stream1d`).

## One-time setup

1. On [pypi.org](https://pypi.org): trusted publisher (GitHub). PyPI project name must match `[project] name` in `pyproject.toml`:
   - PyPI project name: `stream1d`
   - Owner: `jlillywh`
   - Repository: `STREAM-1D`
   - Workflow name: `publish.yml`
   - Environment name: `pypi`
2. On GitHub (`jlillywh/STREAM-1D`): Settings → Environments → **pypi** (no API token needed).

## Release

1. Set version in `Cargo.toml` and `pyproject.toml` (keep in sync).
2. Commit and push to `main`.
3. Tag and push:

```bash
git tag v0.1.0
git push origin v0.1.0
```

Or: Actions → Publish → Run workflow (if `workflow_dispatch` is enabled on `main`).

## Local build (without publishing)

```bash
pip install maturin
maturin build --release --features python --out dist
```
