// Golden's Plaid sync service. It exists for one reason: Plaid's secret must never ship inside a
// desktop app. The app sends its licence token and a Plaid request; this adds the client id and
// secret, forwards the call, and passes Plaid's answer back. It stores nothing and logs no bodies.
//
// Deploy as an AWS Lambda behind a Function URL (payload format 2.0). Environment variables:
//   PLAID_CLIENT_ID, PLAID_SECRET     from the Plaid dashboard
//   PLAID_ENV                         "sandbox" (default) or "production"
//   GOLDEN_LICENCE_PUBLIC_KEY         the same base64url public key the app is built with
//   GOLDEN_APP_MAJOR                  optional, default 1
import { createPublicKey, verify } from "node:crypto";

const HOSTS = {
  sandbox: "https://sandbox.plaid.com",
  production: "https://production.plaid.com",
};

// Only these app paths are served, and each maps to exactly one Plaid endpoint.
const ROUTES = {
  "/link/token": "/link/token/create",
  "/link/exchange": "/item/public_token/exchange",
  "/transactions/sync": "/transactions/sync",
  "/item/remove": "/item/remove",
};

const SPKI_PREFIX = Buffer.from("302a300506032b6570032100", "hex");

const b64u = (s) => Buffer.from(s, "base64url");

/** Check a licence token (`payload.signature`, both base64url) and the Plaid add-on's paid-through date. */
export function checkLicence(token, publicKeyB64, major, today) {
  if (typeof token !== "string" || !token.includes(".")) return { ok: false, why: "licence missing" };
  const [bodyB64, sigB64] = token.trim().split(".");
  let payload;
  try {
    const key = createPublicKey({
      key: Buffer.concat([SPKI_PREFIX, b64u(publicKeyB64)]),
      format: "der",
      type: "spki",
    });
    const body = b64u(bodyB64);
    if (!verify(null, body, key, b64u(sigB64))) return { ok: false, why: "licence signature is not valid" };
    payload = JSON.parse(body.toString("utf8"));
  } catch {
    return { ok: false, why: "licence is malformed" };
  }
  if (payload.product !== "golden") return { ok: false, why: "licence is for a different product" };
  if (payload.major !== major) return { ok: false, why: "licence is for a different version" };
  if (!payload.plaid_until || payload.plaid_until < today) {
    return { ok: false, why: "the bank connection add-on is not active" };
  }
  return { ok: true, payload };
}

const reply = (status, body) => ({
  statusCode: status,
  headers: { "content-type": "application/json" },
  body: JSON.stringify(body),
});

/** What we send Plaid for each route. Anything the app sends beyond these fields is dropped. */
export function plaidBody(path, input, creds) {
  const base = { client_id: creds.clientId, secret: creds.secret };
  switch (path) {
    case "/link/token":
      return {
        ...base,
        client_name: "Golden",
        language: "en",
        country_codes: ["US"],
        products: ["transactions"],
        user: { client_user_id: String(input.client_user_id ?? "golden-user").slice(0, 64) },
      };
    case "/link/exchange":
      return { ...base, public_token: input.public_token };
    case "/transactions/sync": {
      const b = { ...base, access_token: input.access_token, count: 500 };
      if (input.cursor) b.cursor = input.cursor;
      return b;
    }
    case "/item/remove":
      return { ...base, access_token: input.access_token };
    default:
      return null;
  }
}

export function createHandler({ fetchImpl = fetch, env = process.env, now = () => new Date() } = {}) {
  return async function handler(event) {
    const method = event?.requestContext?.http?.method ?? event?.httpMethod ?? "POST";
    const path = event?.rawPath ?? event?.path ?? "";
    if (method !== "POST") return reply(405, { error: "POST only" });
    if (!(path in ROUTES)) return reply(404, { error: "unknown path" });

    let input;
    try {
      const raw = event.isBase64Encoded ? Buffer.from(event.body ?? "", "base64").toString() : event.body ?? "";
      if (raw.length > 20000) return reply(413, { error: "request too large" });
      input = JSON.parse(raw || "{}");
    } catch {
      return reply(400, { error: "body is not JSON" });
    }

    const today = now().toISOString().slice(0, 10);
    const major = Number(env.GOLDEN_APP_MAJOR ?? 1);
    const lic = checkLicence(input.licence, env.GOLDEN_LICENCE_PUBLIC_KEY ?? "", major, today);
    if (!lic.ok) return reply(403, { error_code: "LICENCE", error_message: lic.why });

    const needs = { "/link/exchange": "public_token", "/transactions/sync": "access_token", "/item/remove": "access_token" };
    if (needs[path] && typeof input[needs[path]] !== "string") {
      return reply(400, { error: `${needs[path]} is required` });
    }

    if (!env.PLAID_CLIENT_ID || !env.PLAID_SECRET) return reply(503, { error: "service is not set up" });
    const host = HOSTS[env.PLAID_ENV ?? "sandbox"];
    if (!host) return reply(503, { error: "unknown PLAID_ENV" });

    const body = plaidBody(path, input, { clientId: env.PLAID_CLIENT_ID, secret: env.PLAID_SECRET });
    let res;
    try {
      res = await fetchImpl(host + ROUTES[path], {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(body),
      });
    } catch {
      return reply(502, { error: "could not reach Plaid" });
    }
    const text = await res.text();
    return { statusCode: res.status, headers: { "content-type": "application/json" }, body: text };
  };
}

export const handler = createHandler();
