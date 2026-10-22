import { createRequire } from 'node:module'

const require = createRequire('file:///D:/sdkwork-space/sdkwork-im/apps/sdkwork-im-pc/package.json')
const { Client } = require('pg')

const databaseUrl = process.env.DATABASE_URL ?? 'postgres://sdkwork_ai_dev:sdkworkdev123@127.0.0.1:5432/sdkwork_ai_dev'
const entitlementUrl =
  process.env.SDKWORK_COURSE_ENTITLEMENT_URL ??
  'http://127.0.0.1:3900/app/v3/api/course_entitlements/grant'
const entitlementSecret = process.env.SDKWORK_COURSE_ENTITLEMENT_SECRET ?? ''
const courseSkuId = process.env.SDKWORK_COURSE_SKU_ID ?? '1620'

const client = new Client({ connectionString: databaseUrl })
await client.connect()
try {
  const orders = await client.query(
    `SELECT DISTINCT o.id, o.tenant_id, o.organization_id, o.owner_user_id, i.sku_id
     FROM commerce_order o
     JOIN commerce_order_item i ON i.order_id = o.id AND i.tenant_id = o.tenant_id
     WHERE i.sku_id = $1
       AND (LOWER(o.status) IN ('paid', 'fulfilled', 'completed')
            OR LOWER(o.payment_status) IN ('success', 'paid', 'succeeded'))
     ORDER BY o.id`,
    [courseSkuId],
  )
  for (const order of orders.rows) {
    const offering = await client.query(
      `SELECT id
       FROM course_offering
       WHERE tenant_id = $1
         AND access_policy_json::jsonb ->> 'commerceSkuId' = $2
         AND deleted_at IS NULL
       ORDER BY created_at
       LIMIT 1`,
      [order.tenant_id, courseSkuId],
    )
    const offeringId = offering.rows[0]?.id
    if (!offeringId) {
      console.warn('no course offering for sku', courseSkuId, 'order', order.id)
      continue
    }
    const response = await fetch(entitlementUrl, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'x-sdkwork-entitlement-secret': entitlementSecret,
      },
      body: JSON.stringify({
        offeringId,
        learnerUserId: order.owner_user_id,
        purchaseRef: order.id,
      }),
    })
    const text = await response.text()
    console.log(order.id, response.status, text.slice(0, 300))
    if (!response.ok) process.exitCode = 1
  }
} finally {
  await client.end()
}
