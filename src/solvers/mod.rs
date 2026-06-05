pub mod steady;
pub mod unsteady;

pub use steady::{solve_steady, SteadyInputs, SteadyResult};
pub use unsteady::{solve_unsteady, UnsteadyInputs, UnsteadyResult};
