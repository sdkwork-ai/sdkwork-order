fn normalized(source: &str) -> String {
    source.replace("\r\n", "\n")
}

#[test]
fn membership_create_does_not_inline_payment_or_billing_helpers() {
    let postgres = normalized(include_str!("../src/postgres_membership_order.rs"));
    assert!(
        !postgres.contains("fn insert_payment"),
        "membership repository must not define inline payment insert helpers"
    );
    assert!(
        !postgres.contains("commerce_payment_intent"),
        "membership repository must not insert payment intents at create time"
    );
}

#[test]
fn membership_repository_has_no_server_side_sqlite() {
    let lib = normalized(include_str!("../src/lib.rs"));
    assert!(
        !lib.contains("SqliteCommerceMembershipOrderStore"),
        "server repository must not expose sqlite membership stores"
    );
    assert!(
        !lib.contains("sqlite_"),
        "server repository must not declare sqlite modules"
    );
}

#[test]
fn membership_repository_does_not_reach_into_the_merchandise_catalog() {
    // Membership used to project a SKU row into the merchandise-owned
    // `commerce_product_sku` / `commerce_product_spu` tables and then read the display
    // name back through a LEFT JOIN. Both halves were retired on 2026-09-23: merchandise
    // owns that catalog, and `membership_package` owns its own display name. This
    // repository must neither join nor write those tables.
    //
    // The needles below are deliberately spelled as SQL constructs rather than bare table
    // names. Naming the tables in an explanatory comment is allowed and must not turn
    // this gate red — that is why the source-level check lives in this file instead of a
    // `#[cfg(test)]` module inside `postgres_membership_order.rs`, where the assertion's
    // own literal would have matched the file it was reading.
    let postgres = normalized(include_str!("../src/postgres_membership_order.rs"));
    for forbidden in [
        "LEFT JOIN commerce_product",
        "INNER JOIN commerce_product",
        "INSERT INTO commerce_product",
        "UPDATE commerce_product",
        "DELETE FROM commerce_product",
    ] {
        assert!(
            !postgres.contains(forbidden),
            "membership repository must not reach into the merchandise catalog: {forbidden}"
        );
    }
}
