const withdrawalService = require("../../services/withdrawal-service");
const session = require("../../services/session");
const { formatMinor, formatTime, yuanInputToMinor, requestStatusLabel } = require("../../utils/format");

const PAYOUT_METHODS = ["wechat", "alipay", "bank_card"];

Page({
  data: {
    amountText: "",
    payoutMethodIndex: 0,
    payoutMethods: PAYOUT_METHODS,
    payoutMethodLabels: ["微信", "支付宝", "银行卡"],
    payoutAccountRef: "",
    reasonCode: "",
    submitting: false,
    history: [],
    loadingHistory: true,
    error: "",
  },

  onShow() {
    if (!session.isLoggedIn()) {
      wx.navigateTo({ url: "/pages/login/index" });
      return;
    }
    this.loadHistory();
  },

  onAmountInput(event) {
    this.setData({ amountText: event.detail.value });
  },

  onAccountInput(event) {
    this.setData({ payoutAccountRef: event.detail.value });
  },

  onReasonInput(event) {
    this.setData({ reasonCode: event.detail.value });
  },

  selectPayoutMethod(event) {
    this.setData({ payoutMethodIndex: Number(event.detail.value) || 0 });
  },

  async loadHistory() {
    this.setData({ loadingHistory: true });
    try {
      const history = await withdrawalService.listLocalHistory();
      this.setData({
        history: history.map((entry) => ({
          accountValueRequestId: entry.accountValueRequestId,
          requestNo: entry.requestNo,
          amountText: formatMinor(entry.amount),
          statusText: requestStatusLabel(entry.status),
          timeText: formatTime(entry.createdAt),
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
    const amountMinor = yuanInputToMinor(this.data.amountText);
    if (!amountMinor || amountMinor === "0") {
      this.setData({ error: "请输入有效的提现金额（最多两位小数）" });
      return;
    }
    this.setData({ submitting: true, error: "" });
    try {
      const record = await withdrawalService.createWithdrawalRequest({
        amount: amountMinor,
        currencyCode: "CNY",
        payoutMethod: PAYOUT_METHODS[this.data.payoutMethodIndex],
        payoutAccountRef: this.data.payoutAccountRef.trim() || undefined,
        reasonCode: this.data.reasonCode.trim() || undefined,
      });
      wx.showToast({ title: "提现申请已提交", icon: "success" });
      this.setData({ amountText: "", payoutAccountRef: "", reasonCode: "" });
      this.loadHistory();
    } catch (cause) {
      this.setData({
        error: cause && cause.message ? cause.message : "提现申请失败，请稍后再试",
      });
    } finally {
      this.setData({ submitting: false });
    }
  },
});
