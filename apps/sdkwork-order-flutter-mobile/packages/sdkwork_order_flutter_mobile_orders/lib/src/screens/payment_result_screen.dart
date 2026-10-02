import 'package:flutter/material.dart';

import 'package:sdkwork_order_flutter_mobile_core/sdkwork_order_flutter_mobile_core.dart';

import '../l10n/strings.dart';

/// 支付结果页：一次权威 payment_success 查询 + 结果展示与后续动作.
class PaymentResultScreen extends StatefulWidget {
  const PaymentResultScreen({
    super.key,
    required this.orderService,
    required this.arguments,
  });

  final OrderService orderService;

  /// Route arguments: `{orderId, paymentId?, paymentMethod?}`.
  final Map<String, String> arguments;

  @override
  State<PaymentResultScreen> createState() => _PaymentResultScreenState();
}

class _PaymentResultScreenState extends State<PaymentResultScreen> {
  CashierPhase _phase = CashierPhase.pending;
  bool _loading = true;
  String _detail = '';

  String get _orderId => widget.arguments['orderId'] ?? '';

  @override
  void initState() {
    super.initState();
    _check();
  }

  @override
  Widget build(BuildContext context) {
    final paid = _phase == CashierPhase.paid;
    final failed = _phase != CashierPhase.paid && _phase != CashierPhase.pending;
    final color = paid
        ? const Color(0xFF16A34A)
        : failed
            ? const Color(0xFFDC2626)
            : const Color(0xFFD97706);
    final background = paid
        ? const Color(0xFFF0FDF4)
        : failed
            ? const Color(0xFFFEF2F2)
            : const Color(0xFFFFFBEB);
    return Scaffold(
      appBar: AppBar(title: const Text('支付结果')),
      body: ListView(
        padding: const EdgeInsets.all(24),
        children: [
          Container(
            padding: const EdgeInsets.all(32),
            decoration: BoxDecoration(
              color: background,
              borderRadius: BorderRadius.circular(20),
            ),
            child: Column(
              children: [
                Text(
                  paid
                      ? '支付成功'
                      : failed
                          ? '支付未完成'
                          : '支付处理中',
                  style: TextStyle(
                    fontSize: 24,
                    fontWeight: FontWeight.w800,
                    color: color,
                  ),
                ),
                const SizedBox(height: 8),
                Text(
                  _detail.isNotEmpty
                      ? _detail
                      : paid
                          ? '订单已支付完成。'
                          : failed
                              ? '支付未成功，可重新支付。'
                              : '支付正在处理，请稍后刷新。',
                  style: Theme.of(context).textTheme.bodySmall,
                ),
                const SizedBox(height: 8),
                if (_orderId.isNotEmpty) Text('订单号：$_orderId'),
                if (widget.arguments['paymentId']?.isNotEmpty ?? false)
                  Text('支付单号：${widget.arguments['paymentId']}'),
                if (widget.arguments['paymentMethod']?.isNotEmpty ?? false)
                  Text(
                    '支付方式：'
                    '${paymentMethodLabel(widget.arguments['paymentMethod']!)}',
                  ),
                const SizedBox(height: 8),
                if (_loading)
                  const Padding(
                    padding: EdgeInsets.only(top: 12),
                    child: SizedBox(
                      width: 24,
                      height: 24,
                      child: CircularProgressIndicator(strokeWidth: 2),
                    ),
                  ),
              ],
            ),
          ),
          const SizedBox(height: 24),
          if (!paid) ...[
            FilledButton(
              onPressed: _loading ? null : _check,
              child: const Text('刷新结果'),
            ),
            const SizedBox(height: 12),
            if (isCashierRetryablePhase(_phase))
              OutlinedButton(
                onPressed: () =>
                    Navigator.of(context).pushNamed('/cashier', arguments: _orderId),
                child: const Text('重新支付'),
              ),
            if (isCashierRetryablePhase(_phase)) const SizedBox(height: 12),
          ],
          FilledButton(
            onPressed: () => Navigator.of(context).pushNamed(
              '/order-detail',
              arguments: _orderId,
            ),
            child: const Text('查看订单'),
          ),
          const SizedBox(height: 12),
          OutlinedButton(
            onPressed: () => Navigator.of(context).popUntil((route) => route.isFirst),
            child: const Text('返回首页'),
          ),
        ],
      ),
    );
  }

  Future<void> _check() async {
    setState(() => _loading = true);
    if (_orderId.isEmpty) {
      if (mounted) {
        setState(() {
          _loading = false;
          _phase = CashierPhase.failed;
          _detail = '缺少订单号。';
        });
      }
      return;
    }
    try {
      final status = await widget.orderService.getPaymentSuccess(_orderId);
      final orderStatus = await widget.orderService
          .getOrderStatus(_orderId)
          .then((value) => value.status)
          .catchError((_) => status.status);
      final phase = OrderService.resolvePhase(status, orderStatus);
      if (!mounted) {
        return;
      }
      setState(() {
        _phase = phase;
        _detail = phase == CashierPhase.paid ? '订单已支付完成。' : status.statusName;
        _loading = false;
      });
    } on Exception catch (cause) {
      if (!mounted) {
        return;
      }
      setState(() {
        _loading = false;
        _phase = CashierPhase.failed;
        _detail = '$cause';
      });
    }
  }
}
