import 'package:flutter/material.dart';

import 'package:sdkwork_order_flutter_mobile_core/sdkwork_order_flutter_mobile_core.dart';

import '../l10n/strings.dart';
import '../widgets/common_views.dart';

/// 订单详情：订单信息 + 生命周期事件时间线 + 操作（去支付 / 取消 / 申请退款）.
class OrderDetailScreen extends StatefulWidget {
  const OrderDetailScreen({
    super.key,
    required this.orderService,
    required this.orderId,
  });

  final OrderService orderService;
  final String orderId;

  @override
  State<OrderDetailScreen> createState() => _OrderDetailScreenState();
}

class _OrderDetailScreenState extends State<OrderDetailScreen> {
  Order? _order;
  List<OrderEvent> _events = const <OrderEvent>[];
  bool _loading = true;
  bool _cancelling = false;
  String _error = '';

  @override
  void initState() {
    super.initState();
    _load();
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('订单详情')),
      body: _loading
          ? const SdkworkLoadingView(label: '加载订单...')
          : _error.isNotEmpty
              ? SdkworkErrorView(message: _error, onRetry: _load)
              : RefreshIndicator(
                  onRefresh: _load,
                  child: ListView(
                    padding: const EdgeInsets.all(12),
                    children: [
                      _buildOrderCard(),
                      const SizedBox(height: 12),
                      _buildItemsCard(),
                      const SizedBox(height: 12),
                      _buildEventsCard(),
                    ],
                  ),
                ),
      bottomNavigationBar: _order == null
          ? null
          : SafeArea(
              child: Padding(
                padding: const EdgeInsets.all(16),
                child: Row(
                  children: [
                    if (_order!.isPendingPayment) ...[
                      Expanded(
                        child: OutlinedButton(
                          onPressed: _cancelling ? null : _cancelOrder,
                          child: Text(_cancelling ? '取消中...' : '取消订单'),
                        ),
                      ),
                      const SizedBox(width: 12),
                      Expanded(
                        flex: 2,
                        child: FilledButton(
                          onPressed: _goCashier,
                          child: const Text('去支付'),
                        ),
                      ),
                    ] else ...[
                      Expanded(
                        child: OutlinedButton(
                          onPressed: _goRefund,
                          child: const Text('申请退款'),
                        ),
                      ),
                    ],
                  ],
                ),
              ),
            ),
    );
  }

  Widget _buildOrderCard() {
    final order = _order!;
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Expanded(
                  child: Text(
                    order.subject,
                    style: const TextStyle(
                      fontSize: 17,
                      fontWeight: FontWeight.w700,
                    ),
                  ),
                ),
                Text(
                  statusLabel(order.status, statusName: order.statusName),
                  style: TextStyle(
                    color: statusColor(context, order.status),
                    fontWeight: FontWeight.w600,
                  ),
                ),
              ],
            ),
            const Divider(height: 20),
            _infoRow('订单编号', order.orderSn),
            _infoRow('订单金额', formatAmount(order.totalAmount, currencyCode: order.currencyCode)),
            if (order.paidAmount != null)
              _infoRow('已付金额', formatAmount(order.paidAmount, currencyCode: order.currencyCode)),
            if (order.discountAmount != null)
              _infoRow('优惠金额', formatAmount(order.discountAmount, currencyCode: order.currencyCode)),
            _infoRow('下单时间', formatTime(order.createdAt)),
            if (order.payTime != null) _infoRow('支付时间', formatTime(order.payTime)),
            if (order.expireTime != null) _infoRow('失效时间', formatTime(order.expireTime)),
            if (order.paymentMethod != null)
              _infoRow('支付方式', paymentMethodLabel(order.paymentMethod!)),
            if (order.outTradeNo != null) _infoRow('交易单号', order.outTradeNo!),
          ],
        ),
      ),
    );
  }

  Widget _buildItemsCard() {
    final items = _order!.items;
    if (items.isEmpty) {
      return const SizedBox.shrink();
    }
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text('商品明细', style: Theme.of(context).textTheme.titleSmall),
            const SizedBox(height: 8),
            for (final item in items)
              Padding(
                padding: const EdgeInsets.symmetric(vertical: 6),
                child: Row(
                  children: [
                    Expanded(
                      child: Text(
                        item.productName,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                      ),
                    ),
                    Text('x${item.quantity}'),
                    const SizedBox(width: 12),
                    Text(
                      formatAmount(
                        item.totalAmount,
                        currencyCode: _order!.currencyCode,
                      ),
                    ),
                  ],
                ),
              ),
          ],
        ),
      ),
    );
  }

  Widget _buildEventsCard() {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text('生命周期', style: Theme.of(context).textTheme.titleSmall),
            const SizedBox(height: 8),
            if (_events.isEmpty)
              const Text('暂无事件')
            else
              for (final event in _events)
                Padding(
                  padding: const EdgeInsets.symmetric(vertical: 6),
                  child: Row(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      const Padding(
                        padding: EdgeInsets.only(top: 6),
                        child: Icon(Icons.circle, size: 8),
                      ),
                      const SizedBox(width: 10),
                      Expanded(
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Text(
                              event.message?.isNotEmpty == true
                                  ? event.message!
                                  : '状态 → ${statusLabel(event.toStatus)}',
                              style: Theme.of(context).textTheme.bodyMedium,
                            ),
                            Text(
                              '${formatTime(event.createdAt)} · ${event.eventType}',
                              style: Theme.of(context).textTheme.labelSmall,
                            ),
                          ],
                        ),
                      ),
                    ],
                  ),
                ),
          ],
        ),
      ),
    );
  }

  Widget _infoRow(String label, String value) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 3),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          SizedBox(
            width: 76,
            child: Text(label, style: Theme.of(context).textTheme.bodySmall),
          ),
          Expanded(
            child: Text(value, style: const TextStyle(fontWeight: FontWeight.w500)),
          ),
        ],
      ),
    );
  }

  Future<void> _load() async {
    setState(() {
      _loading = _order == null;
      _error = '';
    });
    try {
      final order = await widget.orderService.getOrder(widget.orderId);
      List<OrderEvent> events;
      try {
        events = await widget.orderService.getOrderEvents(widget.orderId);
      } on Exception {
        // 事件加载失败不阻塞详情（时间线展示“暂无事件”）。
        events = const <OrderEvent>[];
      }
      if (!mounted) {
        return;
      }
      setState(() {
        _order = order;
        _events = events;
        _loading = false;
      });
    } catch (cause) {
      if (!mounted) {
        return;
      }
      setState(() {
        _loading = false;
        _error = '$cause';
      });
    }
  }

  Future<void> _cancelOrder() async {
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (dialogContext) => AlertDialog(
        title: const Text('取消订单'),
        content: const Text('确定要取消该订单吗？取消后无法恢复。'),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(dialogContext).pop(false),
            child: const Text('再想想'),
          ),
          FilledButton(
            onPressed: () => Navigator.of(dialogContext).pop(true),
            child: const Text('取消订单'),
          ),
        ],
      ),
    );
    if (confirmed != true || !mounted) {
      return;
    }
    setState(() => _cancelling = true);
    try {
      await widget.orderService.cancelOrder(widget.orderId);
      await _load();
    } catch (cause) {
      if (!mounted) {
        return;
      }
      setState(() => _cancelling = false);
      ScaffoldMessenger.of(context)
          .showSnackBar(SnackBar(content: Text('$cause')));
    }
  }

  void _goCashier() {
    Navigator.of(context)
        .pushNamed('/cashier', arguments: widget.orderId)
        .then((_) => _load());
  }

  void _goRefund() {
    Navigator.of(context).pushNamed(
      '/refund-requests',
      arguments: <String, String>{
        'orderId': widget.orderId,
        'amount': minorToMajorString(_order?.totalAmount) ?? '',
        'currencyCode': _order?.currencyCode ?? 'CNY',
      },
    );
  }
}
