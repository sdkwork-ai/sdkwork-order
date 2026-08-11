# 订单生命周期与虚拟/实物商品差异矩阵

> 归属：sdkwork-order 订单中心。本文是订单全生命周期各操作在**虚拟商品**与**实物商品**两类订单上的处理差异权威清单，以及库存动作与支付倒计时的规则。

## 1. 订单类型判定

| 维度 | 虚拟商品订单 | 实物商品订单 |
| --- | --- | --- |
| 下单入口 | 充值（`recharge_router`）、会员（`membership_router`）、账户价值（account value / coupon recharge）领域路由 | 通用收银台 `POST /app/v3/api/checkout/sessions/{id}/orders`（`checkout_router`） |
| subject | `points_recharge` / `token_bank_*` / `account_recharge_package` / `coupon_recharge` / `membership` / `virtual_goods` | `product` / `physical` / `physical_shipment`（SKU snapshot `fulfillment_type`） |
| 库存预留 | 无 | 下单时 `reserve_physical_order_inventory`（`checkout_router.rs:450`），置 `fulfillment_status='inventory_reserved'` |
| 支付倒计时 | 30 分钟（`expired_at`，`SDKWORK_ORDER_PAYMENT_EXPIRE_SECONDS` 可配） | 30 分钟（session 与订单同源，session 过期后不可下单） |

## 2. 操作差异矩阵

| 操作 | 虚拟商品订单 | 实物商品订单 | 库存动作 |
| --- | --- | --- | --- |
| 下单 | 领域路由创建订单，`status='pending_payment'` | 收银台下单 + 库存预留 | 实物：reserve（扣 available、加 reserved） |
| 支付结算 | subject 分流履约：积分入账 / 账户价值入账 / 会员激活（`order_payment_settlement.rs`） | `Product` 分支 → `consume_order_inventory`（扣 reserved、加 sold） | 实物：consume |
| 用户取消 | 充值路由取消（无库存） | 通用取消，`fulfillment_status='inventory_reserved'` 才释放库存（`owner_order_cancel.rs`） | 实物：release（回补 available、减 reserved） |
| 管理员取消 | 无库存处理 | 释放库存（`backend_management_lifecycle.rs`） | 实物：release |
| 管理员关闭 | 无库存处理 | 释放库存 | 实物：release |
| 超时自动过期 | scheduler 置 `expired`（虚拟无库存动作） | scheduler 置 `expired` + 释放库存 | 实物：release |
| 发货 | 不适用 | 支付成功后 `PhysicalGoodsFulfillmentPort.fulfill`（consume 已随支付完成）；**商家包裹置 `shipped` → 沿 shipment → fulfillment → 订单推进 `status='shipped'` + 事件**（`postgres_shipment.rs`，幂等） | 实物：consume（支付时） |
| 确认收货 | 不适用 | `POST /app/v3/api/orders/{orderId}/receipt-confirmations`（`orders.receipts.confirm`，幂等）：`paid/fulfilled/shipped` → **`completed`（fulfillment `delivered`）** + 事件 | 无 |
| 退款（资金） | `commerce_order_refund_request`：**仅已支付订单可发起**（创建时校验 `payment_status`，未支付/取消/关闭/过期 → 拒绝）；hold → provider refund → settle，扣回积分/账户价值；成功后同步 `refund_status='refunded'` | 同左；**未发货实物订单退款成功后自动释放预留库存**（`account_value_request_execution.rs`，release 幂等语义对虚拟/已发货订单自动跳过） | 实物未发货：release |
| 售后/退货 | after_sales 状态机（虚拟退款走 refund_request 扣回路径） | after_sales 审核 `completed` 后按 `fulfillment_status` 回补：`awaiting_shipment/shipped/delivered` → restock；`inventory_reserved` → release | 实物：restock（回补 available、减 sold）或 release |
| 退款成功 | 订单 `refund_status='refunded'` 同步 | 同左 + 未发货预留释放 | 实物未发货：release |

## 3. 库存状态机（实物订单）

```
reserve ──► reserved ──► consumed（支付结算）──► restocked（退货回补）
                │
                ├──► released（取消/关闭/过期/未发货退款）
                └──► released（reservation 超时清扫，`reservation_expired`）
```

- `reserved`：下单预留（30 分钟支付窗口内有效，`expires_at` 同步）
- `consumed`：支付成功出库
- `released`：取消/关闭/过期/未发货退款释放（回补 available）
- `restocked`：退货入库（回补 available、冲减 sold）

幂等规则：`release` 对 `released`/`consumed` 跳过；`restock` 对 `restocked`/`released` 跳过；并发扫描由 `FOR UPDATE SKIP LOCKED` + 状态条件保证单次生效。

**一致性兜底（reservation 超时清扫）**：scheduler 每 tick 额外扫描 `status='reserved' AND expires_at <= now` 的预留并释放（`sweep_expired_inventory_reservations`）—— 覆盖释放失败、老订单缺 `expired_at`、废弃支付窗口等所有悬挂场景，与订单当前状态无关。

## 4. 支付倒计时

- 配置：`SDKWORK_ORDER_PAYMENT_EXPIRE_SECONDS`（默认 1800，clamp 60..86400），所有订单类型共用（`sdkwork-order-service/src/config.rs`）
- checkout session 创建时写入 `expires_at`（同窗口），**下单时校验 session 与 quote 均未过期**（`postgres_order.rs` `load_checkout_session_for_order` / `load_checkout_quote_for_order`）
- 响应透出：checkout session / checkout order 响应 `expiresAt`；订单列表/详情 `expireTime`；充值/会员下单响应 `expiresAt`
- 前端据此渲染剩余倒计时
- 支付前二次校验：`pay_owner_order` 拒绝已过期/不可支付订单（`sdkwork-payment` `postgres_owner_order_payment.rs`）

## 5. 超时自动过期（scheduler）

- 进程内嵌于 `sdkwork-api-order-standalone-gateway`（`sdkwork-order-service-host/src/expiration.rs`）
- 配置：
  - `SDKWORK_ORDER_EXPIRATION_SCHEDULER_ENABLED`（默认启用）
  - `SDKWORK_ORDER_EXPIRATION_SCHEDULER_INTERVAL_SECONDS`（默认 60，clamp 10..3600）
  - `SDKWORK_ORDER_EXPIRATION_BATCH_SIZE`（默认 200）
- 每 tick：扫描 `status IN ('draft','pending','pending_payment','unpaid','wait_pay') AND expired_at <= now`（兼容 unix 秒与 RFC3339 两种存储格式）→ 置 `status='expired', payment_status='expired'` + 写 `expired` 事件（actor=system）→ 关闭支付尝试（best-effort）→ 实物订单释放库存（best-effort）
- 幂等：已终态订单跳过；失败仅告警不影响其余订单

## 6. 明确边界（暂不支持）

- after_sales 审核触发真实资金退款（需与 `commerce_order_refund_request` 体系合并）
- 售后多行部分退货的按行库存回补（当前按订单整体回补）
- 独立 worker 进程部署（scheduler 内嵌 gateway）
- 独立 worker 进程部署（scheduler 内嵌 gateway）

## 7. 关键文件索引

| 能力 | 文件 |
| --- | --- |
| 倒计时配置 | `crates/sdkwork-order-service/src/config.rs` |
| checkout 过期时间/校验 | `crates/sdkwork-order-repository-sqlx/src/postgres_checkout.rs`、`postgres_order.rs` |
| 过期扫描 | `crates/sdkwork-order-repository-sqlx/src/postgres_expiration.rs` |
| 过期 scheduler | `crates/sdkwork-order-service-host/src/expiration.rs` |
| 库存预留/释放/回补 | `crates/sdkwork-order-integration-physical-commerce/src/inventory.rs` |
| 取消库存判断 | `crates/sdkwork-routes-order-app-api/src/owner_order_cancel.rs` |
| 售后库存回补 | `crates/sdkwork-routes-order-backend-api/src/backend_commerce_admin_router.rs` |
| 退款订单同步 | `crates/sdkwork-order-service/src/service/account_value_request_execution.rs` |
| after_sales DDL | `database/ddl/baseline/postgres/0001_order_baseline.sql` |
