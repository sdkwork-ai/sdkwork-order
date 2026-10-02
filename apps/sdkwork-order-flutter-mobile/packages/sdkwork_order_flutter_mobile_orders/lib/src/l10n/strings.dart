/// 中文为主文案常量（pre-l10n fragment；完整 i18n 框架延后，见根
/// `docs/decisions.md`）以及展示辅助（状态标签、状态色、时间格式化）。
library;

import 'package:flutter/material.dart';

/// 底部导航与标题.
const String appTitle = 'SDKWork 订单';
const String tabOrders = '订单';
const String tabRecharge = '充值';
const String tabProfile = '我的';

/// 订单状态页签（tab id ↔ 标签），与 mobile-react `OrderTabId` 对齐.
const List<MapEntry<String, String>> orderTabs = <MapEntry<String, String>>[
  MapEntry('all', '全部'),
  MapEntry('pending_payment', '待付款'),
  MapEntry('paid', '已支付'),
  MapEntry('fulfilled', '待收货'),
  MapEntry('completed', '已完成'),
  MapEntry('cancelled', '已取消'),
];

String orderTabLabel(String tabId) {
  for (final tab in orderTabs) {
    if (tab.key == tabId) {
      return tab.value;
    }
  }
  return tabId;
}

const Map<String, String> _statusLabels = <String, String>{
  'pending_payment': '待付款',
  'paid': '已支付',
  'fulfilled': '待收货',
  'shipped': '已发货',
  'delivered': '已送达',
  'completed': '已完成',
  'cancelled': '已取消',
  'canceled': '已取消',
  'expired': '已超时',
  'timeout': '已超时',
  'refunding': '退款中',
  'refunded': '已退款',
  'failed': '已失败',
  'closed': '已关闭',
};

/// 后端状态（小写 wire 值）→ 中文标签；未知状态回退 `statusName`/原值.
String statusLabel(String status, {String? statusName}) {
  final normalized = status.trim().toLowerCase();
  return _statusLabels[normalized] ??
      (statusName != null && statusName.isNotEmpty ? statusName : '处理中');
}

Color statusColor(BuildContext context, String status) {
  final normalized = status.trim().toLowerCase();
  if (normalized == 'pending_payment') {
    return const Color(0xFFD97706);
  }
  if (normalized == 'cancelled' ||
      normalized == 'canceled' ||
      normalized == 'failed' ||
      normalized == 'closed') {
    return const Color(0xFFDC2626);
  }
  if (normalized == 'expired' || normalized == 'timeout') {
    return const Color(0xFF6B7280);
  }
  return Theme.of(context).colorScheme.primary;
}

/// 账户价值请求（提现/退款）状态标签.
const Map<String, String> requestStatusLabels = <String, String>{
  'requested': '已受理',
  'pending': '处理中',
  'approved': '已通过',
  'processing': '打款中',
  'completed': '已完成',
  'rejected': '已驳回',
  'cancelled': '已取消',
  'failed': '已失败',
};

String requestStatusLabel(String status) {
  final normalized = status.trim().toLowerCase();
  return requestStatusLabels[normalized] ?? '处理中';
}

/// 支付方式标签.
String paymentMethodLabel(String method) =>
    const <String, String>{
      'wechat_pay': '微信支付',
      'alipay': '支付宝',
      'balance': '余额',
    }[method] ??
    method;

/// 券码权益标签.
String benefitKindLabel(String? kind) =>
    const <String, String>{
      'token_bank_credit': 'Token Bank 额度',
      'points_credit': '积分',
      'cash_credit': '现金',
      'subscription': '会员权益',
    }[kind] ??
    (kind ?? '权益');

/// 提现方式标签（wire 值 ↔ 中文）.
const List<MapEntry<String, String>> payoutMethods = <MapEntry<String, String>>[
  MapEntry('bank_card', '银行卡'),
  MapEntry('alipay', '支付宝'),
];

String payoutMethodLabel(String method) {
  for (final entry in payoutMethods) {
    if (entry.key == method) {
      return entry.value;
    }
  }
  return method;
}

/// 退款资产标签.
String refundAssetLabel(String asset) =>
    const <String, String>{
      'points': '积分',
      'token_bank': 'Token Bank',
      'cash': '现金',
    }[asset] ??
    asset;

/// ISO 时间 → `yyyy-MM-dd HH:mm` 展示；无法解析时原样返回.
String formatTime(String? value) {
  if (value == null || value.isEmpty) {
    return '--';
  }
  final date = DateTime.tryParse(value);
  if (date == null) {
    return value;
  }
  String pad(int part) => part.toString().padLeft(2, '0');
  return '${date.year}-${pad(date.month)}-${pad(date.day)} '
      '${pad(date.hour)}:${pad(date.minute)}';
}
