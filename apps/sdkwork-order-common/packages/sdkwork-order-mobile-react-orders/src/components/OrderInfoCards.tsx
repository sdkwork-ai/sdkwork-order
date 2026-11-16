import { useTranslation } from "react-i18next";

import { formatOrderDateTime } from "../services/datetime";
import React from "react";
import { showToast } from "@sdkwork/ui-mobile-react";
import {
  formatAmountCny,
  ORDER_PAYMENT_METHOD_LABELS,
  type Order,
  type OrderPaymentMethod,
} from "../services/OrderService";

const PAYMENT_METHOD_ALIASES: Readonly<Record<string, OrderPaymentMethod>> = {
  wechat: "wechat_pay",
  wxpay: "wechat_pay",
  wechat_native: "wechat_pay",
  wechat_jsapi: "wechat_jsapi",
  alipay: "alipay",
  alipay_qr: "alipay",
  alipay_wap: "alipay_wap",
  balance: "balance",
  paypal: "paypal",
  stripe: "stripe_card",
  card: "stripe_card",
};

const EXTRA_PAYMENT_LABELS: Readonly<Record<string, string>> = {
  unionpay: "银联支付",
  cash: "货到付款",
  cod: "货到付款",
  wallet: "钱包余额",
};

function normalizePaymentMethodCode(code: string): string {
  return code.trim().toLowerCase().replace(/[\s-]+/g, "_");
}

function resolvePaymentMethodLabel(code: string): string {
  const normalized = normalizePaymentMethodCode(code);
  const alias = PAYMENT_METHOD_ALIASES[normalized];
  if (alias) {
    return ORDER_PAYMENT_METHOD_LABELS[alias];
  }
  return EXTRA_PAYMENT_LABELS[normalized] ?? code;
}

interface OrderInfoCardsProps {
  order: Order;
}

export const OrderInfoCards: React.FC<OrderInfoCardsProps> = ({ order }) => {
  const { t, i18n } = useTranslation();
  const payable = order.paidAmount ?? order.totalAmount;

  const copyOrderId = async () => {
    try {
      await navigator.clipboard.writeText(order.orderSn);
      showToast(t("orders.copy_success", "复制成功"));
    } catch {
      showToast(t("orders.copy_failed", "复制失败"));
    }
  };

  return (
    <>
      <div className="bg-chat-other-bg rounded-xl p-4 shadow-sm flex flex-col gap-3">
        <h3 className="text-[14px] font-bold text-text-main mb-1">
          {t("orders.order_info", "订单信息")}
        </h3>
        <div className="flex items-center justify-between">
          <span className="text-[13px] text-text-sub">
            {t("orders.order_no", "订单编号")}
          </span>
          <div className="flex items-center gap-2">
            <span className="text-[13px] text-text-main">{order.orderSn}</span>
            <button
              onClick={copyOrderId}
              className="min-h-8 rounded border border-primary-blue/30 px-2 py-1 text-[12px] text-primary-blue active:bg-primary-blue/10"
            >
              {t("orders.copy", "复制")}
            </button>
          </div>
        </div>
        <div className="flex items-center justify-between">
          <span className="text-[13px] text-text-sub">
            {t("orders.create_time", "创建时间")}
          </span>
          <span className="text-[13px] text-text-main">{formatOrderDateTime(order.createdAt, i18n.language)}</span>
        </div>
        {order.payTime && (
          <div className="flex items-center justify-between">
            <span className="text-[13px] text-text-sub">
              {t("orders.pay_time", "付款时间")}
            </span>
            <span className="text-[13px] text-text-main">{formatOrderDateTime(order.payTime, i18n.language)}</span>
          </div>
        )}
        {order.expireTime && (
          <div className="flex items-center justify-between">
            <span className="text-[13px] text-text-sub">
              {t("orders.expire_time", "关闭时间")}
            </span>
            <span className="text-[13px] text-text-main">{order.expireTime}</span>
          </div>
        )}
        {order.outTradeNo && (
          <div className="flex items-center justify-between">
            <span className="text-[13px] text-text-sub">
              {t("orders.out_trade_no", "商户单号")}
            </span>
            <span className="text-[13px] text-text-main">{order.outTradeNo}</span>
          </div>
        )}
        {order.paymentMethod && (
          <div className="flex items-center justify-between">
            <span className="text-[13px] text-text-sub">
              {t("orders.payment_method", "支付方式")}
            </span>
            <span className="text-[13px] text-text-main">
              {t(
                `orders.payment_method_${normalizePaymentMethodCode(order.paymentMethod)}`,
                resolvePaymentMethodLabel(order.paymentMethod),
              )}
            </span>
          </div>
        )}
      </div>

      <div className="bg-chat-other-bg rounded-xl p-4 shadow-sm flex flex-col gap-3">
        <div className="flex items-center justify-between">
          <span className="text-[13px] text-text-sub">
            {t("orders.goods_total", "商品总价")}
          </span>
          <span className="text-[13px] text-text-main">
            {formatAmountCny(order.totalAmount, order.currencyCode, i18n.language)}
          </span>
        </div>
        {order.discountAmount && Number(order.discountAmount) > 0 && (
          <div className="flex items-center justify-between">
            <span className="text-[13px] text-text-sub">
              {t("orders.discount", "优惠金额")}
            </span>
            <span className="text-[13px] text-text-main text-[#FA5151]">
              -{formatAmountCny(order.discountAmount, order.currencyCode, i18n.language)}
            </span>
          </div>
        )}
        <div className="pt-3 border-t border-border-color/50 flex items-center justify-between">
          <span className="text-[14px] font-bold text-text-main">
            {t("orders.payable", "实付款")}
          </span>
          <span className="text-[18px] font-bold text-[#FA5151]">
            {formatAmountCny(payable, order.currencyCode, i18n.language)}
          </span>
        </div>
      </div>
    </>
  );
};
