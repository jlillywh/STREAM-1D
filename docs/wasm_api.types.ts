/**
 * STREAM-1D WASM API TypeScript definitions.
 *
 * Copy into the web app (e.g. `src/types/streams1d.ts`).
 * Field names use snake_case to match Rust/Python JSON and `solveSteady()` payloads.
 *
 * Check `api_version` from `getWasmApiMetadata()` after each engine upgrade.
 */

export type UnitSystem = 'USCustomary' | 'Metric';

export interface CrossSection {
  station: number;
  x: number[];
  y: number[];
  n_stations: number[];
  n_values: number[];
  unit_system: UnitSystem;
  is_overbank?: boolean[];
}

/** Culvert shape codes passed in `culvert_shape_types`. */
export enum CulvertShapeType {
  Circular = 0,
  Box = 1,
  Arch = 2,
  ConspanArch = 3,
}

/**
 * FHWA inlet nomograph codes (`culvert_inlet_types`).
 * Use `0` for legacy Ke-threshold behavior (backward compatible).
 */
export enum CulvertInletType {
  LegacyKeThreshold = 0,
  CircularSquareHeadwall = 1,
  CircularGrooveEnd = 2,
  CircularBeveled45 = 3,
  CircularProjecting = 4,
  BoxSquareEdge = 10,
  BoxFlaredWingwalls = 11,
  BoxBeveledTop = 12,
  ArchProjecting = 20,
  ArchSmoothEntry = 21,
}

export type CulvertControlType = 'inlet' | 'outlet' | 'overtopping';

/**
 * Parallel arrays — index `i` describes the culvert at `culvert_stations[i]`.
 * Omit a Tier 1 array entirely or leave index unset to use engine defaults.
 */
export interface CulvertArrays {
  culvert_stations?: number[];
  culvert_shape_types?: number[];
  culvert_spans?: number[];
  culvert_rises?: number[];
  culvert_roughness_ns?: number[];
  culvert_lengths?: number[];
  culvert_entrance_loss_coeffs?: number[];
  culvert_exit_loss_coeffs?: number[];
  culvert_barrels?: number[];
  culvert_roughness_n_bottoms?: number[];
  culvert_depth_bottom_ns?: number[];
  culvert_depth_blockeds?: number[];
  /** Tier 1 — explicit FHWA inlet type per culvert */
  culvert_inlet_types?: number[];
  /** Tier 1 — optional upstream invert elevation (defaults to adjacent section bed) */
  culvert_z_ups?: number[];
  /** Tier 1 — optional downstream invert elevation */
  culvert_z_downs?: number[];
  /** Tier 1 — roadway crest for overtopping weir (omit for barrel-only) */
  culvert_crest_elevs?: number[];
  /** Tier 1 — weir Cd (default 2.6 US / 1.44 metric when 0 or omitted) */
  culvert_weir_coeffs?: number[];
  /** Tier 1 — weir length (default span × num_barrels when 0 or omitted) */
  culvert_weir_lengths?: number[];
}

export interface BridgeArrays {
  bridge_stations?: number[];
  bridge_low_chords?: number[];
  bridge_high_chords?: number[];
  bridge_pier_widths?: number[];
  bridge_num_piers?: number[];
  bridge_pier_shapes?: number[];
  bridge_weir_coeffs?: number[];
  bridge_orifice_coeffs?: number[];
}

export interface SteadyInputs extends CulvertArrays, BridgeArrays {
  cross_sections: CrossSection[];
  flow_rate: number;
  num_slices?: number;
  coeff_contraction?: number;
  coeff_expansion?: number;
  /** 0 = subcritical, 1 = supercritical, 2 = mixed */
  regime?: number;
  downstream_wsel?: number;
  upstream_wsel?: number;
  max_spacing?: number;
  downstream_bc_type?: number;
  downstream_bc_slope?: number;
  downstream_bc_rating_q?: number[];
  downstream_bc_rating_wsel?: number[];
  upstream_bc_type?: number;
  upstream_bc_slope?: number;
  upstream_bc_rating_q?: number[];
  upstream_bc_rating_wsel?: number[];
  tributary_cross_sections?: CrossSection[];
  tributary_flow_rate?: number;
  junction_main_station?: number;
}

export interface SteadyResult {
  wsel: number[];
  critical_wsel: number[];
  velocity: number[];
  area: number[];
  froude: number[];
  top_width: number[];
  eg_slope: number[];
  tributary_wsel?: number[];
  tributary_velocity?: number[];
  tributary_froude?: number[];
  /** Tier 1 — aligned with `culvert_stations`; omitted when no culverts modeled */
  culvert_control_types?: CulvertControlType[];
}

export interface UnsteadyInputs {
  cross_sections: CrossSection[];
  initial_wsel: number[];
  initial_q: number[];
  dt: number;
  num_steps: number;
  upstream_q_hydrograph: number[];
  downstream_wsel_hydrograph: number[];
  theta?: number;
  num_slices?: number;
  max_spacing?: number;
  coeff_contraction?: number;
  coeff_expansion?: number;
}

export interface UnsteadyResult {
  wsel: number[][];
  q: number[][];
  velocity: number[][];
  max_courant?: number;
  recommended_dt?: number;
}

export interface WasmEnumEntry {
  code: number;
  name: string;
  description: string;
}

export interface WasmApiMetadata {
  engine_version: string;
  api_version: number;
  entry_points: string[];
  field_naming: string;
  culvert_shape_types: WasmEnumEntry[];
  culvert_inlet_types: WasmEnumEntry[];
  culvert_control_types: CulvertControlType[];
  culvert_tier1_fields: {
    inputs: string[];
    outputs: string[];
  };
}

/** Module exports from `pkg/streams1d.js` after `wasm-pack build --target web` */
export interface Streams1dWasmModule {
  default: (url?: string | URL) => Promise<unknown>;
  getEngineVersion: () => string;
  getWasmApiMetadata: () => WasmApiMetadata;
  validateSteadyInputs: (inputs: SteadyInputs) => void;
  solveSteady: (inputs: SteadyInputs) => SteadyResult;
  solveUnsteady: (inputs: UnsteadyInputs) => UnsteadyResult;
}
