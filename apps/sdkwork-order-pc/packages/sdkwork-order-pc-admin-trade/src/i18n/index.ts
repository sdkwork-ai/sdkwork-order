import { tradeAdminEnUsMessages } from "./en-US/commerce/order/admin-trade";
import { tradeAdminZhCnMessages } from "./zh-CN/commerce/order/admin-trade";

export const adminTradeMessages = {
  en: tradeAdminEnUsMessages,
  zh: tradeAdminZhCnMessages,
};

export {
  TradeAdminIntlProvider,
  useTradeAdminI18n,
  type TradeAdminIntlProps,
  type TradeAdminIntlProviderProps,
  type TradeAdminIntlValue,
  type TradeAdminMessageKey,
  type TradeAdminMessagesOverrides,
} from "./intl";
