import 'package:flutter/foundation.dart';

/// Global token-manager equivalent for the Flutter mobile root.
///
/// The IAM Dart family is pending (see root `docs/decisions.md`), so the
/// session holds the platform-issued bearer token only; the login screen
/// seeds it (manual entry) or the app bootstrap injects a dev token.
/// Secure-storage persistence plugs in here once the identity contract
/// lands. Logout clears the token and notifies every listener (transport
/// reads the token through a provider function, so no stale header can
/// survive a sign-out).
class AppSession extends ChangeNotifier {
  AppSession._();

  static final AppSession instance = AppSession._();

  String _token = '';

  String get token => _token;

  bool get isLoggedIn => _token.isNotEmpty;

  void signIn(String token) {
    _token = token.trim();
    notifyListeners();
  }

  void signOut() {
    if (_token.isEmpty) {
      return;
    }
    _token = '';
    notifyListeners();
  }
}
