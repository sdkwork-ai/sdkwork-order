import { adminOrdersEnUsMessages } from "./en-US/commerce/order/admin-orders";
import { adminOrdersZhCnMessages } from "./zh-CN/commerce/order/admin-orders";

export const adminOrdersMessages = {
  en: adminOrdersEnUsMessages,
  zh: adminOrdersZhCnMessages,
};

export {
  AdminOrdersIntlProvider,
  useAdminOrdersI18n,
  type AdminOrdersIntlProps,
  type AdminOrdersIntlProviderProps,
  type AdminOrdersIntlValue,
  type AdminOrdersMessageKey,
  type AdminOrdersMessagesOverrides,
} from "./intl";
