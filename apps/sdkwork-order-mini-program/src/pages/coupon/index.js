const ordersService = require("../../services/order-service");
const { formatMinor } = require("../../utils/format");

Page({
  data: {
    code: "",
    submitting: false,
    result: null,
    error: "",
  },

  onCodeInput(event) {
    this.setData({ code: event.detail.value });
  },

  async redeem() {
    const code = this.data.code.trim();
    if (!code) {
      this.setData({ error: "请输入券码" });
      return;
    }
    if (this.data.submitting) {
      return;
    }
    this.setData({ submitting: true, error: "", result: null });
    try {
      const result = await ordersService.redeemCoupon(code);
      const benefit = result.benefit || {};
      const isTokenBank = benefit.kind === "token_bank_credit";
      this.setData({
        submitting: false,
        result: {
          success: result.status === "completed",
          replayed: result.replayed,
          orderNo: result.orderNo,
          benefitText: isTokenBank
            ? `Token Bank 额度 ${formatMinor(benefit.grantAmount, { symbol: "" })}`
            : benefit.kind === "subscription"
              ? `会员权益 ${benefit.durationDays || ""} 天`
              : "权益已发放",
        },
      });
      if (result.status === "completed") {
        wx.showToast({ title: result.replayed ? "券码已核销过" : "核销成功", icon: "success" });
      } else {
        this.setData({ error: "券码无效或不可用" });
      }
    } catch (cause) {
      this.setData({
        submitting: false,
        error: cause && cause.message ? cause.message : "核销失败，请稍后再试",
      });
    }
  },
});
