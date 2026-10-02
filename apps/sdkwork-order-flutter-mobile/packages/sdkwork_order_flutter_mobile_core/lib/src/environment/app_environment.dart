import 'dart:convert';

/// Typed runtime environment loaded from the dart-define env file
/// (`--dart-define-from-file=env/sdkwork.<profile>.<environment>.json`).
///
/// Keys per `ENVIRONMENT_SPEC.md` §5.1 and the order app: `SDKWORK_ENVIRONMENT`,
/// `SDKWORK_DEPLOYMENT_PROFILE`, `SDKWORK_PROFILE_ID`,
/// `SDKWORK_RUNTIME_TARGET` (`mobile`), `SDKWORK_ORDER_APP_API_BASE_URL`.
class AppEnvironment {
  const AppEnvironment({
    required this.environment,
    required this.deploymentProfile,
    required this.profileId,
    required this.runtimeTarget,
    required this.orderAppApiBaseUrl,
  });

  final String environment;
  final String deploymentProfile;
  final String profileId;
  final String runtimeTarget;
  final String orderAppApiBaseUrl;

  static const _environmentValue = String.fromEnvironment(
    'SDKWORK_ENVIRONMENT',
    defaultValue: 'development',
  );
  static const _deploymentProfileValue = String.fromEnvironment(
    'SDKWORK_DEPLOYMENT_PROFILE',
    defaultValue: 'standalone',
  );
  static const _profileIdValue = String.fromEnvironment(
    'SDKWORK_PROFILE_ID',
    defaultValue: 'sdkwork-order-flutter-mobile',
  );
  static const _runtimeTargetValue = String.fromEnvironment(
    'SDKWORK_RUNTIME_TARGET',
    defaultValue: 'mobile',
  );
  static const _orderAppApiBaseUrlValue = String.fromEnvironment(
    'SDKWORK_ORDER_APP_API_BASE_URL',
    defaultValue: 'https://api-dev.sdkwork.com/app/v3/api',
  );

  factory AppEnvironment.fromDefineValues() => const AppEnvironment(
        environment: _environmentValue,
        deploymentProfile: _deploymentProfileValue,
        profileId: _profileIdValue,
        runtimeTarget: _runtimeTargetValue,
        orderAppApiBaseUrl: _orderAppApiBaseUrlValue,
      );

  Map<String, String> toJson() => <String, String>{
        'environment': environment,
        'deploymentProfile': deploymentProfile,
        'profileId': profileId,
        'runtimeTarget': runtimeTarget,
        'orderAppApiBaseUrl': orderAppApiBaseUrl,
      };

  @override
  String toString() => jsonEncode(toJson());
}
