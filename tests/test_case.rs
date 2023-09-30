use sith::test_case;

#[test_case(one)]
#[test_case(two)]
fn supports_multiple_declarations() {}

#[test_case(one, with(1, 1))]
#[test_case(two, with(2, 2))]
fn supports_multiple_declarations_with_different_inputs(first: usize, second: usize) {
    assert_eq!(first, second);
}

#[test_case(one, with(0))]
#[test_case(two, with(0.0))]
fn supports_multiple_declarations_with_different_types(input: _) {
    assert_eq!(input as usize, 0);
}

#[test_case(one, with(0u64, verbatim(u64)))]
#[test_case(two, with(0f64, verbatim(f64)))]
fn verbatim_and_with_work_together(input: _, r#type: _) {
    assert_eq!(input, r#type::default());
}