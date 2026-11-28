// Golden's terms of use (end user licence agreement). This file is the one source of the text:
// the first-launch dialog and Settings > Terms render it, and docs/legal/terms.md is generated
// from it (a test fails if the two drift apart). Change TERMS_VERSION whenever the meaning
// changes, and everyone is asked to accept again on their next launch.
//
// DRAFT: a lawyer must review this text before release. Placeholders in [BRACKETS] are filled in then.

export const TERMS_VERSION = "2026-11-28";
export const TERMS_TITLE = "Golden Terms of Use";

export interface TermsSection {
  heading: string;
  paragraphs: string[];
}

/** The short summary shown above the full text. It does not replace the terms. */
export const TERMS_SUMMARY: string[] = [
  "Golden is for education and information only. It is not financial, investment, tax or legal advice.",
  "Tax figures, projections and AI chat answers can be wrong. Check anything important with a qualified professional.",
  "Your licence covers your own personal use on up to 2 computers.",
  "Your financial data stays on your computer. We never store, sell or use it.",
];

export const TERMS: TermsSection[] = [
  {
    heading: "1. Agreement",
    paragraphs: [
      "These terms are an agreement between you and High Turtle (\"we\", \"us\"), the maker of Golden. By installing or using Golden you agree to them. If you do not agree, do not use Golden.",
    ],
  },
  {
    heading: "2. Education and information only, not advice",
    paragraphs: [
      "Golden is a tool for education and information. Nothing in Golden is financial, investment, tax, legal, insurance or retirement advice, and we are not acting as your financial adviser, broker, tax preparer, accountant or lawyer. No fiduciary or professional relationship is created by your use of Golden.",
      "Golden does not recommend buying or selling any security or financial product. Before you make a financial, tax or legal decision, talk to a qualified professional who knows your situation.",
    ],
  },
  {
    heading: "3. Figures, projections and tax estimates",
    paragraphs: [
      "Projections, Monte Carlo results, retirement plans, budgets and cash flow forecasts are estimates built from assumptions you choose and from historical data. They are not predictions or guarantees of future results. Markets, inflation, laws and your own life can turn out very differently.",
      "Federal and state tax estimates are approximations. Tax rules are complex, change often and depend on facts Golden may not know. We do not guarantee that any figure is accurate, complete or current. Golden is not tax preparation software and must not be used to file a return.",
      "Imported statements, automatic categories, matching of duplicates and transfers, and text read from documents can contain mistakes. Check the numbers that matter to you.",
    ],
  },
  {
    heading: "4. AI features",
    paragraphs: [
      "Golden's chat, explanations and suggestions are produced by AI models. AI output can be wrong, incomplete or out of date, even when it sounds confident. Treat it as a starting point for your own thinking, not as advice, and check it before you rely on it.",
      "When you use a model on your own computer, your questions stay on your computer. If you choose to connect a model that runs elsewhere, what you send goes to that service under its own terms.",
    ],
  },
  {
    heading: "5. Your licence",
    paragraphs: [
      "When you buy Golden, we grant you a personal, non-exclusive, non-transferable licence to install and use Golden on up to 2 computers that you own or control, for your own personal and household use. The licence is not for business use or for managing other people's money for pay.",
      "You may not sell, rent, lend, share, sublicense or transfer your licence or licence key, or publish it. You may not copy Golden except as needed to install it and make a backup, and you may not modify it, or reverse engineer, decompile or disassemble it, except to the extent the law allows this despite this limit. You may not remove notices or get around the licence or trial checks.",
      "Golden is licensed, not sold. We and our licensors keep all rights not expressly granted to you.",
    ],
  },
  {
    heading: "6. Free trial",
    paragraphs: [
      "Golden includes a 14-day free trial that starts the first time you open it. When the trial ends, Golden becomes read-only until you buy a licence: you can still view and export your data, but not import or edit. One trial per person; do not reinstall or reset the app to get another.",
    ],
  },
  {
    heading: "7. Bank updates add-on (Plaid)",
    paragraphs: [
      "Automatic bank updates are an optional paid subscription on top of the Golden licence, billed monthly or yearly at the price shown when you subscribe. It renews automatically at the end of each period until you cancel. You can cancel at any time; it then stays active until the end of the period you paid for. We may change the subscription price for future periods after giving you notice.",
      "The add-on uses Plaid Inc. to connect to your bank. Plaid's own terms and privacy policy apply to your bank connection, and Plaid, not Golden, sees your bank sign-in. Your transactions pass through Golden's sync service on their way from Plaid to your computer; the service stores nothing and does not log them. Bank data can be delayed, incomplete or wrong, and a bank or Plaid may stop supporting a connection at any time. The add-on needs an internet connection, and connecting a bank turns off Private mode while it is linked.",
    ],
  },
  {
    heading: "8. Payment and refunds",
    paragraphs: [
      "Purchases and subscriptions are sold and processed by our reseller, Paddle.com, which is the merchant of record. Paddle's checkout terms apply to your purchase, including taxes, and refunds are handled under Paddle's refund policy. We never see or store your card details.",
    ],
  },
  {
    heading: "9. Your data and privacy",
    paragraphs: [
      "Golden is built to keep your financial life on your computer. Your statements, transactions, balances, household details, plans and documents are stored only on your computer, in an encrypted database. We do not collect, sell, share or use your financial data, and we do not use it to train AI models or for product development. The only exception is the optional bank updates add-on, which relays your bank transactions to your computer without keeping them (section 7).",
      "Private mode keeps everything local: nothing leaves your computer. Settings > Privacy lists every outside service Golden can reach, what it sends and when.",
      "Golden does not send usage analytics today. If we add anonymous usage or diagnostic data to improve Golden, it will be off until you turn it on, it will never contain financial data, and it will never be sent in Private mode. Feedback you choose to send (your rating, words and optional email, with the app version and operating system) may be used to improve Golden.",
      "You are responsible for keeping your computer secure and for keeping backups. Deleting Golden's data or losing your computer may lose your data for good, because we have no copy.",
    ],
  },
  {
    heading: "10. Third-party services",
    paragraphs: [
      "Golden can link to or work with services we do not control, such as Plaid, Paddle, AI model providers and model download hosts. Their terms and privacy policies apply to your use of them, and we are not responsible for them.",
    ],
  },
  {
    heading: "11. Updates",
    paragraphs: [
      "We may release updates that add, change or remove features. Your licence covers updates within the major version you bought. A future major version may be a separate purchase.",
    ],
  },
  {
    heading: "12. No warranty",
    paragraphs: [
      "Golden is provided \"as is\" and \"as available\", without warranties of any kind, express or implied, including warranties of merchantability, fitness for a particular purpose, accuracy and non-infringement. We do not promise that Golden will be error-free, uninterrupted or secure, or that its results will be accurate or meet your needs. Some places do not allow these exclusions, so some may not apply to you.",
    ],
  },
  {
    heading: "13. Limitation of liability",
    paragraphs: [
      "To the fullest extent the law allows, we will not be liable for any indirect, incidental, special, consequential or punitive damages, or for any loss of money, profits, investments, savings, data or tax benefits, or for penalties or interest, arising from your use of or reliance on Golden, even if we were told they were possible. Decisions you make with Golden are your own.",
      "Our total liability for any claim about Golden is limited to the amount you paid us for Golden in the 12 months before the claim. Some places do not allow these limits, so some may not apply to you.",
    ],
  },
  {
    heading: "14. Ending the licence",
    paragraphs: [
      "You can stop using Golden at any time by uninstalling it. Your licence ends automatically if you break these terms. When it ends you must stop using Golden. Sections 2, 3, 4, 9, 12, 13, 15 and 16 continue after it ends.",
    ],
  },
  {
    heading: "15. Governing law",
    paragraphs: [
      "Golden is sold only in the United States. These terms are governed by the laws of the State of [STATE], without regard to its conflict of law rules, and any dispute will be decided in the state or federal courts located in [STATE], unless the law where you live requires otherwise.",
    ],
  },
  {
    heading: "16. Changes to these terms",
    paragraphs: [
      "We may update these terms. When we make a change, Golden shows you the new terms the next time you open it, and you need to accept them to keep using it. If you do not accept, you can stop using Golden and export your data. The version and date you accepted are saved on your computer.",
    ],
  },
  {
    heading: "17. Contact",
    paragraphs: [
      "Questions about these terms can be sent with Settings > Feedback in the app, or through turtlebox.studio.",
    ],
  },
];

/** The whole text as Markdown, for docs/legal/terms.md and for copying. */
export function termsMarkdown(): string {
  const out = [
    `# ${TERMS_TITLE}`,
    "",
    `Version ${TERMS_VERSION}. Draft: a lawyer must review this text before release.`,
    "",
    "## Summary",
    "",
    ...TERMS_SUMMARY.map((s) => `- ${s}`),
    "",
  ];
  for (const s of TERMS) {
    out.push(`## ${s.heading}`, "");
    for (const p of s.paragraphs) out.push(p, "");
  }
  return out.join("\n");
}

/** Whether the person must (re)accept: never accepted, or accepted an older version. */
export function needsAcceptance(accepted: { version: string } | null | undefined): boolean {
  return !accepted || accepted.version !== TERMS_VERSION;
}
