#![warn(clippy::all, rust_2018_idioms)]

mod app;
pub mod building;
mod fractional_number;
pub mod game_data;
pub mod graph_node;
pub mod link;
pub mod node;
pub mod pin;
pub mod production_app;
pub mod rate_calculator;
pub mod recipe;
pub mod serialization;
pub mod utils;

pub use app::TemplateApp;
pub use fractional_number::FractionalNumber;
pub use graph_node::{GraphNode, GraphNodeType, NodeDisplayData, PendingChange};
pub use utils::{ItemCompare, RecipeCompare, update_save};
