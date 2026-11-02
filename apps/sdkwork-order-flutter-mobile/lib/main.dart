import 'package:flutter/material.dart';

import 'bootstrap/environment.dart';
import 'bootstrap/providers.dart';
import 'bootstrap/sdkwork_tls_trust.dart';
import 'shell/mobile_shell.dart';

void main() {
  // The HTTPS trust layer must be installed before any HTTP client is
  // created, so it runs ahead of binding initialization and services.
  SdkworkTlsTrust.installForApp();

  // 组合根：dart-define 环境注入传输层，各领域服务独立装配（薄入口）。
  final environment = loadBootstrapEnvironment();
  OrderMobileProviders.init(environment);
  debugPrint('sdkwork-order-flutter-mobile runtime: $environment');
  runApp(const SdkworkOrderMobileShell());
}
