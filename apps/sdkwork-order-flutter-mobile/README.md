# sdkwork-order-flutter-mobile

SDKWork **Order** 能力(domain `commerce`, capability `order`)的 Flutter 移动端。
订单中心、收银台、充值中心(Token Bank)、券码兑换、提现、退款申请,全部走
order app-api(`/app/v3/api/orders|recharges|withdrawals`)。

## 运行

```bash
flutter pub get
# standalone 开发环境:
flutter run --dart-define-from-file=env/sdkwork.standalone.development.json
# cloud 生产环境:
flutter run --dart-define-from-file=env/sdkwork.cloud.production.json
```

登录:IAM Dart 家族落地前,沿用平台签发访问令牌 —— 「我的」页点击登录手输 token
(或以 bootstrap 注入),所有请求携带 `Authorization: Bearer <token>`。

## 布局

```text
lib/                 薄入口:main.dart + bootstrap/(环境、providers)+ shell/(底 tab)
packages/
  sdkwork_order_flutter_mobile_core/     transport seam、环境、会话、4 个领域 service、纯逻辑
  sdkwork_order_flutter_mobile_orders/   全部 screens/widgets(构造注入 service)
env/                 10 个 dart-define 文件(standalone|cloud × dev|test|staging|demo|production)
config/app|host/     运行时模板与平台打包元数据(example,无密钥)
sdks/                Dart SDK family 占位(待生成,见 README)
specs/               component.spec.json(type flutter-mobile-app)
docs/decisions.md    架构决策记录
```

## 验证

```bash
flutter analyze      # 0 issues
flutter test         # 根 widget 测试
(cd packages/sdkwork_order_flutter_mobile_core && flutter analyze && flutter test)
(cd packages/sdkwork_order_flutter_mobile_orders && flutter analyze && flutter test)
```

规范依据:`FLUTTER_APP_MOBILE_ARCHITECTURE_SPEC.md`、`APP_FLUTTER_UI_SPEC.md`、
`API_SPEC.md` §4.5/§14/§15、`PAGINATION_SPEC.md`。API 权威:
`apis/app-api/order/order-app-api.openapi.json`。
