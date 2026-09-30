fn normalized(source: &str) -> String {
    source.replace("\r\n", "\n")
}

#[test]
fn recharge_create_does_not_inline_payment_or_billing_helpers() {
    let postgres = normalized(include_str!("../src/postgres_recharge.rs"));
    assert!(
        !postgres.contains("fn insert_payment"),
        "recharge repository must not define inline payment insert helpers"
    );
    assert!(
        !postgres.contains("fn insert_recharge_billing_history"),
        "recharge repository must not define billing history insert helpers"
    );
}

#[test]
fn recharge_payment_queries_use_standard_payment_method_columns() {
    let postgres = normalized(include_str!("../src/postgres_recharge.rs"));
    assert!(
        postgres.contains("SELECT method_key, provider_code\nFROM commerce_payment_method"),
        "recharge repository must read provider_code from the standard payment method table"
    );
    assert!(
        postgres.contains("COALESCE(sort_order, 0) ASC"),
        "recharge repository must order payment methods with standard sort_order"
    );
    for legacy_fragment in [
        "provider AS provider_code",
        "NULLIF(pa.provider,",
        "NULLIF(pi.provider,",
    ] {
        assert!(
            !postgres.contains(legacy_fragment),
            "recharge repository must not read legacy payment provider fragment {legacy_fragment}",
        );
    }
}

#[test]
fn recharge_purchase_intent_reuse_requires_a_future_expiration_boundary() {
    let postgres = normalized(include_str!("../src/postgres_recharge.rs"));
    assert!(postgres.contains("o.expired_at > $10::timestamptz"));
    assert!(postgres.contains("expired_at <= $4::timestamptz"));
    assert!(postgres.contains("expire_stale_recharge_orders(&mut tx, &command).await?"));
    assert!(!postgres.contains("o.expired_at IS NULL OR o.expired_at = '' OR o.expired_at >"));
}

#[test]
fn recharge_repository_has_no_server_side_sqlite() {
    let lib = normalized(include_str!("../src/lib.rs"));
    assert!(
        !lib.contains("SqliteCommerceRechargeStore"),
        "server repository must not expose sqlite recharge stores"
    );
    assert!(
        !lib.contains("SqliteCommerceOrderStore"),
        "server repository must not expose sqlite order stores"
    );
    assert!(
        !lib.contains("sqlite_"),
        "server repository must not declare sqlite modules"
    );
}
