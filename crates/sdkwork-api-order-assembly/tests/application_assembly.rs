use sdkwork_api_order_assembly::OrderAssemblyContract;
use sdkwork_web_core::RouteAuth;

#[test]
fn application_manifest_exports_complete_recharge_surface() {
    let manifest = OrderAssemblyContract::app_route_manifest();
    let packages = manifest
        .match_route("GET", "/app/v3/api/recharges/packages")
        .expect("recharge packages route must be exported by the assembly");
    let create_order = manifest
        .match_route("POST", "/app/v3/api/recharges/orders")
        .expect("recharge order route must be exported by the assembly");
    let create_membership_order = manifest
        .match_route("POST", "/app/v3/api/memberships/orders")
        .expect("membership order route must be exported by the assembly");
    let redeem_coupon = manifest
        .match_route("POST", "/app/v3/api/orders/coupon_redemptions")
        .expect("coupon redemption route must be exported by the assembly");

    assert_eq!(RouteAuth::Public, packages.auth);
    assert_eq!("recharges.packages.list", packages.operation_id);
    assert_eq!(RouteAuth::DualToken, create_order.auth);
    assert_eq!("recharges.orders.create", create_order.operation_id);
    assert_eq!(RouteAuth::DualToken, create_membership_order.auth);
    assert_eq!(
        "memberships.orders.create",
        create_membership_order.operation_id
    );
    assert_eq!(RouteAuth::DualToken, redeem_coupon.auth);
    assert_eq!(
        "orders.couponRedemptions.create",
        redeem_coupon.operation_id
    );
}
