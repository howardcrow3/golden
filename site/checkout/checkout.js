// Hosted checkout for Turtlebox apps. One page serves every app: the link says which Paddle
// price to open (`?price=pri_...`) and which app it is for (`&app=golden`); the app name travels
// to Paddle as custom data so the licence service knows what to issue.

// Paddle client-side tokens are public by design (they can only open checkouts).
export const PADDLE_CLIENT_TOKEN = "live_07ce70479acc6466ead095e609c";

// Keep false until the licence service is live and delivering keys, so nobody can pay for
// something that cannot be delivered. Flip to true in the same change that deploys the service.
export const SALES_OPEN = false;

const PRICE_RE = /^pri_[a-z0-9]{10,40}$/;
const APP_RE = /^[a-z0-9-]{1,32}$/;
const EMAIL_RE = /^[^\s@<>"]{1,64}@[^\s@<>"]{1,255}$/;

/** Read and validate the link. Nothing from the query is used unless it matches a strict pattern. */
export function parseQuery(search) {
  const q = new URLSearchParams(search);
  const priceId = q.get("price") ?? "";
  if (priceId.startsWith("pro_")) {
    return { ok: false, error: "This link has a product ID. It needs a price ID (it starts with pri_)." };
  }
  if (!PRICE_RE.test(priceId)) {
    return { ok: false, error: "This checkout link is missing a valid price." };
  }
  const app = q.get("app") ?? "turtlebox";
  if (!APP_RE.test(app)) {
    return { ok: false, error: "This checkout link has an invalid app name." };
  }
  const email = q.get("email");
  return { ok: true, priceId, app, email: email && EMAIL_RE.test(email) ? email : null };
}

/** The options handed to Paddle.Checkout.open for a parsed link. */
export function checkoutOptions(parsed, successUrl) {
  const opts = {
    items: [{ priceId: parsed.priceId, quantity: 1 }],
    customData: { app: parsed.app },
    settings: { displayMode: "overlay", theme: "light", successUrl },
  };
  if (parsed.email) opts.customer = { email: parsed.email };
  return opts;
}

function show(message) {
  const el = document.getElementById("message");
  if (el) el.textContent = message;
}

export function main(win = window) {
  const parsed = parseQuery(win.location.search);
  if (!parsed.ok) return show(parsed.error);
  if (!SALES_OPEN) return show("Purchases open soon. Nothing has been charged.");
  if (!win.Paddle) return show("The payment form couldn't load. Check your connection and try again.");
  win.Paddle.Initialize({ token: PADDLE_CLIENT_TOKEN });
  const open = () =>
    win.Paddle.Checkout.open(checkoutOptions(parsed, new URL("thanks.html", win.location.href).href));
  document.getElementById("retry")?.addEventListener("click", open);
  show("Opening checkout…");
  open();
}
