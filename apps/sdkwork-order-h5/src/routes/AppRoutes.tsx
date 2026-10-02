import { lazy, Suspense } from "react";
import { Route, Routes } from "react-router-dom";

import { ORDER_MOBILE_ROUTE_DEFINITIONS } from "@sdkwork/order-mobile-react-orders";

import type { SdkworkOrderH5Runtime } from "../bootstrap/runtime";
import { SDKWORK_ORDER_H5_HOST_ROUTES } from "../bootstrap/routes";

type OrderComponentName =
  | "OrderCenter"
  | "OrderDetail"
  | "CashierPage"
  | "VoucherCodePage"
  | "RefundRequestsPage";

/**
 * Lazy-loads an order page while preserving its exact props type. Named
 * property access (not a union index) keeps the component signature intact
 * so the host can inject route-path props (im-h5 module pattern).
 */
function lazyOrderComponent<K extends OrderComponentName>(name: K) {
  return lazy(async () => {
    const module = await import("@sdkwork/order-mobile-react-orders");
    return { default: module[name] };
  });
}

const OrderCenter = lazyOrderComponent("OrderCenter");
const OrderDetail = lazyOrderComponent("OrderDetail");
const CashierPage = lazyOrderComponent("CashierPage");
const VoucherCodePage = lazyOrderComponent("VoucherCodePage");
const RefundRequestsPage = lazyOrderComponent("RefundRequestsPage");

const VipSubscriptionPage = lazy(async () => {
  const module = await import("@sdkwork/order-h5-subscription");
  return { default: module.VipSubscriptionPage };
});
const TokenBankPurchasePage = lazy(async () => {
  const module = await import("@sdkwork/order-h5-subscription");
  return { default: module.TokenBankPurchasePage };
});
const CouponRedemptionPage = lazy(async () => {
  const module = await import("@sdkwork/order-h5-subscription");
  return { default: module.CouponRedemptionPage };
});
const WithdrawPage = lazy(async () => {
  const module = await import("@sdkwork/order-h5-withdraw");
  return { default: module.WithdrawPage };
});
const ValueHomePage = lazy(async () => {
  const module = await import("@sdkwork/order-h5-shell");
  return { default: module.SdkworkOrderH5ValueHomePage };
});

function LoadingPlaceholder() {
  return <div className="sdk-h5-loading">加载中...</div>;
}

function NotFoundPage() {
  return (
    <div className="sdk-h5-page">
      <div className="sdk-h5-empty">页面不存在或已下线</div>
      <div className="sdk-h5-center">
        <a className="sdk-h5-button sdk-h5-button-primary" href="/orders">回到订单</a>
      </div>
    </div>
  );
}

export function AppRoutes({ runtime }: { runtime: SdkworkOrderH5Runtime }) {
  return (
    <Suspense fallback={<LoadingPlaceholder />}>
      <Routes>
        <Route
          element={<OrderCenter />}
          path={ORDER_MOBILE_ROUTE_DEFINITIONS.orderCenter.path}
        />
        <Route
          element={(
            <CashierPage
              orderCenterPath={ORDER_MOBILE_ROUTE_DEFINITIONS.orderCenter.path}
              orderDetailPath={ORDER_MOBILE_ROUTE_DEFINITIONS.orderDetail.path}
            />
          )}
          path={ORDER_MOBILE_ROUTE_DEFINITIONS.orderCashier.path}
        />
        <Route
          element={<VoucherCodePage />}
          path={ORDER_MOBILE_ROUTE_DEFINITIONS.voucherCode.path}
        />
        <Route
          element={<OrderDetail />}
          path={ORDER_MOBILE_ROUTE_DEFINITIONS.orderDetail.path}
        />
        <Route
          element={<RefundRequestsPage />}
          path={ORDER_MOBILE_ROUTE_DEFINITIONS.refundRequests.path}
        />
        <Route
          element={<ValueHomePage />}
          path={SDKWORK_ORDER_H5_HOST_ROUTES.valueHome.path}
        />
        <Route
          element={(
            <VipSubscriptionPage
              cashierPath={ORDER_MOBILE_ROUTE_DEFINITIONS.orderCashier.path}
              service={runtime.orderRuntime.resolveSubscriptionPurchaseService()}
            />
          )}
          path={SDKWORK_ORDER_H5_HOST_ROUTES.valueVip.path}
        />
        <Route
          element={(
            <TokenBankPurchasePage
              cashierPath={ORDER_MOBILE_ROUTE_DEFINITIONS.orderCashier.path}
              service={runtime.orderRuntime.resolveSubscriptionPurchaseService()}
            />
          )}
          path={SDKWORK_ORDER_H5_HOST_ROUTES.valueTokenBank.path}
        />
        <Route
          element={(
            <CouponRedemptionPage
              service={runtime.orderRuntime.resolveSubscriptionPurchaseService()}
            />
          )}
          path={SDKWORK_ORDER_H5_HOST_ROUTES.valueCoupon.path}
        />
        <Route
          element={(
            <WithdrawPage service={runtime.orderRuntime.resolveWithdrawalRequestService()} />
          )}
          path={SDKWORK_ORDER_H5_HOST_ROUTES.withdraw.path}
        />
        <Route element={<NotFoundPage />} path="*" />
      </Routes>
    </Suspense>
  );
}
