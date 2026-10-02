/// App-root bootstrap environment: a thin alias over the core dart-define
/// environment so the entry stays declarative and testable. The typed
/// dart-define reader itself lives in the core package (single source of
/// truth for `SDKWORK_*` keys); `AppEnvironment` is re-exported there.
library;

import 'package:sdkwork_order_flutter_mobile_core/sdkwork_order_flutter_mobile_core.dart';

/// Loads the runtime environment from `--dart-define-from-file`.
AppEnvironment loadBootstrapEnvironment() => AppEnvironment.fromDefineValues();
