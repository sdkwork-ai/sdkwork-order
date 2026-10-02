import 'package:flutter/material.dart';

import 'package:sdkwork_order_flutter_mobile_core/sdkwork_order_flutter_mobile_core.dart';

import '../l10n/strings.dart';

/// 券码兑换：输入码 → POST /orders/coupon_redemptions（幂等键）→ 权益结果.
class CouponRedemptionScreen extends StatefulWidget {
  const CouponRedemptionScreen({super.key, required this.orderService});

  final OrderService orderService;

  @override
  State<CouponRedemptionScreen> createState() => _CouponRedemptionScreenState();
}

class _CouponRedemptionScreenState extends State<CouponRedemptionScreen> {
  final TextEditingController _code = TextEditingController();
  CouponRedemptionResult? _result;
  String? _failure;
  bool _submitting = false;

  @override
  void dispose() {
    _code.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('券码兑换')),
      body: ListView(
        padding: const EdgeInsets.all(20),
        children: [
          TextField(
            controller: _code,
            textCapitalization: TextCapitalization.characters,
            decoration: const InputDecoration(
              labelText: '券码',
              hintText: '请输入兑换码',
              border: OutlineInputBorder(),
            ),
          ),
          const SizedBox(height: 16),
          FilledButton(
            onPressed: _submitting ? null : _redeem,
            child: Text(_submitting ? '核销中...' : '立即兑换'),
          ),
          if (_failure != null) ...[
            const SizedBox(height: 20),
            Card(
              color: const Color(0xFFFEF2F2),
              child: Padding(
                padding: const EdgeInsets.all(16),
                child: Text(
                  _failure!,
                  style: const TextStyle(color: Color(0xFFDC2626)),
                ),
              ),
            ),
          ],
          if (_result != null) ...[
            const SizedBox(height: 20),
            Card(
              color: _result!.completed
                  ? const Color(0xFFF0FDF4)
                  : const Color(0xFFFFFBEB),
              child: Padding(
                padding: const EdgeInsets.all(16),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      _result!.completed ? '兑换成功' : '兑换已受理',
                      style: TextStyle(
                        fontSize: 18,
                        fontWeight: FontWeight.w700,
                        color: _result!.completed
                            ? const Color(0xFF16A34A)
                            : const Color(0xFFD97706),
                      ),
                    ),
                    if (_result!.replayed)
                      const Padding(
                        padding: EdgeInsets.only(top: 4),
                        child: Text('该券码此前已兑换，本次为去重放结果。'),
                      ),
                    const SizedBox(height: 8),
                    Text('权益：${benefitKindLabel(_result!.benefitKind)}'),
                    if (_result!.grantAmount != null)
                      Text('到账额度：${formatMinorAmount(_result!.grantAmount)}'),
                    if (_result!.durationDays != null)
                      Text('有效天数：${_result!.durationDays} 天'),
                    if (_result!.orderNo.isNotEmpty)
                      Text('兑换单号：${_result!.orderNo}'),
                  ],
                ),
              ),
            ),
          ],
        ],
      ),
    );
  }

  Future<void> _redeem() async {
    final code = _code.text.trim();
    if (code.isEmpty) {
      setState(() {
        _failure = '请输入券码';
        _result = null;
      });
      return;
    }
    setState(() {
      _submitting = true;
      _failure = null;
      _result = null;
    });
    try {
      final result = await widget.orderService.redeemCoupon(code);
      if (!mounted) {
        return;
      }
      setState(() {
        _result = result;
        _submitting = false;
      });
    } catch (cause) {
      if (!mounted) {
        return;
      }
      setState(() {
        _failure = '$cause';
        _submitting = false;
      });
    }
  }
}
