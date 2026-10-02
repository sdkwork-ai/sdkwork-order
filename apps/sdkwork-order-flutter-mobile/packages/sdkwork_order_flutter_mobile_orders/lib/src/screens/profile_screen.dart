import 'package:flutter/material.dart';

import 'package:sdkwork_order_flutter_mobile_core/sdkwork_order_flutter_mobile_core.dart';

/// 「我的」页：登录态、订单统计、常用服务入口（券码 / 提现 / 退款）.
class ProfileScreen extends StatefulWidget {
  const ProfileScreen({super.key, required this.orderService, this.session});

  final OrderService orderService;

  /// Defaults to the global session when omitted.
  final AppSession? session;

  @override
  State<ProfileScreen> createState() => _ProfileScreenState();
}

class _ProfileScreenState extends State<ProfileScreen> {
  OrderStatistics? _statistics;
  bool _loadingStats = false;
  String? _statsError;

  AppSession get _session => widget.session ?? AppSession.instance;

  @override
  void initState() {
    super.initState();
    _session.addListener(_onSessionChanged);
    _loadStatistics();
  }

  @override
  void dispose() {
    _session.removeListener(_onSessionChanged);
    super.dispose();
  }

  void _onSessionChanged() {
    if (mounted) {
      _loadStatistics();
    }
  }

  @override
  Widget build(BuildContext context) {
    final session = _session;
    return ListView(
      children: [
        Container(
          color: Theme.of(context).colorScheme.primaryContainer,
          padding: const EdgeInsets.fromLTRB(20, 28, 20, 24),
          child: Row(
            children: [
              CircleAvatar(
                radius: 28,
                child: Text(
                  session.isLoggedIn ? '已' : '未',
                  style: const TextStyle(fontSize: 18),
                ),
              ),
              const SizedBox(width: 16),
              Expanded(
                child: ListenableBuilder(
                  listenable: session,
                  builder: (context, _) => Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        session.isLoggedIn ? '已登录' : '未登录',
                        style: const TextStyle(
                          fontSize: 18,
                          fontWeight: FontWeight.w700,
                        ),
                      ),
                      Text(
                        session.isLoggedIn ? '点击下方退出登录' : '点击登录以同步订单与资产',
                        style: Theme.of(context).textTheme.bodySmall,
                      ),
                    ],
                  ),
                ),
              ),
              FilledButton(
                onPressed: () => Navigator.of(context).pushNamed('/login'),
                child: Text(session.isLoggedIn ? '管理' : '点击登录'),
              ),
            ],
          ),
        ),
        _buildStatisticsCard(),
        SdkworkServicesCard(onNavigate: (route) => Navigator.of(context).pushNamed(route)),
        const Padding(
          padding: EdgeInsets.all(16),
          child: Text(
            '订单 / 充值 / 提现 / 退款能力由 sdkwork-order app-api 提供；'
            'VIP 会员属于 membership 外部能力，不在本端范围内。',
            textAlign: TextAlign.center,
            style: TextStyle(fontSize: 11, color: Color(0xFF9CA3AF)),
          ),
        ),
      ],
    );
  }

  Widget _buildStatisticsCard() {
    final stats = _statistics;
    return Card(
      margin: const EdgeInsets.fromLTRB(16, 16, 16, 0),
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                Text('订单统计', style: Theme.of(context).textTheme.titleSmall),
                if (_session.isLoggedIn && !_loadingStats)
                  TextButton(
                    onPressed: _loadStatistics,
                    child: const Text('刷新'),
                  ),
              ],
            ),
            const SizedBox(height: 8),
            if (!_session.isLoggedIn)
              const Text('登录后查看订单统计')
            else if (_loadingStats)
              const Padding(
                padding: EdgeInsets.all(8),
                child: SizedBox(
                  width: 22,
                  height: 22,
                  child: CircularProgressIndicator(strokeWidth: 2),
                ),
              )
            else if (_statsError != null)
              Text('统计加载失败：$_statsError',
                  style: const TextStyle(color: Color(0xFFDC2626)))
            else if (stats != null)
              Row(
                mainAxisAlignment: MainAxisAlignment.spaceAround,
                children: [
                  _statCell('全部', '${stats.totalOrders}'),
                  _statCell('待付款', '${stats.pendingPayment}'),
                  _statCell('待发货', '${stats.pendingShipment}'),
                  _statCell('待收货', '${stats.pendingReceipt}'),
                  _statCell('已完成', '${stats.completed}'),
                ],
              ),
            if (stats != null && _session.isLoggedIn)
              Padding(
                padding: const EdgeInsets.only(top: 8),
                child: Text(
                  '累计金额：${formatAmount(stats.totalAmount)}',
                  style: Theme.of(context).textTheme.bodySmall,
                ),
              ),
          ],
        ),
      ),
    );
  }

  Widget _statCell(String label, String value) {
    return Column(
      children: [
        Text(
          value,
          style: const TextStyle(fontSize: 18, fontWeight: FontWeight.w700),
        ),
        Text(label, style: Theme.of(context).textTheme.labelSmall),
      ],
    );
  }

  Future<void> _loadStatistics() async {
    if (!_session.isLoggedIn) {
      if (mounted) {
        setState(() {
          _statistics = null;
          _statsError = null;
        });
      }
      return;
    }
    setState(() {
      _loadingStats = true;
      _statsError = null;
    });
    try {
      final stats = await widget.orderService.getStatistics();
      if (!mounted) {
        return;
      }
      setState(() {
        _statistics = stats;
        _loadingStats = false;
      });
    } catch (cause) {
      if (!mounted) {
        return;
      }
      setState(() {
        _statsError = '$cause';
        _loadingStats = false;
      });
    }
  }
}

/// 常用服务入口卡片（路由名由 shell 注册表解析）.
class SdkworkServicesCard extends StatelessWidget {
  const SdkworkServicesCard({super.key, required this.onNavigate});

  final void Function(String route) onNavigate;

  static const _services = <MapEntry<String, IconData>>[
    MapEntry('/coupons', Icons.redeem),
    MapEntry('/withdraw', Icons.account_balance),
    MapEntry('/refund-requests', Icons.assignment_return),
  ];

  static const _labels = <String, String>{
    '/coupons': '券码兑换',
    '/withdraw': '余额提现',
    '/refund-requests': '退款申请',
  };

  @override
  Widget build(BuildContext context) {
    return Card(
      margin: const EdgeInsets.fromLTRB(16, 12, 16, 0),
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text('常用服务', style: Theme.of(context).textTheme.titleSmall),
            const SizedBox(height: 8),
            ..._services.map(
              (entry) => ListTile(
                contentPadding: EdgeInsets.zero,
                leading: Icon(entry.value),
                title: Text(_labels[entry.key] ?? entry.key),
                trailing: const Icon(Icons.chevron_right),
                onTap: () => onNavigate(entry.key),
              ),
            ),
          ],
        ),
      ),
    );
  }
}
