// The email that carries the licence key. Plain text: it is the most reliable way to deliver a
// string the person has to copy.

function esc(s) {
  return s.replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]);
}

export function buildEmail({ token, plaidUntil, boughtCore, support }) {
  const renewal = !boughtCore && plaidUntil;
  const subject = renewal ? "Your Golden bank-connection licence is renewed" : "Your Golden licence key";
  const lines = [
    renewal
      ? `Your Golden bank-connection add-on is paid through ${plaidUntil}. Paste this new licence key into Golden to keep it active:`
      : "Thank you for buying Golden. Here is your licence key:",
    "",
    token,
    "",
    "To activate: open Golden, go to the Licence screen, paste the key, and choose Activate.",
    "It works offline and covers every 1.x update. Keep this email; you can use the key again on a new computer.",
  ];
  if (!renewal && plaidUntil) {
    lines.push("", `The bank-connection add-on is included through ${plaidUntil}.`);
  }
  lines.push("", `Questions? Reply to this email or write to ${support}.`);
  const text = lines.join("\n");
  const html = `<pre style="font:14px/1.5 system-ui,sans-serif;white-space:pre-wrap;word-break:break-all">${esc(text)}</pre>`;
  return { subject, text, html };
}

export async function sendEmail({ apiKey, from, to, email, replyTo }, fetchFn = fetch) {
  const res = await fetchFn("https://api.resend.com/emails", {
    method: "POST",
    headers: { Authorization: `Bearer ${apiKey}`, "Content-Type": "application/json" },
    body: JSON.stringify({
      from,
      to: [to],
      reply_to: replyTo,
      subject: email.subject,
      text: email.text,
      html: email.html,
    }),
  });
  if (!res.ok) throw new Error(`email send failed (${res.status})`);
}
