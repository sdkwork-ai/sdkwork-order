fn normalized(source: &str) -> String {
    source.replace("\r\n", "\n")
}

fn lifecycle_function_source(start_marker: &str, end_marker: &str) -> String {
    let lifecycle = normalized(include_str!("../src/order_lifecycle.rs"));
    let start = lifecycle
        .find(start_marker)
        .unwrap_or_else(|| panic!("{start_marker} must exist"));
    let end = lifecycle
        .find(end_marker)
        .unwrap_or_else(|| panic!("{end_marker} must exist"));
    lifecycle[start..end].to_owned()
}

#[test]
fn order_event_insert_binds_every_sql_placeholder() {
    let source = lifecycle_function_source(
        "pub async fn insert_order_event_postgres",
        "pub async fn insert_order_cancellation_postgres",
    );
    let placeholders = source.chars().filter(|character| *character == '$').count();
    let binds = source.matches(".bind(").count();
    assert_eq!(
        placeholders, 15,
        "event insert must declare $1..$15 placeholders"
    );
    assert_eq!(
        binds, placeholders,
        "event insert must bind every SQL placeholder"
    );
    assert!(
        source.contains("normalize_organization_scope"),
        "event insert must normalize the organization scope to the platform sentinel"
    );
}

#[test]
fn order_cancellation_insert_binds_every_sql_placeholder() {
    let source = lifecycle_function_source(
        "pub async fn insert_order_cancellation_postgres",
        "fn store_error",
    );
    let placeholders = source.chars().filter(|character| *character == '$').count();
    let binds = source.matches(".bind(").count();
    assert_eq!(
        placeholders, 6,
        "cancellation insert must declare $1..$6 placeholders"
    );
    assert_eq!(
        binds, placeholders,
        "cancellation insert must bind every SQL placeholder"
    );
}
