import pytest
import json
import stream1d as st

def test_cross_section_serialization():
    xs = st.CrossSection(
        station=100.0,
        x=[0.0, 10.0],
        y=[1.0, 1.0],
        n_stations=[0.0],
        n_values=[0.035],
        unit_system="USCustomary",
        is_overbank=[False, False]
    )
    d = xs.to_dict()
    assert d['station'] == 100.0
    assert d['x'] == [0.0, 10.0]
    assert d['y'] == [1.0, 1.0]
    assert d['n_stations'] == [0.0]
    assert d['n_values'] == [0.035]
    assert d['unit_system'] == "USCustomary"
    assert d['is_overbank'] == [False, False]

def test_steady_inputs_serialization():
    xs = st.CrossSection(100.0, [0.0, 10.0], [1.0, 1.0], [0.0], [0.035])
    inputs = st.SteadyInputs(
        cross_sections=[xs],
        flow_rate=50.0,
        downstream_bc_type=1,
        downstream_bc_slope=0.002
    )
    d = inputs.to_dict()
    assert d['flow_rate'] == 50.0
    assert d['downstream_bc_type'] == 1
    assert d['downstream_bc_slope'] == 0.002
    assert 'cross_sections' in d
    assert len(d['cross_sections']) == 1

def test_solve_steady_object():
    xs1000 = st.CrossSection(1000.0, [0.0, 0.0, 10.0, 10.0], [6.0, 1.0, 1.0, 6.0], [0.0], [0.025], "Metric")
    xs500 = st.CrossSection(500.0, [0.0, 0.0, 10.0, 10.0], [5.5, 0.5, 0.5, 5.5], [0.0], [0.025], "Metric")
    xs0 = st.CrossSection(0.0, [0.0, 0.0, 10.0, 10.0], [5.0, 0.0, 0.0, 5.0], [0.0], [0.025], "Metric")

    inputs = st.SteadyInputs(
        cross_sections=[xs1000, xs500, xs0],
        flow_rate=15.0,
        num_slices=50,
        regime=0,
        downstream_wsel=1.5,
        downstream_bc_type=0
    )
    res = st.solve_steady(inputs)
    assert 'wsel' in res
    assert 'velocity' in res
    assert len(res['wsel']) == 3
    assert abs(res['wsel'][2] - 1.5) < 1e-4

def test_steady_inputs_culvert_geometry_serialization():
    xs = st.CrossSection(100.0, [0.0, 10.0], [1.0, 1.0], [0.0], [0.035])
    inputs = st.SteadyInputs(
        cross_sections=[xs],
        flow_rate=50.0,
        culvert_stations=[50.0],
        culvert_skew_angles=[15.0],
        culvert_active_barrels=[2],
        culvert_barrel_spans=[[8.0, 6.0]],
        culvert_barrel_rises=[[6.0, 6.0]],
        culvert_shape_types=[st.CULVERT_SHAPE_PIPE_ARCH],
    )
    d = inputs.to_dict()
    assert d['culvert_skew_angles'] == [15.0]
    assert d['culvert_active_barrels'] == [2]
    assert d['culvert_barrel_spans'] == [[8.0, 6.0]]
    assert d['culvert_shape_types'] == [4]

def test_solve_steady_dict():
    payload = {
        'cross_sections': [
            {'station': 100.0, 'x': [0.0, 10.0], 'y': [0.0, 0.0], 'n_stations': [0.0], 'n_values': [0.03], 'unit_system': 'Metric'},
            {'station': 0.0, 'x': [0.0, 10.0], 'y': [0.0, 0.0], 'n_stations': [0.0], 'n_values': [0.03], 'unit_system': 'Metric'}
        ],
        'flow_rate': 10.0,
        'regime': 0,
        'downstream_wsel': 1.0,
        'downstream_bc_type': 0
    }
    res = st.solve_steady(payload)
    assert 'wsel' in res
    assert len(res['wsel']) == 2

def test_solve_steady_integrated_bridge():
    # Simple reach: stations 200, 100, 0
    # Rectangular channel: width = 10m
    # Bed elevations: 0.2m, 0.1m, 0.0m
    # Flow rate: 15.0 cms
    xs200 = st.CrossSection(200.0, [0.0, 0.0, 10.0, 10.0], [10.2, 0.2, 0.2, 10.2], [0.0], [0.03], "Metric")
    xs100 = st.CrossSection(100.0, [0.0, 0.0, 10.0, 10.0], [10.1, 0.1, 0.1, 10.1], [0.0], [0.03], "Metric")
    xs0 = st.CrossSection(0.0, [0.0, 0.0, 10.0, 10.0], [10.0, 0.0, 0.0, 10.0], [0.0], [0.03], "Metric")

    inputs = st.SteadyInputs(
        cross_sections=[xs200, xs100, xs0],
        flow_rate=15.0,
        num_slices=50,
        regime=0,
        downstream_wsel=3.0,
        downstream_bc_type=0,
        bridge_stations=[50.0],  # bridge at station 50 (between 0 and 100)
        bridge_low_chords=[5.0],
        bridge_high_chords=[7.0],
        bridge_pier_widths=[0.5],
        bridge_num_piers=[2],
        bridge_pier_shapes=[0],
        bridge_weir_coeffs=[1.44],
        bridge_orifice_coeffs=[0.5]
    )

    result = st.solve_steady(inputs)
    assert abs(result['wsel'][2] - 3.0) < 1e-4
    assert abs(result['wsel'][1] - 3.00247) < 0.001

def test_steady_inputs_culvert_tier1_serialization():
    xs = st.CrossSection(100.0, [0.0, 10.0], [1.0, 1.0], [0.0], [0.035], "USCustomary")
    inputs = st.SteadyInputs(
        cross_sections=[xs],
        flow_rate=50.0,
        culvert_stations=[50.0],
        culvert_shape_types=[0],
        culvert_spans=[5.0],
        culvert_rises=[5.0],
        culvert_inlet_types=[1],
        culvert_z_ups=[10.5],
        culvert_z_downs=[9.0],
        culvert_crest_elevs=[14.0],
        culvert_weir_coeffs=[2.6],
        culvert_weir_lengths=[20.0],
    )
    d = inputs.to_dict()
    assert d['culvert_inlet_types'] == [1]
    assert d['culvert_z_ups'] == [10.5]
    assert d['culvert_z_downs'] == [9.0]
    assert d['culvert_crest_elevs'] == [14.0]
    assert d['culvert_weir_coeffs'] == [2.6]
    assert d['culvert_weir_lengths'] == [20.0]

def test_steady_inputs_culvert_chart_scale_serialization():
    xs = st.CrossSection(100.0, [0.0, 10.0], [1.0, 1.0], [0.0], [0.035], "USCustomary")
    inputs = st.SteadyInputs(
        cross_sections=[xs],
        flow_rate=50.0,
        culvert_stations=[50.0],
        culvert_shape_types=[0],
        culvert_spans=[5.0],
        culvert_rises=[5.0],
        culvert_chart_numbers=[1],
        culvert_scale_numbers=[2],
    )
    d = inputs.to_dict()
    assert d['culvert_chart_numbers'] == [1]
    assert d['culvert_scale_numbers'] == [2]

def _culvert_channel_us():
    xs200 = st.CrossSection(200.0, [0.0, 0.0, 10.0, 10.0], [12.0, 2.0, 2.0, 12.0], [0.0], [0.02], "USCustomary")
    xs100 = st.CrossSection(100.0, [0.0, 0.0, 10.0, 10.0], [11.0, 1.0, 1.0, 11.0], [0.0], [0.02], "USCustomary")
    xs0 = st.CrossSection(0.0, [0.0, 0.0, 10.0, 10.0], [10.0, 0.0, 0.0, 10.0], [0.0], [0.02], "USCustomary")
    return [xs200, xs100, xs0]

def test_solve_steady_culvert_control_types():
    inputs = st.SteadyInputs(
        cross_sections=_culvert_channel_us(),
        flow_rate=100.0,
        num_slices=50,
        regime=0,
        downstream_wsel=3.0,
        downstream_bc_type=0,
        culvert_stations=[50.0],
        culvert_shape_types=[0],
        culvert_spans=[5.0],
        culvert_rises=[5.0],
        culvert_roughness_ns=[0.012],
        culvert_lengths=[100.0],
        culvert_entrance_loss_coeffs=[0.5],
        culvert_exit_loss_coeffs=[1.0],
        culvert_inlet_types=[1],
    )
    result = st.solve_steady(inputs)
    assert 'culvert_control_types' in result
    assert result['culvert_control_types'] == ['inlet']

def test_solve_steady_culvert_overtopping():
    inputs = st.SteadyInputs(
        cross_sections=_culvert_channel_us(),
        flow_rate=500.0,
        num_slices=50,
        regime=0,
        downstream_wsel=10.0,
        downstream_bc_type=0,
        culvert_stations=[50.0],
        culvert_shape_types=[0],
        culvert_spans=[5.0],
        culvert_rises=[5.0],
        culvert_roughness_ns=[0.012],
        culvert_lengths=[100.0],
        culvert_entrance_loss_coeffs=[0.5],
        culvert_exit_loss_coeffs=[1.0],
        culvert_inlet_types=[1],
        culvert_z_ups=[10.0],
        culvert_z_downs=[9.0],
        culvert_crest_elevs=[14.0],
        culvert_weir_coeffs=[2.6],
        culvert_weir_lengths=[20.0],
        culvert_barrels=[2],
    )
    result = st.solve_steady(inputs)
    assert result['culvert_control_types'] == ['overtopping']
    assert result['wsel'][1] > 14.0

def test_solve_steady_tributary_junction():
    main = [
        st.CrossSection(1000.0, [0.0, 0.0, 10.0, 10.0], [5.2, 0.2, 0.2, 5.2], [0.0], [0.025], "Metric"),
        st.CrossSection(500.0, [0.0, 0.0, 10.0, 10.0], [5.1, 0.1, 0.1, 5.1], [0.0], [0.025], "Metric"),
        st.CrossSection(0.0, [0.0, 0.0, 10.0, 10.0], [5.0, 0.0, 0.0, 5.0], [0.0], [0.025], "Metric"),
    ]
    trib = [
        st.CrossSection(800.0, [0.0, 0.0, 10.0, 10.0], [5.15, 0.15, 0.15, 5.15], [0.0], [0.030], "Metric"),
        st.CrossSection(600.0, [0.0, 0.0, 10.0, 10.0], [5.12, 0.12, 0.12, 5.12], [0.0], [0.030], "Metric"),
        st.CrossSection(400.0, [0.0, 0.0, 10.0, 10.0], [5.10, 0.10, 0.10, 5.10], [0.0], [0.030], "Metric"),
    ]
    inputs = st.SteadyInputs(
        cross_sections=main,
        flow_rate=10.0,
        num_slices=50,
        regime=0,
        downstream_wsel=1.5,
        downstream_bc_type=0,
        tributary_cross_sections=trib,
        tributary_flow_rate=5.0,
        junction_main_station=500.0,
    )
    result = st.solve_steady(inputs)
    assert 'tributary_wsel' in result
    assert len(result['tributary_wsel']) == 3
    assert abs(result['wsel'][1] - result['tributary_wsel'][2]) < 1e-3
    assert result['velocity'][2] > result['velocity'][1]

def test_solve_steady_culvert_tier2a_diagnostics():
    inputs = st.SteadyInputs(
        cross_sections=_culvert_channel_us(),
        flow_rate=100.0,
        num_slices=50,
        regime=0,
        downstream_wsel=3.0,
        downstream_bc_type=0,
        culvert_stations=[50.0],
        culvert_shape_types=[0],
        culvert_spans=[5.0],
        culvert_rises=[5.0],
        culvert_roughness_ns=[0.012],
        culvert_lengths=[100.0],
        culvert_entrance_loss_coeffs=[0.5],
        culvert_exit_loss_coeffs=[1.0],
        culvert_inlet_types=[1],
    )
    result = st.solve_steady(inputs)
    assert result['culvert_q_barrels'][0] == pytest.approx(100.0)
    assert result['culvert_wsel_inlet'][0] == pytest.approx(result['wsel'][1], rel=0.02)
    assert result['culvert_barrel_velocities'][0] > 0.0
    assert result['culvert_barrel_froude'][0] > 0.0

def test_compute_culvert_rating_curve():
    curve = st.compute_culvert_rating_curve({
        'q_values': [50.0, 100.0, 150.0],
        'tw_wsel': 12.0,
        'units': 'USCustomary',
        'shape_type': 0,
        'inlet_type': 1,
        'span': 5.0,
        'rise': 5.0,
        'roughness_n': 0.012,
        'length': 100.0,
        'entrance_loss_coeff': 0.5,
        'exit_loss_coeff': 1.0,
        'z_down': 9.0,
        'z_up': 10.0,
        'manning_n_bottom': 0.012,
        'num_barrels': 1,
    })
    assert len(curve['q']) == 3
    assert curve['wsel'][2] > curve['wsel'][0]
    assert curve['q_barrel'][0] == pytest.approx(50.0)

def test_compute_bridge_rating_curve():
    curve = st.compute_bridge_rating_curve({
        'q_values': [10.0, 20.0, 30.0],
        'low_chord': 5.0,
        'high_chord': 7.0,
        'z_down': 0.0,
        'z_up': 0.0,
        'tw_wsel': 2.5,
        'units': 'Metric',
        'low_flow_method': 3,
        'channel_width': 10.0,
        'manning_n': 0.03,
    })
    assert len(curve['q']) == 3
    assert curve['wsel'][2] > curve['wsel'][0]
    assert len(curve['flow_regimes']) == 3
    assert curve['wsel_down'][0] == pytest.approx(2.5)

def test_steady_inputs_bridge_abutment_per_side_serialization():
    xs = st.CrossSection(100.0, [0.0, 0.0, 10.0, 10.0], [5.0, 0.0, 0.0, 5.0], [0.0], [0.03], 'Metric')
    inputs = st.SteadyInputs(
        cross_sections=[xs, st.CrossSection(0.0, [0.0, 0.0, 10.0, 10.0], [5.0, 0.0, 0.0, 5.0], [0.0], [0.03], 'Metric')],
        flow_rate=15.0,
        downstream_wsel=1.5,
        bridge_stations=[50.0],
        bridge_low_chords=[5.0],
        bridge_high_chords=[7.0],
        bridge_low_flow_methods=[4],
        bridge_abutment_left_widths=[1.0],
        bridge_abutment_right_widths=[4.0],
        bridge_abutment_right_top_elevations=[2.5],
        bridge_abutment_left_top_profile_stations=[[0.0, 1.0]],
        bridge_abutment_left_top_profile_elevations=[[0.0, 0.0]],
    )
    d = inputs.to_dict()
    assert d['bridge_abutment_left_widths'] == [1.0]
    assert d['bridge_abutment_right_widths'] == [4.0]
    assert d['bridge_abutment_right_top_elevations'] == [2.5]
    assert d['bridge_abutment_left_top_profile_stations'] == [[0.0, 1.0]]



def test_compute_bridge_rating_curve_per_side_abutments():
    asymmetric = st.compute_bridge_rating_curve({
        'q_values': [15.0, 20.0],
        'low_chord': 5.0,
        'high_chord': 7.0,
        'z_down': 0.0,
        'z_up': 0.0,
        'tw_wsel': 2.5,
        'units': 'Metric',
        'low_flow_method': 4,
        'channel_width': 10.0,
        'manning_n': 0.03,
        'abutment_left_width': 1.0,
        'abutment_right_width': 4.0,
        'abutment_right_top_elevation': 2.5,
    })
    symmetric = st.compute_bridge_rating_curve({
        'q_values': [15.0, 20.0],
        'low_chord': 5.0,
        'high_chord': 7.0,
        'z_down': 0.0,
        'z_up': 0.0,
        'tw_wsel': 2.5,
        'units': 'Metric',
        'low_flow_method': 4,
        'abutment_block_width': 5.0,
        'channel_width': 10.0,
        'manning_n': 0.03,
    })
    assert abs(asymmetric['wsel'][0] - symmetric['wsel'][0]) > 0.01

