const ordersService = require("../../services/order-service");
const session = require("../../services/session");
const { formatMinor, formatTime, orderStatusLabel } = require("../../utils/format");

Page({
  data: {
    tabs: ordersService.ORDER_TABS,
    activeTab: "all",
    statistics: null,
    orders: [],
    page: 1,
    hasMore: false,
    loading: true,
    loadingMore: false,
    error: "",
    // Scene/tag facets. Collapsed by default to keep the mobile viewport for
    // the list; both dimensions match the other order-centre surfaces.
    facetsOpen: false,
    scenes: [
      { code: "web", label: "Web 商城" },
      { code: "h5", label: "H5" },
      { code: "app", label: "移动 App" },
      { code: "mini_program", label: "小程序" },
      { code: "cashier", label: "收银台链接" },
      { code: "membership", label: "会员订阅" },
      { code: "recharge", label: "账户充值" },
      { code: "platform", label: "平台代下单" },
      { code: "default", label: "未标注" },
    ],
    activeScene: "",
    activeTags: [],
    tagMatch: "any",
    tagDraft: "",
  },

  onLoad(options) {
    const status = options.status && options.status !== "all" ? options.status : "all";
    this.setData({ activeTab: status });
  },

  onShow() {
    if (!session.isLoggedIn()) {
      wx.navigateTo({ url: "/pages/login/index" });
      return;
    }
    this.loadStatistics();
    this.loadOrders(1);
  },

  onPullDownRefresh() {
    Promise.all([this.loadStatistics(), this.loadOrders(1)])
      .catch(() => {})
      .then(() => wx.stopPullDownRefresh());
  },

  onReachBottom() {
    if (!this.data.loadingMore && this.data.hasMore) {
      this.loadOrders(this.data.page + 1);
    }
  },

  async loadStatistics() {
    try {
      const statistics = await ordersService.getStatistics();
      this.setData({ statistics });
    } catch (cause) {
      // statistics is a decoration; the list error path surfaces real failures
    }
  },

  async loadOrders(nextPage) {
    const { activeTab, activeScene, activeTags, orders, tagMatch } = this.data;
    this.setData(nextPage === 1 ? { loading: true, error: "" } : { loadingMore: true });
    try {
      const result = await ordersService.listOrders({
        page: nextPage,
        pageSize: 20,
        status: activeTab,
        scene: activeScene || undefined,
        tags: activeTags,
        tagMatch,
      });
      const rows = result.items.map((order) => ({
        orderId: order.orderId,
        subject: order.subject,
        status: order.status,
        statusText: order.statusName || orderStatusLabel(order.status),
        totalText: formatMinor(order.totalAmount),
        timeText: formatTime(order.createdAt),
        tags: order.tags || [],
      }));
      const hasMore =
        result.pageInfo.hasMore !== undefined
          ? result.pageInfo.hasMore === true
          : rows.length > 0 && rows.length >= (result.pageInfo.pageSize || 20);
      this.setData({
        orders: nextPage === 1 ? rows : orders.concat(rows),
        page: nextPage,
        hasMore,
        loading: false,
        loadingMore: false,
      });
    } catch (cause) {
      this.setData({
        loading: false,
        loadingMore: false,
        error: cause && cause.message ? cause.message : "订单加载失败",
      });
    }
  },

  selectTab(event) {
    const code = event.currentTarget.dataset.code;
    if (code === this.data.activeTab) {
      return;
    }
    this.setData({ activeTab: code, orders: [], hasMore: false });
    this.loadOrders(1);
  },

  /** Expands/collapses the facet panel. */
  toggleFacets() {
    this.setData({ facetsOpen: !this.data.facetsOpen });
  },

  /**
   * Selects one scene (or clears it when the active chip is tapped again).
   * Changing a facet resets the list because the row set changes underneath.
   */
  selectScene(event) {
    const code = event.currentTarget.dataset.code || "";
    const next = this.data.activeScene === code ? "" : code;
    this.setData({ activeScene: next, orders: [], hasMore: false });
    this.loadOrders(1);
  },

  onTagInput(event) {
    this.setData({ tagDraft: event.detail.value || "" });
  },

  /**
   * Commits the typed tag using the same canonical form the API enforces:
   * lowercase, whitespace/`_`/`.`/`-` folded to a single `-`, no leading or
   * trailing separator. Input still containing a character the server rejects
   * (punctuation, quotes) is dropped rather than sent to fail validation.
   */
  commitTag() {
    const raw = String(this.data.tagDraft || "").trim();
    this.setData({ tagDraft: "" });
    const canonical = require("../../services/order-service").normalizeOrderTag(raw);
    if (!canonical || this.data.activeTags.indexOf(canonical) !== -1) {
      return;
    }
    this.setData({ activeTags: this.data.activeTags.concat([canonical]), orders: [], hasMore: false });
    this.loadOrders(1);
  },

  removeTag(event) {
    const tag = event.currentTarget.dataset.tag;
    this.setData({
      activeTags: this.data.activeTags.filter((value) => value !== tag),
      orders: [],
      hasMore: false,
    });
    this.loadOrders(1);
  },

  toggleTagMatch() {
    this.setData({ tagMatch: this.data.tagMatch === "all" ? "any" : "all", orders: [], hasMore: false });
    this.loadOrders(1);
  },

  clearFacets() {
    this.setData({ activeScene: "", activeTags: [], tagMatch: "any", tagDraft: "", orders: [], hasMore: false });
    this.loadOrders(1);
  },

  goDetail(event) {
    wx.navigateTo({ url: `/pages/order-detail/index?id=${event.currentTarget.dataset.id}` });
  },

  goPay(event) {
    wx.navigateTo({
      url: `/pages/cashier/index?orderId=${event.currentTarget.dataset.id}`,
    });
  },

  goRefund(event) {
    wx.navigateTo({
      url: `/pages/refund/index?orderId=${event.currentTarget.dataset.id}`,
    });
  },

  async cancelOrder(event) {
    const orderId = event.currentTarget.dataset.id;
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
      await ordersService.cancelOrder(orderId);
      wx.showToast({ title: "已取消", icon: "success" });
      this.loadOrders(1);
    } catch (cause) {
      wx.showToast({ title: cause && cause.message ? cause.message : "取消失败", icon: "none" });
    }
  },

  async confirmReceipt(event) {
    const orderId = event.currentTarget.dataset.id;
    try {
      await ordersService.confirmReceipt(orderId);
      wx.showToast({ title: "已确认收货", icon: "success" });
      this.loadOrders(1);
    } catch (cause) {
      wx.showToast({ title: cause && cause.message ? cause.message : "确认收货失败", icon: "none" });
    }
  },
});
