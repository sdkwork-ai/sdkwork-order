const ordersService = require("../../services/order-service");
const shipmentService = require("../../services/shipment-service");
const { formatMinor, formatTime, orderStatusLabel, assetLabel } = require("../../utils/format");

Page({
  data: {
    detail: null,
    events: [],
    shipment: null,
    loading: true,
    loadingEvents: false,
    error: "",
  },

  onLoad(options) {
    this.orderId = options.id || "";
  },

  onShow() {
    this.loadDetail();
    this.loadEvents();
  },

  async loadDetail() {
    this.setData({ loading: true, error: "" });
    try {
      const detail = await ordersService.getOrderDetail(this.orderId);
      this.setData({
        detail: {
          ...detail,
          statusText: detail.statusName || orderStatusLabel(detail.status),
          totalText: formatMinor(detail.totalAmount),
          paidText: detail.paidAmount != null ? formatMinor(detail.paidAmount) : "未支付",
          discountText: detail.discountAmount != null ? formatMinor(detail.discountAmount) : null,
          timeText: formatTime(detail.createdAt),
          expireText: detail.expireTime ? formatTime(detail.expireTime) : "",
          items: detail.items.map((item) => ({
            ...item,
            priceText: formatMinor(item.unitPrice),
            lineTotalText: formatMinor(item.totalAmount),
          })),
        },
        loading: false,
      });
      // 物流追踪：仅待收货/已完成拉取；其余状态清空入口。
      const status = String(detail.status || "").trim().toLowerCase();
      if (status === "fulfilled" || status === "completed") {
        this.loadShipment();
      } else {
        this.setData({ shipment: null });
      }
    } catch (cause) {
      this.setData({
        loading: false,
        error: cause && cause.message ? cause.message : "订单加载失败",
      });
    }
  },

  /** 物流追踪组装：无履约返回 null（卡片隐藏）；读取失败降级为不渲染。 */
  async loadShipment() {
    try {
      const assembled = await shipmentService.getOrderShipment(this.orderId);
      if (!assembled || !assembled.shipment) {
        this.setData({ shipment: null });
        return;
      }
      this.setData({
        shipment: {
          shipment: assembled.shipment,
          events: assembled.events.map((event) => ({
            ...event,
            timeText: formatTime(event.eventTime),
          })),
        },
      });
    } catch (cause) {
      this.setData({ shipment: null });
    }
  },

  async loadEvents() {
    this.setData({ loadingEvents: true });
    try {
      const result = await ordersService.getOrderEvents(this.orderId, { pageSize: 20 });
      this.setData({
        events: result.items.map((event) => ({
          eventId: event.eventId,
          title: event.message || event.eventType,
          statusText: orderStatusLabel(event.toStatus),
          actorText: event.actorType ? `${event.actorType}${event.actorId ? ` · ${event.actorId}` : ""}` : "",
          timeText: formatTime(event.createdAt),
        })),
        loadingEvents: false,
      });
    } catch (cause) {
      this.setData({ loadingEvents: false });
    }
  },

  copyOrderSn() {
    const detail = this.data.detail;
    if (detail && detail.orderSn) {
      wx.setClipboardData({ data: detail.orderSn });
    }
  },

  goPay() {
    wx.navigateTo({ url: `/pages/cashier/index?orderId=${this.orderId}` });
  },

  goRefund() {
    wx.navigateTo({ url: `/pages/refund/index?orderId=${this.orderId}` });
  },

  async cancel() {
    const confirmed = await new Promise((resolve) => {
      wx.showModal({
        title: "取消订单",
        content: "确定取消该订单吗？",
        success: (res) => resolve(res.confirm === true),
        fail: () => resolve(false),
      });
    });
    if (!confirmed) {
      return;
    }
    try {
      await ordersService.cancelOrder(this.orderId);
      wx.showToast({ title: "已取消", icon: "success" });
      this.loadDetail();
      this.loadEvents();
    } catch (cause) {
      wx.showToast({ title: cause && cause.message ? cause.message : "取消失败", icon: "none" });
    }
  },

  async confirmReceipt() {
    try {
      await ordersService.confirmReceipt(this.orderId);
      wx.showToast({ title: "已确认收货", icon: "success" });
      this.loadDetail();
      this.loadEvents();
    } catch (cause) {
      wx.showToast({ title: cause && cause.message ? cause.message : "确认收货失败", icon: "none" });
    }
  },

  goOrders() {
    wx.navigateBack({
      fail() {
        wx.switchTab({ url: "/pages/orders/index" });
      },
    });
  },
});
