import 'package:flutter/material.dart';

import 'bootstrap/environment.dart';
import 'bootstrap/providers.dart';
import 'shell/mobile_shell.dart';

void main() {
  // 组合根：dart-define 环境注入传输层，各领域服务独立装配（薄入口）。
  final environment = loadBootstrapEnvironment();
  OrderMobileProviders.init(environment);
  debugPrint('sdkwork-order-flutter-mobile runtime: $environment');
  runApp(const SdkworkOrderMobileShell());
}
