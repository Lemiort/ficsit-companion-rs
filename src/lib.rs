#![warn(clippy::all, rust_2018_idioms)]

mod app;
mod fractional_number;
pub mod building;
pub mod recipe;
pub mod pin;
pub mod link;
pub mod node;
pub mod game_data;
pub mod utils;
pub mod rate_calculator;
pub mod production_app;
pub mod serialization;

pub use app::TemplateApp;
pub use fractional_number::FractionalNumber;
pub use utils::{ItemCompare, RecipeCompare, update_save};
