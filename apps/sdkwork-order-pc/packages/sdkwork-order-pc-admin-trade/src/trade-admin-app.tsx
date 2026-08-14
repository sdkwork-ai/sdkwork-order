import type { OrderAdminService } from "@sdkwork/order-pc-admin-orders";
import { SdkworkOrderAdminOrdersPage } from "@sdkwork/order-pc-admin-orders";
import type { TradeAdminService } from "./trade-admin-service";
import type { TradeAdminIntlProps } from "./i18n/intl";
import {
  SdkworkOrderAfterSalesPage,
  SdkworkOrderRefundsPage,
  SdkworkOrderShipmentsPage,
  SdkworkOrderTradeWorkbenchPage,
  SdkworkOrderWithdrawalsPage,
} from "./pages";
import {
  TRADE_ADMIN_SECTIONS,
  type SdkworkOrderAdminTradeSection,
} from "./contribution";

export interface TradeAdminCapabilities {
  canManageOrders: boolean;
  canReviewTrade: boolean;
}

/** Optional service injection for embedding hosts and tests. */
export interface TradeCenterAdminServices {
  orders?: OrderAdminService;
  trade?: TradeAdminService;
}

export interface SdkworkOrderTradeCenterAdminAppProps extends TradeAdminIntlProps {
  sectionId?: string;
  capabilities: TradeAdminCapabilities;
  services?: TradeCenterAdminServices;
}

function resolveSection(sectionId?: string): SdkworkOrderAdminTradeSection {
  if (TRADE_ADMIN_SECTIONS.includes(sectionId as SdkworkOrderAdminTradeSection)) {
    return sectionId as SdkworkOrderAdminTradeSection;
  }
  return "overview";
}

/**
 * Trading center admin application owned by `sdkwork-order`.
 *
 * Dispatches the trading center section (route `:sectionId?`) to the
 * workbench, order supervision, after-sales, shipments, refunds, and
 * withdrawal screens. Operation capabilities and copy locale are injected by
 * the embedding host — this package never reads host session, permission, or
 * i18n state.
 */
export function SdkworkOrderTradeCenterAdminApp({
  sectionId,
  capabilities,
  services,
  locale,
  messages,
}: SdkworkOrderTradeCenterAdminAppProps) {
  switch (resolveSection(sectionId)) {
    case "orders":
      return (
        <SdkworkOrderAdminOrdersPage
          capabilities={{ canManageOrders: capabilities.canManageOrders }}
          locale={locale}
          service={services?.orders}
        />
      );
    case "after-sales":
      return <SdkworkOrderAfterSalesPage canManage={capabilities.canReviewTrade} locale={locale} messages={messages} service={services?.trade} />;
    case "shipments":
      return <SdkworkOrderShipmentsPage canManage={capabilities.canReviewTrade} locale={locale} messages={messages} service={services?.trade} />;
    case "refunds":
      return <SdkworkOrderRefundsPage canManage={capabilities.canReviewTrade} locale={locale} messages={messages} service={services?.trade} />;
    case "withdrawals":
      return <SdkworkOrderWithdrawalsPage canManage={capabilities.canReviewTrade} locale={locale} messages={messages} service={services?.trade} />;
    default:
      return <SdkworkOrderTradeWorkbenchPage locale={locale} messages={messages} service={services?.trade} />;
  }
}

export default SdkworkOrderTradeCenterAdminApp;
