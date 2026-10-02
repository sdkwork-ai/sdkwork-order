import 'package:flutter/material.dart';

import 'package:sdkwork_order_flutter_mobile_core/sdkwork_order_flutter_mobile_core.dart';

import '../l10n/strings.dart';
import '../widgets/common_views.dart';

/// 退款申请：从订单详情预填进入（GET+POST /orders/refund_requests）.
class RefundRequestScreen extends StatefulWidget {
  const RefundRequestScreen({
    super.key,
    required this.refundService,
    this.prefillOrderId = '',
    this.prefillAmount = '',
    this.prefillCurrencyCode = 'CNY',
  });

  final RefundService refundService;

  /// Route prefill from the order detail (`orderId`, `amount` in major units,
  /// `currencyCode`).
  final String prefillOrderId;
  final String prefillAmount;
  final String prefillCurrencyCode;

  @override
  State<RefundRequestScreen> createState() => _RefundRequestScreenState();
}

class _RefundRequestScreenState extends State<RefundRequestScreen> {
  late final TextEditingController _orderId =
      TextEditingController(text: widget.prefillOrderId);
  late final TextEditingController _amount =
      TextEditingController(text: widget.prefillAmount);
  final TextEditingController _reasonCode = TextEditingController();
  final TextEditingController _reasonDetail = TextEditingController();
  String _targetAsset = 'token_bank';
  List<AccountValueRequest> _requests = const <AccountValueRequest>[];
  bool _loadingList = true;
  bool _submitting = false;
  String? _failure;
  AccountValueRequest? _created;

  static const _assetChoices = <MapEntry<String, String>>[
    MapEntry('token_bank', 'Token Bank'),
    MapEntry('points', '积分'),
    MapEntry('cash', '现金'),
  ];

  @override
  void initState() {
    super.initState();
    _loadRequests();
  }

  @override
  void dispose() {
    _orderId.dispose();
    _amount.dispose();
    _reasonCode.dispose();
    _reasonDetail.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('退款申请')),
      body: ListView(
        padding: const EdgeInsets.all(20),
        children: [
          TextField(
            controller: _orderId,
            decoration: const InputDecoration(
              labelText: '原始订单号',
              border: OutlineInputBorder(),
            ),
          ),
          const SizedBox(height: 16),
          Text('退款资产', style: Theme.of(context).textTheme.titleSmall),
          const SizedBox(height: 8),
          Wrap(
            spacing: 8,
            children: _assetChoices
                .map(
                  (choice) => ChoiceChip(
                    label: Text(choice.value),
                    selected: _targetAsset == choice.key,
                    onSelected: (_) => setState(() => _targetAsset = choice.key),
                  ),
                )
                .toList(),
          ),
          const SizedBox(height: 16),
          TextField(
            controller: _amount,
            keyboardType:
                const TextInputType.numberWithOptions(decimal: true),
            decoration: InputDecoration(
              labelText: '退款金额',
              prefixText: _targetAsset == 'cash' ? '¥ ' : null,
              hintText: _targetAsset == 'cash' ? '最多两位小数' : '退款额度（整数）',
              border: const OutlineInputBorder(),
            ),
          ),
          const SizedBox(height: 16),
          TextField(
            controller: _reasonCode,
            decoration: const InputDecoration(
              labelText: '原因代码（选填）',
              border: OutlineInputBorder(),
            ),
          ),
          const SizedBox(height: 16),
          TextField(
            controller: _reasonDetail,
            maxLines: 3,
            decoration: const InputDecoration(
              labelText: '原因说明（选填）',
              border: OutlineInputBorder(),
            ),
          ),
          const SizedBox(height: 20),
          FilledButton(
            onPressed: _submitting ? null : _submit,
            child: Text(_submitting ? '提交中...' : '提交退款申请'),
          ),
          if (_failure != null) ...[
            const SizedBox(height: 16),
            Text(_failure!, style: const TextStyle(color: Color(0xFFDC2626))),
          ],
          if (_created != null) ...[
            const SizedBox(height: 16),
            Card(
              color: const Color(0xFFF0FDF4),
              child: Padding(
                padding: const EdgeInsets.all(16),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    const Text(
                      '退款申请已受理',
                      style: TextStyle(
                        fontWeight: FontWeight.w700,
                        color: Color(0xFF16A34A),
                      ),
                    ),
                    const SizedBox(height: 8),
                    Text('申请编号：${_created!.requestNo.isNotEmpty ? _created!.requestNo : _created!.requestId}'),
                    Text('状态：${requestStatusLabel(_created!.status)}'),
                    Text(
                      '金额：${formatAmount(_created!.amount, currencyCode: _created!.currencyCode)}',
                    ),
                  ],
                ),
              ),
            ),
          ],
          const SizedBox(height: 24),
          Text('我的退款记录', style: Theme.of(context).textTheme.titleSmall),
          const SizedBox(height: 8),
          if (_loadingList)
            const SdkworkLoadingView(label: '加载退款记录...')
          else if (_requests.isEmpty)
            const SdkworkEmptyView(message: '暂无退款记录')
          else
            ..._requests.map(_requestTile),
        ],
      ),
    );
  }

  Widget _requestTile(AccountValueRequest request) {
    return Card(
      margin: const EdgeInsets.only(bottom: 8),
      child: ListTile(
        title: Text(
          request.requestNo.isNotEmpty ? request.requestNo : request.requestId,
        ),
        subtitle: Text(
          '${refundAssetLabel(request.targetAsset)} · '
          '${formatAmount(request.amount, currencyCode: request.currencyCode)} · '
          '${formatTime(request.createdAt)}',
        ),
        trailing: Text(requestStatusLabel(request.status)),
      ),
    );
  }

  Future<void> _loadRequests() async {
    setState(() => _loadingList = true);
    try {
      final requests = await widget.refundService.listRefundRequests();
      if (!mounted) {
        return;
      }
      setState(() {
        _requests = requests;
        _loadingList = false;
      });
    } catch (cause) {
      if (!mounted) {
        return;
      }
      setState(() => _loadingList = false);
      ScaffoldMessenger.of(context)
          .showSnackBar(SnackBar(content: Text('加载退款记录失败：$cause')));
    }
  }

  Future<void> _submit() async {
    setState(() {
      _failure = null;
      _created = null;
    });
    try {
      final request = await widget.refundService.createRefundRequest(
        originalOrderId: _orderId.text.trim(),
        targetAsset: _targetAsset,
        amount: _amount.text.trim(),
        currencyCode: widget.prefillCurrencyCode,
        reasonCode:
            _reasonCode.text.trim().isEmpty ? null : _reasonCode.text.trim(),
        reasonDetail:
            _reasonDetail.text.trim().isEmpty ? null : _reasonDetail.text.trim(),
      );
      if (!mounted) {
        return;
      }
      setState(() {
        _created = request;
        _submitting = false;
      });
      await _loadRequests();
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
