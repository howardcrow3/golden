import test from "node:test";
import assert from "node:assert/strict";
import { importSigningKey, issueToken, payloadJson } from "../src/licence.js";

// Made by the Rust tool: golden-license issue <this key> ctm_fixture 2026-11-18 [2026-12-18]
const SECRET = Buffer.alloc(32, 7).toString("base64url");
const RUST_WITH_PLAID =
  "eyJwcm9kdWN0IjoiZ29sZGVuIiwibWFqb3IiOjEsImlzc3VlZCI6IjIwMjYtMTEtMTgiLCJwbGFpZF91bnRpbCI6IjIwMjYtMTItMTgiLCJidXllciI6ImN0bV9maXh0dXJlIn0.E1ZYwMv0TqZ3izTtBwqX2gICVC_UUOBCjKMUFf6ns2qqjLf8kgmdmmZMlXamZv5uPFriqZ5CaGQpRuTGAIcFCg";
const RUST_NO_PLAID =
  "eyJwcm9kdWN0IjoiZ29sZGVuIiwibWFqb3IiOjEsImlzc3VlZCI6IjIwMjYtMTEtMTgiLCJwbGFpZF91bnRpbCI6bnVsbCwiYnV5ZXIiOiJjdG1fZml4dHVyZSJ9.FpUHS8cx0qPICUpJJUTJROxYqdLPeUTxA2d3DyWDSuRZQLwJlqJsb301yk7AEHoGbmwtI-xiOOvK_oRG4-ApCA";

test("tokens are byte-identical to the Rust tool's", async () => {
  const key = await importSigningKey(SECRET);
  assert.equal(
    await issueToken(key, { issued: "2026-11-18", plaidUntil: "2026-12-18", buyer: "ctm_fixture" }),
    RUST_WITH_PLAID,
  );
  assert.equal(
    await issueToken(key, { issued: "2026-11-18", buyer: "ctm_fixture" }),
    RUST_NO_PLAID,
  );
});

test("payload field order matches the app's", () => {
  assert.equal(
    payloadJson({ issued: "2026-11-18", buyer: "ctm_x" }),
    '{"product":"golden","major":1,"issued":"2026-11-18","plaid_until":null,"buyer":"ctm_x"}',
  );
});

test("a malformed signing key is refused", async () => {
  await assert.rejects(importSigningKey("short"), /32-byte/);
  await assert.rejects(importSigningKey(undefined), /32-byte/);
});
