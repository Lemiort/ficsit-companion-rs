use ficsit_companion_rs::FractionalNumber;

#[test]
fn test_negative_number_parsing() {
    // Negative integer
    let n1 = FractionalNumber::from_string("-5").expect("failed to parse -5");
    assert_eq!(n1.numerator(), -5);
    assert_eq!(n1.denominator(), 1);

    // Negative decimal
    let n2 = FractionalNumber::from_string("-3.5").expect("failed to parse -3.5");
    assert!((n2.value() + 3.5).abs() < 1e-9);

    // Negative fraction
    let n3 = FractionalNumber::from_string("-3/4").expect("failed to parse -3/4");
    assert_eq!(n3.numerator(), -3);
    assert_eq!(n3.denominator(), 4);

    // Negative in expression
    let n4 = FractionalNumber::from_string("5 + -3").expect("failed to parse 5 + -3");
    assert!((n4.value() - 2.0).abs() < 1e-9);

    // Larger negative integer (regression case)
    let n5 = FractionalNumber::from_string("-6250").expect("failed to parse -6250");
    assert_eq!(n5.numerator(), -6250);

    // Negative decimal multiplied
    let n6 = FractionalNumber::from_string("-1.25 * 4").expect("failed to parse -1.25 * 4");
    assert!((n6.value() + 5.0).abs() < 1e-9);

    // Complex expression with decimals and fractions
    let n7 = FractionalNumber::from_string("(7.2 - 4.8) + 2 / (4/3 - 1/3)").expect("failed to parse (7.2 - 4.8) + 2 / (4/3 - 1/3)");
    // Expected value: (7.2 - 4.8) = 2.4; (4/3 - 1/3) = 1, so 2/1 = 2; total = 4.4 = 22/5
    assert!((n7.value() - 4.4).abs() < 1e-9);
    assert_eq!(n7.numerator(), 22);
    assert_eq!(n7.denominator(), 5);
}
