# 3.4 Extended pier shape catalog

HEC-RAS pier nose coefficients vs STREAM-1D `bridge_pier_shapes` / `pier_shape_type`. **Implemented** API v29 (`PierShape` in [`bridge.rs`](../../src/solvers/bridge.rs)).

**Scope:** one shape per bridge (all piers share $K$ and $C_D$). Pier plan area still comes from width tables (v27–v28). Field mapping: [`hecras_parity.md`](../reference/hecras_parity.md) § Bridge pier editor.

**References:** [HEC-RAS low flow computations](https://www.hec.usace.army.mil/confluence/rasdocs/ras1dtechref/6.3/modeling-bridges/hydraulic-computations-through-the-bridge/low-flow-computations).

---

## Shape ID table (`bridge_pier_shapes` / `pier_shape_type`, API v29)

Canonical mapping of integer codes to `PierShape` in [`bridge.rs`](../../src/solvers/bridge.rs). One value per bridge; default **Square** (`0`) when omitted or unknown.

| Code | Name | Yarnell $K$ | Momentum $C_D$ | HEC-RAS source |
|-----:|------|------------:|---------------:|----------------|
| `0` | Square | 1.25 | 2.00 | Square nose and tail |
| `1` | Semicircular | 0.90 | 1.20 | Semi-circular / circular pier |
| `2` | TwinCylinder | 0.95 | 1.33 | Twin-cylinder **with** diaphragm |
| `3` | Triangular | 1.05 | 1.60 | 90° triangular nose and tail |
| `4` | TwinCylinderNoDiaphragm | 1.05 | 1.33 | Twin-cylinder **without** diaphragm; $C_D$ = elongated semi-circular |
| `5` | TenPileTrestle | 2.50 | 2.00 | Ten-pile trestle bent; $C_D$ = square fallback (no HEC row) |
| `6` | Elliptical2to1 | 0.90† | 0.60 | Elliptical 2:1 L:W |
| `7` | Elliptical4to1 | 0.90† | 0.32 | Elliptical 4:1 L:W |
| `8` | Elliptical8to1 | 0.90† | 0.29 | Elliptical 8:1 L:W |
| `9` | Triangular30 | 1.05‡ | 1.00 | Triangular nose 30° |
| `10` | Triangular60 | 1.05‡ | 1.39 | Triangular nose 60° |
| `11` | Triangular120 | 1.05‡ | 1.72 | Triangular nose 120° |

† HEC-RAS publishes momentum $C_D$ only for elliptical noses — $K=0.90$ (semicircular) used when Yarnell low flow is selected.  
‡ HEC publishes $C_D$ only for acute/obtuse triangles — $K=1.05$ (90° triangular Yarnell row) when Yarnell is selected.

Default when omitted or unknown code: **Square** (`0`).

---

## Importer matrix

| HEC-RAS model / GUI preset | `bridge_pier_shapes` |
|----------------------------|---------------------|
| Square / wall-type square / dual-column square | `0` |
| Round / single circular / wall-type round | `1` |
| Twin round columns **with** diaphragm | `2` |
| 90° triangular / wall-type triangular | `3` |
| Twin round columns **without** diaphragm | `4` |
| Ten-pile trestle bent | `5` |
| Elliptical 2:1 / 4:1 / 8:1 (momentum) | `6` / `7` / `8` |
| Triangular 30° / 60° / 120° (momentum) | `9` / `10` / `11` |
| Hammerhead | width table + closest nose (`0` or `3`) |

---

## Not in scope (3.4)

| Feature | Notes |
|---------|--------|
| Per-pier shape `[bridge][pier]` | One enum per bridge today |
| User override $K$ / $C_D$ | Fixed per enum |
| Floating pier debris | HEC editor option — not modeled |
| Plan polygons (fenders, hammerhead) | v28 nosing / pending §C polygons |

---

## Checklist

- [x] **Survey** — HEC-RAS pier types vs four STREAM shapes
- [x] **Implementation** — `PierShape` values `4`–`11` with documented $K$ / $C_D$
- [x] **Types** — `wasm_api.types.ts` JSDoc; `API_VERSION` 29
- [x] **Solver** — `from_i32`, Yarnell + momentum wiring (unchanged call sites)
- [x] **Tests** — one case per new shape (`test_pier_shape_4` … `test_pier_shape_11`) + coefficient table regression
- [x] **Docs** — Shape ID table (§ Shape ID table above)
- [x] **Docs** — `equations.md` §B, `api_changelog` v29, `hecras_parity.md`
- [ ] **Per-pier shape** — `bridge_pier_shape_types[b][pier]` (future)
- [ ] **Optional** $K$ / $C_D$ overrides (future)
