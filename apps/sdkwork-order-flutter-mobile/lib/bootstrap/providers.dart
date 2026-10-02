import 'package:sdkwork_order_flutter_mobile_core/sdkwork_order_flutter_mobile_core.dart';

/// Composition root for the order Flutter mobile building blocks.
///
/// Bootstrap constructs the transport seam from the dart-define environment
/// and every domain service on top of it; screens receive only the service
/// they need (never the graph, never the transport). Tests install a fake
/// transport through [OrderMobileProviders.initForTesting].
class OrderMobileProviders {
  OrderMobileProviders._(
    this.environment,
    this.transport,
  )   : orders = OrderService(transport),
        shipments = ShipmentService(transport),
        recharge = RechargeService(transport),
        withdrawals = WithdrawalService(transport),
        refunds = RefundService(transport);

  static OrderMobileProviders? _instance;

  final AppEnvironment environment;
  final OrderApiTransport transport;
  final OrderService orders;
  final ShipmentService shipments;
  final RechargeService recharge;
  final WithdrawalService withdrawals;
  final RefundService refunds;

  /// Composition root installed by the app entry.
  static OrderMobileProviders get instance {
    final current = _instance;
    if (current != null) {
      return current;
    }
    return _instance = OrderMobileProviders._(
      AppEnvironment.fromDefineValues(),
      HttpOrderApiTransport(
        appApiBaseUrl: AppEnvironment.fromDefineValues().orderAppApiBaseUrl,
      ),
    );
  }

  /// Installs the composition root from an explicit environment (app entry;
  /// also the injection point for tests).
  static void init(AppEnvironment environment) {
    _instance ??= OrderMobileProviders._(
      environment,
      HttpOrderApiTransport(appApiBaseUrl: environment.orderAppApiBaseUrl),
    );
  }

  /// Installs an explicit transport (tests install fakes here).
  static void initForTesting(AppEnvironment environment, OrderApiTransport transport) {
    _instance = OrderMobileProviders._(environment, transport);
  }

  /// Resets to composition root (test isolation).
  static void resetForTesting() {
    _instance = null;
  }
}
