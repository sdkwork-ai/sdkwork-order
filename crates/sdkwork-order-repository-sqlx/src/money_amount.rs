use sdkwork_contract_service::{CommerceMoney, CommerceServiceError};

pub(crate) fn normalize_money_amount(amount: &str) -> Result<String, CommerceServiceError> {
    parse_minor_units(amount).map(|minor_units| minor_units.to_string())
}

/// Normalizes a stored money column value into a canonical smallest-unit
/// integer string before `CommerceMoney` validation.
///
/// Stored values must be canonical integer cents (passed through unchanged),
/// but legacy writers have emitted major-unit decimals (e.g. `0.00`, `12.50`).
/// Reading those must not fail the whole owner order list, so well-formed
/// decimals are converted to cents here. Truly invalid values (negative,
/// empty, more than two fraction digits) fail with the offending value so the
/// log line is actionable.
pub(crate) fn normalize_stored_money(amount: &str) -> Result<String, CommerceServiceError> {
    let value = amount.trim();
    let mut parts = value.split('.');
    let whole = parts.next().unwrap_or_default();
    let fraction = parts.next().unwrap_or_default();
    let trailing = parts.next().is_some();
    let well_formed = !whole.is_empty()
        && whole.chars().all(|c| c.is_ascii_digit())
        && fraction.chars().all(|c| c.is_ascii_digit())
        && fraction.len() <= 2
        && !trailing;
    if !well_formed {
        return Err(invalid_stored_money(value));
    }
    if fraction.is_empty() {
        // Canonical integer smallest-unit strings pass through unchanged.
        return Ok(whole.to_string());
    }
    let whole_minor = whole
        .parse::<i64>()
        .map_err(|_| invalid_stored_money(value))?;
    let mut padded = fraction.to_string();
    while padded.len() < 2 {
        padded.push('0');
    }
    let cents = padded
        .parse::<i64>()
        .map_err(|_| invalid_stored_money(value))?;
    whole_minor
        .checked_mul(100)
        .and_then(|amount| amount.checked_add(cents))
        .map(|amount| amount.to_string())
        .ok_or_else(|| invalid_stored_money(value))
}

/// Parses a stored money column with `normalize_stored_money` and enriches
/// failures with the column and order context so a corrupt row can be located
/// from the error log instead of surfacing as an opaque 500.
pub(crate) fn commerce_money_stored(
    amount: &str,
    column: &str,
    order_id: &str,
) -> Result<CommerceMoney, CommerceServiceError> {
    let normalized = normalize_stored_money(amount).map_err(|_| {
        CommerceServiceError::storage(format!(
            "invalid stored money amount in column {column} of order {order_id}: {amount:?}"
        ))
    })?;
    CommerceMoney::new(&normalized).map_err(CommerceServiceError::storage)
}

fn invalid_stored_money(value: &str) -> CommerceServiceError {
    CommerceServiceError::storage(format!("invalid stored money amount: {value:?}"))
}

pub(crate) fn commerce_money(amount: &str) -> Result<CommerceMoney, CommerceServiceError> {
    let normalized = normalize_money_amount(amount)?;
    CommerceMoney::new(&normalized).map_err(CommerceServiceError::storage)
}

pub(crate) fn sum_money_amounts<'a>(
    mut amounts: impl Iterator<Item = &'a str>,
) -> Result<String, CommerceServiceError> {
    amounts
        .try_fold(0_i64, |total, amount| {
            let amount = parse_minor_units(amount)?;
            total.checked_add(amount).ok_or_else(|| {
                CommerceServiceError::validation("checkout total amount is too large")
            })
        })
        .map(|total| total.to_string())
}

pub(crate) fn multiply_money_amount(
    amount: &str,
    quantity: i64,
) -> Result<String, CommerceServiceError> {
    if quantity <= 0 {
        return Err(CommerceServiceError::validation(
            "checkout line quantity must be greater than zero",
        ));
    }

    parse_minor_units(amount)?
        .checked_mul(quantity)
        .map(|total| total.to_string())
        .ok_or_else(|| CommerceServiceError::validation("checkout line amount is too large"))
}

fn parse_minor_units(amount: &str) -> Result<i64, CommerceServiceError> {
    let money = CommerceMoney::new(amount).map_err(|error| {
        CommerceServiceError::storage(format!("invalid minor-unit money amount: {error}"))
    })?;
    money.as_str().parse::<i64>().map_err(|_| {
        CommerceServiceError::validation("money amount exceeds the supported integer range")
    })
}

#[cfg(test)]
mod tests {
    use super::{
        multiply_money_amount, normalize_money_amount, normalize_stored_money, sum_money_amounts,
    };

    #[test]
    fn computes_minor_unit_totals_without_major_unit_conversion() {
        assert_eq!(multiply_money_amount("6990", 2).unwrap(), "13980");
        assert_eq!(
            sum_money_amounts(["6990", "6990"].into_iter()).unwrap(),
            "13980"
        );
        assert_eq!(normalize_money_amount("0").unwrap(), "0");
    }

    #[test]
    fn rejects_decimal_and_overflowing_minor_unit_amounts() {
        let maximum = i64::MAX.to_string();

        assert!(multiply_money_amount("69.90", 2).is_err());
        assert!(multiply_money_amount(&maximum, 2).is_err());
        assert!(sum_money_amounts([maximum.as_str(), "1"].into_iter()).is_err());
    }

    #[test]
    fn normalizes_legacy_major_unit_decimals_to_minor_units() {
        assert_eq!(normalize_stored_money("0.00").unwrap(), "0");
        assert_eq!(normalize_stored_money("12.30").unwrap(), "1230");
        assert_eq!(normalize_stored_money("12.3").unwrap(), "1230");
        assert_eq!(normalize_stored_money("640.00").unwrap(), "64000");
        // Canonical integer smallest-unit strings pass through unchanged.
        assert_eq!(normalize_stored_money("64000").unwrap(), "64000");
    }

    #[test]
    fn rejects_truly_invalid_stored_money_values() {
        assert!(normalize_stored_money("-100").is_err());
        assert!(normalize_stored_money("12.345").is_err());
        assert!(normalize_stored_money("1,000").is_err());
        assert!(normalize_stored_money("").is_err());
        assert!(normalize_stored_money("abc").is_err());
    }
}
