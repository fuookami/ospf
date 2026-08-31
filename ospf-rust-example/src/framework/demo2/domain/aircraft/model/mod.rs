//! 飞机领域模型 / Aircraft domain model.
pub mod aircraft_model;
pub mod deck;
pub mod flight_phase;
pub mod formula;
pub mod fuel;
pub mod fuselage;
pub mod hatch_door;
pub mod loading_order;
pub mod neighbour;
pub mod position;
pub mod uld;

pub use aircraft_model::*;
pub use deck::*;
pub use flight_phase::*;
pub use formula::*;
pub use fuel::*;
pub use fuselage::*;
pub use hatch_door::*;
pub use loading_order::*;
pub use neighbour::*;
pub use position::*;
pub use uld::*;
