export function event(kind, priceIds, extra = "") {
  const items = priceIds.map((p) => `{"price":{"id":"${p}"},"quantity":1}`).join(",");
  return `{"event_id":"evt_1","event_type":"${kind}","occurred_at":"2026-11-20T10:11:12.000Z","data":{"id":"txn_01abc","customer_id":"ctm_01xyz","items":[${items}]${extra}}}`;
}
