//! PayPal buyer-approval capture decision.
//!
//! A CAPTURE-intent PayPal order stops at `APPROVED`: the merchant must POST
//! `/v2/checkout/orders/{id}/capture` or no money moves, and PayPal never emits
//! `PAYMENT.CAPTURE.COMPLETED` on its own. The webhook path therefore has to
//! decide, from a *verified and normalized* event, whether this delivery is the
//! approval that must be captured.
//!
//! The decision is a pure function so the money path is unit-tested without a
//! database, an HTTP client or a live PayPal account; the handler only has to
//! call it and forward the result to the adapter.

/// PayPal's buyer-approval event for an Orders v2 order.
pub const PAYPAL_ORDER_APPROVED_EVENT: &str = "CHECKOUT.ORDER.APPROVED";

/// Returns the PayPal order id to capture when this verified event is a buyer
/// approval, otherwise `None`.
///
/// Constraints that keep the payload from steering the capture request:
/// only a `paypal` delivery can match, the event type must be the approval
/// event, and the id must be a single path segment (PayPal ids never contain
/// `/`, `?` or `#`), so a crafted payload cannot retarget the signed capture URL.
#[must_use]
pub fn paypal_capture_order_id<'a>(
    provider_code: &str,
    event_type: Option<&str>,
    resource_id: Option<&'a str>,
) -> Option<&'a str> {
    if !provider_code.trim().eq_ignore_ascii_case("paypal") {
        return None;
    }
    let event_type = event_type?.trim();
    if !event_type.eq_ignore_ascii_case(PAYPAL_ORDER_APPROVED_EVENT) {
        return None;
    }
    let resource_id = resource_id?.trim();
    if resource_id.is_empty()
        || resource_id.len() > 128
        || resource_id
            .chars()
            .any(|character| matches!(character, '/' | '?' | '#' | ' '))
    {
        return None;
    }
    Some(resource_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_approved_paypal_order_is_captured() {
        assert_eq!(
            paypal_capture_order_id(
                "paypal",
                Some("CHECKOUT.ORDER.APPROVED"),
                Some("5O190127TN364715T")
            ),
            Some("5O190127TN364715T")
        );
        // Case and surrounding whitespace are normalised by the provider layer.
        assert_eq!(
            paypal_capture_order_id(" PayPal ", Some(" checkout.order.approved "), Some(" ORDER-1 ")),
            Some("ORDER-1")
        );
    }

    #[test]
    fn other_channels_and_events_never_capture() {
        assert_eq!(
            paypal_capture_order_id("stripe", Some(PAYPAL_ORDER_APPROVED_EVENT), Some("x")),
            None
        );
        assert_eq!(
            paypal_capture_order_id("paypal", Some("PAYMENT.CAPTURE.COMPLETED"), Some("x")),
            None
        );
        assert_eq!(
            paypal_capture_order_id("paypal", Some("CHECKOUT.ORDER.CREATED"), Some("x")),
            None
        );
        assert_eq!(paypal_capture_order_id("paypal", None, Some("x")), None);
    }

    #[test]
    fn an_unusable_resource_id_never_becomes_a_request_path() {
        assert_eq!(paypal_capture_order_id("paypal", Some(PAYPAL_ORDER_APPROVED_EVENT), None), None);
        assert_eq!(
            paypal_capture_order_id("paypal", Some(PAYPAL_ORDER_APPROVED_EVENT), Some("   ")),
            None
        );
        // A crafted id must not steer the signed capture URL elsewhere.
        for hostile in ["../../v2/payments", "ORDER?x=1", "ORDER#frag", "ORD ER"] {
            assert_eq!(
                paypal_capture_order_id("paypal", Some(PAYPAL_ORDER_APPROVED_EVENT), Some(hostile)),
                None,
                "{hostile} must be rejected"
            );
        }
        let too_long = "A".repeat(129);
        assert_eq!(
            paypal_capture_order_id("paypal", Some(PAYPAL_ORDER_APPROVED_EVENT), Some(&too_long)),
            None
        );
    }
}
