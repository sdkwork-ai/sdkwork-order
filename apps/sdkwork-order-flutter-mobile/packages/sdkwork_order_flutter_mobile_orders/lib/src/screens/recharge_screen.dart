import 'package:flutter/material.dart';

import 'package:sdkwork_order_flutter_mobile_core/sdkwork_order_flutter_mobile_core.dart';

import '../widgets/common_views.dart';

/// 充值中心：Token Bank 直充计划 + 充值套餐 → 创建充值订单（幂等键）→
/// 跳转共用收银台.
class RechargeScreen extends StatefulWidget {
  const RechargeScreen({super.key, required this.rechargeService});

  final RechargeService rechargeService;

  @override
  State<RechargeScreen> createState() => _RechargeScreenState();
}

class _RechargeScreenState extends State<RechargeScreen> {
  List<RechargePlan> _plans = const <RechargePlan>[];
  List<RechargePackage> _packages = const <RechargePackage>[];
  bool _loading = true;
  String? _submittingId;
  String _error = '';

  @override
  void initState() {
    super.initState();
    _load();
  }

  @override
  Widget build(BuildContext context) {
    return _loading
        ? const SdkworkLoadingView(label: '加载充值中心...')
        : _error.isNotEmpty
            ? SdkworkErrorView(message: _error, onRetry: _load)
            : RefreshIndicator(
                onRefresh: _load,
                child: ListView(
                  children: [
                    SdkworkSectionCard(
                      title: 'Token Bank 直充计划',
                      children: _plans.isEmpty
                          ? const [Text('暂无可用计划')]
                          : _plans.map(_planTile).toList(),
                    ),
                    SdkworkSectionCard(
                      title: '充值套餐',
                      children: _packages.isEmpty
                          ? const [Text('暂无可用套餐')]
                          : _packages.map(_packageTile).toList(),
                    ),
                    const Padding(
                      padding: EdgeInsets.all(16),
                      child: Text(
                        '提交后进入收银台完成支付；到账金额以服务端核算为准。',
                        style: TextStyle(fontSize: 12, color: Color(0xFF6B7280)),
                      ),
                    ),
                  ],
                ),
              );
  }

  Widget _planTile(RechargePlan plan) {
    final busy = _submittingId == 'plan:${plan.planCode}';
    return ListTile(
      contentPadding: EdgeInsets.zero,
      title: Text(plan.displayName),
      subtitle: Text(
        '到账 ${_majorDisplay(plan.priceAmount)}'
        '${plan.bonusAmount != '0' ? ' + 赠送 ${_minorDisplay(plan.bonusAmount)}' : ''}',
      ),
      trailing: busy
          ? const SizedBox(
              width: 22,
              height: 22,
              child: CircularProgressIndicator(strokeWidth: 2),
            )
          : FilledButton(
              onPressed: () => _buyPlan(plan),
              child: const Text('购买'),
            ),
    );
  }

  Widget _packageTile(RechargePackage package) {
    final busy = _submittingId == 'package:${package.id}';
    return ListTile(
      contentPadding: EdgeInsets.zero,
      title: Text('${_minorDisplay(package.priceAmount)} 充值套餐'),
      subtitle: Text(
        '含 ${package.points} Token'
        '${package.bonusPoints > 0 ? ' + 赠 ${package.bonusPoints}' : ''}',
      ),
      trailing: busy
          ? const SizedBox(
              width: 22,
              height: 22,
              child: CircularProgressIndicator(strokeWidth: 2),
            )
          : FilledButton(
              onPressed: () => _buyPackage(package),
              child: const Text('购买'),
            ),
    );
  }

  String _majorDisplay(String majorPrice) {
    final value = majorPrice.trim();
    final parsed = RegExp(r'^\d+(\.\d{1,2})?$').hasMatch(value);
    return parsed ? '¥$value' : formatMinorAmount(value);
  }

  String _minorDisplay(String minor) => formatMinorAmount(minor);

  Future<void> _load() async {
    setState(() {
      _loading = true;
      _error = '';
    });
    try {
      final plans = await widget.rechargeService.listPlans(status: 'active');
      final packages = await widget.rechargeService.listPackages();
      if (!mounted) {
        return;
      }
      setState(() {
        _plans = plans;
        _packages = packages;
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

  Future<void> _buyPlan(RechargePlan plan) async {
    setState(() => _submittingId = 'plan:${plan.planCode}');
    try {
      final order = await widget.rechargeService.createPlanRechargeOrder(plan);
      if (!mounted) {
        return;
      }
      _goCashier(order.orderId);
    } catch (cause) {
      _showFailure(cause);
    }
  }

  Future<void> _buyPackage(RechargePackage package) async {
    setState(() => _submittingId = 'package:${package.id}');
    try {
      final order =
          await widget.rechargeService.createPackageRechargeOrder(package);
      if (!mounted) {
        return;
      }
      _goCashier(order.orderId);
    } catch (cause) {
      _showFailure(cause);
    }
  }

  void _goCashier(String orderId) {
    setState(() => _submittingId = null);
    Navigator.of(context).pushNamed('/cashier', arguments: orderId);
  }

  void _showFailure(Object cause) {
    if (!mounted) {
      return;
    }
    setState(() => _submittingId = null);
    ScaffoldMessenger.of(context)
        .showSnackBar(SnackBar(content: Text('$cause')));
  }
}
