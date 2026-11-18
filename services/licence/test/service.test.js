import test from "node:test";
import assert from "node:assert/strict";
import { handleWebhook } from "../src/index.js";
import { buildEmail } from "../src/email.js";
import { signBody } from "../src/paddle.js";
import { importSigningKey } from "../src/licence.js";
import { event } from "./helpers.js";

const NOW = 1_800_000_000;
const core = "pri_01m7h6qqb0y4f9r6mftw05y772";
const plaid = "pri_01m7h71ez1v1pen447723fmeje";

function setup() {
  const store = new Map();
  const sent = [];
  const env = {
    PADDLE_WEBHOOK_SECRET: "pdl_ntfset_test_secret",
    PADDLE_API_KEY: "pk",
    PADDLE_ENV: "live",
    GOLDEN_PRICE_ID: core,
    GOLDEN_PLAID_PRICE_ID: plaid,
    LICENCE_SIGNING_KEY: Buffer.alloc(32, 7).toString("base64url"),
    RESEND_API_KEY: "rk",
    MAIL_FROM: "Golden <l@x.test>",
    SUPPORT_EMAIL: "s@x.test",
    LICENCES: { get: async (k) => store.get(k) ?? null, put: async (k, v) => void store.set(k, v) },
  };
  const fakeFetch = async (url, init) => {
    if (url.startsWith("https://api.paddle.com/customers/"))
      return { ok: true, json: async () => ({ data: { email: "buyer@x.test" } }) };
    if (url === "https://api.resend.com/emails") {
      if (env.failMail) return { ok: false, status: 500 };
      sent.push(JSON.parse(init.body));
      return { ok: true };
    }
    throw new Error("unexpected fetch " + url);
  };
  return { env, store, sent, deps: { now: () => NOW, fetch: fakeFetch } };
}

async function post(body, env, deps, secret = env.PADDLE_WEBHOOK_SECRET) {
  const req = new Request("https://x.test/webhook", {
    method: "POST",
    headers: { "paddle-signature": await signBody(secret, NOW, body) },
    body,
  });
  return handleWebhook(req, env, deps);
}

test("a purchase emails a licence the app can verify, once", async () => {
  const { env, sent, deps } = setup();
  const body = event("transaction.completed", [core]);
  const res = await post(body, env, deps);
  assert.equal(res.status, 200);
  assert.deepEqual(await res.json(), { delivered: true });
  assert.equal(sent.length, 1);
  assert.deepEqual(sent[0].to, ["buyer@x.test"]);
  const token = /^[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+$/m.exec(sent[0].text)[0];
  const payload = JSON.parse(Buffer.from(token.split(".")[0], "base64url"));
  assert.equal(payload.buyer, "ctm_01xyz");
  // A Paddle retry of the same transaction does not email again.
  const again = await post(body, env, deps);
  assert.deepEqual(await again.json(), { duplicate: true });
  assert.equal(sent.length, 1);
});

test("the emailed token verifies with the matching public key", async () => {
  const { env, sent, deps } = setup();
  await post(event("transaction.completed", [core]), env, deps);
  const token = /^[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+$/m.exec(sent[0].text)[0];
  const [body, sig] = token.split(".");
  const seed = Buffer.alloc(32, 7);
  const { createPrivateKey, createPublicKey, verify } = await import("node:crypto");
  const priv = createPrivateKey({
    key: Buffer.concat([Buffer.from("302e020100300506032b657004220420", "hex"), seed]),
    format: "der",
    type: "pkcs8",
  });
  const ok = verify(null, Buffer.from(body, "base64url"), createPublicKey(priv), Buffer.from(sig, "base64url"));
  assert.equal(ok, true);
});

test("a bad signature is refused and nothing is sent", async () => {
  const { env, sent, deps } = setup();
  const res = await post(event("transaction.completed", [core]), env, deps, "wrong");
  assert.equal(res.status, 401);
  assert.equal(sent.length, 0);
});

test("unrelated events get a 200 and nothing happens", async () => {
  const { env, sent, deps } = setup();
  const res = await post(event("transaction.paid", [core]), env, deps);
  assert.equal(res.status, 200);
  assert.equal(sent.length, 0);
});

test("a failed email is not recorded, so Paddle's retry delivers it", async () => {
  const { env, store, sent, deps } = setup();
  env.failMail = true;
  await assert.rejects(post(event("transaction.completed", [core]), env, deps), /email send failed/);
  assert.equal(store.size, 0);
  env.failMail = false;
  const res = await post(event("transaction.completed", [core]), env, deps);
  assert.deepEqual(await res.json(), { delivered: true });
  assert.equal(sent.length, 1);
});

test("a malformed purchase is a 400", async () => {
  const { env, deps } = setup();
  const body = `{"event_type":"transaction.completed","data":{"items":[{"price":{"id":"${core}"}}]}}`;
  assert.equal((await post(body, env, deps)).status, 400);
});

test("email wording for first purchase and renewal", () => {
  const first = buildEmail({ token: "T.T", plaidUntil: null, boughtCore: true, support: "s@x.test" });
  assert.equal(first.subject, "Your Golden licence key");
  const renew = buildEmail({ token: "T.T", plaidUntil: "2026-12-20", boughtCore: false, support: "s@x.test" });
  assert.match(renew.subject, /renewed/);
  assert.match(renew.text, /2026-12-20/);
  assert.doesNotMatch(first.html, /<script/);
});

test("importSigningKey is usable from the service", async () => {
  await importSigningKey(Buffer.alloc(32, 1).toString("base64url"));
});
