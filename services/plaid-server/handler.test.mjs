import test from "node:test";
import assert from "node:assert/strict";
import { generateKeyPairSync, sign } from "node:crypto";
import { checkLicence, createHandler, plaidBody, LINK_PAGE } from "./handler.mjs";

const { publicKey, privateKey } = generateKeyPairSync("ed25519");
const rawPub = publicKey.export({ format: "der", type: "spki" }).subarray(12).toString("base64url");

function licence(over = {}) {
  const payload = { product: "golden", major: 1, issued: "2026-01-01", plaid_until: "2027-01-01", buyer: "b1", ...over };
  const body = Buffer.from(JSON.stringify(payload));
  return `${body.toString("base64url")}.${sign(null, body, privateKey).toString("base64url")}`;
}

const env = { PLAID_CLIENT_ID: "cid", PLAID_SECRET: "sec", GOLDEN_LICENCE_PUBLIC_KEY: rawPub };
const now = () => new Date("2026-11-23T12:00:00Z");

function call(path, input, { fetchImpl, e = env, method = "POST" } = {}) {
  const h = createHandler({ fetchImpl, env: e, now });
  return h({ rawPath: path, requestContext: { http: { method } }, body: JSON.stringify(input) });
}

const okFetch = (seen = []) => async (url, init) => {
  seen.push({ url, body: JSON.parse(init.body) });
  return { status: 200, text: async () => '{"ok":true}' };
};

test("a good licence passes and is checked against the add-on date", () => {
  assert.equal(checkLicence(licence(), rawPub, 1, "2026-11-23").ok, true);
  assert.equal(checkLicence(licence({ plaid_until: "2026-11-23" }), rawPub, 1, "2026-11-23").ok, true);
  assert.equal(checkLicence(licence({ plaid_until: "2026-11-22" }), rawPub, 1, "2026-11-23").ok, false);
  assert.equal(checkLicence(licence({ plaid_until: null }), rawPub, 1, "2026-11-23").ok, false);
});

test("wrong product, wrong major, forged and malformed licences are refused", () => {
  assert.equal(checkLicence(licence({ product: "other" }), rawPub, 1, "2026-11-23").ok, false);
  assert.equal(checkLicence(licence({ major: 2 }), rawPub, 1, "2026-11-23").ok, false);
  const [b, s] = licence().split(".");
  const forged = Buffer.from(JSON.stringify({ product: "golden", major: 1, plaid_until: "2099-01-01" })).toString("base64url");
  assert.equal(checkLicence(`${forged}.${s}`, rawPub, 1, "2026-11-23").ok, false);
  assert.equal(checkLicence("nonsense", rawPub, 1, "2026-11-23").ok, false);
  assert.equal(checkLicence(undefined, rawPub, 1, "2026-11-23").ok, false);
  assert.equal(checkLicence(`${b}.${s}`, "not-a-key", 1, "2026-11-23").ok, false);
});

test("sync forwards to Plaid with the secret added and extra fields dropped", async () => {
  const seen = [];
  const r = await call("/transactions/sync", { licence: licence(), access_token: "acc", cursor: "c1", evil: "x" }, { fetchImpl: okFetch(seen) });
  assert.equal(r.statusCode, 200);
  assert.equal(seen[0].url, "https://sandbox.plaid.com/transactions/sync");
  assert.deepEqual(seen[0].body, { client_id: "cid", secret: "sec", access_token: "acc", count: 500, cursor: "c1" });
});

test("production host is used when asked", async () => {
  const seen = [];
  await call("/item/remove", { licence: licence(), access_token: "a" }, { fetchImpl: okFetch(seen), e: { ...env, PLAID_ENV: "production" } });
  assert.equal(seen[0].url, "https://production.plaid.com/item/remove");
});

test("link token and exchange bodies", () => {
  const creds = { clientId: "c", secret: "s" };
  const t = plaidBody("/link/token", { client_user_id: "u1" }, creds);
  assert.deepEqual(t.products, ["transactions"]);
  assert.deepEqual(t.country_codes, ["US"]);
  assert.equal(t.user.client_user_id, "u1");
  assert.equal(plaidBody("/link/exchange", { public_token: "p" }, creds).public_token, "p");
  assert.equal(plaidBody("/nope", {}, creds), null);
});

test("no licence means 403 and Plaid is never called", async () => {
  let called = false;
  const r = await call("/transactions/sync", { access_token: "a" }, { fetchImpl: async () => { called = true; } });
  assert.equal(r.statusCode, 403);
  assert.equal(JSON.parse(r.body).error_code, "LICENCE");
  assert.equal(called, false);
});

test("unknown paths, wrong methods, bad bodies, missing tokens", async () => {
  assert.equal((await call("/admin", {})).statusCode, 404);
  assert.equal((await call("/transactions/sync", {}, { method: "GET" })).statusCode, 405);
  const h = createHandler({ env, now });
  assert.equal((await h({ rawPath: "/transactions/sync", body: "{oops" })).statusCode, 400);
  assert.equal((await call("/transactions/sync", { licence: licence() }, { fetchImpl: okFetch() })).statusCode, 400);
  assert.equal((await call("/link/exchange", { licence: licence() }, { fetchImpl: okFetch() })).statusCode, 400);
});

test("oversized bodies are refused", async () => {
  const h = createHandler({ env, now });
  assert.equal((await h({ rawPath: "/link/token", body: "x".repeat(20001) })).statusCode, 413);
});

test("missing secrets give 503 and an unreachable Plaid gives 502", async () => {
  assert.equal((await call("/link/token", { licence: licence() }, { e: { GOLDEN_LICENCE_PUBLIC_KEY: rawPub } })).statusCode, 503);
  const r = await call("/link/token", { licence: licence() }, { fetchImpl: async () => { throw new Error("down"); } });
  assert.equal(r.statusCode, 502);
});

test("Plaid's own error is passed through with its status", async () => {
  const f = async () => ({ status: 400, text: async () => '{"error_code":"ITEM_LOGIN_REQUIRED"}' });
  const r = await call("/transactions/sync", { licence: licence(), access_token: "a" }, { fetchImpl: f });
  assert.equal(r.statusCode, 400);
  assert.match(r.body, /ITEM_LOGIN_REQUIRED/);
});

test("the secret never appears in a response", async () => {
  const r = await call("/link/token", { licence: licence() }, { fetchImpl: okFetch() });
  assert.ok(!r.body.includes("sec"));
});

test("GET /link serves the sign-in page with no secrets and no caching", async () => {
  const h = createHandler({ env, now });
  const r = await h({ rawPath: "/link", requestContext: { http: { method: "GET" } } });
  assert.equal(r.statusCode, 200);
  assert.match(r.headers["content-type"], /text\/html/);
  assert.equal(r.headers["cache-control"], "no-store");
  assert.ok(r.body.includes("cdn.plaid.com/link/v2/stable/link-initialize.js"));
  assert.ok(r.body.includes("golden://plaid?public_token="));
  assert.ok(!r.body.includes("client_id") && !r.body.includes("PLAID_SECRET"));
  assert.equal(r.body, LINK_PAGE);
});

test("other GETs are still refused", async () => {
  const h = createHandler({ env, now });
  assert.equal((await h({ rawPath: "/transactions/sync", requestContext: { http: { method: "GET" } } })).statusCode, 405);
});

test("the OAuth return address is added to the link token when configured", () => {
  const t = plaidBody("/link/token", {}, { clientId: "c", secret: "s", redirectUri: "https://x/link" });
  assert.equal(t.redirect_uri, "https://x/link");
  assert.equal("redirect_uri" in plaidBody("/link/token", {}, { clientId: "c", secret: "s" }), false);
});

test("a JSON body that is not an object is a 400, not a crash", async () => {
  const h = createHandler({ env, now });
  for (const body of ["null", "[]", "7", '"x"']) {
    const r = await h({ rawPath: "/transactions/sync", requestContext: { http: { method: "POST" } }, body });
    assert.equal(r.statusCode, 400, body);
  }
});

test("the paid-through day still works in the evening in US time zones", async () => {
  // 8pm in California on 2026-11-23 is 04:00 UTC on 2026-11-24.
  const h = createHandler({ fetchImpl: okFetch(), env, now: () => new Date("2026-11-24T04:00:00Z") });
  const ok = await h({ rawPath: "/item/remove", requestContext: { http: { method: "POST" } },
    body: JSON.stringify({ licence: licence({ plaid_until: "2026-11-23" }), access_token: "a" }) });
  assert.equal(ok.statusCode, 200);
  // A full day later it has lapsed everywhere.
  const late = createHandler({ fetchImpl: okFetch(), env, now: () => new Date("2026-11-25T13:00:00Z") });
  const r = await late({ rawPath: "/item/remove", requestContext: { http: { method: "POST" } },
    body: JSON.stringify({ licence: licence({ plaid_until: "2026-11-23" }), access_token: "a" }) });
  assert.equal(r.statusCode, 403);
});
