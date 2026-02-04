#![warn(clippy::all, rust_2018_idioms)]
// Allow some pedantic lints that are too noisy for this codebase
#![allow(clippy::too_many_lines)]
#![allow(clippy::indexing_slicing)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::iter_over_hash_type)]
#![allow(clippy::return_and_then)]
#![allow(clippy::bind_instead_of_map)]
#![allow(clippy::clone_on_copy)]
#![allow(clippy::map_clone)]
#![allow(clippy::str_to_string)]
#![allow(clippy::explicit_iter_loop)]
#![allow(clippy::let_underscore_untyped)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::unnecessary_mut_passed)]
#![allow(clippy::needless_pass_by_value)]
#![allow(clippy::needless_range_loop)]
#![allow(clippy::let_underscore_must_use)]
#![allow(clippy::use_self)]
#![allow(clippy::match_same_arms)]
#![allow(clippy::unwrap_or_default)]
#![allow(clippy::cloned_instead_of_copied)]
#![allow(clippy::unused_enumerate_index)]
#![allow(clippy::manual_map)]
#![allow(clippy::type_complexity)]
#![allow(clippy::manual_let_else)]
#![allow(clippy::let_and_return)]
#![allow(clippy::explicit_into_iter_loop)]
#![allow(clippy::get_first)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::collapsible_else_if)]
#![allow(clippy::assign_op_pattern)]
#![allow(clippy::only_used_in_recursion)]
#![allow(clippy::redundant_type_annotations)]
#![allow(clippy::print_stdout)]
#![allow(clippy::single_match_else)]
#![allow(clippy::unused_self)]
#![allow(clippy::needless_borrows_for_generic_args)]
#![allow(clippy::for_kv_map)]
#![allow(clippy::needless_borrow)]
#![allow(clippy::useless_let_if_seq)]
#![allow(clippy::unused_trait_names)]
#![allow(clippy::too_long_first_doc_paragraph)]
#![allow(clippy::map_err_ignore)]
#![allow(clippy::redundant_closure)]
#![allow(clippy::derivable_impls)]

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
