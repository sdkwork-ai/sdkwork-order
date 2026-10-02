import 'package:flutter/material.dart';

import 'package:sdkwork_order_flutter_mobile_core/sdkwork_order_flutter_mobile_core.dart';
import 'package:sdkwork_order_flutter_mobile_orders/sdkwork_order_flutter_mobile_orders.dart';

import '../bootstrap/providers.dart';

/// Application routes for the order flow (route ids align with the H5/PC
/// order surfaces; physical paths are mobile-local).
Map<String, WidgetBuilder> buildOrderMobileRoutes() => <String, WidgetBuilder>{
      '/login': (context) => const LoginScreen(),
      '/order-detail': (context) => OrderDetailScreen(
            orderService: _orders,
            orderId: '${_routeArguments(context)}',
          ),
      '/cashier': (context) => CashierScreen(
            orderService: _orders,
            orderId: '${_routeArguments(context)}',
          ),
      '/payment-result': (context) => PaymentResultScreen(
            orderService: _orders,
            arguments: _paymentResultArguments(context),
          ),
      '/coupons': (context) => CouponRedemptionScreen(orderService: _orders),
      '/withdraw': (context) =>
          WithdrawalScreen(withdrawalService: _withdrawals),
      '/refund-requests': (context) => _refundRequestsScreen(context),
    };

OrderService get _orders => OrderMobileProviders.instance.orders;
RechargeService get _recharge => OrderMobileProviders.instance.recharge;
WithdrawalService get _withdrawals => OrderMobileProviders.instance.withdrawals;

RefundRequestScreen _refundRequestsScreen(BuildContext context) {
  final arguments = _routeArguments(context);
  final prefill = arguments is Map
      ? arguments.map((key, value) => MapEntry('$key', '$value'))
      : const <String, String>{};
  return RefundRequestScreen(
    refundService: RefundService(OrderMobileProviders.instance.transport),
    prefillOrderId: prefill['orderId'] ?? '',
    prefillAmount: prefill['amount'] ?? '',
    prefillCurrencyCode: prefill['currencyCode'] ?? 'CNY',
  );
}

Map<String, String> _paymentResultArguments(BuildContext context) {
  final arguments = _routeArguments(context);
  if (arguments is Map<String, String>) {
    return arguments;
  }
  if (arguments is Map) {
    return arguments.map((key, value) => MapEntry('$key', '$value'));
  }
  return const <String, String>{};
}

Object? _routeArguments(BuildContext context) =>
    ModalRoute.of(context)?.settings.arguments;

/// Bottom-tab mobile shell: 订单 / 充值 / 我的.
class SdkworkOrderMobileShell extends StatelessWidget {
  const SdkworkOrderMobileShell({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: appTitle,
      theme: ThemeData(
        colorScheme: ColorScheme.fromSeed(seedColor: const Color(0xFFE93B3D)),
        useMaterial3: true,
      ),
      onGenerateRoute: (settings) {
        final builder = buildOrderMobileRoutes()[settings.name];
        if (builder == null) {
          return null;
        }
        return MaterialPageRoute<void>(builder: builder, settings: settings);
      },
      home: const SdkworkOrderHomeTabs(),
    );
  }
}

class SdkworkOrderHomeTabs extends StatefulWidget {
  const SdkworkOrderHomeTabs({super.key});

  @override
  State<SdkworkOrderHomeTabs> createState() => _SdkworkOrderHomeTabsState();
}

class _SdkworkOrderHomeTabsState extends State<SdkworkOrderHomeTabs> {
  int _tabIndex = 0;

  @override
  Widget build(BuildContext context) {
    final pages = <Widget>[
      OrdersScreen(orderService: _orders),
      RechargeScreen(rechargeService: _recharge),
      ProfileScreen(orderService: _orders),
    ];

    return Scaffold(
      appBar: AppBar(
        title: Text(_tabIndex == 0 ? '订单中心' : _tabIndex == 1 ? '充值中心' : appTitle),
      ),
      body: IndexedStack(index: _tabIndex, children: pages),
      bottomNavigationBar: NavigationBar(
        selectedIndex: _tabIndex,
        onDestinationSelected: (index) => setState(() => _tabIndex = index),
        destinations: const <Widget>[
          NavigationDestination(
            icon: Icon(Icons.receipt_long_outlined),
            selectedIcon: Icon(Icons.receipt_long),
            label: tabOrders,
          ),
          NavigationDestination(
            icon: Icon(Icons.account_balance_wallet_outlined),
            selectedIcon: Icon(Icons.account_balance_wallet),
            label: tabRecharge,
          ),
          NavigationDestination(
            icon: Icon(Icons.person_outline),
            selectedIcon: Icon(Icons.person),
            label: tabProfile,
          ),
        ],
      ),
    );
  }
}
