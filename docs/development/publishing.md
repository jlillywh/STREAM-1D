# PyPI publishing

Package name: **streams1d** (`pip install streams1d`).

## One-time setup

1. On [pypi.org](https://pypi.org): add a trusted publisher (GitHub):
   - PyPI project name: `streams1d`
   - Owner: `jlillywh`
   - Repository: `STREAM-1D`
   - Workflow name: `publish.yml`
   - Environment name: `pypi`
2. On GitHub (`jlillywh/STREAM-1D`): Settings → Environments → create **pypi** (no secrets required for trusted publishing).

## Release

1. Set version in `Cargo.toml` and `pyproject.toml` (keep them in sync).
2. Commit and push to `main`.
3. Tag and push:

```bash
git tag v0.1.0
git push origin v0.1.0
```

Workflow [`.github/workflows/publish.yml`](../../.github/workflows/publish.yml) builds Linux (x86_64, aarch64), Windows, and macOS wheels plus an sdist, then publishes via OIDC.

## Local build (without publishing)

```bash
pip install maturin
maturin build --release --features python --out dist
```
