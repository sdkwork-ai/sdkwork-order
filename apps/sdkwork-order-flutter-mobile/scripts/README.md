# scripts/

Build and verification helpers for the Flutter mobile root live here as the
root grows. Standard commands today (spec
`FLUTTER_APP_MOBILE_ARCHITECTURE_SPEC.md` §10):

```bash
flutter pub get
flutter analyze
flutter test
flutter run --dart-define-from-file=env/sdkwork.standalone.development.json
flutter build apk  --dart-define-from-file=env/sdkwork.cloud.production.json
flutter build appbundle --dart-define-from-file=env/sdkwork.cloud.production.json
# iOS builds require macOS + Apple tooling:
flutter build ipa --dart-define-from-file=env/sdkwork.cloud.production.json
```

`scripts/` intentionally stays thin until repository tooling orchestrates
Flutter (pnpm aliases land with the module `bin/` wiring).
