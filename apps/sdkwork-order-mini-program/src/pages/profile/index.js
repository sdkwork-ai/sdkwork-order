const session = require("../../services/session");

Page({
  data: {
    loggedIn: false,
    environmentText: "",
    profileIdText: "",
  },

  onShow() {
    const app = getApp();
    const globalData = app && app.globalData ? app.globalData : {};
    this.setData({
      loggedIn: session.isLoggedIn(),
      environmentText: `${globalData.deploymentProfile || "standalone"}.${globalData.environment || "development"}`,
      profileIdText: globalData.profileId || "standalone.development",
    });
  },

  goOrders() {
    wx.switchTab({ url: "/pages/orders/index" });
  },

  goRecharge() {
    wx.switchTab({ url: "/pages/recharge/index" });
  },

  goCoupon() {
    wx.navigateTo({ url: "/pages/coupon/index" });
  },

  goWithdraw() {
    wx.navigateTo({ url: "/pages/withdraw/index" });
  },

  goRefund() {
    wx.navigateTo({ url: "/pages/refund/index" });
  },

  goLogin() {
    wx.navigateTo({ url: "/pages/login/index" });
  },

  logout() {
    session.clearToken();
    this.setData({ loggedIn: false });
    wx.showToast({ title: "已退出", icon: "success" });
  },
});
