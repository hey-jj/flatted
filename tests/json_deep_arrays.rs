use flatted::parse;

#[test]
fn parses_very_deep_json_arrays_without_stack_overflow() {
    let text = format!("{}0{}", "[".repeat(10_000), "]".repeat(10_000));

    assert!(parse(&text, None).is_ok());
}
