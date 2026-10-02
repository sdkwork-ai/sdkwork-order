const session = require("../../services/session");

Page({
  data: {
    token: "",
    busy: false,
    error: "",
  },

  onShow() {
    this.setData({ token: session.getToken() });
  },

  onTokenInput(event) {
    this.setData({ token: event.detail.value });
  },

  async login() {
    const token = this.data.token.trim();
    if (!token) {
      this.setData({ error: "请输入访问令牌" });
      return;
    }
    this.setData({ busy: true, error: "" });
    session.setToken(token);
    this.setData({ busy: false });
    wx.showToast({ title: "登录成功", icon: "success" });
    setTimeout(() => {
      wx.navigateBack({
        fail() {
          wx.switchTab({ url: "/pages/profile/index" });
        },
      });
    }, 600);
  },

  logout() {
    session.clearToken();
    this.setData({ token: "" });
    wx.showToast({ title: "已退出", icon: "success" });
  },
});
