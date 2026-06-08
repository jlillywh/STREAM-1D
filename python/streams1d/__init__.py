import json
from typing import List, Dict, Optional, Any

# Import native binary solver
try:
    from . import _streams1d
except ImportError:
    # Handle direct local importing during development/testing
    import _streams1d

class CrossSection:
    def __init__(
        self,
        station: float,
        x: List[float],
        y: List[float],
        n_stations: List[float],
        n_values: List[float],
        unit_system: str = "Metric",
        is_overbank: Optional[List[bool]] = None
    ):
        self.station = station
        self.x = x
        self.y = y
        self.n_stations = n_stations
        self.n_values = n_values
        self.unit_system = unit_system
        self.is_overbank = is_overbank

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
        # Bridge parameters
        bridge_stations: Optional[List[float]] = None,
        bridge_low_chords: Optional[List[float]] = None,
        bridge_high_chords: Optional[List[float]] = None,
        bridge_pier_widths: Optional[List[float]] = None,
        bridge_num_piers: Optional[List[int]] = None,
        bridge_pier_shapes: Optional[List[int]] = None,
        bridge_weir_coeffs: Optional[List[float]] = None,
        bridge_orifice_coeffs: Optional[List[float]] = None,
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
        self.bridge_stations = bridge_stations or []
        self.bridge_low_chords = bridge_low_chords or []
        self.bridge_high_chords = bridge_high_chords or []
        self.bridge_pier_widths = bridge_pier_widths or []
        self.bridge_num_piers = bridge_num_piers or []
        self.bridge_pier_shapes = bridge_pier_shapes or []
        self.bridge_weir_coeffs = bridge_weir_coeffs or []
        self.bridge_orifice_coeffs = bridge_orifice_coeffs or []
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
            'bridge_stations': self.bridge_stations,
            'bridge_low_chords': self.bridge_low_chords,
            'bridge_high_chords': self.bridge_high_chords,
            'bridge_pier_widths': self.bridge_pier_widths,
            'bridge_num_piers': self.bridge_num_piers,
            'bridge_pier_shapes': self.bridge_pier_shapes,
            'bridge_weir_coeffs': self.bridge_weir_coeffs,
            'bridge_orifice_coeffs': self.bridge_orifice_coeffs,
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

    def to_dict(self) -> dict:
        return {
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
        }

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
    json_out = _streams1d.solve_steady_json(json_in)
    return json.loads(json_out)

def compute_culvert_rating_curve(payload: Any) -> dict:
    """
    Headwater vs discharge at fixed tailwater for one culvert.
    Pass a dict with `q_values` plus culvert geometry fields (`tw_wsel`, `span`, etc.).
    """
    if not isinstance(payload, dict):
        raise TypeError("compute_culvert_rating_curve expects a dict payload")
    json_out = _streams1d.compute_culvert_rating_curve_json(json.dumps(payload))
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
    json_out = _streams1d.solve_unsteady_json(json_in)
    return json.loads(json_out)
