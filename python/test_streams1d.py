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
