// Licence tokens, byte-for-byte what crates/fin-license produces: base64url(JSON payload) "."
// base64url(Ed25519 signature of those JSON bytes). Ed25519 is deterministic, so the same key and
// payload give the same token in Rust and here (test/licence.test.js pins a Rust-made token).

const b64u = {
  encode(bytes) {
    return Buffer.from(bytes).toString("base64url");
  },
  decode(text) {
    return new Uint8Array(Buffer.from(text, "base64url"));
  },
};

// PKCS#8 prefix that wraps a raw 32-byte Ed25519 seed so WebCrypto will import it.
const PKCS8_PREFIX = Uint8Array.from([
  0x30, 0x2e, 0x02, 0x01, 0x00, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x04, 0x22, 0x04, 0x20,
]);

export async function importSigningKey(secretB64) {
  const seed = b64u.decode((secretB64 ?? "").trim());
  if (seed.length !== 32) throw new Error("LICENCE_SIGNING_KEY must be a 32-byte base64url key");
  const der = new Uint8Array(PKCS8_PREFIX.length + 32);
  der.set(PKCS8_PREFIX);
  der.set(seed, PKCS8_PREFIX.length);
  return crypto.subtle.importKey("pkcs8", der, { name: "Ed25519" }, false, ["sign"]);
}

/** The JSON the app expects: same fields and order as Rust's LicensePayload. */
export function payloadJson({ issued, plaidUntil = null, buyer, major = 1 }) {
  return JSON.stringify({
    product: "golden",
    major,
    issued,
    plaid_until: plaidUntil,
    buyer,
  });
}

export async function issueToken(key, fields) {
  const body = new TextEncoder().encode(payloadJson(fields));
  const sig = new Uint8Array(await crypto.subtle.sign({ name: "Ed25519" }, key, body));
  return `${b64u.encode(body)}.${b64u.encode(sig)}`;
}
