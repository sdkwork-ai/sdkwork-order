const ordersService = require("../../services/order-service");
const session = require("../../services/session");
const { formatMinor, yuanInputToMinor, requestStatusLabel } = require("../../utils/format");

const TARGET_ASSETS = ["cash", "token_bank", "points"];

Page({
  data: {
    orderId: "",
    amountText: "",
    assetIndex: 0,
    targetAssets: TARGET_ASSETS,
    assetLabels: ["现金", "Token Bank", "积分"],
    reasonCode: "",
    reasonDetail: "",
    submitting: false,
    history: [],
    loadingHistory: true,
    error: "",
  },

  onLoad(options) {
    this.setData({ orderId: options.orderId || "" });
  },

  onShow() {
    if (!session.isLoggedIn()) {
      wx.navigateTo({ url: "/pages/login/index" });
      return;
    }
    this.loadHistory();
  },

  onOrderIdInput(event) {
    this.setData({ orderId: event.detail.value });
  },

  onAmountInput(event) {
    this.setData({ amountText: event.detail.value });
  },

  onReasonCodeInput(event) {
    this.setData({ reasonCode: event.detail.value });
  },

  onReasonDetailInput(event) {
    this.setData({ reasonDetail: event.detail.value });
  },

  selectAsset(event) {
    this.setData({ assetIndex: Number(event.detail.value) || 0 });
  },

  async loadHistory() {
    this.setData({ loadingHistory: true });
    try {
      const result = await ordersService.listRefundRequests({ page: 1, pageSize: 10 });
      this.setData({
        history: result.items.map((entry) => ({
          accountValueRequestId: entry.accountValueRequestId,
          title: entry.originalOrderId ? `原订单退款 ${formatMinor(entry.amount)}` : `退款 ${formatMinor(entry.amount)}`,
          statusText: requestStatusLabel(entry.status),
          timeText: entry.createdAt,
        })),
        loadingHistory: false,
      });
    } catch (cause) {
      this.setData({ loadingHistory: false });
    }
  },

  async submit() {
    if (this.data.submitting) {
      return;
    }
    const orderId = this.data.orderId.trim();
    if (!orderId) {
      this.setData({ error: "请输入原订单 ID（可在订单详情复制）" });
      return;
    }
    const amountMinor = yuanInputToMinor(this.data.amountText);
    if (!amountMinor || amountMinor === "0") {
      this.setData({ error: "请输入有效的退款金额（最多两位小数）" });
      return;
    }
    this.setData({ submitting: true, error: "" });
    try {
      await ordersService.createRefundRequest({
        originalOrderId: orderId,
        targetAsset: TARGET_ASSETS[this.data.assetIndex],
        amount: amountMinor,
        currencyCode: "CNY",
        reasonCode: this.data.reasonCode.trim() || undefined,
        reasonDetail: this.data.reasonDetail.trim() || undefined,
      });
      wx.showToast({ title: "退款申请已提交", icon: "success" });
      this.setData({ amountText: "", reasonCode: "", reasonDetail: "" });
      this.loadHistory();
    } catch (cause) {
      this.setData({
        error: cause && cause.message ? cause.message : "退款申请失败，请稍后再试",
      });
    } finally {
      this.setData({ submitting: false });
    }
  },

  goOrderDetail() {
    if (this.data.orderId.trim()) {
      wx.navigateTo({ url: `/pages/order-detail/index?id=${this.data.orderId.trim()}` });
    }
  },
});
