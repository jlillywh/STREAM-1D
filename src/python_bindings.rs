//! PyO3 JSON entrypoints (thin wrappers over `json_api`).

use pyo3::prelude::*;

#[pyfunction]
#[pyo3(name = "solve_steady_json")]
pub fn solve_steady_json_py(inputs_json: &str) -> PyResult<String> {
    crate::json_api::steady_result_json(inputs_json)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e))
}

#[pyfunction]
#[pyo3(name = "compute_culvert_rating_curve_json")]
pub fn compute_culvert_rating_curve_json_py(inputs_json: &str) -> PyResult<String> {
    crate::json_api::culvert_rating_curve_json(inputs_json)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e))
}

#[pyfunction]
#[pyo3(name = "compute_bridge_rating_curve_json")]
pub fn compute_bridge_rating_curve_json_py(inputs_json: &str) -> PyResult<String> {
    crate::json_api::bridge_rating_curve_json(inputs_json)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e))
}

#[pyfunction]
#[pyo3(name = "solve_unsteady_json")]
pub fn solve_unsteady_json_py(inputs_json: &str) -> PyResult<String> {
    crate::json_api::unsteady_result_json(inputs_json)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e))
}

#[pymodule]
pub fn register(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(solve_steady_json_py, m)?)?;
    m.add_function(wrap_pyfunction!(compute_culvert_rating_curve_json_py, m)?)?;
    m.add_function(wrap_pyfunction!(compute_bridge_rating_curve_json_py, m)?)?;
    m.add_function(wrap_pyfunction!(solve_unsteady_json_py, m)?)?;
    Ok(())
}
