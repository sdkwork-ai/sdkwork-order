import i18n from "i18next";
import { initReactI18next } from "react-i18next";

/**
 * Shell-owned react-i18next initialization for the order H5 root (im-h5
 * commons pattern: the side-effect module is imported by the shell barrel, so
 * consuming the shell readies i18n before any `useTranslation` call).
 *
 * The default locale is `zh-CN`; the pages of the composed mobile packages
 * render inline fallbacks for keys this fragment does not carry, so the
 * surface stays readable in every host locale.
 */
const ORDER_H5_I18N_RESOURCES = {
  zh: {
    translation: {
      shell: {
        app_title: "SDKWork 订单",
        tab_orders: "订单",
        tab_value: "价值",
        tab_withdraw: "提现",
        value_title: "账户价值",
        value_subtitle: "会员订阅、算力储备与券码兑换",
        value_vip: "VIP 订阅",
        value_vip_desc: "开通/续费会员权益",
        value_token_bank: "Token Bank",
        value_token_bank_desc: "算力积分充值",
        value_coupon: "券码兑换",
        value_coupon_desc: "兑换优惠券到账户",
      },
    },
  },
  en: {
    translation: {
      shell: {
        app_title: "SDKWork Orders",
        tab_orders: "Orders",
        tab_value: "Value",
        tab_withdraw: "Withdraw",
        value_title: "Account Value",
        value_subtitle: "Membership, Token Bank and coupon redemption",
        value_vip: "VIP Subscription",
        value_vip_desc: "Purchase or renew membership",
        value_token_bank: "Token Bank",
        value_token_bank_desc: "Top up compute points",
        value_coupon: "Coupon Code",
        value_coupon_desc: "Redeem a coupon to your account",
      },
    },
  },
} as const;

if (!i18n.isInitialized) {
  void i18n
    .use(initReactI18next)
    .init({
      defaultNS: "translation",
      fallbackLng: "zh-CN",
      interpolation: {
        escapeValue: false,
      },
      lng: "zh-CN",
      resources: ORDER_H5_I18N_RESOURCES,
      returnNull: false,
    });
}

export default i18n;
