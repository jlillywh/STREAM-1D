/**
 * STREAM-1D WASM API types (snake_case; matches Rust/Python JSON).
 * Copy into the web app. Version history: `docs/reference/api_changelog.md`.
 * Modifier semantics: `docs/reference/equations.md` §H0. Densified-node inheritance: §H1.
 */

export type UnitSystem = 'USCustomary' | 'Metric';

/** See equations.md §H0. */
export interface BlockedObstruction {
  stations: number[];
  elevations: number[];
}

/** One normal ineffective-flow block (reach lateral `station`, activation `elevation`). */
export interface IneffectiveBlock {
  station: number;
  elevation: number;
}

/** Normal ineffective flow (OR across blocks). See equations.md §H0. */
export interface IneffectiveFlowAreas {
  left_blocks: IneffectiveBlock[];
  right_blocks: IneffectiveBlock[];
}

/** One guide-bank polyline on an approach or departure cut (reach lateral coordinates). */
export interface GuideBankPolyline {
  stations: number[];
  elevations: number[];
}

/** Simplified left or right guide-bank toe (station + elevation). */
export interface GuideBankToe {
  station: number;
  elevation: number;
}

/** Guide banks on approach/departure cuts (reach `x`). */
export interface GuideBanks {
  left_polylines?: GuideBankPolyline[];
  right_polylines?: GuideBankPolyline[];
  left_toe?: GuideBankToe;
  right_toe?: GuideBankToe;
}

export interface CrossSection {
  station: number;
  x: number[];
  y: number[];
  n_stations: number[];
  n_values: number[];
  unit_system: UnitSystem;
  is_overbank?: boolean[];
  /** Permanent fill. See equations.md §H0. */
  blocked_obstructions?: BlockedObstruction[];
  /** Normal ineffective flow (alias `ineffective_areas`). See equations.md §H0. */
  ineffective_flow_areas?: IneffectiveFlowAreas;
  ineffective_areas?: IneffectiveFlowAreas;
  /** Guide banks on approach / departure cuts (reach lateral `x`). */
  guide_banks?: GuideBanks;
}

/** Culvert shape codes passed in `culvert_shape_types`. */
export enum CulvertShapeType {
  Circular = 0,
  Box = 1,
  Arch = 2,
  ConspanArch = 3,
  PipeArch = 4,
  Elliptical = 5,
  Horseshoe = 6,
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
  /** Skew angle in degrees from normal to flow (0 = perpendicular) */
  culvert_skew_angles?: number[];
  /** Open barrels (≤ culvert_barrels); omit to use all barrels */
  culvert_active_barrels?: number[];
  /** Per-barrel span/diameter — `culvert_barrel_spans[i][j]` for culvert i, barrel j */
  culvert_barrel_spans?: number[][];
  /** Per-barrel rise — `culvert_barrel_rises[i][j]` for culvert i, barrel j */
  culvert_barrel_rises?: number[][];
}

export interface BridgeArrays {
  bridge_stations?: number[];
  bridge_low_chords?: number[];
  bridge_high_chords?: number[];
  bridge_pier_widths?: number[];
  bridge_num_piers?: number[];
  /**
   * Pier nose shape per bridge for Yarnell K and momentum C_D.
   * 0 square, 1 semicircular, 2 twin-cylinder w/ diaphragm, 3 triangular 90°,
   * 4 twin-cylinder w/o diaphragm, 5 ten-pile trestle, 6–8 elliptical 2:1/4:1/8:1,
   * 9–11 triangular 30°/60°/120° (API v29).
   */
  bridge_pier_shapes?: number[];
  bridge_weir_coeffs?: number[];
  bridge_orifice_coeffs?: number[];
  /** Total horizontal width blocked by left + right abutments (per bridge). */
  bridge_abutment_block_widths?: number[];
  /** Left abutment width per bridge (perpendicular to flow). With right widths, overrides legacy total. */
  bridge_abutment_left_widths?: number[];
  bridge_abutment_right_widths?: number[];
  /** Outer-face station in opening coordinates (default: opening left/right edge). */
  bridge_abutment_left_stations?: number[];
  bridge_abutment_right_stations?: number[];
  /** Constant top elevation per bridge (omit for full-height abutment). */
  bridge_abutment_left_top_elevations?: number[];
  bridge_abutment_right_top_elevations?: number[];
  /** Piecewise top profile per bridge `[bridge][point]` (≥ 2 points). */
  bridge_abutment_left_top_profile_stations?: number[][];
  bridge_abutment_left_top_profile_elevations?: number[][];
  bridge_abutment_right_top_profile_stations?: number[][];
  bridge_abutment_right_top_profile_elevations?: number[][];
  /** Low-flow method: 0 = auto, 1 = Yarnell, 2 = momentum, 3 = energy, 4 = WSPRO. */
  bridge_low_flow_methods?: number[];
  /** High-flow method: 0 = pressure/weir, 1 = energy. */
  bridge_high_flow_methods?: number[];
  /**
   * Reach length through each bridge for friction (user units) when BU/BD faces coincide (legacy).
   * When explicit BU/BD `CrossSection.station` values differ, friction uses their spacing
   * (summing interior cuts); this field does not override a shorter face spacing.
   */
  bridge_lengths?: number[];
  /**
   * HEC-RAS bridge friction weighting per bridge (energy / WSPRO / Class B energy path only).
   * **Omit or `0` (default):** opening friction only (BU→BD), matching HEC-RAS when approach/departure
   * friction weighting is not enabled.
   * **`1`:** three segments — approach→BU + BU→BD + BD→departure — using approach/departure cuts
   * (`bridge_approach_cross_sections`, `bridge_departure_cross_sections`, or nearest reach nodes).
   */
  bridge_friction_weighting?: number[];
  /**
   * Override approach friction reach length per bridge (user units). `0` = auto from
   * `|station(approach) − station(BU)|`. Only used when `bridge_friction_weighting[b] === 1`.
   */
  bridge_approach_friction_lengths?: number[];
  /**
   * Override departure friction reach length per bridge (user units). `0` = auto from
   * `|station(BD) − station(departure)|`. Only used when `bridge_friction_weighting[b] === 1`.
   */
  bridge_departure_friction_lengths?: number[];
  /**
   * Net opening area / conveyance multiplier per bridge (0–1]. Omit or `1.0` = no extra blockage.
   * See `bridge_ice_debris.md` §A.
   */
  bridge_opening_blockage_factors?: number[];
  /** Floating pier debris total width per bridge `[bridge][pier]` (opening coordinates). */
  bridge_pier_debris_widths?: number[][];
  /** Floating pier debris height below WSEL per bridge `[bridge][pier]` (user units). */
  bridge_pier_debris_heights?: number[][];
  /** Constant ice thickness through opening per bridge (user units). Requires `bridge_ice_modes[b] === 1`. */
  bridge_ice_thicknesses?: number[];
  /** Ice mode per bridge: `0` = none, `1` = constant thickness, `2` = reserved. */
  bridge_ice_modes?: number[];
  /** Roadway ice lowering weir crest per bridge (user units). */
  bridge_deck_ice_thicknesses?: number[];
  /** WSPRO contracted-opening discharge coefficient C per bridge (typical 0.7–0.9). */
  bridge_wspro_coeffs?: number[];
  /** Sluice-gate pressure coefficient when only upstream is submerged. 0 = auto (HEC-RAS Y3/Z). */
  bridge_pressure_flow_coeffs_inlet?: number[];
  /** Max weir submergence ratio before switching to energy method (default 0.98). */
  bridge_max_weir_submergence?: number[];
  /** Deck profile stations across opening per bridge `[bridge][point]`. */
  bridge_deck_stations?: number[][];
  /** Low chord elevation at each deck station `[bridge][point]`. */
  bridge_deck_low_elevations?: number[][];
  /** High chord elevation at each deck station `[bridge][point]`. */
  bridge_deck_high_elevations?: number[][];
  /** Bridge ineffective (opening `s`). Same hydraulics as §H0. Flat array = one block per bridge. */
  bridge_ineffective_left_stations?: number[] | number[][];
  /** Activation elevations for left ineffective blocks per bridge `[bridge][block]`. */
  bridge_ineffective_left_elevations?: number[] | number[][];
  /** Right ineffective-flow stations per bridge `[bridge][block]`. */
  bridge_ineffective_right_stations?: number[] | number[][];
  /** Activation elevations for right ineffective blocks per bridge `[bridge][block]`. */
  bridge_ineffective_right_elevations?: number[] | number[][];
  /** Upstream-face ineffective blocks (fall back to legacy shared fields). */
  bridge_ineffective_left_stations_upstream?: number[] | number[][];
  bridge_ineffective_left_elevations_upstream?: number[] | number[][];
  bridge_ineffective_right_stations_upstream?: number[] | number[][];
  bridge_ineffective_right_elevations_upstream?: number[] | number[][];
  /** Downstream-face ineffective blocks (fall back to legacy shared fields). */
  bridge_ineffective_left_stations_downstream?: number[] | number[][];
  bridge_ineffective_left_elevations_downstream?: number[] | number[][];
  bridge_ineffective_right_stations_downstream?: number[] | number[][];
  bridge_ineffective_right_elevations_downstream?: number[] | number[][];
  /** Skew from normal to flow, degrees per bridge (0–59°). */
  bridge_skew_angles?: number[];
  /** Pier centerline stations across opening per bridge `[bridge][pier]`. */
  bridge_pier_stations?: number[][];
  /** Pier top width (perpendicular to flow) per bridge `[bridge][pier]` — linear taper with `bridge_pier_bottom_widths`. */
  bridge_pier_top_widths?: number[][];
  bridge_pier_bottom_widths?: number[][];
  /** Piecewise pier width profile per bridge `[bridge][pier][point]` (absolute elevation + width). */
  bridge_pier_width_elevations?: number[][][];
  bridge_pier_width_values?: number[][][];
  /** Optional cap/base elevations for top/bottom pair (omit when using width profile). */
  bridge_pier_top_elevations?: number[][];
  bridge_pier_base_elevations?: number[][];
  /** Footing shorthand per bridge `[bridge][pier]` — top of pile cap / bottom of shaft. */
  bridge_pier_footing_top_elevations?: number[][];
  bridge_pier_footing_widths?: number[][];
  bridge_pier_footing_bottom_elevations?: number[][];
  /** Upstream nosing length (flow-normal) per bridge `[bridge][pier]`. */
  bridge_pier_nosing_lengths?: number[][];
  bridge_pier_nosing_widths?: number[][];
  /** HEC-RAS BU (bridge upstream face) cross section per bridge. */
  bridge_upstream_cross_sections?: CrossSection[];
  /** HEC-RAS BD (bridge downstream face) cross section per bridge. */
  bridge_downstream_cross_sections?: CrossSection[];
  /** Optional interior bridge cuts per bridge `[bridge][section]`, US → DS. */
  bridge_internal_cross_sections?: CrossSection[][];
  /** Reach XS lateral `x` at bridge opening station 0 per bridge (explicit anchor). */
  bridge_opening_reach_station_origins?: number[];
  /**
   * Opening ↔ reach anchor mode per bridge: 0 = BU left `min(x)`, 1 = reach river station,
   * 2 = explicit lateral `x` (requires `bridge_opening_reach_station_origins`).
   */
  bridge_opening_anchor_modes?: number[];
  /** Longitudinal reach river station (user units) for anchor mode 1 per bridge. */
  bridge_opening_anchor_reach_stations?: number[];
  /** Explicit approach (upstream) cross section per bridge — HEC-RAS section 4 equivalent. */
  bridge_approach_cross_sections?: CrossSection[];
  /** Explicit departure (exit) cross section per bridge. */
  bridge_departure_cross_sections?: CrossSection[];
  /** Reach river station of approach cut when explicit section omitted. */
  bridge_approach_reach_stations?: number[];
  /** Reach river station of departure cut when explicit section omitted. */
  bridge_departure_reach_stations?: number[];
  /** Guide banks on approach cut when not on `CrossSection.guide_banks`. */
  bridge_approach_guide_banks?: GuideBanks[];
  /** Guide banks on departure cut when not on `CrossSection.guide_banks`. */
  bridge_departure_guide_banks?: GuideBanks[];
  /**
   * Unified roadway embankment per bridge (API v26). Composes deck, abutment, ineffective,
   * and embankment blocked tops from grade profiles. See `equations.md` §G2.
   */
  bridge_roadway_embankments?: (BridgeRoadwayEmbankment | null)[];
}

/** Opening frame: s=0 at left deck edge. */
export interface EmbankmentPolyline {
  stations: number[];
  elevations: number[];
}

export interface BridgeDeckInput {
  stations: number[];
  low_elevations: number[];
  high_elevations: number[];
}

export interface IneffectiveBlockPoint {
  station: number;
  elevation: number;
}

export interface RoadwayAbutmentInput {
  outer_station?: number;
  width: number;
  top_elevation?: number;
  top_profile?: EmbankmentPolyline;
}

export interface RoadwayEmbankmentSide {
  /** Grade line drives ineffective activation and blocked top when derived (default). */
  embankment_profile?: EmbankmentPolyline;
  ineffective_blocks?: IneffectiveBlockPoint[];
  abutment?: RoadwayAbutmentInput;
  derive_ineffective?: boolean;
  derive_blocked?: boolean;
}

export interface BridgeIneffectiveFaceOverride {
  left_blocks?: IneffectiveBlockPoint[];
  right_blocks?: IneffectiveBlockPoint[];
}

export interface BridgeRoadwayEmbankment {
  deck: BridgeDeckInput;
  left?: RoadwayEmbankmentSide;
  right?: RoadwayEmbankmentSide;
  ineffective_faces?: {
    upstream?: BridgeIneffectiveFaceOverride;
    downstream?: BridgeIneffectiveFaceOverride;
  };
  derive_ineffective?: boolean;
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
  /**
   * Reach modifier inheritance on `max_spacing` interior nodes: 0 = none (default),
   * 1 = upstream, 2 = downstream, 3 = nearest. See `equations.md` §H1.
   */
  densify_reach_modifier_policy?: number;
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

/** Non-fatal issues from `validateSteadyInputs` (parse errors still throw). */
export interface SteadyValidationResult {
  warnings: string[];
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
  /** Tier 2a — inlet-control headwater per culvert */
  culvert_wsel_inlet?: number[];
  culvert_wsel_outlet?: number[];
  culvert_q_barrels?: number[];
  culvert_q_weirs?: number[];
  culvert_barrel_depths?: number[];
  culvert_barrel_velocities?: number[];
  culvert_barrel_froude?: number[];
}

/** Inputs for `computeCulvertRatingCurve` — `q` in culvert fields is ignored. */
export interface CulvertRatingCurveInputs {
  q_values: number[];
  q?: number;
  tw_wsel: number;
  units: UnitSystem;
  shape_type: number;
  inlet_type?: number;
  span: number;
  rise: number;
  roughness_n: number;
  length: number;
  entrance_loss_coeff: number;
  exit_loss_coeff: number;
  z_down: number;
  z_up: number;
  manning_n_bottom?: number;
  depth_bottom_n?: number;
  depth_blocked?: number;
  ds_velocity?: number;
  us_velocity?: number;
  crest_elev?: number;
  weir_coeff?: number;
  weir_length?: number;
  num_barrels?: number;
  active_barrels?: number;
  skew_deg?: number;
  barrel_spans?: number[];
  barrel_rises?: number[];
}

export interface CulvertRatingCurveResult {
  q: number[];
  wsel: number[];
  control_types: CulvertControlType[];
  wsel_inlet: number[];
  wsel_outlet: number[];
  q_barrel: number[];
  q_weir: number[];
  barrel_depth: number[];
  barrel_velocity: number[];
  barrel_froude: number[];
}

/**
 * Inputs for `computeBridgeRatingCurve` — `q` is ignored; fields mirror standalone bridge solve params.
 * **Reverse flow (v31):** negative `q_values` supported; `tw_wsel_reverse` for BU TW when `Q < 0`.
 * `Q = 0` samples are skipped. Direction is not inferred from stages. See `bridge_reverse_flow_rating.md`.
 * **Ice / debris (v32):** `opening_blockage_factor`, `pier_debris_widths`, `pier_debris_heights`,
 * `ice_thickness`, `ice_mode`, `deck_ice_thickness` — see `bridge_ice_debris.md`.
 */
export interface BridgeRatingCurveInputs {
  q_values: number[];
  q?: number;
  low_chord: number;
  high_chord: number;
  z_down: number;
  z_up: number;
  /** Tailwater at BD when `q_values > 0` (user units). */
  tw_wsel: number;
  /** Tailwater at BU when `q_values < 0`. Omit to reuse `tw_wsel`. */
  tw_wsel_reverse?: number;
  units: UnitSystem;
  pier_width?: number;
  num_piers?: number;
  /** Same codes as `bridge_pier_shapes` (0–11, API v29). */
  pier_shape_type?: number;
  weir_coeff?: number;
  orifice_coeff?: number;
  abutment_block_width?: number;
  abutment_left_width?: number;
  abutment_right_width?: number;
  abutment_left_station?: number;
  abutment_right_station?: number;
  abutment_left_top_elevation?: number;
  abutment_right_top_elevation?: number;
  abutment_left_top_profile_stations?: number[];
  abutment_left_top_profile_elevations?: number[];
  abutment_right_top_profile_stations?: number[];
  abutment_right_top_profile_elevations?: number[];
  low_flow_method?: number;
  /** High-flow method: 0 = pressure/weir, 1 = energy. */
  high_flow_method?: number;
  length?: number;
  /**
   * Friction weighting for `computeBridgeRatingCurve`: omit/`0` = opening only (HEC-RAS default),
   * `1` = approach + opening + departure segments.
   */
  friction_weighting?: number;
  /** Approach segment length override (user units). `0` = auto from approach/BU river stations. */
  approach_friction_length?: number;
  /** Departure segment length override (user units). `0` = auto from BD/departure river stations. */
  departure_friction_length?: number;
  /** Net opening area multiplier (0–1]. Omit or `1.0` = no extra blockage. */
  opening_blockage_factor?: number;
  pier_debris_widths?: number[];
  pier_debris_heights?: number[];
  ice_thickness?: number;
  /** `0` = none, `1` = constant thickness, `2` = reserved. */
  ice_mode?: number;
  deck_ice_thickness?: number;
  wspro_coeff?: number;
  coeff_contraction?: number;
  coeff_expansion?: number;
  pressure_coeff_inlet?: number;
  max_weir_submergence?: number;
  skew_deg?: number;
  pier_stations?: number[];
  /** Tapered pier widths per pier (rating curve / standalone solve; no `bridge_` prefix). */
  pier_top_widths?: number[];
  pier_bottom_widths?: number[];
  pier_width_elevations?: number[][];
  pier_width_values?: number[][];
  pier_top_elevations?: number[];
  pier_base_elevations?: number[];
  pier_footing_top_elevations?: number[];
  pier_footing_widths?: number[];
  pier_footing_bottom_elevations?: number[];
  pier_nosing_lengths?: number[];
  pier_nosing_widths?: number[];
  deck_stations?: number[];
  deck_low_elevations?: number[];
  deck_high_elevations?: number[];
  ineffective_left_station?: number;
  ineffective_left_elevation?: number;
  ineffective_right_station?: number;
  ineffective_right_elevation?: number;
  /** Multi-block left ineffective stations (falls back to scalar `ineffective_left_station`). */
  ineffective_left_stations?: number[];
  ineffective_left_elevations?: number[];
  ineffective_right_stations?: number[];
  ineffective_right_elevations?: number[];
  ineffective_left_station_upstream?: number;
  ineffective_left_elevation_upstream?: number;
  ineffective_right_station_upstream?: number;
  ineffective_right_elevation_upstream?: number;
  ineffective_left_stations_upstream?: number[];
  ineffective_left_elevations_upstream?: number[];
  ineffective_right_stations_upstream?: number[];
  ineffective_right_elevations_upstream?: number[];
  ineffective_left_station_downstream?: number;
  ineffective_left_elevation_downstream?: number;
  ineffective_right_station_downstream?: number;
  ineffective_right_elevation_downstream?: number;
  ineffective_left_stations_downstream?: number[];
  ineffective_left_elevations_downstream?: number[];
  ineffective_right_stations_downstream?: number[];
  ineffective_right_elevations_downstream?: number[];
  channel_width?: number;
  manning_n?: number;
  num_slices?: number;
  xs_up?: CrossSection;
  xs_down?: CrossSection;
  /** Reach XS lateral `x` at bridge opening station 0. */
  opening_reach_station_origin?: number;
  /** Optional interior bridge cuts (US → DS); stored for future multi-segment hydraulics. */
  xs_internal?: CrossSection[];
  /** Unified roadway embankment (API v26). See `equations.md` §G2. */
  roadway_embankment?: BridgeRoadwayEmbankment;
}

export interface BridgeRatingCurveResult {
  q: number[];
  /** Upstream headwater (same role as culvert rating `wsel`). */
  wsel: number[];
  wsel_down: number[];
  flow_regimes: BridgeFlowRegime[];
  head_losses: number[];
}

/** Culvert fields accepted by `solveUnsteady` (same keys as `SteadyInputs`, API v7+). */
export type UnsteadyCulvertInputs = Partial<
  Pick<
    SteadyInputs,
    | 'culvert_stations'
    | 'culvert_shape_types'
    | 'culvert_spans'
    | 'culvert_rises'
    | 'culvert_roughness_ns'
    | 'culvert_lengths'
    | 'culvert_entrance_loss_coeffs'
    | 'culvert_exit_loss_coeffs'
    | 'culvert_barrels'
    | 'culvert_roughness_n_bottoms'
    | 'culvert_depth_bottom_ns'
    | 'culvert_depth_blockeds'
    | 'culvert_inlet_types'
    | 'culvert_z_ups'
    | 'culvert_z_downs'
    | 'culvert_crest_elevs'
    | 'culvert_weir_coeffs'
    | 'culvert_weir_lengths'
    | 'culvert_skew_angles'
    | 'culvert_active_barrels'
    | 'culvert_barrel_spans'
    | 'culvert_barrel_rises'
  >
>;

export interface UnsteadyInputs extends UnsteadyCulvertInputs, BridgeArrays {
  cross_sections: CrossSection[];
  initial_wsel: number[];
  initial_q: number[];
  dt: number;
  num_steps: number;
  upstream_q_hydrograph: number[];
  downstream_wsel_hydrograph: number[];
  /** 0 = known WSEL hydrograph (default), 1 = critical depth, 2 = friction slope, 3 = rating curve */
  downstream_bc_type?: number;
  downstream_bc_slope?: number;
  downstream_bc_rating_q?: number[];
  downstream_bc_rating_wsel?: number[];
  /** Reserved — upstream Q(t) remains default when omitted */
  upstream_wsel_hydrograph?: number[];
  upstream_bc_type?: number;
  upstream_bc_slope?: number;
  upstream_bc_rating_q?: number[];
  upstream_bc_rating_wsel?: number[];
  theta?: number;
  num_slices?: number;
  max_spacing?: number;
  /** Same as `SteadyInputs.densify_reach_modifier_policy` — `equations.md` §H1. */
  densify_reach_modifier_policy?: number;
  coeff_contraction?: number;
  coeff_expansion?: number;
  /**
   * Inline structure post-step coupling order when culverts and bridges are both present:
   * 0 = combined downstream-first (default), 1 = culverts then bridges, 2 = bridges then culverts.
   */
  structure_coupling_order?: number;
  /**
   * Preissmann structure coupling (API v33+): 0 = post-step only (default),
   * 1 = reserved (reach–structure–reach outer loop, not implemented),
   * 2 = hybrid implicit + explicit fallback where needed.
   */
  unsteady_structure_coupling_mode?: number;
}

export interface UnsteadyResult {
  wsel: number[][];
  q: number[][];
  velocity: number[][];
  max_courant?: number;
  recommended_dt?: number;
  /** Present when culverts are modeled — [time_step][culvert_index] */
  culvert_control_types?: CulvertControlType[][];
  culvert_wsel_inlet?: number[][];
  culvert_wsel_outlet?: number[][];
  culvert_q_barrels?: number[][];
  culvert_q_weirs?: number[][];
  culvert_barrel_depths?: number[][];
  culvert_barrel_velocities?: number[][];
  culvert_barrel_froude?: number[][];
  /** Present when bridges are modeled — [time_step][bridge_index] */
  bridge_flow_regimes?: BridgeFlowRegime[][];
  bridge_wsel_upstream?: number[][];
  bridge_wsel_downstream?: number[][];
  bridge_head_losses?: number[][];
  /** Present when inline structures are modeled — one value per time step (API v34). */
  structure_coupling_converged?: boolean[];
  structure_implicit_interval_count?: number[];
  structure_explicit_fallback_count?: number[];
}

export type BridgeFlowRegime = 'low_a' | 'low_b' | 'low_c' | 'pressure' | 'weir' | 'energy';

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
  culvert_tier2a_fields: {
    steady_outputs: string[];
    unsteady_outputs: string[];
    rating_curve_entry_point: string;
  };
  culvert_geometry_fields: {
    inputs: string[];
  };
  bridge_fields: {
    inputs: string[];
    unsteady_outputs: string[];
    flow_regimes: BridgeFlowRegime[];
    rating_curve_entry_point: string;
    /** Flattened keys for `computeBridgeRatingCurve` (not `bridge_*` prefixed). */
    rating_curve_inputs: string[];
    rating_curve_outputs: string[];
  };
  structure_coupling_orders: WasmEnumEntry[];
  /** API v33+ — Preissmann structure coupling mode (`unsteady_structure_coupling_mode`). */
  unsteady_structure_coupling_modes: WasmEnumEntry[];
  /** API v34 — per-step structure coupling diagnostics on `UnsteadyResult`. */
  unsteady_structure_coupling_outputs: string[];
}

/** Module exports from `pkg/stream1d.js` after `wasm-pack build --target web` */
export interface Streams1dWasmModule {
  default: (url?: string | URL) => Promise<unknown>;
  getEngineVersion: () => string;
  getWasmApiMetadata: () => WasmApiMetadata;
  validateSteadyInputs: (inputs: SteadyInputs) => SteadyValidationResult;
  solveSteady: (inputs: SteadyInputs) => SteadyResult;
  solveUnsteady: (inputs: UnsteadyInputs) => UnsteadyResult;
  computeCulvertRatingCurve: (inputs: CulvertRatingCurveInputs) => CulvertRatingCurveResult;
  computeBridgeRatingCurve: (inputs: BridgeRatingCurveInputs) => BridgeRatingCurveResult;
}
