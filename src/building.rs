use crate::fractional_number::FractionalNumber;
use serde::{Deserialize, Serialize};

/// Represents a building that can produce/consume items
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Building {
    pub name: String,
    pub somersloop_mult: FractionalNumber, // Multiplier for somersloop boost
    pub power: f64, // Base power consumption/generation
    pub power_exponent: f64, // Exponent for underclocking power calculation
    pub somersloop_power_exponent: f64, // Exponent for somersloop power calculation
    pub variable_power: bool, // Whether this building has variable power consumption
}

impl Building {
    pub fn new(
        name: String,
        somersloop_mult: FractionalNumber,
        power: f64,
        power_exponent: f64,
        somersloop_power_exponent: f64,
        variable_power: bool,
    ) -> Self {
        Self {
            name,
            somersloop_mult,
            power,
            power_exponent,
            somersloop_power_exponent,
            variable_power,
        }
    }
}
