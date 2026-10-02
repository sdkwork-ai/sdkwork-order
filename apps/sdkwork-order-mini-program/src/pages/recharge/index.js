const rechargeService = require("../../services/recharge-service");
const session = require("../../services/session");
const { formatMinor, formatTime } = require("../../utils/format");

const PLAN_PERIOD_LABELS = {
  monthly: "月付",
  quarterly: "季付",
  yearly: "年付",
  continuous_monthly: "连续包月",
  continuous_yearly: "连续包年",
};

Page({
  data: {
    segment: "plans",
    plans: [],
    packages: [],
    recentOrders: [],
    loading: true,
    loadingMore: false,
    submittingId: "",
    page: 1,
    hasMore: false,
    error: "",
  },

  onShow() {
    if (!session.isLoggedIn()) {
      wx.navigateTo({ url: "/pages/login/index" });
      return;
    }
    this.loadCatalog();
    this.loadRecentOrders(1);
  },

  onReachBottom() {
    if (this.data.hasMore && !this.data.loadingMore) {
      this.loadRecentOrders(this.data.page + 1);
    }
  },

  switchSegment(event) {
    const segment = event.currentTarget.dataset.segment;
    if (segment !== this.data.segment) {
      this.setData({ segment });
    }
  },

  async loadCatalog() {
    this.setData({ loading: true, error: "" });
    try {
      const [plans, packages] = await Promise.all([
        rechargeService.listPlans().catch(() => []),
        rechargeService.listPackages().catch(() => []),
      ]);
      this.setData({
        plans: plans.map((plan) => ({
          ...plan,
          priceText: formatMinor(plan.priceAmount),
          grantText: `${plan.grantAmount}${plan.bonusAmount !== "0" ? ` +${plan.bonusAmount}` : ""}`,
          periodText: PLAN_PERIOD_LABELS[plan.planPeriod] || plan.planPeriod,
        })),
        packages: packages.map((pkg) => ({
          ...pkg,
          priceText: formatMinor(pkg.priceAmount),
          pointsText: `${pkg.points}${pkg.bonusPoints > 0 ? ` +${pkg.bonusPoints}` : ""}`,
        })),
        loading: false,
      });
    } catch (cause) {
      this.setData({
        loading: false,
        error: cause && cause.message ? cause.message : "充值目录加载失败",
      });
    }
  },

  async loadRecentOrders(nextPage) {
    this.setData(nextPage === 1 ? {} : { loadingMore: true });
    try {
      const result = await rechargeService.listRechargeOrders({ page: nextPage, pageSize: 10 });
      const rows = result.items.map((order) => ({
        orderId: order.orderId,
        orderNo: order.orderNo,
        subject: order.subject || "充值订单",
        amountText: formatMinor(order.amount),
        points: order.points,
        status: order.status,
        timeText: formatTime(order.createdAt),
      }));
      const hasMore =
        result.pageInfo.hasMore !== undefined
          ? result.pageInfo.hasMore === true
          : rows.length >= (result.pageInfo.pageSize || 10);
      this.setData({
        recentOrders: nextPage === 1 ? rows : this.data.recentOrders.concat(rows),
        page: nextPage,
        hasMore,
        loadingMore: false,
      });
    } catch (cause) {
      this.setData({ loadingMore: false });
    }
  },

  async buyPlan(event) {
    await this.submitRecharge({
      submitId: `plan:${event.currentTarget.dataset.code}`,
      subject: "token_bank_plan_purchase",
      targetAsset: "token_bank",
      planCode: event.currentTarget.dataset.code,
      planPeriod: event.currentTarget.dataset.period,
      priceAmount: event.currentTarget.dataset.price,
      currencyCode: event.currentTarget.dataset.currency,
      grantAmount: event.currentTarget.dataset.grant,
    });
  },

  async buyPackage(event) {
    await this.submitRecharge({
      submitId: `package:${event.currentTarget.dataset.id}`,
      subject: "points_recharge",
      targetAsset: "points",
      packageId: event.currentTarget.dataset.id,
      priceAmount: event.currentTarget.dataset.price,
      currencyCode: event.currentTarget.dataset.currency,
    });
  },

  async submitRecharge(input) {
    if (this.data.submittingId) {
      return;
    }
    let amount;
    try {
      amount = rechargeService.planPriceToMinor(input.priceAmount);
    } catch (cause) {
      wx.showToast({ title: cause && cause.message ? cause.message : "价格无效", icon: "none" });
      return;
    }
    this.setData({ submittingId: input.submitId, error: "" });
    try {
      const created = await rechargeService.createRechargeOrder({
        subject: input.subject,
        targetAsset: input.targetAsset,
        amount,
        grantAmount: input.grantAmount,
        currencyCode: input.currencyCode || "CNY",
        packageId: input.packageId,
        planCode: input.planCode,
        planPeriod: input.planPeriod,
        paymentMethod: "wechat_jsapi",
        source: "mini-program",
      });
      wx.showToast({ title: "充值订单已创建", icon: "success" });
      if (created.orderId) {
        wx.navigateTo({
          url: `/pages/payment-result/index?orderId=${created.orderId}&mode=recharge&status=pending`,
        });
      }
      this.loadRecentOrders(1);
    } catch (cause) {
      wx.showToast({
        title: cause && cause.message ? cause.message : "充值下单失败，请稍后再试",
        icon: "none",
      });
    } finally {
      this.setData({ submittingId: "" });
    }
  },

  goOrderResult(event) {
    const orderId = event.currentTarget.dataset.id;
    if (orderId) {
      wx.navigateTo({
        url: `/pages/payment-result/index?orderId=${orderId}&mode=recharge&status=pending`,
      });
    }
  },

  async cancelOrder(event) {
    const orderId = event.currentTarget.dataset.id;
    const confirmed = await new Promise((resolve) => {
      wx.showModal({
        title: "取消充值",
        content: "确定取消该充值订单吗？",
        success: (res) => resolve(res.confirm === true),
        fail: () => resolve(false),
      });
    });
    if (!confirmed) {
      return;
    }
    try {
      await rechargeService.cancelRechargeOrder(orderId);
      wx.showToast({ title: "已取消", icon: "success" });
      this.loadRecentOrders(1);
    } catch (cause) {
      wx.showToast({ title: cause && cause.message ? cause.message : "取消失败", icon: "none" });
    }
  },
});
