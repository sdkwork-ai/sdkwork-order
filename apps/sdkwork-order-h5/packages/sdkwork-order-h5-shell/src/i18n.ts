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
        value_token_bank_desc: "算力额度充值",
        value_coupon: "券码兑换",
        value_coupon_desc: "兑换优惠券到账户",
        value_refunds: "我的退款",
        value_refunds_desc: "退款申请与进度",
      },
      subscription: {
        coupon_title: "优惠券兑换",
        coupon_banner_title: "兑换优惠券",
        coupon_banner_desc: "输入兑换码，激活会员权益或向 Token Bank 存入算力额度。",
        coupon_code_label: "兑换码",
        coupon_code_placeholder: "请输入兑换码",
        coupon_code_required: "请输入兑换码",
        coupon_redeem: "立即兑换",
        coupon_redeeming: "正在兑换...",
        coupon_redeemed: "兑换成功",
        coupon_token_bank_credited: "算力额度已存入 Token Bank：{{amount}}",
        coupon_points_credited: "会员积分已发放：{{amount}}",
        coupon_cash_credited: "现金已发放：{{amount}}",
        coupon_subscription_activated: "会员订阅已激活",
        coupon_credited: "权益已发放",
        coupon_redeem_again: "继续兑换",
        coupon_notice: "温馨提示：兑换码激活的权益以平台规则为准，不可转赠、不可提现。",
        coupon_benefit_delivery_failed: "券已核销，但权益发放失败；请重试，重试不会重复扣券。",
        coupon_invalid_or_expired: "兑换码无效或已过期，请检查兑换码。",
        coupon_already_used: "该优惠券已被使用。",
        coupon_request_failed: "兑换请求未完成，请稍后重试。",
        coupon_support_hint: "如多次失败，请联系客服并提供追踪号",
        coupon_trace_id: "追踪号",
        coupon_trace_id_copy: "复制",
        coupon_trace_id_copied: "已复制",
        coupon_retry: "重试",
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
        value_refunds: "My Refunds",
        value_refunds_desc: "Refund requests and progress",
      },
      subscription: {
        coupon_title: "Redeem a coupon",
        coupon_banner_title: "Redeem your coupon",
        coupon_banner_desc:
          "Enter a redemption code to activate membership benefits or add compute quota to your Token Bank.",
        coupon_code_label: "Redemption code",
        coupon_code_placeholder: "Enter your redemption code",
        coupon_code_required: "Enter a redemption code",
        coupon_redeem: "Redeem now",
        coupon_redeeming: "Redeeming...",
        coupon_redeemed: "Redeemed",
        coupon_token_bank_credited: "Compute quota added to Token Bank: {{amount}}",
        coupon_points_credited: "Membership points credited: {{amount}}",
        coupon_cash_credited: "Cash credited: {{amount}}",
        coupon_subscription_activated: "Membership subscription activated",
        coupon_credited: "Benefit granted",
        coupon_redeem_again: "Redeem another code",
        coupon_notice:
          "Note: benefits activated by a redemption code follow the platform rules and cannot be transferred or withdrawn.",
        coupon_benefit_delivery_failed:
          "The coupon was redeemed, but the benefit could not be delivered. Please retry — retrying will not consume another coupon.",
        coupon_invalid_or_expired: "This code is invalid or has expired. Check the code and try again.",
        coupon_already_used: "This coupon has already been used.",
        coupon_request_failed: "The redemption request did not complete. Please try again.",
        coupon_support_hint: "If it keeps failing, contact support and provide the trace ID.",
        coupon_trace_id: "Trace ID",
        coupon_trace_id_copy: "Copy",
        coupon_trace_id_copied: "Copied",
        coupon_retry: "Retry",
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
