const ordersService = require("../../services/order-service");
const rechargeService = require("../../services/recharge-service");

/**
 * Payment result page.
 *
 * mode=order (default): polls GET /orders/{orderId}/payment_success.
 * mode=recharge: polls GET /recharges/orders/{orderId} (recharge orders are
 * paid through the recharge create flow, not the order payments endpoint).
 * Both modes cap polling at a bounded window, then take one final terminal
 * reading before settling.
 */
const POLL_WINDOW_MS = 2 * 60 * 1000;

const PAID_STATUSES = ["paid", "completed", "succeeded"];
const FAILED_STATUSES = ["failed", "cancelled", "expired", "rejected"];

Page({
  data: {
    orderId: "",
    paymentId: "",
    paymentMethod: "",
    mode: "order",
    status: "pending",
    title: "支付处理中",
    description: "支付正在确认，请稍候。",
    tone: "warning",
    loading: true,
  },

  onLoad(options) {
    this.setData({
      orderId: options.orderId || "",
      paymentId: options.paymentId || "",
      paymentMethod: options.paymentMethod || "",
      mode: options.mode === "recharge" ? "recharge" : "order",
      status: options.status || "pending",
    });
    this.render(this.data.status);
    if (this.data.status === "pending" && this.data.orderId) {
      this.poll();
    } else {
      this.setData({ loading: false });
    }
  },

  onUnload() {
    this.stopPolling = true;
  },

  render(status) {
    if (status === "success") {
      this.setData({
        status: "success",
        title: "支付成功",
        description: this.data.mode === "recharge" ? "充值已到账。" : "商家将尽快处理您的订单。",
        tone: "success",
        loading: false,
      });
    } else if (status === "failed") {
      this.setData({
        status: "failed",
        title: "支付失败",
        description: "支付未成功，可重新支付或联系客服。",
        tone: "danger",
        loading: false,
      });
    } else if (status === "expired") {
      this.setData({
        status: "expired",
        title: "支付已超时",
        description: "确认超时，请稍后在订单中核实最新状态。",
        tone: "warning",
        loading: false,
      });
    }
  },

  async poll() {
    this.stopPolling = false;
    try {
      const result =
        this.data.mode === "recharge"
          ? await this.pollRecharge()
          : await ordersService.pollPaymentSuccess(this.data.orderId, {
              deadlineMs: Date.now() + POLL_WINDOW_MS,
            });
      if (this.stopPolling) {
        return;
      }
      if (result.paid) {
        this.render("success");
      } else if (result.timedOut) {
        this.render("expired");
      } else {
        this.render("failed");
      }
    } catch (cause) {
      if (this.stopPolling) {
        return;
      }
      this.render("expired");
    }
  },

  async pollRecharge() {
    const deadline = Date.now() + POLL_WINDOW_MS;
    let delay = 3000;
    for (;;) {
      const order = await rechargeService.getRechargeOrder(this.data.orderId);
      const status = String(order.status || "").toLowerCase();
      if (PAID_STATUSES.includes(status)) {
        return { paid: true, timedOut: false };
      }
      if (FAILED_STATUSES.includes(status)) {
        return { paid: false, timedOut: false };
      }
      if (Date.now() >= deadline) {
        return { paid: false, timedOut: true };
      }
      await new Promise((resolve) => setTimeout(resolve, delay));
      delay = Math.min(Math.round(delay * 1.5), 8000);
    }
  },

  goOrders() {
    wx.switchTab({ url: "/pages/orders/index" });
  },

  goRecharge() {
    wx.switchTab({ url: "/pages/recharge/index" });
  },

  retryPay() {
    if (!this.data.orderId) {
      return;
    }
    wx.redirectTo({
      url:
        `/pages/cashier/index?orderId=${this.data.orderId}` +
        (this.data.paymentMethod ? `&paymentMethod=${this.data.paymentMethod}` : ""),
    });
  },
});
