use num_rational::Rational64;
use num_traits::{Signed, Zero};
use serde::ser::SerializeTupleStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::ops::{Add, AddAssign, Div, Mul, MulAssign, Sub, SubAssign};

/// A fractional number using rational arithmetic for precise calculations.
/// This is a wrapper around Rational64 with additional features like string parsing
/// and expression evaluation (supporting +, -, *, /, parentheses).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct FractionalNumber(Rational64);

impl Serialize for FractionalNumber {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut ts = serializer.serialize_tuple_struct("FractionalNumber", 2)?;
        ts.serialize_field(&self.numerator())?;
        ts.serialize_field(&self.denominator())?;
        ts.end()
    }
}

impl<'de> Deserialize<'de> for FractionalNumber {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_tuple_struct("FractionalNumber", 2, FractionalNumberVisitor)
    }
}

struct FractionalNumberVisitor;

impl<'de> serde::de::Visitor<'de> for FractionalNumberVisitor {
    type Value = FractionalNumber;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a fractional number with numerator and denominator")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let numerator = seq
            .next_element()?
            .ok_or_else(|| serde::de::Error::invalid_length(0, &self))?;
        let denominator = seq
            .next_element()?
            .ok_or_else(|| serde::de::Error::invalid_length(1, &self))?;
        Ok(FractionalNumber::new(numerator, denominator))
    }
}

impl FractionalNumber {
    /// Create a new FractionalNumber from numerator and denominator
    pub fn new(numerator: i64, denominator: i64) -> Self {
        Self(Rational64::new(numerator, denominator))
    }

    /// Parse a string that can contain:
    /// - Simple fractions: "3/4"
    /// - Decimal numbers: "1.5"
    /// - Expressions with operators: "1 + 2 * 3"
    /// - Expressions with parentheses: "(1 + 2) * 3"
    pub fn from_string(s: &str) -> Result<Self, String> {
        // Implementation of Shunting yard algorithm for parsing mathematical expressions
        let precedence = |op: char| match op {
            '+' | '-' => 1,
            '*' | '/' => 2,
            _ => 0,
        };

        let mut postfix: Vec<String> = Vec::new();
        let mut operators: Vec<char> = Vec::new();
        let chars: Vec<char> = s.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            let c = chars[i];

            // Skip spaces
            if c.is_whitespace() {
                i += 1;
                continue;
            }

            // If it's a digit or decimal point, or a unary '-' starting a numeric literal, parse the number
            let is_unary_minus = if c == '-' {
                // Check next char exists and is a digit or '.'
                let next_is_num =
                    (i + 1) < chars.len() && (chars[i + 1].is_numeric() || chars[i + 1] == '.');
                if !next_is_num {
                    false
                } else {
                    // Determine previous non-whitespace char (if any)
                    if i == 0 {
                        true
                    } else {
                        let mut j = i as isize - 1;
                        while j >= 0 && chars[j as usize].is_whitespace() {
                            j -= 1;
                        }
                        if j < 0 {
                            true
                        } else {
                            matches!(chars[j as usize], '+' | '-' | '*' | '/' | '(')
                        }
                    }
                }
            } else {
                false
            };

            if c.is_numeric() || c == '.' || is_unary_minus {
                let start = i;
                // If unary minus, include the '-' sign
                if is_unary_minus {
                    i += 1; // include the '-'
                }
                while i < chars.len() && (chars[i].is_numeric() || chars[i] == '.') {
                    i += 1;
                }
                postfix.push(chars[start..i].iter().collect());
                continue;
            }

            // If it's an operator
            if matches!(c, '+' | '-' | '*' | '/') {
                // Pop operators with higher or equal precedence
                while !operators.is_empty()
                    && operators.last() != Some(&'(')
                    && precedence(*operators.last().unwrap()) >= precedence(c)
                {
                    postfix.push(operators.pop().unwrap().to_string());
                }
                operators.push(c);
            } else if c == '(' {
                operators.push(c);
            } else if c == ')' {
                // Pop until we find the matching '('
                while !operators.is_empty() && operators.last() != Some(&'(') {
                    postfix.push(operators.pop().unwrap().to_string());
                }
                if operators.is_empty() {
                    return Err("Mismatched parentheses".to_string());
                }
                operators.pop(); // Remove the '('
            }

            i += 1;
        }

        // Pop remaining operators
        while let Some(op) = operators.pop() {
            if op == '(' {
                return Err("Mismatched parentheses".to_string());
            }
            postfix.push(op.to_string());
        }

        // Evaluate postfix expression
        let mut values: Vec<FractionalNumber> = Vec::new();

        for token in postfix {
            let first_char = token.chars().next().unwrap();
            if first_char.is_numeric()
                || first_char == '.'
                || (first_char == '-'
                    && token.len() > 1
                    && (token.chars().nth(1).unwrap().is_numeric()
                        || token.chars().nth(1).unwrap() == '.'))
            {
                // Parse number (allowing a leading negative sign)
                if let Some(slash_pos) = token.find('/') {
                    // It's a fraction possibly containing decimals in numerator/denominator
                    let num_str = &token[..slash_pos];
                    let den_str = &token[slash_pos + 1..];
                    let num_fn = FractionalNumber::from_string(num_str)
                        .map_err(|_| format!("Invalid numerator: {}", num_str))?;
                    let den_fn = FractionalNumber::from_string(den_str)
                        .map_err(|_| format!("Invalid denominator: {}", den_str))?;
                    if den_fn.is_zero() {
                        return Err("Division by zero".to_string());
                    }
                    values.push(num_fn / den_fn);
                } else if let Some(dot_pos) = token.find('.') {
                    // It's a decimal (handle negative integer part correctly, e.g. "-3.5")
                    let int_part = if dot_pos == 0 {
                        0
                    } else {
                        token[..dot_pos]
                            .parse::<i64>()
                            .map_err(|_| format!("Invalid integer part: {}", &token[..dot_pos]))?
                    };
                    let frac_part = token[dot_pos + 1..].parse::<i64>().map_err(|_| {
                        format!("Invalid fractional part: {}", &token[dot_pos + 1..])
                    })?;
                    let decimals = (token.len() - dot_pos - 1) as u32;
                    let denominator = 10_i64.pow(decimals);
                    // Preserve sign of integer part while adding fractional part magnitude
                    let numerator = if int_part < 0 {
                        -(int_part.abs() * denominator + frac_part)
                    } else {
                        int_part * denominator + frac_part
                    };
                    values.push(FractionalNumber::new(numerator, denominator));
                } else {
                    // It's an integer (may be negative)
                    let num = token
                        .parse::<i64>()
                        .map_err(|_| format!("Invalid integer: {}", token))?;
                    values.push(FractionalNumber::new(num, 1));
                }
            } else {
                // It's an operator
                if values.len() < 2 {
                    return Err("Invalid expression".to_string());
                }
                let b = values.pop().unwrap();
                let a = values.pop().unwrap();
                let result = match token.as_str() {
                    "+" => a + b,
                    "-" => a - b,
                    "*" => a * b,
                    "/" => {
                        if b.numerator() == 0 {
                            return Err("Division by zero".to_string());
                        }
                        a / b
                    }
                    _ => return Err(format!("Invalid operator: {}", token)),
                };
                values.push(result);
            }
        }

        if values.len() != 1 {
            return Err("Invalid expression".to_string());
        }

        Ok(values.pop().unwrap())
    }

    pub fn numerator(&self) -> i64 {
        *self.0.numer()
    }

    pub fn denominator(&self) -> i64 {
        *self.0.denom()
    }

    pub fn value(&self) -> f64 {
        self.numerator() as f64 / self.denominator() as f64
    }

    /// Get the fraction as a string (e.g., "3/4" or "5" for whole numbers)
    pub fn to_fraction_string(&self) -> String {
        if self.denominator() == 1 {
            self.numerator().to_string()
        } else {
            format!("{}/{}", self.numerator(), self.denominator())
        }
    }

    /// Get the decimal value as a string with 3 decimal places
    pub fn to_float_string(&self) -> String {
        format!("{:.3}", self.value())
    }

    pub fn abs(&self) -> Self {
        Self(self.0.abs())
    }

    pub fn is_zero(&self) -> bool {
        self.0.is_zero()
    }

    pub fn is_negative(&self) -> bool {
        self.0.is_negative()
    }
}

impl Default for FractionalNumber {
    fn default() -> Self {
        Self::new(0, 1)
    }
}

impl From<i64> for FractionalNumber {
    fn from(n: i64) -> Self {
        Self::new(n, 1)
    }
}

impl From<(i64, i64)> for FractionalNumber {
    fn from((n, d): (i64, i64)) -> Self {
        Self::new(n, d)
    }
}

impl fmt::Display for FractionalNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_fraction_string())
    }
}

impl std::fmt::Debug for FractionalNumber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_fraction_string())
    }
}

impl Add for FractionalNumber {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl AddAssign for FractionalNumber {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}

impl Sub for FractionalNumber {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl SubAssign for FractionalNumber {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0;
    }
}

impl Mul for FractionalNumber {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self(self.0 * rhs.0)
    }
}

impl MulAssign for FractionalNumber {
    fn mul_assign(&mut self, rhs: Self) {
        self.0 *= rhs.0;
    }
}

impl Div for FractionalNumber {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        Self(self.0 / rhs.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_creation() {
        let f = FractionalNumber::new(3, 4);
        assert_eq!(f.numerator(), 3);
        assert_eq!(f.denominator(), 4);
        assert_eq!(f.value(), 0.75);
    }

    #[test]
    fn test_simplification() {
        let f = FractionalNumber::new(6, 8);
        assert_eq!(f.numerator(), 3);
        assert_eq!(f.denominator(), 4);
    }

    #[test]
    fn test_arithmetic() {
        let a = FractionalNumber::new(1, 2);
        let b = FractionalNumber::new(1, 3);

        let sum = a + b;
        assert_eq!(sum.numerator(), 5);
        assert_eq!(sum.denominator(), 6);

        let diff = a - b;
        assert_eq!(diff.numerator(), 1);
        assert_eq!(diff.denominator(), 6);

        let prod = a * b;
        assert_eq!(prod.numerator(), 1);
        assert_eq!(prod.denominator(), 6);

        let quot = a / b;
        assert_eq!(quot.numerator(), 3);
        assert_eq!(quot.denominator(), 2);
    }

    #[test]
    fn test_parse_decimal() {
        let f = FractionalNumber::from_string("1.5").unwrap();
        assert_eq!(f.numerator(), 3);
        assert_eq!(f.denominator(), 2);
    }

    #[test]
    fn test_parse_expression() {
        let f = FractionalNumber::from_string("1 + 2").unwrap();
        assert_eq!(f.numerator(), 3);
        assert_eq!(f.denominator(), 1);

        let f = FractionalNumber::from_string("1 + 2 * 3").unwrap();
        assert_eq!(f.numerator(), 7);
        assert_eq!(f.denominator(), 1);

        let f = FractionalNumber::from_string("(1 + 2) * 3").unwrap();
        assert_eq!(f.numerator(), 9);
        assert_eq!(f.denominator(), 1);
    }

    #[test]
    fn test_parse_decimal_division() {
        let f = FractionalNumber::from_string("300.0/75.000000").unwrap();
        // 300 / 75 == 4
        assert_eq!(f.value(), 4.0);
        assert_eq!(f.numerator(), 4);
        assert_eq!(f.denominator(), 1);
    }

    #[test]
    fn test_to_string() {
        let f = FractionalNumber::new(3, 4);
        assert_eq!(f.to_fraction_string(), "3/4");
        assert_eq!(f.to_float_string(), "0.750");

        let g = FractionalNumber::new(5, 1);
        assert_eq!(g.to_fraction_string(), "5");
    }

    #[test]
    fn test_to_fraction_string() {
        let f = FractionalNumber::new(9, 16);
        assert_eq!(f.to_fraction_string(), "9/16");

        let g = FractionalNumber::new(5, 1);
        assert_eq!(g.to_fraction_string(), "5");
    }

    #[test]
    fn test_from_fraction_string() {
        let f = FractionalNumber::from_string("9/16").unwrap();
        assert_eq!(f.to_fraction_string(), "9/16");
    }
}
