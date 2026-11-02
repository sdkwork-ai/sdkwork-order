//! SQL persistence error mapping shared across order repositories.

use sdkwork_contract_service::CommerceServiceError;

pub fn map_sql_store_error(context: &str, error: impl std::fmt::Display) -> CommerceServiceError {
    let message = format!("{context}: {error}");
    if is_duplicate_key_message(&message) {
        return CommerceServiceError::conflict(format!("{context}: duplicate request"));
    }
    CommerceServiceError::storage(message)
}

pub(crate) fn map_sqlx_store_error(context: &str, error: sqlx::Error) -> CommerceServiceError {
    if is_sqlx_unique_violation(&error) {
        return CommerceServiceError::conflict(format!("{context}: duplicate request"));
    }
    CommerceServiceError::storage(format!("{context}: {error}"))
}

fn is_sqlx_unique_violation(error: &sqlx::Error) -> bool {
    match error {
        sqlx::Error::Database(db) => db
            .code()
            .is_some_and(|code| code == "23505" || code == "2067"),
        _ => false,
    }
}

fn is_duplicate_key_message(message: &str) -> bool {
    let normalized = message.to_ascii_lowercase();
    normalized.contains("unique constraint")
        || normalized.contains("duplicate key")
        || normalized.contains("unique violation")
}
