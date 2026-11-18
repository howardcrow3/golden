import test from "node:test";
import assert from "node:assert/strict";
import { event } from "./helpers.js";
import { signBody, verifySignature, licenceFromEvent, addMonths, fetchCustomerEmail } from "../src/paddle.js";

const SECRET = "pdl_ntfset_test_secret";
const NOW = 1_800_000_000;
const PRICES = { core: "pri_01m7h6qqb0y4f9r6mftw05y772", plaid: "pri_01m7h71ez1v1pen447723fmeje" };


test("a correct signature verifies; tampering, wrong secret, old and future fail", async () => {
  const body = '{"a":1}';
  const h = await signBody(SECRET, NOW, body);
  assert.equal(await verifySignature(h, body, SECRET, NOW), null);
  assert.equal(await verifySignature(h, '{"a":2}', SECRET, NOW), "signature mismatch");
  assert.equal(await verifySignature(h, body, "other", NOW), "signature mismatch");
  assert.equal(await verifySignature(h, body, SECRET, NOW + 1000), "stale timestamp");
  assert.equal(await verifySignature(h, body, SECRET, NOW - 1000), "stale timestamp");
});

test("missing or malformed headers are refused", async () => {
  assert.ok(await verifySignature(null, "{}", SECRET, NOW));
  assert.ok(await verifySignature("garbage", "{}", SECRET, NOW));
  assert.ok(await verifySignature("ts=abc;h1=00", "{}", SECRET, NOW));
  assert.ok(await verifySignature(`ts=${NOW};h1=00`, "{}", "", NOW));
});

test("either h1 may match while a secret is rotating", async () => {
  const body = "{}";
  const good = (await signBody(SECRET, NOW, body)).split("h1=")[1];
  assert.equal(await verifySignature(`ts=${NOW};h1=${"0".repeat(64)};h1=${good}`, body, SECRET, NOW), null);
});

test("core purchase", () => {
  const f = licenceFromEvent(event("transaction.completed", [PRICES.core]), PRICES);
  assert.deepEqual(f, {
    transactionId: "txn_01abc",
    customerId: "ctm_01xyz",
    issued: "2026-11-20",
    plaidUntil: null,
    boughtCore: true,
    buyer: "ctm_01xyz",
  });
});

test("plaid uses the billing period end, else one month on", () => {
  const withEnd = licenceFromEvent(
    event("transaction.completed", [PRICES.plaid], ',"billing_period":{"ends_at":"2026-12-20T00:00:00Z"}'),
    PRICES,
  );
  assert.equal(withEnd.plaidUntil, "2026-12-20");
  assert.equal(withEnd.boughtCore, false);
  const without = licenceFromEvent(event("transaction.completed", [PRICES.plaid]), PRICES);
  assert.equal(without.plaidUntil, "2026-12-20");
});

test("both items together", () => {
  const f = licenceFromEvent(event("transaction.completed", [PRICES.core, PRICES.plaid]), PRICES);
  assert.equal(f.boughtCore, true);
  assert.equal(f.plaidUntil, "2026-12-20");
});

test("other events and other prices are ignored", () => {
  assert.equal(licenceFromEvent(event("transaction.paid", [PRICES.core]), PRICES), null);
  assert.equal(licenceFromEvent(event("transaction.completed", ["pri_other"]), PRICES), null);
});

test("a purchase missing data throws", () => {
  assert.throws(() => licenceFromEvent('{"event_type":"transaction.completed","data":{"items":[{"price":{"id":"' + PRICES.core + '"}}]}}', PRICES));
  assert.throws(() => licenceFromEvent("not json", PRICES));
});

test("addMonths clamps like chrono", () => {
  assert.equal(addMonths("2026-01-31", 1), "2026-02-28");
  assert.equal(addMonths("2028-01-31", 1), "2028-02-29");
  assert.equal(addMonths("2026-12-15", 1), "2027-01-15");
});

test("customer email lookup", async () => {
  let seen;
  const ok = async (url, init) => {
    seen = { url, init };
    return { ok: true, json: async () => ({ data: { email: "a@b.co" } }) };
  };
  assert.equal(await fetchCustomerEmail("ctm_1", "k", "live", ok), "a@b.co");
  assert.equal(seen.url, "https://api.paddle.com/customers/ctm_1");
  assert.equal(seen.init.headers.Authorization, "Bearer k");
  await fetchCustomerEmail("ctm_1", "k", "sandbox", ok);
  assert.match(seen.url, /^https:\/\/sandbox-api/);
  await assert.rejects(fetchCustomerEmail("c", "k", "live", async () => ({ ok: false, status: 404 })), /404/);
});
