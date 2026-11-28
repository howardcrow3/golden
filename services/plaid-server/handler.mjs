// Golden's Plaid sync service. It exists for one reason: Plaid's secret must never ship inside a
// desktop app. The app sends its licence token and a Plaid request; this adds the client id and
// secret, forwards the call, and passes Plaid's answer back. It stores nothing and logs no bodies.
//
// Deploy as an AWS Lambda behind a Function URL (payload format 2.0). Environment variables:
//   PLAID_CLIENT_ID, PLAID_SECRET     from the Plaid dashboard
//   PLAID_ENV                         "sandbox" (default) or "production"
//   GOLDEN_LICENCE_PUBLIC_KEY         the same base64url public key the app is built with
//   PLAID_REDIRECT_URI                optional: this service's own /link address, registered in the
//                                     Plaid dashboard, needed by banks that sign in through an app
//   GOLDEN_APP_MAJOR                  optional, default 1
import { createPublicKey, verify } from "node:crypto";

const HOSTS = {
  sandbox: "https://sandbox.plaid.com",
  production: "https://production.plaid.com",
};

// Only these app paths are served (plus GET /link, the sign-in page), and each maps to exactly one Plaid endpoint.
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

/** The page that opens Plaid's sign-in window. It holds no secrets: the link token arrives in the
 * address, and on success the person is sent back to the app with `golden://plaid?...`. */
export const LINK_PAGE = `<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<meta name="referrer" content="no-referrer"><title>Connect your bank to Golden</title>
<style>body{font:16px system-ui,sans-serif;margin:0;display:grid;place-items:center;min-height:100vh;background:#f7f5ef;color:#222}
main{max-width:30rem;padding:2rem;text-align:center}</style></head>
<body><main><h1>Connect your bank</h1><p id="msg">Opening your bank's sign-in…</p></main>
<script src="https://cdn.plaid.com/link/v2/stable/link-initialize.js"></script>
<script>
(function () {
  var msg = document.getElementById("msg");
  var params = new URLSearchParams(location.search);
  var token = params.get("token");
  var state = params.get("state") || "";
  var oauth = params.has("oauth_state_id");
  try {
    if (token) { sessionStorage.setItem("golden_link_token", token); sessionStorage.setItem("golden_link_state", state); }
    else if (oauth) { token = sessionStorage.getItem("golden_link_token"); state = sessionStorage.getItem("golden_link_state") || ""; }
  } catch (e) {}
  if (!token) { msg.textContent = "This link has expired. Go back to Golden and press Connect again."; return; }
  var cfg = {
    token: token,
    onSuccess: function (publicToken, meta) {
      var name = (meta && meta.institution && meta.institution.name) || "";
      msg.textContent = "Connected. Returning to Golden…";
      location.href = "golden://plaid?public_token=" + encodeURIComponent(publicToken) +
        "&institution=" + encodeURIComponent(name) + "&state=" + encodeURIComponent(state);
    },
    onExit: function () { msg.textContent = "Not connected. You can close this tab and go back to Golden."; }
  };
  if (oauth) cfg.receivedRedirectUri = location.href;
  Plaid.create(cfg).open();
})();
</script></body></html>`;

const reply = (status, body) => ({
  statusCode: status,
  headers: { "content-type": "application/json" },
  body: JSON.stringify(body),
});

/** What we send Plaid for each route. Anything the app sends beyond these fields is dropped. */
export function plaidBody(path, input, creds) {
  // creds.redirectUri is the registered OAuth return address (this service's /link page).
  const base = { client_id: creds.clientId, secret: creds.secret };
  switch (path) {
    case "/link/token":
      return {
        ...base,
        ...(creds.redirectUri ? { redirect_uri: creds.redirectUri } : {}),
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
    if (method === "GET" && path === "/link") {
      return {
        statusCode: 200,
        headers: {
          "content-type": "text/html; charset=utf-8",
          "cache-control": "no-store",
          "referrer-policy": "no-referrer",
        },
        body: LINK_PAGE,
      };
    }
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
    // "null" parses fine but has no fields to read; anything but a plain object is refused.
    if (input === null || typeof input !== "object" || Array.isArray(input)) {
      return reply(400, { error: "body must be a JSON object" });
    }

    // The app checks plaid_until against the person's local date, so "today" here is the date
    // Anywhere on Earth (UTC-12): the paid-through day is honoured until it ends in every time
    // zone, instead of lapsing at 4pm in California when the UTC date rolls over.
    const today = new Date(now().getTime() - 12 * 3600 * 1000).toISOString().slice(0, 10);
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

    const body = plaidBody(path, input, { clientId: env.PLAID_CLIENT_ID, secret: env.PLAID_SECRET, redirectUri: env.PLAID_REDIRECT_URI });
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
