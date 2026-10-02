import 'dart:async';

import 'package:flutter/material.dart';

import 'package:sdkwork_order_flutter_mobile_core/sdkwork_order_flutter_mobile_core.dart';

import '../l10n/strings.dart';
import '../widgets/common_views.dart';

/// 收银台：支付方式选择 → 创建支付（幂等键）→ 轮询状态（退避 + 到期终检）→
/// 终态展示。纯轮询轮转，无原生渠道依赖（原生通道见 docs/decisions.md 跟进项）。
class CashierScreen extends StatefulWidget {
  const CashierScreen({super.key, required this.orderService, required this.orderId});

  final OrderService orderService;
  final String orderId;

  @override
  State<CashierScreen> createState() => _CashierScreenState();
}

class _CashierScreenState extends State<CashierScreen> {
  Order? _order;
  String _selectedMethod = OrderService.orderPaymentMethods.first;
  CashierPhase _phase = CashierPhase.pending;
  PaymentSession? _session;
  int _remainingSeconds = 0;
  int _consecutivePollFailures = 0;
  bool _loading = true;
  bool _creating = false;
  String _error = '';
  String? _pollNotice;

  bool _disposed = false;
  Timer? _pollTimer;
  Timer? _countdownTimer;

  @override
  void initState() {
    super.initState();
    _loadOrder();
  }

  @override
  void dispose() {
    _disposed = true;
    _pollTimer?.cancel();
    _countdownTimer?.cancel();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('收银台')),
      body: _loading
          ? const SdkworkLoadingView(label: '加载收银台...')
          : _error.isNotEmpty && _order == null
              ? SdkworkErrorView(message: _error, onRetry: _loadOrder)
              : ListView(
                  padding: const EdgeInsets.all(16),
                  children: [
                    _buildAmountCard(),
                    const SizedBox(height: 12),
                    if (_phase == CashierPhase.pending) ...[
                      _buildMethodCard(),
                      if (_pollNotice != null)
                        Padding(
                          padding: const EdgeInsets.only(top: 8),
                          child: Text(
                            _pollNotice!,
                            style: Theme.of(context).textTheme.labelSmall,
                          ),
                        ),
                    ],
                    if (_phase == CashierPhase.paid) _buildPaidCard(),
                    if (_phase == CashierPhase.expired ||
                        _phase == CashierPhase.cancelled ||
                        _phase == CashierPhase.failed)
                      _buildClosedCard(),
                  ],
                ),
      bottomNavigationBar: _phase == CashierPhase.pending
          ? SafeArea(
              child: Padding(
                padding: const EdgeInsets.all(16),
                child: FilledButton(
                  onPressed: _creating ? null : _createPayment,
                  child: Text(
                    _creating
                        ? '支付中...'
                        : '立即支付 ${formatAmount(_order?.totalAmount, currencyCode: _order?.currencyCode)}',
                  ),
                ),
              ),
            )
          : SafeArea(
              child: Padding(
                padding: const EdgeInsets.all(16),
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    if (isCashierRetryablePhase(_phase))
                      FilledButton(
                        onPressed: _resetToPending,
                        child: const Text('重新支付'),
                      ),
                    const SizedBox(height: 8),
                    OutlinedButton(
                      onPressed: () =>
                          Navigator.of(context).pushReplacementNamed(
                        '/payment-result',
                        arguments: <String, String>{
                          'orderId': widget.orderId,
                          'paymentId': _session?.paymentId ?? '',
                          'paymentMethod': _session?.paymentMethod ?? '',
                        },
                      ),
                      child: const Text('查看支付结果'),
                    ),
                  ],
                ),
              ),
            ),
    );
  }

  Widget _buildAmountCard() {
    final order = _order;
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(20),
        child: Column(
          children: [
            Text(
              formatAmount(order?.totalAmount, currencyCode: order?.currencyCode),
              style: TextStyle(
                fontSize: 28,
                fontWeight: FontWeight.w800,
                color: Theme.of(context).colorScheme.primary,
              ),
            ),
            const SizedBox(height: 4),
            Text('订单号：${widget.orderId}'),
            if (_phase == CashierPhase.pending && order != null)
              Text(
                '支付剩余时间 ${formatCashierCountdown(_remainingSeconds)}',
                style: const TextStyle(color: Color(0xFFD97706)),
              ),
          ],
        ),
      ),
    );
  }

  Widget _buildMethodCard() {
    return Card(
      child: Padding(
        padding: const EdgeInsets.symmetric(vertical: 8),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 4),
              child: Text('选择支付方式', style: Theme.of(context).textTheme.titleSmall),
            ),
            RadioGroup<String>(
              groupValue: _selectedMethod,
              onChanged: (value) {
                if (value != null) {
                  setState(() => _selectedMethod = value);
                }
              },
              child: Column(
                children: [
                  for (final method in OrderService.orderPaymentMethods)
                    RadioListTile<String>(
                      value: method,
                      title: Text(paymentMethodLabel(method)),
                    ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildPaidCard() {
    return Card(
      color: const Color(0xFFF0FDF4),
      child: Padding(
        padding: const EdgeInsets.all(24),
        child: Column(
          children: const [
            Icon(Icons.check_circle, color: Color(0xFF16A34A), size: 48),
            SizedBox(height: 8),
            Text(
              '支付成功',
              style: TextStyle(
                fontSize: 22,
                fontWeight: FontWeight.w800,
                color: Color(0xFF16A34A),
              ),
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildClosedCard() {
    final message = switch (_phase) {
      CashierPhase.expired => '支付已超时，订单已关闭。',
      CashierPhase.cancelled => '订单已取消。',
      _ => '支付未完成，可重新发起。',
    };
    return Card(
      color: const Color(0xFFFEF2F2),
      child: Padding(
        padding: const EdgeInsets.all(24),
        child: Column(
          children: [
            const Icon(Icons.error_outline, color: Color(0xFFDC2626), size: 48),
            const SizedBox(height: 8),
            Text(
              message,
              style: const TextStyle(color: Color(0xFFDC2626)),
            ),
          ],
        ),
      ),
    );
  }

  Future<void> _loadOrder() async {
    setState(() {
      _loading = true;
      _error = '';
    });
    try {
      final order = await widget.orderService.getOrder(widget.orderId);
      if (_disposed) {
        return;
      }
      // 已支付订单直接进入终态，不再提供支付方式选择。
      final status = await widget.orderService.getOrderStatus(widget.orderId);
      if (_disposed) {
        return;
      }
      final phase = OrderService.resolvePhase(status, order.status);
      setState(() {
        _order = order;
        _phase = phase;
        _loading = false;
        _remainingSeconds = computeCashierRemainingSeconds(
          order.expireTime,
          DateTime.now().millisecondsSinceEpoch,
          DateTime.now().millisecondsSinceEpoch,
        );
      });
      if (phase == CashierPhase.pending) {
        _startCountdown(order.expireTime);
      }
    } catch (cause) {
      if (_disposed) {
        return;
      }
      setState(() {
        _loading = false;
        _error = '$cause';
      });
    }
  }

  Future<void> _createPayment() async {
    setState(() {
      _creating = true;
      _error = '';
    });
    try {
      final session =
          await widget.orderService.createPayment(widget.orderId, _selectedMethod);
      if (_disposed) {
        return;
      }
      setState(() {
        _session = session;
        _creating = false;
        _consecutivePollFailures = 0;
        _pollNotice = null;
        _remainingSeconds = computeCashierRemainingSeconds(
          _order?.expireTime,
          DateTime.now().millisecondsSinceEpoch,
          DateTime.now().millisecondsSinceEpoch,
        );
      });
      _startCountdown(_order?.expireTime);
      _schedulePoll(0);
    } catch (cause) {
      if (_disposed || !mounted) {
        return;
      }
      setState(() => _creating = false);
      ScaffoldMessenger.of(context)
          .showSnackBar(SnackBar(content: Text('$cause')));
    }
  }

  void _startCountdown(String? expireTime) {
    _countdownTimer?.cancel();
    _countdownTimer = Timer.periodic(const Duration(seconds: 1), (timer) {
      if (_disposed || _phase != CashierPhase.pending) {
        timer.cancel();
        return;
      }
      final next = computeCashierRemainingSeconds(
        expireTime,
        DateTime.now().millisecondsSinceEpoch,
        DateTime.now().millisecondsSinceEpoch,
      );
      if (mounted) {
        setState(() => _remainingSeconds = next);
      }
      if (next <= 0) {
        timer.cancel();
        // 到期终检：最后一次权威查询后再宣布过期。
        _pollOnce(finalCheck: true);
      }
    });
  }

  void _schedulePoll(int delayMs) {
    _pollTimer?.cancel();
    _pollTimer = Timer(Duration(milliseconds: delayMs), () {
      _pollOnce().then((_) {
        if (!_disposed && _phase == CashierPhase.pending) {
          _schedulePoll(nextPollDelayMs(consecutiveFailures: _consecutivePollFailures));
        }
      });
    });
  }

  Future<void> _pollOnce({bool finalCheck = false}) async {
    try {
      final status = await widget.orderService.getPaymentSuccess(widget.orderId);
      final orderStatus = await widget.orderService
          .getOrderStatus(widget.orderId)
          .then((value) => value.status)
          .catchError((_) => status.status);
      if (_disposed) {
        return;
      }
      _consecutivePollFailures = 0;
      final phase = OrderService.resolvePhase(status, orderStatus);
      if (phase != CashierPhase.pending && mounted) {
        setState(() {
          _phase = phase;
          _pollNotice = null;
        });
      } else if (finalCheck && mounted && _phase == CashierPhase.pending) {
        setState(() => _phase = CashierPhase.expired);
      }
    } catch (cause) {
      if (_disposed) {
        return;
      }
      // 瞬时网络错误保持待支付并对下一次轮询退避；持续中断以提示代替失败
      // （永久失败通过倒计时到期终检收敛）。
      _consecutivePollFailures += 1;
      if (_consecutivePollFailures >= cashierPollBackoffAfterFailures && mounted) {
        setState(() => _pollNotice = '网络不稳定，正在自动重试…');
      }
      if (finalCheck && mounted && _phase == CashierPhase.pending) {
        setState(() => _phase = CashierPhase.expired);
      }
    }
  }

  void _resetToPending() {
    setState(() {
      _phase = CashierPhase.pending;
      _session = null;
      _pollNotice = null;
      _consecutivePollFailures = 0;
    });
  }
}
