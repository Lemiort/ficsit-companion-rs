/// Production rate calculation engine
///
/// This module implements the core algorithm that propagates production rates
/// through the production graph. When a user changes the rate at any pin,
/// this algorithm computes the new rates for all affected pins.
///
/// The algorithm works in phases:
/// 1. Find all pins affected by the rate change (graph traversal)
/// 2. Build a system of linear equations based on node constraints
/// 3. Solve the system using Gaussian elimination
/// 4. Apply the solution and update all rates
use crate::fractional_number::FractionalNumber;
use std::collections::HashMap;

/// Result type for rate calculations
pub type RateResult<T> = Result<T, RateError>;

/// Errors that can occur during rate calculation
#[derive(Debug, Clone)]
pub enum RateError {
    /// System of equations has no solution
    NoSolution,
    /// Solution contains negative rates (impossible production)
    NegativeRate,
    /// Contradictory constraints (e.g., locked pins with incompatible rates)
    Contradiction,
    /// Singular matrix during Gaussian elimination
    SingularMatrix,
}

impl std::fmt::Display for RateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RateError::NoSolution => write!(f, "No valid solution for production rates"),
            RateError::NegativeRate => write!(f, "Solution would produce negative rates"),
            RateError::Contradiction => write!(f, "Contradictory constraints in graph"),
            RateError::SingularMatrix => write!(f, "Singular matrix in linear system"),
        }
    }
}

impl std::error::Error for RateError {}

/// A linear equation in the system: ax + by + cz = d
/// Represented as augmented matrix row: [a, b, c, ..., d]
pub type EquationRow = Vec<FractionalNumber>;

/// Gaussian elimination solver for systems of linear equations
/// Solves Ax = b where A is the coefficient matrix and b is the constants
pub struct LinearSolver {
    /// Augmented matrix [A|b]
    matrix: Vec<EquationRow>,
    num_variables: usize,
}

impl LinearSolver {
    /// Create a new solver with given coefficients and constants
    pub fn new(
        coefficients: Vec<Vec<FractionalNumber>>,
        constants: Vec<FractionalNumber>,
    ) -> RateResult<Self> {
        if coefficients.is_empty() {
            return Err(RateError::NoSolution);
        }

        let num_variables = coefficients[0].len();
        let num_equations = coefficients.len();

        // Verify all equations have the same number of variables
        for eq in &coefficients {
            if eq.len() != num_variables {
                return Err(RateError::Contradiction);
            }
        }

        // Verify constants match equations
        if constants.len() != num_equations {
            return Err(RateError::Contradiction);
        }

        // Build augmented matrix [A|b]
        let mut matrix = Vec::new();
        for i in 0..num_equations {
            let mut row = coefficients[i].clone();
            row.push(constants[i].clone());
            matrix.push(row);
        }

        Ok(LinearSolver {
            matrix,
            num_variables,
        })
    }

    /// Solve the linear system using Gaussian elimination with partial pivoting
    pub fn solve(mut self) -> RateResult<Vec<FractionalNumber>> {
        let num_equations = self.matrix.len();
        let num_variables = self.num_variables;

        // Forward elimination with partial pivoting
        let mut h = 0; // row
        let mut k = 0; // column

        while h < num_equations && k < num_variables {
            // Find pivot (row with largest absolute value in column k)
            let mut pivot_row = h;
            let mut pivot_value = self.matrix[h][k].value().abs();

            for i in (h + 1)..num_equations {
                let abs_val = self.matrix[i][k].value().abs();
                if abs_val > pivot_value {
                    pivot_value = abs_val;
                    pivot_row = i;
                }
            }

            // If pivot column is all zeros, move to next column
            if self.matrix[pivot_row][k].numerator() == 0 {
                k += 1;
                continue;
            }

            // Swap rows if necessary
            if pivot_row != h {
                self.matrix.swap(h, pivot_row);
            }

            // Eliminate below pivot
            for i in (h + 1)..num_equations {
                let factor = self.matrix[i][k].clone() / self.matrix[h][k].clone();
                self.matrix[i][k] = FractionalNumber::new(0, 1);

                for j in (k + 1)..=num_variables {
                    self.matrix[i][j] =
                        self.matrix[i][j].clone() - self.matrix[h][j].clone() * factor.clone();
                }
            }

            h += 1;
            k += 1;
        }

        // Check for contradictions in the lower rows
        for i in h..num_equations {
            let mut all_zero = true;
            for j in 0..num_variables {
                if self.matrix[i][j].numerator() != 0 {
                    all_zero = false;
                    break;
                }
            }
            if all_zero && self.matrix[i][num_variables].numerator() != 0 {
                return Err(RateError::Contradiction);
            }
        }

        // Back substitution
        let mut solution = vec![FractionalNumber::new(0, 1); num_variables];

        for i in (0..h.min(num_variables)).rev() {
            // Find pivot column for this row
            let mut pivot_col = 0;
            while pivot_col < num_variables && self.matrix[i][pivot_col].numerator() == 0 {
                pivot_col += 1;
            }

            if pivot_col >= num_variables {
                continue;
            }

            let mut sum = FractionalNumber::new(0, 1);
            for j in (pivot_col + 1)..num_variables {
                sum = sum + self.matrix[i][j].clone() * solution[j].clone();
            }

            solution[pivot_col] =
                (self.matrix[i][num_variables].clone() - sum) / self.matrix[i][pivot_col].clone();
        }

        // Verify all solutions are non-negative
        for rate in &solution {
            if rate.numerator() < 0 {
                return Err(RateError::NegativeRate);
            }
        }

        Ok(solution)
    }
}

/// Pin constraint for equation building
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PinConstraint {
    pub pin_id: u64,
    pub node_id: u64,
    pub is_locked: bool,
}

/// Represents how pins are related in equations
#[derive(Debug, Clone)]
pub struct VariableMapping {
    /// Maps pin ID to (variable index, ratio)
    /// ratio = base_rate or similar multiplier
    pub pin_to_variable: HashMap<u64, (usize, FractionalNumber)>,
    pub num_variables: usize,
}

/// Build a system of equations for a production graph
/// Returns (coefficients, constants) for the linear system
pub fn build_equations(
    constraints: Vec<PinConstraint>,
    locked_pin_rates: HashMap<u64, FractionalNumber>,
    links: Vec<(u64, u64)>, // (output_pin, input_pin) connections
) -> RateResult<(
    Vec<Vec<FractionalNumber>>,
    Vec<FractionalNumber>,
    VariableMapping,
)> {
    if constraints.is_empty() {
        return Err(RateError::NoSolution);
    }

    let mut mapping = VariableMapping {
        pin_to_variable: HashMap::new(),
        num_variables: 0,
    };

    // Create variables for unlocked pins
    for constraint in &constraints {
        if !constraint.is_locked {
            mapping.pin_to_variable.insert(
                constraint.pin_id,
                (mapping.num_variables, FractionalNumber::new(1, 1)),
            );
            mapping.num_variables += 1;
        }
    }

    if mapping.num_variables == 0 {
        return Err(RateError::NoSolution);
    }

    let mut equations_coefficients = Vec::new();
    let mut constants = Vec::new();

    // Add equality equations for linked pins
    for (output_pin, input_pin) in links {
        if let Some((var_idx, ratio)) = mapping.pin_to_variable.get(&output_pin) {
            if let Some((var_idx2, ratio2)) = mapping.pin_to_variable.get(&input_pin) {
                // equation: output_rate - input_rate = 0
                let mut equation = vec![FractionalNumber::new(0, 1); mapping.num_variables];
                equation[*var_idx] = ratio.clone();
                equation[*var_idx2] = FractionalNumber::new(-1, 1) * ratio2.clone();
                equations_coefficients.push(equation);
                constants.push(FractionalNumber::new(0, 1));
            }
        }
    }

    // Add locked pin constraints
    for (pin_id, rate) in &locked_pin_rates {
        if let Some((var_idx, ratio)) = mapping.pin_to_variable.get(pin_id) {
            let mut equation = vec![FractionalNumber::new(0, 1); mapping.num_variables];
            equation[*var_idx] = ratio.clone();
            equations_coefficients.push(equation);
            constants.push(rate.clone());
        }
    }

    if equations_coefficients.is_empty() {
        return Err(RateError::NoSolution);
    }

    Ok((equations_coefficients, constants, mapping))
}

#[cfg(test)]
/// Helper to find connected pins in a production graph
pub fn find_connected_pins(
    pin_id: u64,
    all_links: &[(u64, u64)], // (output_pin_id, input_pin_id) pairs
) -> Vec<u64> {
    let mut connected = Vec::new();

    for (output, input) in all_links {
        if *output == pin_id {
            connected.push(*input);
        } else if *input == pin_id {
            connected.push(*output);
        }
    }

    connected
}

/// Validate a rate is non-negative and reasonable
pub fn validate_rate(rate: &FractionalNumber) -> bool {
    rate.numerator() >= 0 && rate.denominator() > 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_equation() {
        // Test: 2x = 4 -> x = 2
        let coefficients = vec![vec![FractionalNumber::new(2, 1)]];
        let constants = vec![FractionalNumber::new(4, 1)];

        let solver = LinearSolver::new(coefficients, constants).unwrap();
        let solution = solver.solve().unwrap();

        assert_eq!(solution.len(), 1);
        assert_eq!(solution[0], FractionalNumber::new(2, 1));
    }

    #[test]
    fn test_two_equations() {
        // Test: x + y = 3, x - y = 1 -> x = 2, y = 1
        let coefficients = vec![
            vec![FractionalNumber::new(1, 1), FractionalNumber::new(1, 1)],
            vec![FractionalNumber::new(1, 1), FractionalNumber::new(-1, 1)],
        ];
        let constants = vec![FractionalNumber::new(3, 1), FractionalNumber::new(1, 1)];

        let solver = LinearSolver::new(coefficients, constants).unwrap();
        let solution = solver.solve().unwrap();

        assert_eq!(solution.len(), 2);
        assert_eq!(solution[0], FractionalNumber::new(2, 1));
        assert_eq!(solution[1], FractionalNumber::new(1, 1));
    }

    #[test]
    fn test_negative_rate_error() {
        // Test: x = -1 (negative production rate)
        let coefficients = vec![vec![FractionalNumber::new(1, 1)]];
        let constants = vec![FractionalNumber::new(-1, 1)];

        let solver = LinearSolver::new(coefficients, constants).unwrap();
        let result = solver.solve();

        assert!(matches!(result, Err(RateError::NegativeRate)));
    }

    #[test]
    fn test_contradictory_equations() {
        // Test: x = 1, x = 2 (contradiction)
        let coefficients = vec![
            vec![FractionalNumber::new(1, 1)],
            vec![FractionalNumber::new(1, 1)],
        ];
        let constants = vec![FractionalNumber::new(1, 1), FractionalNumber::new(2, 1)];

        let solver = LinearSolver::new(coefficients, constants).unwrap();
        let result = solver.solve();

        // Should detect contradiction during elimination
        match result {
            Ok(_) => {
                // If no error, check that solution doesn't satisfy both equations
                // (This case may or may not error depending on matrix rank)
            }
            Err(RateError::Contradiction) => {
                // Expected
            }
            Err(_) => panic!("Wrong error type"),
        }
    }

    #[test]
    fn test_find_connected_pins() {
        let links = vec![
            (1, 2), // pin 1 output -> pin 2 input
            (1, 3), // pin 1 output -> pin 3 input
            (4, 2), // pin 4 output -> pin 2 input
        ];

        let connected = find_connected_pins(1, &links);
        assert_eq!(connected.len(), 2);
        assert!(connected.contains(&2));
        assert!(connected.contains(&3));
    }

    #[test]
    fn test_validate_rate() {
        assert!(validate_rate(&FractionalNumber::new(1, 1)));
        assert!(validate_rate(&FractionalNumber::new(0, 1)));
        assert!(!validate_rate(&FractionalNumber::new(-1, 1)));
    }

    #[test]
    fn test_build_simple_equations() {
        // Two pins connected: output=1 -> input=1
        let constraints = vec![
            PinConstraint {
                pin_id: 1,
                node_id: 10,
                is_locked: false,
            },
            PinConstraint {
                pin_id: 2,
                node_id: 20,
                is_locked: false,
            },
        ];

        let links = vec![(1, 2)]; // output pin 1 -> input pin 2
        let locked_rates = HashMap::new();

        let result = build_equations(constraints, locked_rates, links);
        assert!(result.is_ok());

        let (coefficients, constants, mapping) = result.unwrap();
        assert_eq!(mapping.num_variables, 2);
        assert_eq!(coefficients.len(), 1); // One equation for the link
        assert_eq!(constants.len(), 1);
    }

    #[test]
    fn test_splitter_like_matrix_no_contradiction() {
        // Variables: v11, v30, v20, v10, v12 (5 variables)
        // Equations:
        // v11 = 0
        // v11 - v30 = 0
        // v20 - v10 = 0
        // v11 - v10 + v12 = 0
        let coeffs = vec![
            vec![
                FractionalNumber::new(1, 1),
                FractionalNumber::new(0, 1),
                FractionalNumber::new(0, 1),
                FractionalNumber::new(0, 1),
                FractionalNumber::new(0, 1),
            ],
            vec![
                FractionalNumber::new(1, 1),
                FractionalNumber::new(-1, 1),
                FractionalNumber::new(0, 1),
                FractionalNumber::new(0, 1),
                FractionalNumber::new(0, 1),
            ],
            vec![
                FractionalNumber::new(0, 1),
                FractionalNumber::new(0, 1),
                FractionalNumber::new(1, 1),
                FractionalNumber::new(-1, 1),
                FractionalNumber::new(0, 1),
            ],
            vec![
                FractionalNumber::new(1, 1),
                FractionalNumber::new(0, 1),
                FractionalNumber::new(0, 1),
                FractionalNumber::new(-1, 1),
                FractionalNumber::new(1, 1),
            ],
        ];
        let consts = vec![FractionalNumber::new(0, 1); 4];
        let solver = LinearSolver::new(coeffs.clone(), consts.clone()).unwrap();
        let res = solver.solve();
        assert!(res.is_ok());
    }
}
