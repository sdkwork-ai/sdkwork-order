# sdkwork-order-mini-program

SDKWork Order 能力(domain `commerce`,capability `order`)的原生微信小程序端。
原生 `weixin-mini-program` 源码树(`project.config.json` 的 `miniprogramRoot` 指向 `src/`),
根 `src/` 保持薄:入口、bootstrap、路由贡献、SDK client 组合。

- App API 前缀:`/app/v3/api/orders`(权威契约:
  `apis/app-api/order/order-app-api.openapi.json`)
- 响应封套:`{ code: 0, data, traceId }`;错误为 HTTP 4xx/5xx
  `application/problem+json`(numeric `code` + `traceId`)
- 金额一律 minor-unit 整数字符串;id / int64 字段一律字符串(§13.6)

## 结构

- `src/services/transport.js` — 唯一 `wx.request` seam(Bearer、15s 超时、
  封套解包、problem+json 错误映射、`Idempotency-Key` 写命令头);契约测试与
  lint 强制全 app 仅此一处调用。
- `src/services/order-service.js` / `recharge-service.js` / `withdrawal-service.js`
  — 领域服务(1:1 对应 app-api 域)。
- `src/bootstrap/` — `environment.ts`(runtime-env 契约)、`sdkClients.ts`
  (TS 类型门面)、`routes.ts`(路由贡献 + placement)。
- `config/mini-program/runtime-env.<profile>.<env>.json` — 运行时环境模板
  (提交 `.example`;`scripts/build-runtime.mjs` 选择一个 profile 物化为
  `src/runtime-env.json`,该文件 git-ignore)。
- `config/host/mp-weixin.example.json` — 平台 appid 等宿主配置模板(无密钥)。
- `project.private.config.json` — 微信开发者工具本机配置,不提交
  (提供 `.example`)。

## 常用命令

```bash
node scripts/build-runtime.mjs                # 物化 src/runtime-env.json(默认 standalone.dev)
node scripts/build-runtime.mjs --deployment-profile cloud --environment prod
node --test tests/                            # 架构契约测试
node scripts/typecheck.mjs                    # tsc --noEmit(或无 TS 依赖时的语法 pass)
node scripts/lint.mjs                         # JSON 有效性 / 唯一 seam / 禁止 import / 密钥扫描
```

小程序开发流:`node scripts/build-runtime.mjs` 后用微信开发者工具打开本目录
(`project.config.json`),本地联调可在宿主本地
`config/mini-program/runtime-env.standalone.dev.json` 将 `appApiBaseUrl` 指向
`http://127.0.0.1:3900/app/v3/api` 并开启"不校验合法域名"。

## 边界

- 本目录**不在**根 `pnpm-workspace.yaml` 中,无 npm 依赖:原生小程序树由
  微信开发者工具构建,脚本与测试只用 Node 标准库。
- 决策记录见 `docs/decisions.md`;组件契约见 `specs/component.spec.json`。
