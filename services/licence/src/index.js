// Golden licence service (Cloudflare Worker). Paddle calls POST /webhook after each payment; we
// verify it, sign a licence, and email it. Anything unexpected returns a non-2xx so Paddle retries.
import { issueToken, importSigningKey } from "./licence.js";
import { verifySignature, licenceFromEvent, fetchCustomerEmail } from "./paddle.js";
import { buildEmail, sendEmail } from "./email.js";

const json = (status, body) =>
  new Response(JSON.stringify(body), { status, headers: { "content-type": "application/json" } });

export async function handleWebhook(request, env, deps = {}) {
  const now = deps.now ?? (() => Math.floor(Date.now() / 1000));
  const doFetch = deps.fetch ?? fetch;
  const body = await request.text();
  const bad = await verifySignature(
    request.headers.get("paddle-signature"),
    body,
    env.PADDLE_WEBHOOK_SECRET,
    now(),
  );
  if (bad) return json(401, { error: bad });

  let fields;
  try {
    fields = licenceFromEvent(body, { core: env.GOLDEN_PRICE_ID, plaid: env.GOLDEN_PLAID_PRICE_ID });
  } catch (e) {
    return json(400, { error: String(e.message) });
  }
  if (!fields) return json(200, { ignored: true });

  const seen = `txn:${fields.transactionId}`;
  if (await env.LICENCES.get(seen)) return json(200, { duplicate: true });

  const key = await importSigningKey(env.LICENCE_SIGNING_KEY);
  const token = await issueToken(key, fields);
  const to = await fetchCustomerEmail(fields.customerId, env.PADDLE_API_KEY, env.PADDLE_ENV, doFetch);
  await sendEmail(
    {
      apiKey: env.RESEND_API_KEY,
      from: env.MAIL_FROM,
      to,
      replyTo: env.SUPPORT_EMAIL,
      email: buildEmail({ ...fields, token, support: env.SUPPORT_EMAIL }),
    },
    doFetch,
  );
  // Recorded only after the email went out: a failed send makes Paddle retry the whole thing.
  await env.LICENCES.put(seen, fields.customerId);
  return json(200, { delivered: true });
}

export default {
  async fetch(request, env) {
    const { pathname } = new URL(request.url);
    if (pathname === "/health") return json(200, { ok: true });
    if (pathname === "/webhook" && request.method === "POST") {
      try {
        return await handleWebhook(request, env);
      } catch (e) {
        console.error("webhook failed:", e.message);
        return json(500, { error: "failed, will retry" });
      }
    }
    return json(404, { error: "not found" });
  },
};
