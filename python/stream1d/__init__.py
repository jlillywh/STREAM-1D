import json
from typing import List, Dict, Optional, Any, Union

# Flat [s0, s1] = one block per bridge; nested [[s0, s1], [s2]] = multi-block per bridge.
BridgeIneffectiveBlocks = Union[List[float], List[List[float]]]

# Culvert shape codes (match Rust/WASM `culvert_shape_types`)
CULVERT_SHAPE_CIRCULAR = 0
CULVERT_SHAPE_BOX = 1
CULVERT_SHAPE_ARCH = 2
CULVERT_SHAPE_CONSPAN = 3
CULVERT_SHAPE_PIPE_ARCH = 4
CULVERT_SHAPE_ELLIPTICAL = 5
CULVERT_SHAPE_HORSESHOE = 6

# Import native binary solver
try:
    from . import _stream1d
except ImportError:
    # Handle direct local importing during development/testing
    import _stream1d

class CrossSection:
    """Reach or bridge-face polyline. Optional `ineffective_flow_areas` uses reach lateral `x` (API v22 on BU/BD cuts)."""

    def __init__(
        self,
        station: float,
        x: List[float],
        y: List[float],
        n_stations: List[float],
        n_values: List[float],
        unit_system: str = "Metric",
        is_overbank: Optional[List[bool]] = None,
        blocked_obstructions: Optional[List[Dict[str, List[float]]]] = None,
        ineffective_flow_areas: Optional[Dict[str, List[Dict[str, float]]]] = None,
    ):
        self.station = station
        self.x = x
        self.y = y
        self.n_stations = n_stations
        self.n_values = n_values
        self.unit_system = unit_system
        self.is_overbank = is_overbank
        self.blocked_obstructions = blocked_obstructions
        self.ineffective_flow_areas = ineffective_flow_areas

    def to_dict(self) -> dict:
        res = {
            'station': self.station,
            'x': self.x,
            'y': self.y,
            'n_stations': self.n_stations,
            'n_values': self.n_values,
            'unit_system': self.unit_system
        }
        if self.is_overbank is not None:
            res['is_overbank'] = self.is_overbank
        if self.blocked_obstructions is not None:
            res['blocked_obstructions'] = self.blocked_obstructions
        if self.ineffective_flow_areas is not None:
            res['ineffective_flow_areas'] = self.ineffective_flow_areas
        return res

class SteadyInputs:
    def __init__(
        self,
        cross_sections: List[CrossSection],
        flow_rate: float,
        num_slices: Optional[int] = 100,
        coeff_contraction: Optional[float] = 0.1,
        coeff_expansion: Optional[float] = 0.3,
        regime: int = 0, # 0 = Subcritical, 1 = Supercritical, 2 = Mixed
        downstream_wsel: Optional[float] = None,
        upstream_wsel: Optional[float] = None,
        max_spacing: Optional[float] = None,
        # Culvert parameters
        culvert_stations: Optional[List[float]] = None,
        culvert_shape_types: Optional[List[int]] = None,
        culvert_spans: Optional[List[float]] = None,
        culvert_rises: Optional[List[float]] = None,
        culvert_roughness_ns: Optional[List[float]] = None,
        culvert_lengths: Optional[List[float]] = None,
        culvert_entrance_loss_coeffs: Optional[List[float]] = None,
        culvert_exit_loss_coeffs: Optional[List[float]] = None,
        culvert_barrels: Optional[List[int]] = None,
        culvert_roughness_n_bottoms: Optional[List[float]] = None,
        culvert_depth_bottom_ns: Optional[List[float]] = None,
        culvert_depth_blockeds: Optional[List[float]] = None,
        culvert_inlet_types: Optional[List[int]] = None,
        culvert_z_ups: Optional[List[float]] = None,
        culvert_z_downs: Optional[List[float]] = None,
        culvert_crest_elevs: Optional[List[float]] = None,
        culvert_weir_coeffs: Optional[List[float]] = None,
        culvert_weir_lengths: Optional[List[float]] = None,
        culvert_skew_angles: Optional[List[float]] = None,
        culvert_active_barrels: Optional[List[int]] = None,
        culvert_barrel_spans: Optional[List[List[float]]] = None,
        culvert_barrel_rises: Optional[List[List[float]]] = None,
        # Bridge parameters
        bridge_stations: Optional[List[float]] = None,
        bridge_low_chords: Optional[List[float]] = None,
        bridge_high_chords: Optional[List[float]] = None,
        bridge_pier_widths: Optional[List[float]] = None,
        bridge_num_piers: Optional[List[int]] = None,
        bridge_pier_shapes: Optional[List[int]] = None,
        bridge_weir_coeffs: Optional[List[float]] = None,
        bridge_orifice_coeffs: Optional[List[float]] = None,
        bridge_abutment_block_widths: Optional[List[float]] = None,
        bridge_abutment_left_widths: Optional[List[float]] = None,
        bridge_abutment_right_widths: Optional[List[float]] = None,
        bridge_abutment_left_stations: Optional[List[float]] = None,
        bridge_abutment_right_stations: Optional[List[float]] = None,
        bridge_abutment_left_top_elevations: Optional[List[float]] = None,
        bridge_abutment_right_top_elevations: Optional[List[float]] = None,
        bridge_abutment_left_top_profile_stations: Optional[List[List[float]]] = None,
        bridge_abutment_left_top_profile_elevations: Optional[List[List[float]]] = None,
        bridge_abutment_right_top_profile_stations: Optional[List[List[float]]] = None,
        bridge_abutment_right_top_profile_elevations: Optional[List[List[float]]] = None,
        bridge_low_flow_methods: Optional[List[int]] = None,
        bridge_high_flow_methods: Optional[List[int]] = None,
        bridge_lengths: Optional[List[float]] = None,
        bridge_friction_weighting: Optional[List[int]] = None,
        bridge_approach_friction_lengths: Optional[List[float]] = None,
        bridge_departure_friction_lengths: Optional[List[float]] = None,
        bridge_wspro_coeffs: Optional[List[float]] = None,
        bridge_pressure_flow_coeffs_inlet: Optional[List[float]] = None,
        bridge_max_weir_submergence: Optional[List[float]] = None,
        bridge_deck_stations: Optional[List[List[float]]] = None,
        bridge_deck_low_elevations: Optional[List[List[float]]] = None,
        bridge_deck_high_elevations: Optional[List[List[float]]] = None,
        bridge_ineffective_left_stations: Optional[BridgeIneffectiveBlocks] = None,
        bridge_ineffective_left_elevations: Optional[BridgeIneffectiveBlocks] = None,
        bridge_ineffective_right_stations: Optional[BridgeIneffectiveBlocks] = None,
        bridge_ineffective_right_elevations: Optional[BridgeIneffectiveBlocks] = None,
        bridge_ineffective_left_stations_upstream: Optional[BridgeIneffectiveBlocks] = None,
        bridge_ineffective_left_elevations_upstream: Optional[BridgeIneffectiveBlocks] = None,
        bridge_ineffective_right_stations_upstream: Optional[BridgeIneffectiveBlocks] = None,
        bridge_ineffective_right_elevations_upstream: Optional[BridgeIneffectiveBlocks] = None,
        bridge_ineffective_left_stations_downstream: Optional[BridgeIneffectiveBlocks] = None,
        bridge_ineffective_left_elevations_downstream: Optional[BridgeIneffectiveBlocks] = None,
        bridge_ineffective_right_stations_downstream: Optional[BridgeIneffectiveBlocks] = None,
        bridge_ineffective_right_elevations_downstream: Optional[BridgeIneffectiveBlocks] = None,
        bridge_skew_angles: Optional[List[float]] = None,
        bridge_pier_stations: Optional[List[List[float]]] = None,
        bridge_pier_top_widths: Optional[List[List[float]]] = None,
        bridge_pier_bottom_widths: Optional[List[List[float]]] = None,
        bridge_pier_width_elevations: Optional[List[List[List[float]]]] = None,
        bridge_pier_width_values: Optional[List[List[List[float]]]] = None,
        bridge_pier_top_elevations: Optional[List[List[float]]] = None,
        bridge_pier_base_elevations: Optional[List[List[float]]] = None,
        bridge_upstream_cross_sections: Optional[List[CrossSection]] = None,
        bridge_downstream_cross_sections: Optional[List[CrossSection]] = None,
        bridge_internal_cross_sections: Optional[List[List[CrossSection]]] = None,
        bridge_opening_reach_station_origins: Optional[List[float]] = None,
        bridge_opening_anchor_modes: Optional[List[int]] = None,
        bridge_opening_anchor_reach_stations: Optional[List[float]] = None,
        # Boundary conditions
        downstream_bc_type: Optional[int] = None,
        downstream_bc_slope: Optional[float] = None,
        downstream_bc_rating_q: Optional[List[float]] = None,
        downstream_bc_rating_wsel: Optional[List[float]] = None,
        upstream_bc_type: Optional[int] = None,
        upstream_bc_slope: Optional[float] = None,
        upstream_bc_rating_q: Optional[List[float]] = None,
        upstream_bc_rating_wsel: Optional[List[float]] = None,
        # Tributary junction (steady subcritical)
        tributary_cross_sections: Optional[List[CrossSection]] = None,
        tributary_flow_rate: Optional[float] = None,
        junction_main_station: Optional[float] = None,
    ):
        self.cross_sections = cross_sections
        self.flow_rate = flow_rate
        self.num_slices = num_slices
        self.coeff_contraction = coeff_contraction
        self.coeff_expansion = coeff_expansion
        self.regime = regime
        self.downstream_wsel = downstream_wsel
        self.upstream_wsel = upstream_wsel
        self.max_spacing = max_spacing
        self.culvert_stations = culvert_stations or []
        self.culvert_shape_types = culvert_shape_types or []
        self.culvert_spans = culvert_spans or []
        self.culvert_rises = culvert_rises or []
        self.culvert_roughness_ns = culvert_roughness_ns or []
        self.culvert_lengths = culvert_lengths or []
        self.culvert_entrance_loss_coeffs = culvert_entrance_loss_coeffs or []
        self.culvert_exit_loss_coeffs = culvert_exit_loss_coeffs or []
        self.culvert_barrels = culvert_barrels or []
        self.culvert_roughness_n_bottoms = culvert_roughness_n_bottoms or []
        self.culvert_depth_bottom_ns = culvert_depth_bottom_ns or []
        self.culvert_depth_blockeds = culvert_depth_blockeds or []
        self.culvert_inlet_types = culvert_inlet_types or []
        self.culvert_z_ups = culvert_z_ups or []
        self.culvert_z_downs = culvert_z_downs or []
        self.culvert_crest_elevs = culvert_crest_elevs or []
        self.culvert_weir_coeffs = culvert_weir_coeffs or []
        self.culvert_weir_lengths = culvert_weir_lengths or []
        self.culvert_skew_angles = culvert_skew_angles or []
        self.culvert_active_barrels = culvert_active_barrels or []
        self.culvert_barrel_spans = culvert_barrel_spans or []
        self.culvert_barrel_rises = culvert_barrel_rises or []
        self.bridge_stations = bridge_stations or []
        self.bridge_low_chords = bridge_low_chords or []
        self.bridge_high_chords = bridge_high_chords or []
        self.bridge_pier_widths = bridge_pier_widths or []
        self.bridge_num_piers = bridge_num_piers or []
        self.bridge_pier_shapes = bridge_pier_shapes or []
        self.bridge_weir_coeffs = bridge_weir_coeffs or []
        self.bridge_orifice_coeffs = bridge_orifice_coeffs or []
        self.bridge_abutment_block_widths = bridge_abutment_block_widths or []
        self.bridge_abutment_left_widths = bridge_abutment_left_widths or []
        self.bridge_abutment_right_widths = bridge_abutment_right_widths or []
        self.bridge_abutment_left_stations = bridge_abutment_left_stations or []
        self.bridge_abutment_right_stations = bridge_abutment_right_stations or []
        self.bridge_abutment_left_top_elevations = bridge_abutment_left_top_elevations or []
        self.bridge_abutment_right_top_elevations = bridge_abutment_right_top_elevations or []
        self.bridge_abutment_left_top_profile_stations = bridge_abutment_left_top_profile_stations or []
        self.bridge_abutment_left_top_profile_elevations = bridge_abutment_left_top_profile_elevations or []
        self.bridge_abutment_right_top_profile_stations = bridge_abutment_right_top_profile_stations or []
        self.bridge_abutment_right_top_profile_elevations = bridge_abutment_right_top_profile_elevations or []
        self.bridge_low_flow_methods = bridge_low_flow_methods or []
        self.bridge_high_flow_methods = bridge_high_flow_methods or []
        self.bridge_lengths = bridge_lengths or []
        self.bridge_friction_weighting = bridge_friction_weighting or []
        self.bridge_approach_friction_lengths = bridge_approach_friction_lengths or []
        self.bridge_departure_friction_lengths = bridge_departure_friction_lengths or []
        self.bridge_wspro_coeffs = bridge_wspro_coeffs or []
        self.bridge_pressure_flow_coeffs_inlet = bridge_pressure_flow_coeffs_inlet or []
        self.bridge_max_weir_submergence = bridge_max_weir_submergence or []
        self.bridge_deck_stations = bridge_deck_stations or []
        self.bridge_deck_low_elevations = bridge_deck_low_elevations or []
        self.bridge_deck_high_elevations = bridge_deck_high_elevations or []
        self.bridge_ineffective_left_stations = bridge_ineffective_left_stations or []
        self.bridge_ineffective_left_elevations = bridge_ineffective_left_elevations or []
        self.bridge_ineffective_right_stations = bridge_ineffective_right_stations or []
        self.bridge_ineffective_right_elevations = bridge_ineffective_right_elevations or []
        self.bridge_ineffective_left_stations_upstream = bridge_ineffective_left_stations_upstream or []
        self.bridge_ineffective_left_elevations_upstream = bridge_ineffective_left_elevations_upstream or []
        self.bridge_ineffective_right_stations_upstream = bridge_ineffective_right_stations_upstream or []
        self.bridge_ineffective_right_elevations_upstream = bridge_ineffective_right_elevations_upstream or []
        self.bridge_ineffective_left_stations_downstream = bridge_ineffective_left_stations_downstream or []
        self.bridge_ineffective_left_elevations_downstream = bridge_ineffective_left_elevations_downstream or []
        self.bridge_ineffective_right_stations_downstream = bridge_ineffective_right_stations_downstream or []
        self.bridge_ineffective_right_elevations_downstream = bridge_ineffective_right_elevations_downstream or []
        self.bridge_skew_angles = bridge_skew_angles or []
        self.bridge_pier_stations = bridge_pier_stations or []
        self.bridge_pier_top_widths = bridge_pier_top_widths or []
        self.bridge_pier_bottom_widths = bridge_pier_bottom_widths or []
        self.bridge_pier_width_elevations = bridge_pier_width_elevations or []
        self.bridge_pier_width_values = bridge_pier_width_values or []
        self.bridge_pier_top_elevations = bridge_pier_top_elevations or []
        self.bridge_pier_base_elevations = bridge_pier_base_elevations or []
        self.bridge_upstream_cross_sections = bridge_upstream_cross_sections or []
        self.bridge_downstream_cross_sections = bridge_downstream_cross_sections or []
        self.bridge_internal_cross_sections = bridge_internal_cross_sections or []
        self.bridge_opening_reach_station_origins = bridge_opening_reach_station_origins or []
        self.bridge_opening_anchor_modes = bridge_opening_anchor_modes or []
        self.bridge_opening_anchor_reach_stations = bridge_opening_anchor_reach_stations or []
        self.downstream_bc_type = downstream_bc_type
        self.downstream_bc_slope = downstream_bc_slope
        self.downstream_bc_rating_q = downstream_bc_rating_q
        self.downstream_bc_rating_wsel = downstream_bc_rating_wsel
        self.upstream_bc_type = upstream_bc_type
        self.upstream_bc_slope = upstream_bc_slope
        self.upstream_bc_rating_q = upstream_bc_rating_q
        self.upstream_bc_rating_wsel = upstream_bc_rating_wsel
        self.tributary_cross_sections = tributary_cross_sections
        self.tributary_flow_rate = tributary_flow_rate
        self.junction_main_station = junction_main_station

    def to_dict(self) -> dict:
        res = {
            'cross_sections': [xs.to_dict() for xs in self.cross_sections],
            'flow_rate': self.flow_rate,
            'num_slices': self.num_slices,
            'coeff_contraction': self.coeff_contraction,
            'coeff_expansion': self.coeff_expansion,
            'regime': self.regime,
            'downstream_wsel': self.downstream_wsel,
            'upstream_wsel': self.upstream_wsel,
            'max_spacing': self.max_spacing,
            'culvert_stations': self.culvert_stations,
            'culvert_shape_types': self.culvert_shape_types,
            'culvert_spans': self.culvert_spans,
            'culvert_rises': self.culvert_rises,
            'culvert_roughness_ns': self.culvert_roughness_ns,
            'culvert_lengths': self.culvert_lengths,
            'culvert_entrance_loss_coeffs': self.culvert_entrance_loss_coeffs,
            'culvert_exit_loss_coeffs': self.culvert_exit_loss_coeffs,
            'culvert_barrels': self.culvert_barrels,
            'culvert_roughness_n_bottoms': self.culvert_roughness_n_bottoms,
            'culvert_depth_bottom_ns': self.culvert_depth_bottom_ns,
            'culvert_depth_blockeds': self.culvert_depth_blockeds,
            'culvert_inlet_types': self.culvert_inlet_types,
            'culvert_z_ups': self.culvert_z_ups,
            'culvert_z_downs': self.culvert_z_downs,
            'culvert_crest_elevs': self.culvert_crest_elevs,
            'culvert_weir_coeffs': self.culvert_weir_coeffs,
            'culvert_weir_lengths': self.culvert_weir_lengths,
            'culvert_skew_angles': self.culvert_skew_angles,
            'culvert_active_barrels': self.culvert_active_barrels,
            'culvert_barrel_spans': self.culvert_barrel_spans,
            'culvert_barrel_rises': self.culvert_barrel_rises,
            'bridge_stations': self.bridge_stations,
            'bridge_low_chords': self.bridge_low_chords,
            'bridge_high_chords': self.bridge_high_chords,
            'bridge_pier_widths': self.bridge_pier_widths,
            'bridge_num_piers': self.bridge_num_piers,
            'bridge_pier_shapes': self.bridge_pier_shapes,
            'bridge_weir_coeffs': self.bridge_weir_coeffs,
            'bridge_orifice_coeffs': self.bridge_orifice_coeffs,
            'bridge_abutment_block_widths': self.bridge_abutment_block_widths,
            'bridge_abutment_left_widths': self.bridge_abutment_left_widths,
            'bridge_abutment_right_widths': self.bridge_abutment_right_widths,
            'bridge_abutment_left_stations': self.bridge_abutment_left_stations,
            'bridge_abutment_right_stations': self.bridge_abutment_right_stations,
            'bridge_abutment_left_top_elevations': self.bridge_abutment_left_top_elevations,
            'bridge_abutment_right_top_elevations': self.bridge_abutment_right_top_elevations,
            'bridge_abutment_left_top_profile_stations': self.bridge_abutment_left_top_profile_stations,
            'bridge_abutment_left_top_profile_elevations': self.bridge_abutment_left_top_profile_elevations,
            'bridge_abutment_right_top_profile_stations': self.bridge_abutment_right_top_profile_stations,
            'bridge_abutment_right_top_profile_elevations': self.bridge_abutment_right_top_profile_elevations,
            'bridge_low_flow_methods': self.bridge_low_flow_methods,
            'bridge_high_flow_methods': self.bridge_high_flow_methods,
            'bridge_lengths': self.bridge_lengths,
            'bridge_friction_weighting': self.bridge_friction_weighting,
            'bridge_approach_friction_lengths': self.bridge_approach_friction_lengths,
            'bridge_departure_friction_lengths': self.bridge_departure_friction_lengths,
            'bridge_wspro_coeffs': self.bridge_wspro_coeffs,
            'bridge_pressure_flow_coeffs_inlet': self.bridge_pressure_flow_coeffs_inlet,
            'bridge_max_weir_submergence': self.bridge_max_weir_submergence,
            'bridge_deck_stations': self.bridge_deck_stations,
            'bridge_deck_low_elevations': self.bridge_deck_low_elevations,
            'bridge_deck_high_elevations': self.bridge_deck_high_elevations,
            'bridge_ineffective_left_stations': self.bridge_ineffective_left_stations,
            'bridge_ineffective_left_elevations': self.bridge_ineffective_left_elevations,
            'bridge_ineffective_right_stations': self.bridge_ineffective_right_stations,
            'bridge_ineffective_right_elevations': self.bridge_ineffective_right_elevations,
            'bridge_ineffective_left_stations_upstream': self.bridge_ineffective_left_stations_upstream,
            'bridge_ineffective_left_elevations_upstream': self.bridge_ineffective_left_elevations_upstream,
            'bridge_ineffective_right_stations_upstream': self.bridge_ineffective_right_stations_upstream,
            'bridge_ineffective_right_elevations_upstream': self.bridge_ineffective_right_elevations_upstream,
            'bridge_ineffective_left_stations_downstream': self.bridge_ineffective_left_stations_downstream,
            'bridge_ineffective_left_elevations_downstream': self.bridge_ineffective_left_elevations_downstream,
            'bridge_ineffective_right_stations_downstream': self.bridge_ineffective_right_stations_downstream,
            'bridge_ineffective_right_elevations_downstream': self.bridge_ineffective_right_elevations_downstream,
            'bridge_skew_angles': self.bridge_skew_angles,
            'bridge_pier_stations': self.bridge_pier_stations,
            'bridge_pier_top_widths': self.bridge_pier_top_widths,
            'bridge_pier_bottom_widths': self.bridge_pier_bottom_widths,
            'bridge_pier_width_elevations': self.bridge_pier_width_elevations,
            'bridge_pier_width_values': self.bridge_pier_width_values,
            'bridge_pier_top_elevations': self.bridge_pier_top_elevations,
            'bridge_pier_base_elevations': self.bridge_pier_base_elevations,
            'bridge_upstream_cross_sections': self.bridge_upstream_cross_sections,
            'bridge_downstream_cross_sections': self.bridge_downstream_cross_sections,
            'bridge_internal_cross_sections': self.bridge_internal_cross_sections,
            'bridge_opening_reach_station_origins': self.bridge_opening_reach_station_origins,
            'bridge_opening_anchor_modes': self.bridge_opening_anchor_modes,
            'bridge_opening_anchor_reach_stations': self.bridge_opening_anchor_reach_stations,
        }
        if self.downstream_bc_type is not None:
            res['downstream_bc_type'] = self.downstream_bc_type
        if self.downstream_bc_slope is not None:
            res['downstream_bc_slope'] = self.downstream_bc_slope
        if self.downstream_bc_rating_q is not None:
            res['downstream_bc_rating_q'] = self.downstream_bc_rating_q
        if self.downstream_bc_rating_wsel is not None:
            res['downstream_bc_rating_wsel'] = self.downstream_bc_rating_wsel
        if self.upstream_bc_type is not None:
            res['upstream_bc_type'] = self.upstream_bc_type
        if self.upstream_bc_slope is not None:
            res['upstream_bc_slope'] = self.upstream_bc_slope
        if self.upstream_bc_rating_q is not None:
            res['upstream_bc_rating_q'] = self.upstream_bc_rating_q
        if self.upstream_bc_rating_wsel is not None:
            res['upstream_bc_rating_wsel'] = self.upstream_bc_rating_wsel
        if self.tributary_cross_sections is not None:
            res['tributary_cross_sections'] = [xs.to_dict() for xs in self.tributary_cross_sections]
        if self.tributary_flow_rate is not None:
            res['tributary_flow_rate'] = self.tributary_flow_rate
        if self.junction_main_station is not None:
            res['junction_main_station'] = self.junction_main_station
        return res

class UnsteadyInputs:
    def __init__(
        self,
        cross_sections: List[CrossSection],
        initial_wsel: List[float],
        initial_q: List[float],
        dt: float,
        num_steps: int,
        upstream_q_hydrograph: List[float],
        downstream_wsel_hydrograph: List[float],
        theta: Optional[float] = 0.6,
        num_slices: Optional[int] = 100,
        max_spacing: Optional[float] = None,
        coeff_contraction: Optional[float] = 0.1,
        coeff_expansion: Optional[float] = 0.3,
        culvert_stations: Optional[List[float]] = None,
        culvert_shape_types: Optional[List[int]] = None,
        culvert_spans: Optional[List[float]] = None,
        culvert_rises: Optional[List[float]] = None,
        culvert_roughness_ns: Optional[List[float]] = None,
        culvert_lengths: Optional[List[float]] = None,
        culvert_entrance_loss_coeffs: Optional[List[float]] = None,
        culvert_exit_loss_coeffs: Optional[List[float]] = None,
        culvert_barrels: Optional[List[int]] = None,
        culvert_roughness_n_bottoms: Optional[List[float]] = None,
        culvert_depth_bottom_ns: Optional[List[float]] = None,
        culvert_depth_blockeds: Optional[List[float]] = None,
        culvert_inlet_types: Optional[List[int]] = None,
        culvert_z_ups: Optional[List[float]] = None,
        culvert_z_downs: Optional[List[float]] = None,
        culvert_crest_elevs: Optional[List[float]] = None,
        culvert_weir_coeffs: Optional[List[float]] = None,
        culvert_weir_lengths: Optional[List[float]] = None,
        culvert_skew_angles: Optional[List[float]] = None,
        culvert_active_barrels: Optional[List[int]] = None,
        culvert_barrel_spans: Optional[List[List[float]]] = None,
        culvert_barrel_rises: Optional[List[List[float]]] = None,
        bridge_stations: Optional[List[float]] = None,
        bridge_low_chords: Optional[List[float]] = None,
        bridge_high_chords: Optional[List[float]] = None,
        bridge_pier_widths: Optional[List[float]] = None,
        bridge_num_piers: Optional[List[int]] = None,
        bridge_pier_shapes: Optional[List[int]] = None,
        bridge_weir_coeffs: Optional[List[float]] = None,
        bridge_orifice_coeffs: Optional[List[float]] = None,
        bridge_abutment_block_widths: Optional[List[float]] = None,
        bridge_abutment_left_widths: Optional[List[float]] = None,
        bridge_abutment_right_widths: Optional[List[float]] = None,
        bridge_abutment_left_stations: Optional[List[float]] = None,
        bridge_abutment_right_stations: Optional[List[float]] = None,
        bridge_abutment_left_top_elevations: Optional[List[float]] = None,
        bridge_abutment_right_top_elevations: Optional[List[float]] = None,
        bridge_abutment_left_top_profile_stations: Optional[List[List[float]]] = None,
        bridge_abutment_left_top_profile_elevations: Optional[List[List[float]]] = None,
        bridge_abutment_right_top_profile_stations: Optional[List[List[float]]] = None,
        bridge_abutment_right_top_profile_elevations: Optional[List[List[float]]] = None,
        bridge_low_flow_methods: Optional[List[int]] = None,
        bridge_high_flow_methods: Optional[List[int]] = None,
        bridge_lengths: Optional[List[float]] = None,
        bridge_friction_weighting: Optional[List[int]] = None,
        bridge_approach_friction_lengths: Optional[List[float]] = None,
        bridge_departure_friction_lengths: Optional[List[float]] = None,
        bridge_wspro_coeffs: Optional[List[float]] = None,
        bridge_pressure_flow_coeffs_inlet: Optional[List[float]] = None,
        bridge_max_weir_submergence: Optional[List[float]] = None,
        bridge_deck_stations: Optional[List[List[float]]] = None,
        bridge_deck_low_elevations: Optional[List[List[float]]] = None,
        bridge_deck_high_elevations: Optional[List[List[float]]] = None,
        bridge_ineffective_left_stations: Optional[BridgeIneffectiveBlocks] = None,
        bridge_ineffective_left_elevations: Optional[BridgeIneffectiveBlocks] = None,
        bridge_ineffective_right_stations: Optional[BridgeIneffectiveBlocks] = None,
        bridge_ineffective_right_elevations: Optional[BridgeIneffectiveBlocks] = None,
        bridge_ineffective_left_stations_upstream: Optional[BridgeIneffectiveBlocks] = None,
        bridge_ineffective_left_elevations_upstream: Optional[BridgeIneffectiveBlocks] = None,
        bridge_ineffective_right_stations_upstream: Optional[BridgeIneffectiveBlocks] = None,
        bridge_ineffective_right_elevations_upstream: Optional[BridgeIneffectiveBlocks] = None,
        bridge_ineffective_left_stations_downstream: Optional[BridgeIneffectiveBlocks] = None,
        bridge_ineffective_left_elevations_downstream: Optional[BridgeIneffectiveBlocks] = None,
        bridge_ineffective_right_stations_downstream: Optional[BridgeIneffectiveBlocks] = None,
        bridge_ineffective_right_elevations_downstream: Optional[BridgeIneffectiveBlocks] = None,
        bridge_skew_angles: Optional[List[float]] = None,
        bridge_pier_stations: Optional[List[List[float]]] = None,
        bridge_pier_top_widths: Optional[List[List[float]]] = None,
        bridge_pier_bottom_widths: Optional[List[List[float]]] = None,
        bridge_pier_width_elevations: Optional[List[List[List[float]]]] = None,
        bridge_pier_width_values: Optional[List[List[List[float]]]] = None,
        bridge_pier_top_elevations: Optional[List[List[float]]] = None,
        bridge_pier_base_elevations: Optional[List[List[float]]] = None,
        bridge_upstream_cross_sections: Optional[List[CrossSection]] = None,
        bridge_downstream_cross_sections: Optional[List[CrossSection]] = None,
        bridge_internal_cross_sections: Optional[List[List[CrossSection]]] = None,
        bridge_opening_reach_station_origins: Optional[List[float]] = None,
        bridge_opening_anchor_modes: Optional[List[int]] = None,
        bridge_opening_anchor_reach_stations: Optional[List[float]] = None,
        structure_coupling_order: Optional[int] = None,
        unsteady_structure_coupling_mode: Optional[int] = None,
    ):
        self.cross_sections = cross_sections
        self.initial_wsel = initial_wsel
        self.initial_q = initial_q
        self.dt = dt
        self.num_steps = num_steps
        self.upstream_q_hydrograph = upstream_q_hydrograph
        self.downstream_wsel_hydrograph = downstream_wsel_hydrograph
        self.theta = theta
        self.num_slices = num_slices
        self.max_spacing = max_spacing
        self.coeff_contraction = coeff_contraction
        self.coeff_expansion = coeff_expansion
        self.culvert_stations = culvert_stations or []
        self.culvert_shape_types = culvert_shape_types or []
        self.culvert_spans = culvert_spans or []
        self.culvert_rises = culvert_rises or []
        self.culvert_roughness_ns = culvert_roughness_ns or []
        self.culvert_lengths = culvert_lengths or []
        self.culvert_entrance_loss_coeffs = culvert_entrance_loss_coeffs or []
        self.culvert_exit_loss_coeffs = culvert_exit_loss_coeffs or []
        self.culvert_barrels = culvert_barrels or []
        self.culvert_roughness_n_bottoms = culvert_roughness_n_bottoms or []
        self.culvert_depth_bottom_ns = culvert_depth_bottom_ns or []
        self.culvert_depth_blockeds = culvert_depth_blockeds or []
        self.culvert_inlet_types = culvert_inlet_types or []
        self.culvert_z_ups = culvert_z_ups or []
        self.culvert_z_downs = culvert_z_downs or []
        self.culvert_crest_elevs = culvert_crest_elevs or []
        self.culvert_weir_coeffs = culvert_weir_coeffs or []
        self.culvert_weir_lengths = culvert_weir_lengths or []
        self.culvert_skew_angles = culvert_skew_angles or []
        self.culvert_active_barrels = culvert_active_barrels or []
        self.culvert_barrel_spans = culvert_barrel_spans or []
        self.culvert_barrel_rises = culvert_barrel_rises or []
        self.bridge_stations = bridge_stations or []
        self.bridge_low_chords = bridge_low_chords or []
        self.bridge_high_chords = bridge_high_chords or []
        self.bridge_pier_widths = bridge_pier_widths or []
        self.bridge_num_piers = bridge_num_piers or []
        self.bridge_pier_shapes = bridge_pier_shapes or []
        self.bridge_weir_coeffs = bridge_weir_coeffs or []
        self.bridge_orifice_coeffs = bridge_orifice_coeffs or []
        self.bridge_abutment_block_widths = bridge_abutment_block_widths or []
        self.bridge_abutment_left_widths = bridge_abutment_left_widths or []
        self.bridge_abutment_right_widths = bridge_abutment_right_widths or []
        self.bridge_abutment_left_stations = bridge_abutment_left_stations or []
        self.bridge_abutment_right_stations = bridge_abutment_right_stations or []
        self.bridge_abutment_left_top_elevations = bridge_abutment_left_top_elevations or []
        self.bridge_abutment_right_top_elevations = bridge_abutment_right_top_elevations or []
        self.bridge_abutment_left_top_profile_stations = bridge_abutment_left_top_profile_stations or []
        self.bridge_abutment_left_top_profile_elevations = bridge_abutment_left_top_profile_elevations or []
        self.bridge_abutment_right_top_profile_stations = bridge_abutment_right_top_profile_stations or []
        self.bridge_abutment_right_top_profile_elevations = bridge_abutment_right_top_profile_elevations or []
        self.bridge_low_flow_methods = bridge_low_flow_methods or []
        self.bridge_high_flow_methods = bridge_high_flow_methods or []
        self.bridge_lengths = bridge_lengths or []
        self.bridge_friction_weighting = bridge_friction_weighting or []
        self.bridge_approach_friction_lengths = bridge_approach_friction_lengths or []
        self.bridge_departure_friction_lengths = bridge_departure_friction_lengths or []
        self.bridge_wspro_coeffs = bridge_wspro_coeffs or []
        self.bridge_pressure_flow_coeffs_inlet = bridge_pressure_flow_coeffs_inlet or []
        self.bridge_max_weir_submergence = bridge_max_weir_submergence or []
        self.bridge_deck_stations = bridge_deck_stations or []
        self.bridge_deck_low_elevations = bridge_deck_low_elevations or []
        self.bridge_deck_high_elevations = bridge_deck_high_elevations or []
        self.bridge_ineffective_left_stations = bridge_ineffective_left_stations or []
        self.bridge_ineffective_left_elevations = bridge_ineffective_left_elevations or []
        self.bridge_ineffective_right_stations = bridge_ineffective_right_stations or []
        self.bridge_ineffective_right_elevations = bridge_ineffective_right_elevations or []
        self.bridge_ineffective_left_stations_upstream = bridge_ineffective_left_stations_upstream or []
        self.bridge_ineffective_left_elevations_upstream = bridge_ineffective_left_elevations_upstream or []
        self.bridge_ineffective_right_stations_upstream = bridge_ineffective_right_stations_upstream or []
        self.bridge_ineffective_right_elevations_upstream = bridge_ineffective_right_elevations_upstream or []
        self.bridge_ineffective_left_stations_downstream = bridge_ineffective_left_stations_downstream or []
        self.bridge_ineffective_left_elevations_downstream = bridge_ineffective_left_elevations_downstream or []
        self.bridge_ineffective_right_stations_downstream = bridge_ineffective_right_stations_downstream or []
        self.bridge_ineffective_right_elevations_downstream = bridge_ineffective_right_elevations_downstream or []
        self.bridge_skew_angles = bridge_skew_angles or []
        self.bridge_pier_stations = bridge_pier_stations or []
        self.bridge_pier_top_widths = bridge_pier_top_widths or []
        self.bridge_pier_bottom_widths = bridge_pier_bottom_widths or []
        self.bridge_pier_width_elevations = bridge_pier_width_elevations or []
        self.bridge_pier_width_values = bridge_pier_width_values or []
        self.bridge_pier_top_elevations = bridge_pier_top_elevations or []
        self.bridge_pier_base_elevations = bridge_pier_base_elevations or []
        self.bridge_upstream_cross_sections = bridge_upstream_cross_sections or []
        self.bridge_downstream_cross_sections = bridge_downstream_cross_sections or []
        self.bridge_internal_cross_sections = bridge_internal_cross_sections or []
        self.bridge_opening_reach_station_origins = bridge_opening_reach_station_origins or []
        self.bridge_opening_anchor_modes = bridge_opening_anchor_modes or []
        self.bridge_opening_anchor_reach_stations = bridge_opening_anchor_reach_stations or []
        self.structure_coupling_order = structure_coupling_order
        self.unsteady_structure_coupling_mode = unsteady_structure_coupling_mode

    def to_dict(self) -> dict:
        res = {
            'cross_sections': [xs.to_dict() for xs in self.cross_sections],
            'initial_wsel': self.initial_wsel,
            'initial_q': self.initial_q,
            'dt': self.dt,
            'num_steps': self.num_steps,
            'upstream_q_hydrograph': self.upstream_q_hydrograph,
            'downstream_wsel_hydrograph': self.downstream_wsel_hydrograph,
            'theta': self.theta,
            'num_slices': self.num_slices,
            'max_spacing': self.max_spacing,
            'coeff_contraction': self.coeff_contraction,
            'coeff_expansion': self.coeff_expansion,
            'culvert_stations': self.culvert_stations,
            'culvert_shape_types': self.culvert_shape_types,
            'culvert_spans': self.culvert_spans,
            'culvert_rises': self.culvert_rises,
            'culvert_roughness_ns': self.culvert_roughness_ns,
            'culvert_lengths': self.culvert_lengths,
            'culvert_entrance_loss_coeffs': self.culvert_entrance_loss_coeffs,
            'culvert_exit_loss_coeffs': self.culvert_exit_loss_coeffs,
            'culvert_barrels': self.culvert_barrels,
            'culvert_roughness_n_bottoms': self.culvert_roughness_n_bottoms,
            'culvert_depth_bottom_ns': self.culvert_depth_bottom_ns,
            'culvert_depth_blockeds': self.culvert_depth_blockeds,
            'culvert_inlet_types': self.culvert_inlet_types,
            'culvert_z_ups': self.culvert_z_ups,
            'culvert_z_downs': self.culvert_z_downs,
            'culvert_crest_elevs': self.culvert_crest_elevs,
            'culvert_weir_coeffs': self.culvert_weir_coeffs,
            'culvert_weir_lengths': self.culvert_weir_lengths,
            'culvert_skew_angles': self.culvert_skew_angles,
            'culvert_active_barrels': self.culvert_active_barrels,
            'culvert_barrel_spans': self.culvert_barrel_spans,
            'culvert_barrel_rises': self.culvert_barrel_rises,
            'bridge_stations': self.bridge_stations,
            'bridge_low_chords': self.bridge_low_chords,
            'bridge_high_chords': self.bridge_high_chords,
            'bridge_pier_widths': self.bridge_pier_widths,
            'bridge_num_piers': self.bridge_num_piers,
            'bridge_pier_shapes': self.bridge_pier_shapes,
            'bridge_weir_coeffs': self.bridge_weir_coeffs,
            'bridge_orifice_coeffs': self.bridge_orifice_coeffs,
            'bridge_abutment_block_widths': self.bridge_abutment_block_widths,
            'bridge_abutment_left_widths': self.bridge_abutment_left_widths,
            'bridge_abutment_right_widths': self.bridge_abutment_right_widths,
            'bridge_abutment_left_stations': self.bridge_abutment_left_stations,
            'bridge_abutment_right_stations': self.bridge_abutment_right_stations,
            'bridge_abutment_left_top_elevations': self.bridge_abutment_left_top_elevations,
            'bridge_abutment_right_top_elevations': self.bridge_abutment_right_top_elevations,
            'bridge_abutment_left_top_profile_stations': self.bridge_abutment_left_top_profile_stations,
            'bridge_abutment_left_top_profile_elevations': self.bridge_abutment_left_top_profile_elevations,
            'bridge_abutment_right_top_profile_stations': self.bridge_abutment_right_top_profile_stations,
            'bridge_abutment_right_top_profile_elevations': self.bridge_abutment_right_top_profile_elevations,
            'bridge_low_flow_methods': self.bridge_low_flow_methods,
            'bridge_high_flow_methods': self.bridge_high_flow_methods,
            'bridge_lengths': self.bridge_lengths,
            'bridge_friction_weighting': self.bridge_friction_weighting,
            'bridge_approach_friction_lengths': self.bridge_approach_friction_lengths,
            'bridge_departure_friction_lengths': self.bridge_departure_friction_lengths,
            'bridge_wspro_coeffs': self.bridge_wspro_coeffs,
            'bridge_pressure_flow_coeffs_inlet': self.bridge_pressure_flow_coeffs_inlet,
            'bridge_max_weir_submergence': self.bridge_max_weir_submergence,
            'bridge_deck_stations': self.bridge_deck_stations,
            'bridge_deck_low_elevations': self.bridge_deck_low_elevations,
            'bridge_deck_high_elevations': self.bridge_deck_high_elevations,
            'bridge_ineffective_left_stations': self.bridge_ineffective_left_stations,
            'bridge_ineffective_left_elevations': self.bridge_ineffective_left_elevations,
            'bridge_ineffective_right_stations': self.bridge_ineffective_right_stations,
            'bridge_ineffective_right_elevations': self.bridge_ineffective_right_elevations,
            'bridge_ineffective_left_stations_upstream': self.bridge_ineffective_left_stations_upstream,
            'bridge_ineffective_left_elevations_upstream': self.bridge_ineffective_left_elevations_upstream,
            'bridge_ineffective_right_stations_upstream': self.bridge_ineffective_right_stations_upstream,
            'bridge_ineffective_right_elevations_upstream': self.bridge_ineffective_right_elevations_upstream,
            'bridge_ineffective_left_stations_downstream': self.bridge_ineffective_left_stations_downstream,
            'bridge_ineffective_left_elevations_downstream': self.bridge_ineffective_left_elevations_downstream,
            'bridge_ineffective_right_stations_downstream': self.bridge_ineffective_right_stations_downstream,
            'bridge_ineffective_right_elevations_downstream': self.bridge_ineffective_right_elevations_downstream,
            'bridge_skew_angles': self.bridge_skew_angles,
            'bridge_pier_stations': self.bridge_pier_stations,
            'bridge_pier_top_widths': self.bridge_pier_top_widths,
            'bridge_pier_bottom_widths': self.bridge_pier_bottom_widths,
            'bridge_pier_width_elevations': self.bridge_pier_width_elevations,
            'bridge_pier_width_values': self.bridge_pier_width_values,
            'bridge_pier_top_elevations': self.bridge_pier_top_elevations,
            'bridge_pier_base_elevations': self.bridge_pier_base_elevations,
            'bridge_upstream_cross_sections': self.bridge_upstream_cross_sections,
            'bridge_downstream_cross_sections': self.bridge_downstream_cross_sections,
            'bridge_internal_cross_sections': self.bridge_internal_cross_sections,
            'bridge_opening_reach_station_origins': self.bridge_opening_reach_station_origins,
            'bridge_opening_anchor_modes': self.bridge_opening_anchor_modes,
            'bridge_opening_anchor_reach_stations': self.bridge_opening_anchor_reach_stations,
        }
        if self.structure_coupling_order is not None:
            res['structure_coupling_order'] = self.structure_coupling_order
        if self.unsteady_structure_coupling_mode is not None:
            res['unsteady_structure_coupling_mode'] = self.unsteady_structure_coupling_mode
        return res

def solve_steady(inputs: Any) -> dict:
    """
    Executes a steady-state gradually varied backwater sweep.
    Returns a dictionary of result arrays (wsel, critical_wsel, velocity, area, top_width, froude, eg_slope).
    Inputs can be a SteadyInputs instance or a dictionary.
    """
    if hasattr(inputs, "to_dict"):
        payload = inputs.to_dict()
    else:
        payload = inputs
    json_in = json.dumps(payload)
    json_out = _stream1d.solve_steady_json(json_in)
    return json.loads(json_out)

def compute_culvert_rating_curve(payload: Any) -> dict:
    """
    Headwater vs discharge at fixed tailwater for one culvert.
    Pass a dict with `q_values` plus culvert geometry fields (`tw_wsel`, `span`, etc.).
    """
    if not isinstance(payload, dict):
        raise TypeError("compute_culvert_rating_curve expects a dict payload")
    json_out = _stream1d.compute_culvert_rating_curve_json(json.dumps(payload))
    return json.loads(json_out)

def compute_bridge_rating_curve(payload: Any) -> dict:
    """
    Upstream headwater vs discharge at fixed tailwater for one bridge opening.
    Pass a dict with `q_values` plus bridge geometry fields (`low_chord`, `high_chord`, `tw_wsel`, etc.).
    Per-side abutments use flattened keys: `abutment_left_width`, `abutment_right_width`,
    `abutment_left_top_elevation`, `abutment_right_top_elevation`, optional profile arrays, or legacy
    `abutment_block_width` for symmetric split.
    """
    if not isinstance(payload, dict):
        raise TypeError("compute_bridge_rating_curve expects a dict payload")
    json_out = _stream1d.compute_bridge_rating_curve_json(json.dumps(payload))
    return json.loads(json_out)

def solve_unsteady(inputs: Any) -> dict:
    """
    Executes an unsteady flow routing simulation.
    Inputs can be an UnsteadyInputs instance or a dictionary.
    """
    if hasattr(inputs, "to_dict"):
        payload = inputs.to_dict()
    else:
        payload = inputs
    json_in = json.dumps(payload)
    json_out = _stream1d.solve_unsteady_json(json_in)
    return json.loads(json_out)
