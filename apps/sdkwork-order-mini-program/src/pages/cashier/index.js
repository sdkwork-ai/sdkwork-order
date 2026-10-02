const ordersService = require("../../services/order-service");
const { formatMinor, formatCountdown, orderStatusLabel } = require("../../utils/format");

/**
 * Cashier page: pick a payment method → create the payment session
 * (POST /orders/{orderId}/payments) → poll payment_success with backoff and a
 * deadline final check → jump to the payment result page.
 *
 * `paymentSession.paymentParams` is the wx.requestPayment handoff point: once
 * the WeChat pay provider contract for MP lands, launchProviderPayment calls
 * wx.requestPayment with those params before polling starts (docs/decisions.md).
 */

// WeChat MP runs inside the WeChat app; JSAPI is the primary channel. The
// backend whitelist is wechat_pay | wechat_jsapi | alipay | alipay_wap | balance.
const PAYMENT_METHODS = [
  { code: "wechat_jsapi", label: "微信支付" },
  { code: "wechat_pay", label: "微信支付（扫码）" },
  { code: "balance", label: "余额支付" },
];

Page({
  data: {
    orderId: "",
    subject: "",
    amountText: "",
    expireTime: "",
    methods: PAYMENT_METHODS,
    selectedCode: PAYMENT_METHODS[0].code,
    countdownText: "",
    phase: "loading",
    paying: false,
    error: "",
  },

  onLoad(options) {
    this.setData({ orderId: options.orderId || "" });
    this.preselectedCode = options.paymentMethod || "";
  },

  onShow() {
    this.loadOrder();
  },

  onHide() {
    this.stopPolling = true;
  },

  onUnload() {
    this.stopPolling = true;
  },

  async loadOrder() {
    if (!this.data.orderId) {
      this.setData({ phase: "failed", error: "缺少订单参数" });
      return;
    }
    try {
      const detail = await ordersService.getOrderDetail(this.data.orderId);
      const selected = PAYMENT_METHODS.some((method) => method.code === this.preselectedCode)
        ? this.preselectedCode
        : PAYMENT_METHODS[0].code;
      this.setData({
        subject: detail.subject,
        amountText: formatMinor(detail.totalAmount),
        expireTime: detail.expireTime || "",
        selectedCode: selected,
        phase: "ready",
      });
    } catch (cause) {
      this.setData({
        phase: "failed",
        error: cause && cause.message ? cause.message : "订单加载失败",
      });
    }
  },

  selectMethod(event) {
    this.setData({ selectedCode: event.currentTarget.dataset.code });
  },

  async pay() {
    const { orderId, selectedCode } = this.data;
    if (!selectedCode) {
      this.setData({ error: "请选择支付方式" });
      return;
    }
    this.setData({ paying: true, error: "" });
    try {
      const paymentSession = await ordersService.createPayment(orderId, selectedCode);
      await this.launchProviderPayment(paymentSession);
      this.startPolling(paymentSession);
    } catch (cause) {
      this.setData({
        paying: false,
        error: cause && cause.message ? cause.message : "发起支付失败，请稍后再试",
      });
    }
  },

  /**
   * Provider launch seam. The backend returns paymentParams for the selected
   * channel; until the WeChat pay provider contract lands, the MP cashier
   * relies on backend-side settlement and goes straight to polling.
   */
  async launchProviderPayment(paymentSession) {
    if (paymentSession && paymentSession.paymentMethod === "wechat_jsapi" && false) {
      // Future plug point:
      // wx.requestPayment({ ...paymentSession.paymentParams, success, fail });
    }
    return Promise.resolve(paymentSession);
  },

  startPolling(paymentSession) {
    this.stopPolling = false;
    const expireMs = this.data.expireTime ? Date.parse(this.data.expireTime) : Number.NaN;
    const deadlineMs = Math.min(
      Number.isFinite(expireMs) ? expireMs : Number.POSITIVE_INFINITY,
      Date.now() + ordersService.CASHIER_TTL_MS,
    );
    ordersService
      .pollPaymentSuccess(orderIdSafe(paymentSession, this.data.orderId), {
        deadlineMs,
        onCountdown: (remainingSeconds) => {
          if (!this.stopPolling) {
            this.setData({ countdownText: formatCountdown(remainingSeconds), phase: "polling" });
          }
        },
      })
      .then((result) => {
        if (this.stopPolling) {
          return;
        }
        const statusParam = result.paid ? "success" : result.timedOut ? "expired" : "failed";
        wx.redirectTo({
          url:
            `/pages/payment-result/index?orderId=${this.data.orderId}` +
            `&paymentId=${paymentSession.paymentId}&paymentMethod=${paymentSession.paymentMethod}` +
            `&status=${statusParam}`,
        });
      })
      .catch((cause) => {
        if (this.stopPolling) {
          return;
        }
        this.setData({
          paying: false,
          phase: "ready",
          error: cause && cause.message ? cause.message : "支付结果查询失败",
        });
      });
  },

  cancel() {
    wx.redirectTo({ url: "/pages/orders/index?status=pending_payment" });
  },
});

function orderIdSafe(paymentSession, fallback) {
  return paymentSession && paymentSession.orderId ? paymentSession.orderId : fallback;
}
