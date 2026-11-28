import { describe, expect, it } from "vitest";
import doc from "../../../../docs/legal/terms.md?raw";
import { TERMS, TERMS_SUMMARY, TERMS_VERSION, needsAcceptance, termsMarkdown } from "./terms";

describe("terms of use", () => {
  it("asks on first launch and again when the version changes", () => {
    expect(needsAcceptance(null)).toBe(true);
    expect(needsAcceptance(undefined)).toBe(true);
    expect(needsAcceptance({ version: "2000-01-01" })).toBe(true);
    expect(needsAcceptance({ version: TERMS_VERSION })).toBe(false);
  });

  it("covers what the licence must say", () => {
    const all = termsMarkdown().toLowerCase();
    for (const must of [
      "not financial, investment, tax or legal advice",
      "qualified professional",
      "up to 2 computers",
      "non-transferable",
      "reverse engineer",
      "14-day free trial",
      "plaid",
      "paddle",
      "refund",
      "ai output can be wrong",
      "as is",
      "limitation of liability",
      "governing law",
      "changes to these terms",
      "private mode",
    ]) {
      expect(all, must).toContain(must);
    }
  });

  it("never claims a right to use financial data", () => {
    const privacy = TERMS.find((s) => s.heading.includes("privacy"))!.paragraphs.join(" ");
    expect(privacy).toContain("we do not use it to train AI models or for product development");
    expect(privacy).toContain("off until you turn it on");
    expect(privacy).toContain("never be sent in Private mode");
    expect(TERMS_SUMMARY.join(" ")).toContain("We never store, sell or use it");
  });

  it("docs/legal/terms.md matches the text the app shows", () => {
    // If this fails, regenerate it from apps/desktop:
    // node --experimental-strip-types -e 'import("./src/lib/terms.ts").then(m=>process.stdout.write(m.termsMarkdown()))' > ../../docs/legal/terms.md
    expect(doc).toBe(termsMarkdown());
  });
});
