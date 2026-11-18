// Paddle webhook handling: check the signature, then turn a completed purchase into licence fields.
// Mirrors crates/fin-license/src/paddle.rs (which is the reference and has the same tests).

const enc = new TextEncoder();

function hex(bytes) {
  return [...new Uint8Array(bytes)].map((b) => b.toString(16).padStart(2, "0")).join("");
}

async function hmacHex(secret, message) {
  const key = await crypto.subtle.importKey(
    "raw",
    enc.encode(secret),
    { name: "HMAC", hash: "SHA-256" },
    false,
    ["sign"],
  );
  return hex(await crypto.subtle.sign("HMAC", key, enc.encode(message)));
}

function safeEqual(a, b) {
  if (a.length !== b.length) return false;
  let diff = 0;
  for (let i = 0; i < a.length; i++) diff |= a.charCodeAt(i) ^ b.charCodeAt(i);
  return diff === 0;
}

/** The header Paddle would send (used by tests). */
export async function signBody(secret, ts, body) {
  return `ts=${ts};h1=${await hmacHex(secret, `${ts}:${body}`)}`;
}

/**
 * Check a Paddle-Signature header against the raw body. Returns null when valid, or a short
 * reason string. Any matching h1 is enough (Paddle sends two while a secret is rotating).
 */
export async function verifySignature(header, body, secret, nowUnix, toleranceSecs = 300) {
  if (!header || !secret) return "missing signature";
  let ts = null;
  const sigs = [];
  for (const part of header.split(";")) {
    const [k, v] = part.trim().split("=");
    if (k === "ts" && /^\d+$/.test(v ?? "")) ts = Number(v);
    if (k === "h1" && v) sigs.push(v);
  }
  if (ts === null || sigs.length === 0) return "malformed signature header";
  if (Math.abs(nowUnix - ts) > toleranceSecs) return "stale timestamp";
  const want = await hmacHex(secret, `${ts}:${body}`);
  return sigs.some((s) => safeEqual(s, want)) ? null : "signature mismatch";
}

function day(text) {
  const m = /^(\d{4}-\d{2}-\d{2})/.exec(text ?? "");
  return m ? m[1] : null;
}

/** Add calendar months, clamping to the end of a shorter month (Jan 31 + 1 = Feb 28), like chrono. */
export function addMonths(isoDay, n) {
  const [y, m, d] = isoDay.split("-").map(Number);
  const total = y * 12 + (m - 1) + n;
  const ny = Math.floor(total / 12);
  const nm = (total % 12) + 1;
  const last = new Date(Date.UTC(ny, nm, 0)).getUTCDate();
  const pad = (x) => String(x).padStart(2, "0");
  return `${ny}-${pad(nm)}-${pad(Math.min(d, last))}`;
}

/**
 * Turn a verified webhook body into licence fields, or null for anything that is not a
 * completed purchase of one of our prices (so the endpoint can answer 200 and Paddle won't retry).
 * Throws Error for a purchase that is missing data we need.
 */
export function licenceFromEvent(bodyText, prices) {
  const event = JSON.parse(bodyText);
  if (event?.event_type !== "transaction.completed") return null;
  const data = event.data;
  if (!data) throw new Error("no data");
  let core = false;
  let plaid = false;
  for (const item of data.items ?? []) {
    const id = item?.price?.id ?? item?.price_id;
    if (id && id === prices.core) core = true;
    else if (id && id === prices.plaid) plaid = true;
  }
  if (!core && !plaid) return null;
  if (!data.id) throw new Error("no transaction id");
  if (!data.customer_id) throw new Error("no customer id");
  const issued = day(event.occurred_at) ?? day(data.billed_at);
  if (!issued) throw new Error("no date");
  const plaidUntil = plaid
    ? (day(data.billing_period?.ends_at) ?? addMonths(issued, 1))
    : null;
  return {
    transactionId: data.id,
    customerId: data.customer_id,
    issued,
    plaidUntil,
    boughtCore: core,
    buyer: data.customer_id,
  };
}

/** Ask Paddle for the buyer's email (the webhook only carries the customer id). */
export async function fetchCustomerEmail(customerId, apiKey, env, fetchFn = fetch) {
  const base = env === "sandbox" ? "https://sandbox-api.paddle.com" : "https://api.paddle.com";
  const res = await fetchFn(`${base}/customers/${encodeURIComponent(customerId)}`, {
    headers: { Authorization: `Bearer ${apiKey}` },
  });
  if (!res.ok) throw new Error(`Paddle customer lookup failed (${res.status})`);
  const email = (await res.json())?.data?.email;
  if (!email) throw new Error("Paddle customer has no email");
  return email;
}
