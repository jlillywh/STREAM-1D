//! Bridge hydraulics unit tests (split by physics area).

use super::*;
pub(crate) use crate::geometry::{row_at_elevation, CrossSection, IneffectiveFlowAreas};
pub(crate) use crate::solvers::deck_vent_geometry::DeckVentUserInput;
pub(crate) use crate::solvers::pier_geometry::{
    resolve_pier_width_specs, PierAttachmentsUserInput, PierWidthUserInput,
};

mod common;
mod coupling;
mod high_flow;
mod low_flow;
mod opening;
mod rating;

pub(crate) use common::*;
