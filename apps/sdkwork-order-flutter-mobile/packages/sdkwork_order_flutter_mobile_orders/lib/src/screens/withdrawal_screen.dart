import 'package:flutter/material.dart';

import 'package:sdkwork_order_flutter_mobile_core/sdkwork_order_flutter_mobile_core.dart';

import '../l10n/strings.dart';

/// 提现：表单 + POST /withdrawals/requests（幂等键）→ 受理结果.
class WithdrawalScreen extends StatefulWidget {
  const WithdrawalScreen({super.key, required this.withdrawalService});

  final WithdrawalService withdrawalService;

  @override
  State<WithdrawalScreen> createState() => _WithdrawalScreenState();
}

class _WithdrawalScreenState extends State<WithdrawalScreen> {
  final TextEditingController _amount = TextEditingController();
  final TextEditingController _account = TextEditingController();
  final TextEditingController _reason = TextEditingController();
  String _payoutMethod = payoutMethods.first.key;
  AccountValueRequest? _request;
  String? _failure;
  bool _submitting = false;

  @override
  void dispose() {
    _amount.dispose();
    _account.dispose();
    _reason.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('余额提现')),
      body: ListView(
        padding: const EdgeInsets.all(20),
        children: [
          TextField(
            controller: _amount,
            keyboardType:
                const TextInputType.numberWithOptions(decimal: true),
            decoration: const InputDecoration(
              labelText: '提现金额',
              prefixText: '¥ ',
              hintText: '请输入提现金额（最多两位小数）',
              border: OutlineInputBorder(),
            ),
          ),
          const SizedBox(height: 16),
          DropdownButtonFormField<String>(
            initialValue: _payoutMethod,
            decoration: const InputDecoration(
              labelText: '提现方式',
              border: OutlineInputBorder(),
            ),
            items: payoutMethods
                .map(
                  (entry) => DropdownMenuItem<String>(
                    value: entry.key,
                    child: Text(entry.value),
                  ),
                )
                .toList(),
            onChanged: (value) {
              if (value != null) {
                setState(() => _payoutMethod = value);
              }
            },
          ),
          const SizedBox(height: 16),
          TextField(
            controller: _account,
            decoration: InputDecoration(
              labelText: '收款账号',
              hintText: _payoutMethod == 'bank_card' ? '请输入银行卡号' : '请输入支付宝账号',
              border: const OutlineInputBorder(),
            ),
          ),
          const SizedBox(height: 16),
          TextField(
            controller: _reason,
            decoration: const InputDecoration(
              labelText: '事由（选填）',
              border: OutlineInputBorder(),
            ),
          ),
          const SizedBox(height: 20),
          FilledButton(
            onPressed: _submitting ? null : _submit,
            child: Text(_submitting ? '提交中...' : '提交提现申请'),
          ),
          if (_failure != null) ...[
            const SizedBox(height: 16),
            Text(_failure!, style: const TextStyle(color: Color(0xFFDC2626))),
          ],
          if (_request != null) ...[
            const SizedBox(height: 20),
            Card(
              color: const Color(0xFFF0FDF4),
              child: Padding(
                padding: const EdgeInsets.all(16),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    const Text(
                      '提现申请已提交',
                      style: TextStyle(
                        fontSize: 18,
                        fontWeight: FontWeight.w700,
                        color: Color(0xFF16A34A),
                      ),
                    ),
                    const SizedBox(height: 8),
                    Text('申请编号：${_request!.requestNo.isNotEmpty ? _request!.requestNo : _request!.requestId}'),
                    Text('状态：${requestStatusLabel(_request!.status)}'),
                    Text('金额：${formatAmount(_request!.amount, currencyCode: _request!.currencyCode)}'),
                    Text('申请时间：${formatTime(_request!.createdAt)}'),
                    const SizedBox(height: 4),
                    const Text(
                      '平台审核通过后打款，冻结金额以账户侧为准。',
                      style: TextStyle(fontSize: 12, color: Color(0xFF6B7280)),
                    ),
                  ],
                ),
              ),
            ),
          ],
        ],
      ),
    );
  }

  Future<void> _submit() async {
    setState(() {
      _failure = null;
      _request = null;
    });
    final amount = _amount.text.trim();
    final account = _account.text.trim();
    if (toMinorUnitString(amount) == null || toMinorUnitString(amount) == '0') {
      setState(() => _failure = '请输入有效的提现金额（最多两位小数）');
      return;
    }
    if (account.isEmpty) {
      setState(() => _failure = '请填写收款账号');
      return;
    }
    setState(() => _submitting = true);
    try {
      final request = await widget.withdrawalService.createWithdrawalRequest(
        majorAmount: amount,
        payoutMethod: _payoutMethod,
        payoutAccountRef: account,
        reasonCode: _reason.text.trim().isEmpty ? null : _reason.text.trim(),
      );
      if (!mounted) {
        return;
      }
      setState(() {
        _request = request;
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
