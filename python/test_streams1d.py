import pytest
import json
import streams1d as st

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

def test_solve_unsteady_object():
    xs1000 = st.CrossSection(1000.0, [0.0, 0.0, 10.0, 10.0], [6.0, 1.0, 1.0, 6.0], [0.0], [0.02], "Metric")
    xs500 = st.CrossSection(500.0, [0.0, 0.0, 10.0, 10.0], [5.5, 0.5, 0.5, 5.5], [0.0], [0.02], "Metric")
    xs0 = st.CrossSection(0.0, [0.0, 0.0, 10.0, 10.0], [5.0, 0.0, 0.0, 5.0], [0.0], [0.02], "Metric")

    inputs = st.UnsteadyInputs(
        cross_sections=[xs1000, xs500, xs0],
        initial_wsel=[2.0, 1.5, 1.0],
        initial_q=[14.0, 14.0, 14.0],
        dt=60.0,
        num_steps=5,
        upstream_q_hydrograph=[14.0] * 5,
        downstream_wsel_hydrograph=[1.0] * 5,
        theta=0.6,
        num_slices=50
    )
    res = st.solve_unsteady(inputs)
    assert 'wsel' in res
    assert 'q' in res
    assert len(res['wsel']) == 5
    assert len(res['wsel'][0]) == 3

def test_solve_unsteady_dict():
    payload = {
        'cross_sections': [
            {'station': 1000.0, 'x': [0.0, 0.0, 10.0, 10.0], 'y': [6.0, 1.0, 1.0, 6.0], 'n_stations': [0.0], 'n_values': [0.02], 'unit_system': 'Metric'},
            {'station': 500.0, 'x': [0.0, 0.0, 10.0, 10.0], 'y': [5.5, 0.5, 0.5, 5.5], 'n_stations': [0.0], 'n_values': [0.02], 'unit_system': 'Metric'},
            {'station': 0.0, 'x': [0.0, 0.0, 10.0, 10.0], 'y': [5.0, 0.0, 0.0, 5.0], 'n_stations': [0.0], 'n_values': [0.02], 'unit_system': 'Metric'}
        ],
        'initial_wsel': [2.0, 1.5, 1.0],
        'initial_q': [14.0, 14.0, 14.0],
        'dt': 60.0,
        'num_steps': 5,
        'upstream_q_hydrograph': [14.0] * 5,
        'downstream_wsel_hydrograph': [1.0] * 5,
        'theta': 0.6,
        'num_slices': 50
    }
    res = st.solve_unsteady(payload)
    assert 'wsel' in res
    assert len(res['wsel']) == 5

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
    assert result['wsel'][1] > 3.0

